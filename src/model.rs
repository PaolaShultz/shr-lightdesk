use std::collections::{BTreeMap, BTreeSet};

pub type FixtureId = u16;
pub type Target = (FixtureId, Attribute);
pub type Look = BTreeMap<Target, i32>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Attribute {
    Intensity,
    Red,
    Green,
    Blue,
    Pan,
    Tilt,
    Zoom,
}
impl Attribute {
    pub const ALL: [Self; 7] = [
        Self::Intensity,
        Self::Red,
        Self::Green,
        Self::Blue,
        Self::Pan,
        Self::Tilt,
        Self::Zoom,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Intensity => "INT",
            Self::Red => "RED",
            Self::Green => "GREEN",
            Self::Blue => "BLUE",
            Self::Pan => "PAN",
            Self::Tilt => "TILT",
            Self::Zoom => "ZOOM",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|a| a.name().eq_ignore_ascii_case(s))
    }
    pub fn range(self) -> (i32, i32) {
        match self {
            Self::Pan => (-2700, 2700),
            Self::Tilt => (-1350, 1350),
            Self::Zoom => (50, 450),
            _ => (0, 1000),
        }
    }
    pub fn default_value(self) -> i32 {
        match self {
            Self::Red | Self::Green | Self::Blue => 1000,
            Self::Zoom => 250,
            _ => 0,
        }
    }
    pub fn display(self, value: i32) -> String {
        format!(
            "{:.1}{}",
            f64::from(value) / 10.0,
            if matches!(self, Self::Pan | Self::Tilt | Self::Zoom) {
                "deg"
            } else {
                "%"
            }
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixture {
    pub id: FixtureId,
    pub name: String,
    pub kind: &'static str,
    pub position: (u16, u16),
    pub attributes: Vec<Attribute>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    pub id: u16,
    pub name: String,
    pub members: Vec<FixtureId>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Manual,
    Assist,
    Auto,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Destination {
    Playback,
    Automation,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Default,
    Playback { slot: usize, cue: u16 },
    Automation,
    Hold,
    Programmer,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub value: i32,
    pub source: Source,
    pub final_value: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stored {
    pub id: u16,
    pub name: String,
    pub values: Look,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Playback {
    pub cue: u16,
    pub values: Look,
    pub order: u64,
    pub level: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub show: String,
    pub epoch: u64,
    pub revision: u64,
    pub fixtures: Vec<Fixture>,
    pub groups: Vec<Group>,
    pub mode: Mode,
    pub programmer: Look,
    pub holds: Look,
    pub automation: Look,
    pub proposals: Look,
    pub grants: BTreeMap<Target, (i32, i32)>,
    pub playback_only: BTreeSet<Target>,
    pub cues: BTreeMap<u16, Stored>,
    pub palettes: BTreeMap<u16, Stored>,
    pub playbacks: [Option<Playback>; 2],
    pub master: i32,
    pub blackout: bool,
    /// Authoritative only within the in-process simulator; never physical feedback.
    pub resolved: BTreeMap<Target, Resolved>,
}
impl Snapshot {
    pub fn supported(&self, t: Target) -> bool {
        self.fixtures
            .iter()
            .any(|f| f.id == t.0 && f.attributes.contains(&t.1))
    }
    pub fn targets(&self) -> Vec<Target> {
        self.fixtures
            .iter()
            .flat_map(|f| f.attributes.iter().map(move |a| (f.id, *a)))
            .collect()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Set {
        targets: Vec<Target>,
        value: i32,
    },
    ClearToHold,
    Release {
        targets: Vec<Target>,
        destination: Destination,
    },
    Mode(Mode),
    Master(i32),
    Blackout(bool),
    Record {
        id: u16,
        palette: bool,
        replace: bool,
    },
    RecallPalette {
        id: u16,
        fixtures: Vec<FixtureId>,
    },
    Go {
        cue: u16,
        slot: usize,
    },
    Off {
        slot: usize,
    },
    PlaybackLevel {
        slot: usize,
        value: i32,
    },
    Grant {
        targets: Vec<Target>,
        low: i32,
        high: i32,
    },
    Propose {
        target: Target,
        value: i32,
    },
    Accept {
        targets: Vec<Target>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub epoch: u64,
    pub revision: u64,
    pub command: Command,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reply {
    pub id: u64,
    pub epoch: u64,
    pub result: Result<Snapshot, String>,
}

/// A local adapter seam, not an agreed SHR Lux wire protocol. There is deliberately
/// no Lux implementation until the owner supplies a capability/control contract.
pub trait Authority {
    fn snapshot(&self) -> Snapshot;
    fn preview_release(
        &self,
        targets: Vec<Target>,
        destination: Destination,
    ) -> Result<Snapshot, String>;
    fn request(&mut self, request: &Request) -> Reply;
}
