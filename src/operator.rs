//! Headless, bounded lighting operator seams. No native/device bindings or authority logic.
use std::collections::VecDeque;

use crate::{
    model::{Authority, Reply, Request, Snapshot},
    render::{self, Scene},
    surface::{
        ResultState, Surface,
        actions::{Action, PadAction},
    },
};

pub const INPUT_CAPACITY: usize = 64;
pub const PUMP_BUDGET: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeskRole {
    Audio,
    Lighting,
}
/// An observation supplied by the role owner, not a local ownership claim.
/// Physical identity/profile checks and process-held locks remain provider work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub generation: u64,
    pub endpoint: String,
    pub role: DeskRole,
}
#[derive(Clone, Debug)]
pub enum OperatorInput {
    /// Already interpreted text/menu intent; native key edges use KeyDown/KeyUp.
    Keyboard(Action),
    KeyDown {
        key: u8,
        action: Action,
    },
    KeyUp {
        key: u8,
    },
    Midi {
        generation: u64,
        endpoint: String,
        bytes: [u8; 3],
    },
    Hit {
        layout: u64,
        x: u32,
        y: u32,
    },
}
#[derive(Clone, Debug)]
pub enum Event {
    Assignment(Assignment),
    DeviceLost { generation: u64 },
    Focus(bool),
    Resize { width: u32, height: u32 },
    RendererLost,
    RendererRestored { layout: u64 },
    Reconnect(Snapshot),
    AuthorityLost,
    Reply(Reply),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedState {
    Off,
    Available,
    Selected,
    Hold,
    Pending,
    Confirmed,
    Fault,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedFrame {
    pub generation: u64,
    pub endpoint: String,
    pub pads: [LedState; 8],
    pub revision: u64,
    pub result: ResultState,
}
#[derive(Default, Debug)]
pub struct PumpResult {
    pub processed: usize,
    pub refused: usize,
    /// Requests are returned to an independent authority worker; never auto-ACKed.
    pub requests: Vec<Request>,
}
struct Queued {
    recovery: u64,
    modal_context: bool,
    invalidated_modal: bool,
    input: OperatorInput,
}

pub struct Operator {
    pub surface: Surface,
    assignment: Option<Assignment>,
    generation: u64,
    input: VecDeque<Queued>,
    recovery: u64,
    focused: bool,
    renderer: bool,
    width: u32,
    height: u32,
    layout: u64,
    led: Option<LedFrame>,
    last_led_ms: Option<u64>,
    delivered_led: Option<LedFrame>,
    held_keys: [bool; 128],
}
impl Operator {
    pub fn new(surface: Surface) -> Self {
        Self {
            surface,
            assignment: None,
            generation: 0,
            input: VecDeque::new(),
            recovery: 0,
            focused: true,
            renderer: true,
            width: render::WIDTH,
            height: render::HEIGHT,
            layout: 1,
            led: None,
            last_led_ms: None,
            delivered_led: None,
            held_keys: [false; 128],
        }
    }
    pub fn queued(&self) -> usize {
        self.input.len()
    }
    pub fn layout(&self) -> u64 {
        self.layout
    }
    pub fn assignment(&self) -> Option<&Assignment> {
        self.assignment.as_ref()
    }
    fn lighting(&self, generation: u64, endpoint: &str) -> bool {
        self.assignment.as_ref().is_some_and(|a| {
            a.role == DeskRole::Lighting && a.generation == generation && a.endpoint == endpoint
        })
    }
    fn recover(&mut self) -> Result<(), String> {
        self.input.clear();
        self.held_keys = [true; 128];
        self.surface.action(Action::ContextLost)?;
        self.recovery = self
            .recovery
            .checked_add(1)
            .ok_or("input generation exhausted")?;
        Ok(())
    }
    fn recover_focus(&mut self) {
        // MIDI belongs to its assigned role regardless of desktop focus.
        self.input
            .retain(|q| matches!(q.input, OperatorInput::Midi { .. }));
        for queued in &mut self.input {
            queued.invalidated_modal |= queued.modal_context;
        }
        self.held_keys = [true; 128];
        let menu = self.surface.menu;
        self.surface.context();
        self.surface.menu = menu;
        self.surface.notice = "Keyboard context lost / drafts discarded".into();
    }
    /// Lifecycle/authority events bypass input saturation. They never enter an audio queue.
    pub fn event(&mut self, event: Event) -> Result<(), String> {
        match event {
            Event::Assignment(a) => {
                if self.generation == u64::MAX || a.generation == u64::MAX {
                    self.generation = u64::MAX;
                    self.assignment = None;
                    self.led = None;
                    self.delivered_led = None;
                    self.recover()?;
                    return Err("role generation exhausted".into());
                }
                if a.generation == 0
                    || a.endpoint.is_empty()
                    || a.endpoint.len() > 64
                    || !a
                        .endpoint
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                {
                    return Err("invalid synthetic role observation".into());
                }
                if a.generation <= self.generation {
                    return if self.assignment.as_ref() == Some(&a) {
                        Ok(())
                    } else {
                        Err("stale/conflicting role generation".into())
                    };
                }
                self.recover()?;
                self.generation = a.generation;
                self.surface.controller.owned_identity = a.endpoint.clone();
                self.assignment = Some(a);
                self.led = None;
                self.last_led_ms = None;
                self.delivered_led = None;
            }
            Event::DeviceLost { generation } => {
                if generation != self.generation || self.assignment.is_none() {
                    return Err("stale device loss".into());
                }
                self.recover()?;
                self.assignment = None;
                self.led = None;
            }
            Event::Focus(focused) => {
                if self.focused != focused {
                    self.recover_focus();
                }
                self.focused = focused;
            }
            Event::Resize { width, height } => {
                self.layout = self
                    .layout
                    .checked_add(1)
                    .ok_or("layout generation exhausted")?;
                self.width = width;
                self.height = height;
            }
            Event::RendererLost => {
                self.layout = self
                    .layout
                    .checked_add(1)
                    .ok_or("layout generation exhausted")?;
                self.renderer = false;
            }
            Event::RendererRestored { layout } => {
                if layout != self.layout {
                    return Err("stale renderer restore".into());
                }
                self.renderer = true;
            }
            Event::Reconnect(snapshot) => {
                self.recover()?;
                self.surface.reconnect(snapshot);
                // A reconnect must receive a new generation from the role owner.
                self.assignment = None;
                self.led = None;
            }
            Event::AuthorityLost => {
                self.recover()?;
                self.surface.disconnect();
            }
            Event::Reply(reply) => self.surface.acknowledge(reply),
        }
        self.update_led();
        Ok(())
    }
    pub fn enqueue(&mut self, input: OperatorInput) -> Result<(), String> {
        if matches!(&input, OperatorInput::KeyDown { key, .. } if *key > 127) {
            return Err("invalid key".into());
        }
        if self.recovery == u64::MAX {
            return Err("input generation exhausted".into());
        }
        match &input {
            OperatorInput::Keyboard(action) | OperatorInput::KeyDown { action, .. } => {
                if !self.focused {
                    return Err("keyboard unfocused".into());
                }
                if matches!(action, Action::Text(s) if s.len() > 16)
                    || matches!(action, Action::Select(ids) if ids.len() > 32)
                {
                    return Err("input payload capacity".into());
                }
            }
            OperatorInput::KeyUp { key } => {
                if *key > 127 {
                    return Err("invalid key".into());
                }
            }
            OperatorInput::Midi {
                generation,
                endpoint,
                ..
            } => {
                if !self.lighting(*generation, endpoint) {
                    return Err("lighting role unavailable/stale".into());
                }
            }
            OperatorInput::Hit { layout, .. } => {
                if !self.focused || !self.renderer || *layout != self.layout {
                    return Err("pointer context unavailable/stale".into());
                }
            }
        }
        if self.input.len() == INPUT_CAPACITY {
            self.recover()?;
            self.surface.notice =
                "Input overflow: intent discarded; release controls and reacquire pickup".into();
            self.update_led();
            return Err("input overflow; event refused".into());
        }
        self.input.push_back(Queued {
            recovery: self.recovery,
            modal_context: self.surface.modal.is_some() || self.surface.preview.is_some(),
            invalidated_modal: false,
            input,
        });
        Ok(())
    }
    pub fn pump(&mut self, authority: &impl Authority) -> PumpResult {
        let mut result = PumpResult::default();
        for _ in 0..PUMP_BUDGET {
            let Some(q) = self.input.pop_front() else {
                break;
            };
            result.processed += 1;
            let action = if q.recovery != self.recovery {
                Err("stale input recovery generation".into())
            } else {
                match q.input {
                    OperatorInput::Keyboard(action) if self.focused => self.surface.action(action),
                    OperatorInput::KeyUp { key } => {
                        self.held_keys[usize::from(key)] = false;
                        Ok(None)
                    }
                    OperatorInput::KeyDown { key, action } if self.focused => {
                        let held = &mut self.held_keys[usize::from(key)];
                        if *held {
                            Ok(None)
                        } else {
                            *held = true;
                            self.surface.action(action)
                        }
                    }
                    OperatorInput::Midi {
                        generation,
                        endpoint,
                        bytes,
                    } if self.lighting(generation, &endpoint) => {
                        match self.surface.controller.decode(&endpoint, &bytes) {
                            Some(_) if q.invalidated_modal => {
                                Err("stale controller modal intent after keyboard focus change"
                                    .into())
                            }
                            Some(input) => self.surface.input(input, authority),
                            None => Ok(None),
                        }
                    }
                    OperatorInput::Hit { layout, x, y }
                        if self.focused && self.renderer && layout == self.layout =>
                    {
                        match self
                            .logical_point(x, y)
                            .and_then(|(x, y)| render::hit_fixture(&self.surface, x, y))
                        {
                            Some(id) => self.surface.action(Action::Select(vec![id])),
                            None => Ok(None),
                        }
                    }
                    _ => Err("stale input context".into()),
                }
            };
            let request =
                action.and_then(|command| command.map(|c| self.surface.queue(c)).transpose());
            match request {
                Ok(Some(request)) => result.requests.push(request),
                Ok(None) => {}
                Err(reason) => {
                    result.refused += 1;
                    self.surface.notice = reason;
                }
            }
        }
        self.update_led();
        result
    }
    fn logical_point(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        if self.width == 0 || self.height == 0 || x >= self.width || y >= self.height {
            return None;
        }
        let (w, h) = if u64::from(self.width) * u64::from(render::HEIGHT)
            <= u64::from(self.height) * u64::from(render::WIDTH)
        {
            (
                self.width,
                (u64::from(self.width) * u64::from(render::HEIGHT) / u64::from(render::WIDTH))
                    as u32,
            )
        } else {
            (
                (u64::from(self.height) * u64::from(render::WIDTH) / u64::from(render::HEIGHT))
                    as u32,
                self.height,
            )
        };
        if w == 0 || h == 0 {
            return None;
        }
        let (left, top) = ((self.width - w) / 2, (self.height - h) / 2);
        if x < left || y < top || x - left >= w || y - top >= h {
            return None;
        }
        Some((
            (u64::from(x - left) * u64::from(render::WIDTH) / u64::from(w)) as u32,
            (u64::from(y - top) * u64::from(render::HEIGHT) / u64::from(h)) as u32,
        ))
    }
    pub fn scene(&self) -> Option<Scene> {
        (self.renderer && self.width > 0 && self.height > 0).then(|| render::scene(&self.surface))
    }
    fn update_led(&mut self) {
        let Some(a) = &self.assignment else {
            self.led = None;
            return;
        };
        if a.role != DeskRole::Lighting {
            self.led = None;
            return;
        }
        let mut pads = [LedState::Off; 8];
        for (i, state) in pads.iter_mut().enumerate() {
            let action = self.surface.pad_action(i);
            *state = match action {
                PadAction::Unavailable => LedState::Off,
                PadAction::Page(page) if page == self.surface.page => LedState::Selected,
                PadAction::Multi if self.surface.additive => LedState::Selected,
                PadAction::ClearHold if !self.surface.confirmed.holds.is_empty() => LedState::Hold,
                PadAction::Blackout if self.surface.confirmed.blackout => LedState::Fault,
                _ => LedState::Available,
            };
        }
        if self.surface.pending.is_some() {
            pads = [LedState::Pending; 8];
        }
        if !self.surface.connected
            || self.surface.confirmed.blackout
            || matches!(self.surface.result, ResultState::Rejected(_))
        {
            pads = [LedState::Fault; 8];
        }
        self.led = Some(LedFrame {
            generation: a.generation,
            endpoint: a.endpoint.clone(),
            pads,
            revision: self.surface.confirmed.revision,
            result: self.surface.result.clone(),
        });
    }
    /// Independent latest-only LED mailbox, bounded to 20 batches/s on injected time.
    /// Returning a frame is desired feedback, never evidence of a physical LED.
    pub fn take_led(&mut self, now_ms: u64) -> Option<LedFrame> {
        if self
            .last_led_ms
            .is_some_and(|last| now_ms < last || now_ms - last < 50)
        {
            return None;
        }
        let frame = self.led.take()?;
        if self.delivered_led.as_ref() == Some(&frame) {
            return None;
        }
        if !self.lighting(frame.generation, &frame.endpoint) {
            return None;
        }
        self.last_led_ms = Some(now_ms);
        self.delivered_led = Some(frame.clone());
        Some(frame)
    }
}
