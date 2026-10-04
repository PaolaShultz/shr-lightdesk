//! Surface-only semantic actions and detached drafts; no lighting evaluation.
use super::{Layer, Page};
use crate::model::{Attribute, FixtureId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Page(Page),
    Select(Vec<FixtureId>),
    Deselect,
    Layer(Layer),
    Multi(bool),
    Navigate(i32),
    Attribute(i32),
    Edit(Attribute),
    Text(String),
    Backspace,
    Record {
        palette: bool,
        replace: bool,
        id: u16,
    },
    Blackout(bool),
    ClearHold,
    Confirm,
    Cancel,
    Back,
    ContextLost,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Draft {
    Attribute {
        attribute: Attribute,
        text: String,
    },
    Record {
        palette: bool,
        replace: bool,
        text: String,
    },
    BlackoutOff,
}
#[derive(Clone, Debug)]
pub struct Modal {
    pub revision: u64,
    pub epoch: u64,
    pub draft: Draft,
    pub selection: Vec<FixtureId>,
    pub show: String,
}
/// Parse declared percent/degrees in tenths, with no float rounding or exponent.
pub fn typed_value(text: &str) -> Result<i32, String> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let mut parts = digits.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_some()
        || fraction.is_some_and(|v| v.len() != 1 || !v.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid value: enter a number with at most one decimal place".into());
    }
    let whole: i32 = whole.parse().map_err(|_| "value too large")?;
    let value = whole
        .checked_mul(10)
        .and_then(|v| v.checked_add(fraction.map_or(0, |s| i32::from(s.as_bytes()[0] - b'0'))))
        .ok_or("value too large")?;
    Ok(if negative { -value } else { value })
}

/// Semantic pad layout shared by input dispatch and desired LED feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadAction {
    Page(Page),
    Multi,
    Deselect,
    Menu,
    Back,
    Navigate(i32),
    Backspace,
    Confirm,
    Cancel,
    Record,
    ClearHold,
    ReleasePlayback,
    Edit,
    Blackout,
    Unavailable,
}
