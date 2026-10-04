//! Pure complete-message translation for an explicitly SYNTHETIC controller.
//! This profile is not a MiniLab preset and must not be installed on hardware.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderMode {
    Absolute,
    TwosComplement,
    BinaryOffset,
    SignedBit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Key(usize),
    Pad(usize),
    Rotary(usize, u8),
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub key_channel: u8,
    pub pad_channel: u8,
    pub rotary_channel: u8,
    pub anchor: u8,
    pub cc: [u8; 16],
    pub pads: [u8; 8],
    pub modes: [EncoderMode; 16],
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            key_channel: 0,
            pad_channel: 9,
            rotary_channel: 0,
            anchor: 48,
            cc: std::array::from_fn(|i| 16 + i as u8),
            pads: std::array::from_fn(|i| 36 + i as u8),
            modes: [EncoderMode::Absolute; 16],
        }
    }
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if [self.key_channel, self.pad_channel, self.rotary_channel]
            .iter()
            .any(|v| *v > 15)
            || self.anchor > 127
            || self.cc.iter().chain(&self.pads).any(|v| *v > 127)
        {
            return Err("invalid MIDI profile range".into());
        }
        if self
            .cc
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != 16
            || self
                .pads
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != 8
        {
            return Err("duplicate physical message".into());
        }
        if self.key_channel == self.pad_channel {
            return Err("key/pad note channels overlap; learn a distinct input layer".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Pickup {
    pub previous: Option<u8>,
    pub acquired: bool,
}
impl Pickup {
    pub fn apply(&mut self, value: u8, target: u8) -> bool {
        let close = value.abs_diff(target) <= 2;
        let crossed = self
            .previous
            .is_some_and(|p| (p < target && value >= target) || (p > target && value <= target));
        self.previous = Some(value);
        self.acquired |= close || crossed;
        self.acquired
    }
}
pub fn relative(mode: EncoderMode, v: u8) -> Option<i32> {
    if v > 127 {
        return None;
    }
    let d = match mode {
        EncoderMode::Absolute => return None,
        EncoderMode::TwosComplement => {
            if v == 64 {
                0
            } else if v < 64 {
                i32::from(v)
            } else {
                i32::from(v) - 128
            }
        }
        EncoderMode::BinaryOffset => i32::from(v) - 64,
        EncoderMode::SignedBit => {
            if v < 64 {
                i32::from(v)
            } else {
                -i32::from(v - 64)
            }
        }
    };
    Some(d.clamp(-8, 8))
}
pub struct Controller {
    pub profile: Profile,
    held_keys: [bool; 128],
    held_pads: [bool; 8],
    pub pickup: [Pickup; 16],
    pub fine: bool,
    pub owned_identity: String,
}
impl Default for Controller {
    fn default() -> Self {
        Self::new(Profile::default()).unwrap()
    }
}
impl Controller {
    pub fn new(profile: Profile) -> Result<Self, String> {
        profile.validate()?;
        Ok(Self {
            profile,
            held_keys: [false; 128],
            held_pads: [false; 8],
            pickup: [Pickup::default(); 16],
            fine: false,
            owned_identity: "light-sim".into(),
        })
    }
    /// Preserve down edges across page changes; a held pad cannot execute in a new context.
    pub fn context_changed(&mut self) {
        self.pickup = [Pickup::default(); 16];
    }
    /// Require a release before a new press after loss. No notes are forwarded.
    pub fn disconnected(&mut self) {
        self.held_keys = [true; 128];
        self.held_pads = [true; 8];
        self.fine = false;
        self.context_changed();
    }
    pub fn decode(&mut self, identity: &str, bytes: &[u8]) -> Option<Input> {
        if identity != self.owned_identity || bytes.len() != 3 || bytes[1] > 127 || bytes[2] > 127 {
            return None;
        }
        let ch = bytes[0] & 15;
        let status = bytes[0] & 0xf0;
        let key = usize::from(bytes[1]);
        if status == 0xb0 && ch == self.profile.rotary_channel {
            return self
                .profile
                .cc
                .iter()
                .position(|v| *v == bytes[1])
                .map(|i| Input::Rotary(i, bytes[2]));
        }
        if !matches!(status, 0x80 | 0x90) {
            return None;
        }
        let down = status == 0x90 && bytes[2] > 0;
        if ch == self.profile.pad_channel {
            let i = self.profile.pads.iter().position(|v| *v == bytes[1])?;
            let old = self.held_pads[i];
            self.held_pads[i] = down;
            return (down && !old).then_some(Input::Pad(i));
        }
        if ch == self.profile.key_channel {
            let old = self.held_keys[key];
            self.held_keys[key] = down;
            if down && !old {
                return bytes[1]
                    .checked_sub(self.profile.anchor)
                    .map(|n| Input::Key(usize::from(n)));
            }
        }
        None
    }
    pub fn value(&mut self, index: usize, raw: u8, current: i32, range: (i32, i32)) -> Option<i32> {
        if index >= 16 || raw > 127 {
            return None;
        }
        let (lo, hi) = range;
        match self.profile.modes[index] {
            EncoderMode::Absolute => {
                let target = ((current - lo) * 127 / (hi - lo)).clamp(0, 127) as u8;
                let previous = self.pickup[index].previous;
                if !self.pickup[index].apply(raw, target) {
                    return None;
                }
                if self.fine {
                    previous.map(|p| (current + i32::from(raw) - i32::from(p)).clamp(lo, hi))
                } else {
                    Some(lo + i32::from(raw) * (hi - lo) / 127)
                }
            }
            mode => relative(mode, raw)
                .filter(|v| *v != 0)
                .map(|d| (current + d * if self.fine { 1 } else { 10 }).clamp(lo, hi)),
        }
    }
}
/// Only a verified identity may be assigned; identical or duplicate identities
/// require a human gesture assignment in the future broker, never first-match.
pub fn unique_endpoint<'a>(candidates: &'a [String], identity: &str) -> Result<&'a str, String> {
    let matches: Vec<_> = candidates
        .iter()
        .filter(|s| s.as_str() == identity)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.as_str()),
        [] => Err("device absent".into()),
        _ => Err("ambiguous identity; explicit assignment required".into()),
    }
}
