#[path = "actions.rs"]
pub mod actions;
use crate::{
    controller::{Controller, Input},
    model::*,
};
use actions::{Action, Draft, Modal, PadAction, typed_value};
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
    pub modal: Option<Modal>,
    pub selection_focus: usize,
    pub attribute_focus: usize,
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
            modal: None,
            selection_focus: 0,
            attribute_focus: 0,
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
    pub fn pad_action(&self, index: usize) -> PadAction {
        use PadAction::*;
        let pads = if self.modal.is_some() || self.preview.is_some() {
            [
                Navigate(-1),
                Navigate(1),
                Unavailable,
                Unavailable,
                Unavailable,
                Backspace,
                Confirm,
                Cancel,
            ]
        } else if self.menu {
            [
                Record,
                ClearHold,
                ReleasePlayback,
                Edit,
                Blackout,
                Page(crate::surface::Page::Health),
                Confirm,
                Back,
            ]
        } else {
            [
                Page(crate::surface::Page::Stage),
                Page(crate::surface::Page::Programmer),
                Page(crate::surface::Page::Playbacks),
                Page(crate::surface::Page::Library),
                Multi,
                Deselect,
                Menu,
                Back,
            ]
        };
        pads.get(index).copied().unwrap_or(Unavailable)
    }
    pub fn context(&mut self) {
        self.menu = false;
        self.preview = None;
        self.modal = None;
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
        self.modal = None;
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
        self.context();
        self.menu = false;
        self.connected = false;
        self.pending = None;
        self.preview = None;
        self.result = ResultState::Disconnected;
        self.notice = "Pending intent discarded / obtain fresh snapshot".into();
        self.controller.disconnected();
    }
    pub fn reconnect(&mut self, s: Snapshot) {
        self.context();
        self.menu = false;
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
        self.modal = None;
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
    /// Both keyboard and synthetic controller operations enter here.
    pub fn action(&mut self, action: Action) -> Result<Option<Command>, String> {
        match action {
            Action::Page(page) => {
                self.page = page;
                self.menu = false;
                self.context();
            }
            Action::Select(ids) => self.select(&ids)?,
            Action::Deselect => self.deselect(),
            Action::Layer(layer) => {
                self.layer = layer;
                self.selection_focus = 0;
                self.context();
            }
            Action::Multi(value) => {
                self.additive = value;
                self.context();
            }
            Action::Attribute(delta) => {
                self.attribute_focus =
                    (self.attribute_focus as i64 + i64::from(delta)).rem_euclid(7) as usize;
                self.context();
            }
            Action::Navigate(delta) => {
                self.controller.context_changed();
                if let Some(Modal {
                    draft: Draft::Record { text, .. },
                    ..
                }) = &mut self.modal
                {
                    let id = text.parse::<i32>().unwrap_or(1);
                    *text = id.saturating_add(delta).clamp(1, 32).to_string();
                } else if let Some(Modal {
                    draft: Draft::Attribute { attribute, text },
                    ..
                }) = &mut self.modal
                {
                    let (low, high) = attribute.range();
                    let value = typed_value(text)
                        .unwrap_or(low)
                        .saturating_add(delta)
                        .clamp(low, high);
                    *text = format!(
                        "{}{}.{:01}",
                        if value < 0 { "-" } else { "" },
                        value.abs() / 10,
                        value.abs() % 10
                    );
                } else if self.modal.is_some() || self.preview.is_some() {
                    return Err("confirmation has no navigation field".into());
                } else {
                    let count = match self.layer {
                        Layer::Fixtures => self.confirmed.fixtures.len(),
                        Layer::Groups => self.confirmed.groups.len(),
                    };
                    if count > 0 {
                        self.selection_focus = (self.selection_focus as i64 + i64::from(delta))
                            .rem_euclid(count as i64)
                            as usize;
                    }
                    self.context();
                }
            }
            Action::Edit(attribute) => {
                self.idle()?;
                let targets = self.targets(attribute);
                if targets.is_empty() || targets.iter().any(|t| !self.confirmed.supported(*t)) {
                    return Err("attribute unavailable for complete selection".into());
                }
                self.begin(Draft::Attribute {
                    attribute,
                    text: String::new(),
                });
            }
            Action::Record {
                palette,
                replace,
                id,
            } => {
                self.idle()?;
                self.begin(Draft::Record {
                    palette,
                    replace,
                    text: id.to_string(),
                });
            }
            Action::Text(value) => {
                self.controller.context_changed();
                if value.len() > 16 {
                    return Err("draft text too long".into());
                }
                match &mut self.modal.as_mut().ok_or("no typed editor")?.draft {
                    Draft::Attribute { text, .. } | Draft::Record { text, .. } => *text = value,
                    Draft::BlackoutOff => {
                        return Err("blackout confirmation has no text field".into());
                    }
                }
            }
            Action::Backspace => match &mut self.modal.as_mut().ok_or("no typed editor")?.draft {
                Draft::Attribute { text, .. } | Draft::Record { text, .. } => {
                    text.pop();
                    self.controller.context_changed();
                }
                Draft::BlackoutOff => return Err("blackout confirmation has no text field".into()),
            },
            Action::ClearHold => {
                self.idle()?;
                return Ok(Some(Command::ClearToHold));
            }
            Action::Blackout(true) => {
                self.idle()?;
                return Ok(Some(Command::Blackout(true)));
            }
            Action::Blackout(false) => {
                self.idle()?;
                if !self.confirmed.blackout {
                    return Err("blackout already off".into());
                }
                self.begin(Draft::BlackoutOff);
            }
            Action::Confirm => {
                self.idle()?;
                if let Some(modal) = &self.modal {
                    if modal.revision != self.confirmed.revision
                        || modal.epoch != self.confirmed.epoch
                        || modal.show != self.confirmed.show
                        || modal.selection != self.selected.iter().copied().collect::<Vec<_>>()
                    {
                        self.context();
                        return Err("draft stale; reopen editor".into());
                    }
                    let command = match &modal.draft {
                        Draft::Attribute { attribute, text } => {
                            let value = typed_value(text)?;
                            let (low, high) = attribute.range();
                            let targets = self.targets(*attribute);
                            if value < low || value > high {
                                return Err("value outside attribute range".into());
                            }
                            if targets.is_empty()
                                || targets.iter().any(|t| !self.confirmed.supported(*t))
                            {
                                return Err("attribute unavailable for complete selection".into());
                            }
                            Command::Set { targets, value }
                        }
                        Draft::Record {
                            palette,
                            replace,
                            text,
                        } => {
                            let id: u16 = text
                                .parse()
                                .map_err(|_| "record destination must be 1..32")?;
                            if !(1..=32).contains(&id) {
                                return Err("record destination must be 1..32".into());
                            }
                            let store = if *palette {
                                &self.confirmed.palettes
                            } else {
                                &self.confirmed.cues
                            };
                            if store.contains_key(&id) != *replace {
                                return Err("destination exists/missing; choose record or update explicitly".into());
                            }
                            Command::Record {
                                id,
                                palette: *palette,
                                replace: *replace,
                            }
                        }
                        Draft::BlackoutOff => Command::Blackout(false),
                    };
                    self.modal = None;
                    self.menu = false;
                    return Ok(Some(command));
                }
                return self.confirm_release().map(Some);
            }
            Action::Cancel | Action::Back => {
                let had_panel = self.modal.is_some() || self.preview.is_some() || self.menu;
                self.context();
                self.menu = false;
                if !had_panel {
                    self.page = Page::Stage;
                }
                self.notice = "Cancelled / applied simulation state retained".into();
            }
            Action::ContextLost => {
                self.context();
                self.menu = false;
                self.controller.disconnected();
                self.notice =
                    "Input context lost / drafts discarded / release controls to rearm".into();
            }
        }
        self.describe_draft();
        Ok(None)
    }
    fn idle(&self) -> Result<(), String> {
        if !self.connected {
            Err("disconnected: edits disabled".into())
        } else if self.pending.is_some() {
            Err("busy: resolve pending request first".into())
        } else {
            Ok(())
        }
    }
    fn begin(&mut self, draft: Draft) {
        self.context();
        self.modal = Some(Modal {
            revision: self.confirmed.revision,
            epoch: self.confirmed.epoch,
            draft,
            selection: self.selected.iter().copied().collect(),
            show: self.confirmed.show.clone(),
        });
        self.menu = true;
    }
    fn describe_draft(&mut self) {
        if let Some(modal) = &self.modal {
            self.notice = format!(
                "SIMULATOR draft {:?} / confirm, cancel or back / physical UNKNOWN",
                modal.draft
            );
        }
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
        if self.modal.is_some() || self.preview.is_some() {
            [
                "PREVIOUS",
                "NEXT",
                "--",
                "--",
                "--",
                "BACKSPACE",
                "CONFIRM",
                "CANCEL",
            ]
        } else if self.menu {
            [
                "RECORD CUE",
                "CLEAR HOLD",
                "RETURN PB",
                "EDIT ATTR",
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
            Input::Action(action) => self.action(action),
            Input::Key(slot) => {
                if self.modal.is_some() {
                    if slot > 9 {
                        return Err("editor keys are digits 0..9".into());
                    }
                    let text = match &self.modal.as_ref().unwrap().draft {
                        Draft::Attribute { text, .. } | Draft::Record { text, .. } => {
                            format!("{text}{slot}")
                        }
                        Draft::BlackoutOff => {
                            return Err("confirm or cancel blackout reveal".into());
                        }
                    };
                    return self.action(Action::Text(text));
                }
                let ids = match self.layer {
                    Layer::Fixtures => self.confirmed.fixtures.get(slot).map(|f| vec![f.id]),
                    Layer::Groups => self.confirmed.groups.get(slot).map(|g| g.members.clone()),
                };
                if let Some(ids) = ids {
                    self.selection_focus = slot;
                    return self.action(Action::Select(ids));
                }
                Ok(None)
            }
            Input::Pad(i) => {
                if i >= 8 {
                    return Err("invalid pad".into());
                }
                match self.pad_action(i) {
                    PadAction::Page(page) => self.action(Action::Page(page)),
                    PadAction::Multi => self.action(Action::Multi(!self.additive)),
                    PadAction::Deselect => self.action(Action::Deselect),
                    PadAction::Menu => {
                        self.context();
                        self.menu = true;
                        Ok(None)
                    }
                    PadAction::Back => self.action(Action::Back),
                    PadAction::Navigate(delta) => self.action(Action::Navigate(delta)),
                    PadAction::Backspace => self.action(Action::Backspace),
                    PadAction::Confirm => self.action(Action::Confirm),
                    PadAction::Cancel => self.action(Action::Cancel),
                    PadAction::Record => self.action(Action::Record {
                        id: 1,
                        palette: false,
                        replace: false,
                    }),
                    PadAction::ClearHold => self.action(Action::ClearHold),
                    PadAction::ReleasePlayback => {
                        self.release_preview(
                            authority,
                            Attribute::ALL[self.attribute_focus],
                            Destination::Playback,
                        )?;
                        Ok(None)
                    }
                    PadAction::Edit => {
                        self.action(Action::Edit(Attribute::ALL[self.attribute_focus]))
                    }
                    PadAction::Blackout => self.action(Action::Blackout(!self.confirmed.blackout)),
                    PadAction::Unavailable => {
                        Err("modal panel: use confirm/cancel or destination navigation".into())
                    }
                }
            }
            Input::Rotary(index, raw) => {
                if index >= 16 {
                    return Err("invalid rotary".into());
                }
                if let Some(modal) = &self.modal {
                    if index != 0 {
                        return Err("draft uses rotary 1 only".into());
                    }
                    let (current, range, attribute) = match &modal.draft {
                        Draft::Attribute { attribute, text } => (
                            if text.is_empty() {
                                0.clamp(attribute.range().0, attribute.range().1)
                            } else {
                                typed_value(text)?
                            },
                            attribute.range(),
                            true,
                        ),
                        Draft::Record { text, .. } => (
                            text.parse::<i32>()
                                .map_err(|_| "invalid record destination")?,
                            (1, 32),
                            false,
                        ),
                        Draft::BlackoutOff => {
                            return Err("confirm or cancel blackout reveal".into());
                        }
                    };
                    if !(range.0..=range.1).contains(&current) {
                        return Err("draft outside allowed range".into());
                    }
                    if let Some(value) = self.controller.value(index, raw, current, range) {
                        let text = if attribute {
                            format!(
                                "{}{}.{:01}",
                                if value < 0 { "-" } else { "" },
                                value.abs() / 10,
                                value.abs() % 10
                            )
                        } else {
                            value.to_string()
                        };
                        let pickup = self.controller.pickup;
                        let result = self.action(Action::Text(text));
                        self.controller.pickup = pickup;
                        return result;
                    }
                    return Ok(None);
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
                if index < 7 {
                    self.action(Action::Edit(Attribute::ALL[index]))?;
                    self.action(Action::Text(format!(
                        "{}{}.{:01}",
                        if value < 0 { "-" } else { "" },
                        value.abs() / 10,
                        value.abs() % 10
                    )))?;
                    return self.action(Action::Confirm);
                }
                Ok(Some(if index == 15 {
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
