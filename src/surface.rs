use crate::{
    controller::{Controller, Input},
    model::*,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Stage,
    Programmer,
    Library,
    Playbacks,
    Automation,
    Patch,
    Health,
}
impl Page {
    pub const ALL: [Self; 7] = [
        Self::Stage,
        Self::Programmer,
        Self::Library,
        Self::Playbacks,
        Self::Automation,
        Self::Patch,
        Self::Health,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Stage => "STAGE",
            Self::Programmer => "PROGRAMMER",
            Self::Library => "LIBRARY",
            Self::Playbacks => "PLAYBACKS",
            Self::Automation => "AUTOMATION",
            Self::Patch => "PATCH",
            Self::Health => "HEALTH",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.name().eq_ignore_ascii_case(s))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layer {
    Fixtures,
    Groups,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResultState {
    Ready,
    Pending(u64),
    Confirmed(u64),
    Rejected(String),
    Disconnected,
}
#[derive(Clone, Debug)]
pub struct ReleasePreview {
    pub revision: u64,
    pub targets: Vec<Target>,
    pub destination: Destination,
    pub after: Snapshot,
}

pub struct Surface {
    pub confirmed: Snapshot,
    pub selected: BTreeSet<FixtureId>,
    pub page: Page,
    pub layer: Layer,
    pub additive: bool,
    pub connected: bool,
    pub pending: Option<Request>,
    pub result: ResultState,
    pub preview: Option<ReleasePreview>,
    pub controller: Controller,
    pub menu: bool,
    pub notice: String,
    next_id: u64,
}
impl Surface {
    pub fn new(snapshot: Snapshot) -> Self {
        Self {
            confirmed: snapshot,
            selected: BTreeSet::new(),
            page: Page::Stage,
            layer: Layer::Fixtures,
            additive: false,
            connected: true,
            pending: None,
            result: ResultState::Ready,
            preview: None,
            controller: Controller::default(),
            menu: false,
            notice: "OFFLINE ONLY / no physical output".into(),
            next_id: 1,
        }
    }
    pub fn targets(&self, a: Attribute) -> Vec<Target> {
        self.selected.iter().map(|id| (*id, a)).collect()
    }
    pub fn select(&mut self, ids: &[FixtureId]) -> Result<(), String> {
        if ids
            .iter()
            .any(|id| !self.confirmed.fixtures.iter().any(|f| f.id == *id))
        {
            return Err("unknown fixture; selection retained".into());
        }
        if !self.additive {
            self.selected.clear();
        }
        for id in ids {
            if self.additive && self.selected.contains(id) {
                self.selected.remove(id);
            } else {
                self.selected.insert(*id);
            }
        }
        self.context();
        Ok(())
    }
    pub fn deselect(&mut self) {
        self.selected.clear();
        self.context();
    }
    pub fn context(&mut self) {
        self.preview = None;
        self.controller.context_changed();
    }
    pub fn queue(&mut self, command: Command) -> Result<Request, String> {
        if !self.connected {
            return Err("disconnected: edits disabled".into());
        }
        if self.pending.is_some() {
            return Err("busy: resolve pending request first".into());
        }
        let r = Request {
            id: self.next_id,
            epoch: self.confirmed.epoch,
            revision: self.confirmed.revision,
            command,
        };
        self.next_id += 1;
        self.notice = format!("Requested {:?}", r.command);
        self.pending = Some(r.clone());
        self.result = ResultState::Pending(r.id);
        self.preview = None;
        Ok(r)
    }
    pub fn acknowledge(&mut self, reply: Reply) {
        if !self.connected
            || !self
                .pending
                .as_ref()
                .is_some_and(|r| r.id == reply.id && r.epoch == reply.epoch)
        {
            return;
        }
        self.pending = None;
        match reply.result {
            Ok(s)
                if s.epoch == self.confirmed.epoch
                    && s.show == self.confirmed.show
                    && s.revision >= self.confirmed.revision =>
            {
                self.confirmed = s;
                self.result = ResultState::Confirmed(reply.id);
                self.notice = format!(
                    "Applied simulation revision {} / no physical output",
                    self.confirmed.revision
                );
            }
            Ok(_) => {
                self.result = ResultState::Rejected("invalid snapshot identity/revision".into())
            }
            Err(e) => self.result = ResultState::Rejected(e),
        }
        self.context();
    }
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.pending = None;
        self.preview = None;
        self.result = ResultState::Disconnected;
        self.notice = "Pending intent discarded / obtain fresh snapshot".into();
        self.controller.disconnected();
    }
    pub fn reconnect(&mut self, s: Snapshot) {
        self.confirmed = s;
        self.connected = true;
        self.pending = None;
        self.preview = None;
        self.result = ResultState::Ready;
        self.notice = "Fresh simulation snapshot / no queued replay".into();
        self.selected
            .retain(|id| self.confirmed.fixtures.iter().any(|f| f.id == *id));
        self.controller.disconnected();
    }
    pub fn release_preview(
        &mut self,
        authority: &impl Authority,
        a: Attribute,
        destination: Destination,
    ) -> Result<(), String> {
        if !self.connected || self.pending.is_some() {
            return Err("fresh idle connection required".into());
        }
        let targets = self.targets(a);
        let after = authority.preview_release(targets.clone(), destination)?;
        if after.epoch != self.confirmed.epoch
            || after.show != self.confirmed.show
            || after.revision != self.confirmed.revision + 1
        {
            return Err("authority changed; refresh before preview".into());
        }
        self.preview = Some(ReleasePreview {
            revision: self.confirmed.revision,
            targets,
            destination,
            after,
        });
        self.notice = "Review release: explicit 0 s SIMULATION transition. confirm / cancel".into();
        Ok(())
    }
    pub fn confirm_release(&mut self) -> Result<Command, String> {
        let p = self.preview.take().ok_or("no release preview")?;
        if p.revision != self.confirmed.revision {
            return Err("release preview stale".into());
        }
        Ok(Command::Release {
            targets: p.targets,
            destination: p.destination,
        })
    }
    pub fn rotary_label(index: usize) -> &'static str {
        [
            "INT",
            "RED",
            "GREEN",
            "BLUE",
            "PAN",
            "TILT",
            "ZOOM",
            "STROBE --",
            "PB1 LEVEL",
            "PB2 LEVEL",
            "FX RATE --",
            "FX SIZE --",
            "FADE --",
            "DELAY --",
            "PAGE --",
            "GRAND MASTER",
        ][index]
    }
    pub fn pad_labels(&self) -> [&'static str; 8] {
        if self.menu {
            [
                "RECORD CUE",
                "CLEAR HOLD",
                "RETURN PB",
                "AUTO VIEW",
                "BLACKOUT",
                "HEALTH",
                "CONFIRM",
                "CANCEL",
            ]
        } else {
            [
                "STAGE",
                "PROGRAMMER",
                "PLAYBACKS",
                "LIBRARY",
                "MULTI",
                "DESELECT",
                "ACTIONS",
                "BACK",
            ]
        }
    }
    pub fn input(
        &mut self,
        event: Input,
        authority: &impl Authority,
    ) -> Result<Option<Command>, String> {
        match event {
            Input::Key(slot) => {
                let ids = match self.layer {
                    Layer::Fixtures => self.confirmed.fixtures.get(slot).map(|f| vec![f.id]),
                    Layer::Groups => self.confirmed.groups.get(slot).map(|g| g.members.clone()),
                };
                if let Some(ids) = ids {
                    self.select(&ids)?;
                }
                Ok(None)
            }
            Input::Pad(i) => {
                if self.menu {
                    match i {
                        0 => {
                            let id = (1..=32)
                                .find(|id| !self.confirmed.cues.contains_key(id))
                                .ok_or("cue store full")?;
                            return Ok(Some(Command::Record {
                                id,
                                palette: false,
                                replace: false,
                            }));
                        }
                        1 => return Ok(Some(Command::ClearToHold)),
                        2 => self.release_preview(
                            authority,
                            Attribute::Intensity,
                            Destination::Playback,
                        )?,
                        3 => {
                            self.page = Page::Automation;
                            self.menu = false;
                            self.context();
                        }
                        4 => {
                            if self.confirmed.blackout {
                                self.notice =
                                    "Blackout latched: type blackout off to release deliberately"
                                        .into();
                            } else {
                                return Ok(Some(Command::Blackout(true)));
                            }
                        }
                        5 => {
                            self.page = Page::Health;
                            self.menu = false;
                            self.context();
                        }
                        6 => return self.confirm_release().map(Some),
                        _ => {
                            self.menu = false;
                            self.context();
                        }
                    }
                } else {
                    match i {
                        0 => self.page = Page::Stage,
                        1 => self.page = Page::Programmer,
                        2 => self.page = Page::Playbacks,
                        3 => self.page = Page::Library,
                        4 => self.additive = !self.additive,
                        5 => self.selected.clear(),
                        6 => self.menu = true,
                        _ => self.page = Page::Stage,
                    }
                    self.context();
                }
                Ok(None)
            }
            Input::Rotary(index, raw) => {
                if index >= 16 {
                    return Err("invalid rotary".into());
                }
                // Context menus never perform hidden continuous edits.
                if self.menu || self.preview.is_some() {
                    return Err("close action/release panel before turning controls".into());
                }
                let (current, range) = if index < 7 {
                    let a = Attribute::ALL[index];
                    let ts = self.targets(a);
                    if ts.is_empty() || ts.iter().any(|t| !self.confirmed.supported(*t)) {
                        return Err("attribute unavailable for complete selection".into());
                    }
                    let current = self.confirmed.resolved[&ts[0]].value;
                    if ts
                        .iter()
                        .any(|t| self.confirmed.resolved[t].value != current)
                    {
                        return Err("mixed values: use explicit set before rotary pickup".into());
                    }
                    (current, a.range())
                } else if matches!(index, 8 | 9) {
                    (
                        self.confirmed.playbacks[index - 8]
                            .as_ref()
                            .ok_or("playback inactive")?
                            .level,
                        (0, 1000),
                    )
                } else if index == 15 {
                    (self.confirmed.master, (0, 1000))
                } else {
                    return Err("control unavailable".into());
                };
                let Some(value) = self.controller.value(index, raw, current, range) else {
                    self.notice = format!("K{:02}: pickup waiting at current target", index + 1);
                    return Ok(None);
                };
                if value == current {
                    return Ok(None);
                }
                Ok(Some(if index < 7 {
                    Command::Set {
                        targets: self.targets(Attribute::ALL[index]),
                        value,
                    }
                } else if index == 15 {
                    Command::Master(value)
                } else {
                    Command::PlaybackLevel {
                        slot: index - 8,
                        value,
                    }
                }))
            }
        }
    }
}
