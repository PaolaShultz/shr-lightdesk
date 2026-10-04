//! Explicitly synthetic lighting authority. No clock, fixture encoding, effects,
//! DMX, physical output, persistent show, or SHR Lux algorithms live here.
use crate::model::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub struct Simulator {
    state: Snapshot,
    replies: VecDeque<(Request, Reply)>,
    high_water: u64,
    activation: u64,
}
impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}
impl Simulator {
    pub fn new() -> Self {
        let fixtures = [
            (1, "FRONT L", "DIM", (170, 430)),
            (2, "FRONT R", "DIM", (790, 430)),
            (11, "REAR L", "RGB", (120, 120)),
            (12, "REAR MID L", "RGB", (360, 120)),
            (13, "REAR MID R", "RGB", (600, 120)),
            (14, "REAR R", "RGB", (840, 120)),
            (21, "MOVE L", "MOVER", (120, 280)),
            (22, "MOVE R", "MOVER", (840, 280)),
        ]
        .into_iter()
        .map(|(id, name, kind, position)| Fixture {
            id,
            name: name.into(),
            kind,
            position,
            attributes: Attribute::ALL[..match kind {
                "DIM" => 1,
                "RGB" => 4,
                _ => 7,
            }]
                .to_vec(),
        })
        .collect();
        let groups = [
            (1, "FRONT", vec![1, 2]),
            (2, "REAR", vec![11, 12, 13, 14]),
            (3, "MOVERS", vec![21, 22]),
        ]
        .into_iter()
        .map(|(id, name, members)| Group {
            id,
            name: name.into(),
            members,
        })
        .collect();
        let state = Snapshot {
            show: "SYNTHETIC BAND / UNSAVED".into(),
            epoch: 1,
            revision: 0,
            fixtures,
            groups,
            mode: Mode::Manual,
            programmer: BTreeMap::new(),
            holds: BTreeMap::new(),
            automation: BTreeMap::new(),
            proposals: BTreeMap::new(),
            grants: BTreeMap::new(),
            playback_only: BTreeSet::new(),
            cues: BTreeMap::new(),
            palettes: BTreeMap::new(),
            playbacks: [None, None],
            master: 1000,
            blackout: false,
            resolved: BTreeMap::new(),
        };
        let mut s = Self {
            state,
            replies: VecDeque::new(),
            high_water: 0,
            activation: 0,
        };
        s.resolve();
        s
    }
    fn targets(&self, targets: &[Target]) -> Result<(), String> {
        if targets.is_empty() || targets.len() > 64 {
            return Err("empty or oversized target scope".into());
        }
        let mut seen = BTreeSet::new();
        for t in targets {
            if !self.state.supported(*t) || !seen.insert(*t) {
                return Err("unsupported or duplicate target; nothing changed".into());
            }
        }
        Ok(())
    }
    fn valid_value(t: Target, v: i32) -> bool {
        let (l, h) = t.1.range();
        (l..=h).contains(&v)
    }
    fn apply(&mut self, c: &Command) -> Result<(), String> {
        match c {
            Command::Set { targets, value } => {
                self.targets(targets)?;
                if targets.iter().any(|t| !Self::valid_value(*t, *value)) {
                    return Err("value outside capability bounds".into());
                }
                for t in targets {
                    self.state.programmer.insert(*t, *value);
                    self.state.holds.remove(t);
                }
            }
            Command::ClearToHold => {
                for (t, v) in std::mem::take(&mut self.state.programmer) {
                    self.state.holds.insert(t, v);
                }
            }
            Command::Release {
                targets,
                destination,
            } => {
                self.targets(targets)?;
                if *destination == Destination::Automation
                    && (self.state.mode != Mode::Auto
                        || targets.iter().any(|t| {
                            !self.state.grants.contains_key(t)
                                || !self.state.automation.contains_key(t)
                        }))
                {
                    return Err("AUTO and a bounded applied automation target are required".into());
                }
                for t in targets {
                    self.state.programmer.remove(t);
                    self.state.holds.remove(t);
                    if *destination == Destination::Playback {
                        self.state.playback_only.insert(*t);
                    } else {
                        self.state.playback_only.remove(t);
                    }
                }
            }
            Command::Mode(mode) => {
                if *mode != self.state.mode {
                    for (t, r) in &self.state.resolved {
                        if !self.state.programmer.contains_key(t) {
                            self.state.holds.insert(*t, r.value);
                        }
                    }
                    self.state.mode = *mode;
                }
            }
            Command::Master(value) => {
                if !(0..=1000).contains(value) {
                    return Err("master must be 0..100%".into());
                }
                self.state.master = *value;
            }
            Command::Blackout(value) => self.state.blackout = *value,
            Command::Record {
                id,
                palette,
                replace,
            } => {
                if *id == 0 || *id > 32 {
                    return Err("storage ID must be 1..32".into());
                }
                if self.state.programmer.is_empty() {
                    return Err("programmer is empty; holds are not recordable".into());
                }
                let store = if *palette {
                    &mut self.state.palettes
                } else {
                    &mut self.state.cues
                };
                if store.contains_key(id) != *replace {
                    return Err(
                        "use explicit update for an existing item, record for a new one".into(),
                    );
                }
                store.insert(
                    *id,
                    Stored {
                        id: *id,
                        name: format!("{} {id:02}", if *palette { "PALETTE" } else { "CUE" }),
                        values: self.state.programmer.clone(),
                    },
                );
            }
            Command::RecallPalette { id, fixtures } => {
                let p = self.state.palettes.get(id).ok_or("palette missing")?;
                if fixtures.is_empty()
                    || fixtures
                        .iter()
                        .any(|id| !self.state.fixtures.iter().any(|f| f.id == *id))
                {
                    return Err("invalid palette selection".into());
                }
                let values: Look = p
                    .values
                    .iter()
                    .filter(|(t, _)| fixtures.contains(&t.0))
                    .map(|(t, v)| (*t, *v))
                    .collect();
                if values.is_empty() {
                    return Err("palette has no data for selection".into());
                }
                for (t, v) in values {
                    self.state.programmer.insert(t, v);
                    self.state.holds.remove(&t);
                }
            }
            Command::Go { cue, slot } => {
                if *slot >= 2 {
                    return Err("playback slot must be 1 or 2".into());
                }
                let cue = self.state.cues.get(cue).ok_or("cue missing")?;
                self.activation += 1;
                self.state.playbacks[*slot] = Some(Playback {
                    cue: cue.id,
                    values: cue.values.clone(),
                    order: self.activation,
                    level: 1000,
                });
            }
            Command::Off { slot } => {
                if *slot >= 2 {
                    return Err("playback slot must be 1 or 2".into());
                }
                self.state.playbacks[*slot] = None;
            }
            Command::PlaybackLevel { slot, value } => {
                if *slot >= 2 || !(0..=1000).contains(value) {
                    return Err("invalid playback level".into());
                }
                self.state.playbacks[*slot]
                    .as_mut()
                    .ok_or("playback inactive")?
                    .level = *value;
            }
            Command::Grant { targets, low, high } => {
                self.targets(targets)?;
                if low > high
                    || targets
                        .iter()
                        .any(|t| !Self::valid_value(*t, *low) || !Self::valid_value(*t, *high))
                {
                    return Err("invalid grant bounds".into());
                }
                for t in targets {
                    // Changing permission/bounds never changes the look by itself.
                    if !self.state.programmer.contains_key(t) {
                        self.state.holds.insert(*t, self.state.resolved[t].value);
                    }
                    self.state.grants.insert(*t, (*low, *high));
                    if let Some(v) = self.state.automation.get_mut(t) {
                        *v = (*v).clamp(*low, *high);
                    }
                }
            }
            Command::Propose { target, value } => {
                self.targets(&[*target])?;
                if self.state.mode == Mode::Manual {
                    return Err("MANUAL: automation proposals disabled".into());
                }
                if !Self::valid_value(*target, *value) {
                    return Err("proposal outside fixture bounds".into());
                }
                self.state.proposals.insert(*target, *value);
                if self.state.mode == Mode::Auto
                    && let Some((l, h)) = self.state.grants.get(target)
                {
                    self.state
                        .automation
                        .insert(*target, (*value).clamp(*l, *h));
                }
            }
            Command::Accept { targets } => {
                self.targets(targets)?;
                if self.state.mode != Mode::Assist
                    || targets
                        .iter()
                        .any(|t| !self.state.proposals.contains_key(t))
                {
                    return Err("ASSIST and a current proposal required".into());
                }
                for t in targets {
                    self.state.programmer.insert(*t, self.state.proposals[t]);
                    self.state.holds.remove(t);
                }
            }
        }
        self.state.revision += 1;
        self.resolve();
        Ok(())
    }
    fn resolve(&mut self) {
        self.state.resolved.clear();
        for t in self.state.targets() {
            let mut value = t.1.default_value();
            let mut source = Source::Default;
            let mut newest = 0;
            for (slot, p) in self.state.playbacks.iter().enumerate() {
                if let Some(p) = p
                    && let Some(v) = p.values.get(&t)
                {
                    let candidate = if t.1 == Attribute::Intensity {
                        *v * p.level / 1000
                    } else {
                        *v
                    };
                    if (t.1 == Attribute::Intensity
                        && (source == Source::Default
                            || candidate > value
                            || candidate == value && p.order > newest))
                        || (t.1 != Attribute::Intensity && p.order > newest)
                    {
                        value = candidate;
                        source = Source::Playback { slot, cue: p.cue };
                        newest = p.order;
                    }
                }
            }
            if self.state.mode == Mode::Auto
                && !self.state.playback_only.contains(&t)
                && let Some(v) = self.state.automation.get(&t)
            {
                value = *v;
                source = Source::Automation;
            }
            if let Some(v) = self.state.holds.get(&t) {
                value = *v;
                source = Source::Hold;
            }
            if let Some(v) = self.state.programmer.get(&t) {
                value = *v;
                source = Source::Programmer;
            }
            let final_value = if t.1 == Attribute::Intensity {
                if self.state.blackout {
                    0
                } else {
                    value * self.state.master / 1000
                }
            } else {
                value
            };
            self.state.resolved.insert(
                t,
                Resolved {
                    value,
                    source,
                    final_value,
                },
            );
        }
    }
    /// Calculate the same zero-time synthetic release without changing authority.
    pub fn preview(
        state: &Snapshot,
        targets: Vec<Target>,
        destination: Destination,
    ) -> Result<Snapshot, String> {
        let mut sim = Self {
            state: state.clone(),
            replies: VecDeque::new(),
            high_water: 0,
            activation: state.revision,
        };
        sim.apply(&Command::Release {
            targets,
            destination,
        })?;
        Ok(sim.state)
    }
}
impl Authority for Simulator {
    fn preview_release(
        &self,
        targets: Vec<Target>,
        destination: Destination,
    ) -> Result<Snapshot, String> {
        Self::preview(&self.state, targets, destination)
    }
    fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    fn request(&mut self, r: &Request) -> Reply {
        let error = |s: &str| Reply {
            id: r.id,
            epoch: r.epoch,
            result: Err(s.into()),
        };
        if r.epoch != self.state.epoch {
            return error("wrong epoch");
        }
        if let Some((old, reply)) = self.replies.iter().find(|(old, _)| old.id == r.id) {
            return if old == r {
                reply.clone()
            } else {
                error("request ID reused with different content")
            };
        }
        if r.id <= self.high_water {
            return error("expired request ID");
        }
        self.high_water = r.id;
        let result = if r.revision != self.state.revision {
            Err("revision conflict; refresh and review again".into())
        } else {
            self.apply(&r.command).map(|()| self.state.clone())
        };
        let reply = Reply {
            id: r.id,
            epoch: r.epoch,
            result,
        };
        if self.replies.len() == 64 {
            self.replies.pop_front();
        }
        self.replies.push_back((r.clone(), reply.clone()));
        reply
    }
}
