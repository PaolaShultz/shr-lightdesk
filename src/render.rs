// Scene primitives and PSF/SVG backend adapted from SHR Desk (MIT).
// Copyright (c) 2026 GigPies contributors. See THIRD_PARTY.md.
use crate::{model::*, surface::*};
use std::fmt::Write;
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
const BG: &str = "#10151d";
const PANEL: &str = "#192330";
const EDGE: &str = "#3c4f63";
const TEXT: &str = "#e4e8e9";
const DIM: &str = "#9caebc";
const CYAN: &str = "#66dfd3";
const AMBER: &str = "#f1bd6b";
const RED: &str = "#f47c85";

/// One fixed scene backend for review; a future GPU backend consumes the same primitives.
#[derive(Clone, Debug)]
pub enum Primitive {
    Rect {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        fill: String,
    },
    Text {
        x: u32,
        y: u32,
        value: String,
        color: &'static str,
    },
    Line {
        x1: u32,
        y1: u32,
        x2: u32,
        y2: u32,
        color: &'static str,
    },
}
#[derive(Default)]
pub struct Scene {
    pub primitives: Vec<Primitive>,
}
impl Scene {
    pub(crate) fn rect(&mut self, x: u32, y: u32, w: u32, h: u32, fill: impl Into<String>) {
        self.primitives.push(Primitive::Rect {
            x,
            y,
            w,
            h,
            fill: fill.into(),
        });
    }
    pub(crate) fn text(&mut self, x: u32, y: u32, value: impl Into<String>, color: &'static str) {
        self.primitives.push(Primitive::Text {
            x,
            y,
            value: value.into(),
            color,
        });
    }
    fn line(&mut self, x1: u32, y1: u32, x2: u32, y2: u32, color: &'static str) {
        self.primitives.push(Primitive::Line {
            x1,
            y1,
            x2,
            y2,
            color,
        });
    }
    pub(crate) fn panel(&mut self, x: u32, y: u32, w: u32, h: u32, title: &str) {
        self.rect(x, y, w, h, EDGE);
        self.rect(x + 1, y + 1, w - 2, h - 2, PANEL);
        self.text(x + 12, y + 8, title, DIM);
        self.line(x, y + 40, x + w, y + 40, EDGE);
    }
    pub fn in_bounds(&self) -> bool {
        self.primitives.iter().all(|p| match p {
            Primitive::Rect { x, y, w, h, .. } => x + w <= WIDTH && y + h <= HEIGHT,
            Primitive::Text { x, y, value, .. } => {
                *x + value.chars().count() as u32 * 12 <= WIDTH && y + 24 <= HEIGHT
            }
            Primitive::Line { x1, y1, x2, y2, .. } => {
                *x1 <= WIDTH && *x2 <= WIDTH && *y1 <= HEIGHT && *y2 <= HEIGHT
            }
        })
    }
}

/// Bundled, unmodified Debian PSF2 font. Glyph bits and Unicode table are parsed
/// rather than assuming glyph index equals Unicode codepoint.
pub struct Font {
    pub glyphs: Vec<(char, Vec<u8>)>,
}
impl Default for Font {
    fn default() -> Self {
        let data = include_bytes!("../assets/fonts/Uni2-TerminusBold24x12.psf");
        let num = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(data[20..24].try_into().unwrap()) as usize;
        let mut glyphs = Vec::new();
        for (index, mapping) in data[32 + num * size..]
            .split(|b| *b == 255)
            .take(num)
            .enumerate()
        {
            // PSF sequence entries begin at FE; aliases before that map independently.
            let aliases = mapping.split(|b| *b == 254).next().unwrap();
            if let Ok(text) = std::str::from_utf8(aliases) {
                for c in text.chars() {
                    glyphs.push((c, data[32 + index * size..32 + (index + 1) * size].to_vec()));
                }
            }
        }
        Self { glyphs }
    }
}
impl Font {
    pub fn supports(&self, c: char) -> bool {
        self.glyphs.iter().any(|(ch, _)| *ch == c)
    }
    fn path(&self, c: char) -> String {
        let Some((_, bits)) = self
            .glyphs
            .iter()
            .find(|(ch, _)| *ch == c)
            .or_else(|| self.glyphs.iter().find(|(ch, _)| *ch == '?'))
        else {
            return String::new();
        };
        let mut path = String::new();
        for y in 0..24 {
            for x in 0..12 {
                if bits[y * 2 + x / 8] & (0x80 >> (x % 8)) != 0 {
                    write!(path, "M{x} {y}h1v1h-1z").unwrap();
                }
            }
        }
        path
    }
}

pub fn svg(scene: &Scene) -> String {
    let font = Font::default();
    let mut chars = std::collections::BTreeSet::new();
    for p in &scene.primitives {
        if let Primitive::Text { value, .. } = p {
            chars.extend(value.chars());
        }
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\" role=\"img\"><title>SHR Lightdesk offline simulation - no physical output</title><defs>"
    );
    for c in chars {
        writeln!(out, "<path id=\"g{}\" d=\"{}\"/>", c as u32, font.path(c)).unwrap();
    }
    out.push_str("</defs><g shape-rendering=\"crispEdges\">\n");
    for p in &scene.primitives {
        match p {
            Primitive::Rect { x, y, w, h, fill } => writeln!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\"/>"
            )
            .unwrap(),
            Primitive::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => writeln!(
                out,
                "<path d=\"M{x1} {y1}L{x2} {y2}\" fill=\"none\" stroke=\"{color}\"/>"
            )
            .unwrap(),
            Primitive::Text { x, y, value, color } => {
                writeln!(
                    out,
                    "<g fill=\"{color}\"><title>{}</title>",
                    value
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                )
                .unwrap();
                for (i, c) in value.chars().enumerate() {
                    writeln!(
                        out,
                        "<use href=\"#g{}\" x=\"{}\" y=\"{y}\"/>",
                        c as u32,
                        x + i as u32 * 12
                    )
                    .unwrap();
                }
                out.push_str("</g>\n");
            }
        }
    }
    out.push_str("</g></svg>\n");
    out
}

fn source(s: &Source) -> String {
    match s {
        Source::Default => "DEFAULT".into(),
        Source::Playback { slot, cue } => format!("PB{}/Q{cue}", slot + 1),
        Source::Automation => "AUTO".into(),
        Source::Hold => "HOLD".into(),
        Source::Programmer => "PROGRAMMER".into(),
    }
}
fn short(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.pop();
        out.push('~');
    }
    out
}
fn fmt_value(a: Attribute, v: Option<&i32>) -> String {
    v.map_or("--".into(), |v| a.display(*v))
}
pub fn scene(d: &Surface) -> Scene {
    let mut s = Scene::default();
    let state = &d.confirmed;
    s.rect(0, 0, WIDTH, HEIGHT, BG);
    s.rect(0, 0, WIDTH, 48, PANEL);
    s.text(24, 12, "SHR LIGHTDESK", CYAN);
    s.text(216, 12, format!("/ {}", d.page.name()), TEXT);
    s.text(
        444,
        12,
        format!("OFFLINE SIM / {:?}", state.mode).to_uppercase(),
        AMBER,
    );
    s.text(
        804,
        12,
        if d.connected {
            "LINK: SIMULATED"
        } else {
            "DISCONNECTED / STALE"
        },
        if d.connected { DIM } else { RED },
    );
    s.text(
        1152,
        12,
        format!("GM {:.1}%", f64::from(state.master) / 10.0),
        TEXT,
    );
    s.text(
        1344,
        12,
        if state.blackout {
            "[BLACKOUT LATCHED]"
        } else {
            "BLACKOUT OFF"
        },
        if state.blackout { RED } else { DIM },
    );
    s.text(1644, 12, format!("REV {} / DMX --", state.revision), AMBER);
    s.text(
        24,
        64,
        "STAGE | PROGRAMMER | LIBRARY | PLAYBACKS | AUTOMATION | PATCH | HEALTH",
        DIM,
    );
    s.text(
        1200,
        64,
        format!(
            "KEYS: {:?} / {} / {} SEL",
            d.layer,
            if d.additive { "MULTI" } else { "REPLACE" },
            d.selected.len()
        )
        .to_uppercase(),
        CYAN,
    );
    match d.page {
        Page::Stage => stage(&mut s, d),
        Page::Programmer => programmer(&mut s, d),
        Page::Library => library(&mut s, d),
        Page::Playbacks => playbacks(&mut s, d),
        Page::Automation => automation(&mut s, d),
        Page::Patch => patch(&mut s, d),
        Page::Health => health(&mut s, d),
    }
    s.rect(0, 936, WIDTH, 144, PANEL);
    s.line(0, 936, WIDTH, 936, EDGE);
    for i in 0..16 {
        let x = 24 + (i % 8) as u32 * 234;
        let y = 944 + (i / 8) as u32 * 24;
        let available = (i < 7
            && !d.selected.is_empty()
            && d.targets(Attribute::ALL[i])
                .iter()
                .all(|t| state.supported(*t)))
            || (matches!(i, 8 | 9) && state.playbacks[i - 8].is_some())
            || i == 15;
        let available = available && d.connected && !d.menu && d.preview.is_none();
        let marker = if available {
            if d.controller.pickup[i].acquired {
                "="
            } else {
                "?"
            }
        } else {
            "-"
        };
        s.text(
            x,
            y,
            format!("K{:02}{marker} {}", i + 1, Surface::rotary_label(i)),
            if available { TEXT } else { DIM },
        );
    }
    for (i, label) in d.pad_labels().iter().enumerate() {
        let x = 24 + i as u32 * 234;
        s.rect(
            x,
            996,
            222,
            32,
            if i == 4 && d.additive {
                "#473929"
            } else {
                "#293b4b"
            },
        );
        s.text(x + 12, 1000, format!("P{} {label}", i + 1), TEXT);
    }
    let result = match &d.result {
        ResultState::Ready => "READY".into(),
        ResultState::Pending(id) => format!("PENDING #{id}"),
        ResultState::Confirmed(id) => format!("CONFIRMED SIM #{id}"),
        ResultState::Rejected(e) => format!("REJECTED: {e}"),
        ResultState::Disconnected => "DISCONNECTED: values stale; writes disabled".into(),
    };
    s.text(
        24,
        1044,
        short(&format!("{result} | {}", d.notice), 156),
        AMBER,
    );
    s
}
fn inspector(s: &mut Scene, d: &Surface) {
    s.panel(1212, 108, 684, 540, "FIRST SELECTED / PRE-MASTER AUTHORITY");
    s.text(
        1236,
        160,
        short(&format!("FIXTURES {:?}", d.selected), 52),
        CYAN,
    );
    s.text(1236, 208, "ATTR     VALUE        SOURCE", DIM);
    let id = d.selected.first().copied();
    for (i, a) in Attribute::ALL.iter().enumerate() {
        let y = 248 + i as u32 * 48;
        if let Some(r) = id.and_then(|id| d.confirmed.resolved.get(&(id, *a))) {
            s.text(
                1236,
                y,
                format!(
                    "{:<8} {:<12} {}",
                    a.name(),
                    a.display(r.value),
                    source(&r.source)
                ),
                if matches!(r.source, Source::Hold | Source::Programmer) {
                    AMBER
                } else {
                    TEXT
                },
            );
        } else {
            s.text(
                1236,
                y,
                format!("{:<8} --           UNAVAILABLE", a.name()),
                DIM,
            );
        }
    }
    s.panel(1212, 672, 684, 240, "RELEASE / OPERATOR INTENT");
    if let Some(p) = &d.preview {
        s.text(
            1236,
            720,
            format!("RETURN {:?} / {} targets", p.destination, p.targets.len()).to_uppercase(),
            AMBER,
        );
        if let Some(t) = p.targets.first() {
            s.text(
                1236,
                768,
                format!(
                    "F{} {}: {} -> {}",
                    t.0,
                    t.1.name(),
                    t.1.display(d.confirmed.resolved[t].value),
                    t.1.display(p.after.resolved[t].value)
                ),
                TEXT,
            );
        }
        s.text(1236, 816, "0 s SIM ONLY / confirm or cancel", AMBER);
        s.text(1236, 864, "Revision pinned; selection cancels preview", DIM);
    } else {
        s.text(1236, 720, "Deselect changes selection only.", TEXT);
        s.text(1236, 768, "Clear to Hold preserves the look.", TEXT);
        s.text(1236, 816, "Return requires a scoped preview.", TEXT);
        s.text(1236, 864, "PHYSICAL OUTPUT: UNKNOWN / NO DRIVER", AMBER);
    }
}
fn stage(s: &mut Scene, d: &Surface) {
    s.panel(
        24,
        108,
        1164,
        444,
        "STAGE PLAN / LOGICAL PREVIEW / NOT PHOTOMETRY",
    );
    s.rect(60, 168, 1092, 348, "#121d27");
    s.text(480, 456, "BAND / AUDIENCE BELOW", DIM);
    for f in &d.confirmed.fixtures {
        let (x, y, _, _) = fixture_rect(f);
        let r = &d.confirmed.resolved[&(f.id, Attribute::Intensity)];
        s.rect(
            x,
            y,
            204,
            64,
            if d.selected.contains(&f.id) {
                CYAN
            } else {
                EDGE
            },
        );
        s.rect(x + 2, y + 2, 200, 60, PANEL);
        let rgb = [Attribute::Red, Attribute::Green, Attribute::Blue].map(|a| {
            d.confirmed
                .resolved
                .get(&(f.id, a))
                .map_or(1000, |v| v.final_value)
        });
        let components = rgb.map(|v| (v * r.final_value * 255 / 1_000_000).clamp(0, 255));
        s.rect(
            x + 180,
            y + 7,
            16,
            16,
            format!(
                "#{:02x}{:02x}{:02x}",
                components[0], components[1], components[2]
            ),
        );
        s.text(
            x + 8,
            y + 5,
            format!(
                "{} F{:02} {}",
                if d.selected.contains(&f.id) { ">" } else { " " },
                f.id,
                f.kind
            ),
            TEXT,
        );
        s.text(
            x + 8,
            y + 32,
            format!(
                "{:>5.1}% {}",
                f64::from(r.final_value) / 10.0,
                short(&source(&r.source), 7)
            ),
            if matches!(r.source, Source::Programmer | Source::Hold) {
                AMBER
            } else {
                DIM
            },
        );
    }
    s.panel(
        24,
        576,
        1164,
        336,
        "STATE LEDGER / FINAL = SIMULATED POST-MASTER VALUE",
    );
    s.text(
        48,
        624,
        " ID   SELECT   PROGRAM    HOLD      BASE      FINAL     OWNER",
        DIM,
    );
    for (i, f) in d.confirmed.fixtures.iter().enumerate() {
        let t = (f.id, Attribute::Intensity);
        let r = &d.confirmed.resolved[&t];
        s.text(
            48,
            660 + i as u32 * 28,
            format!(
                "{:>3}   {:<6}   {:<9}  {:<8}  {:<8}  {:<8}  {}",
                f.id,
                if d.selected.contains(&f.id) {
                    "[SEL]"
                } else {
                    "--"
                },
                fmt_value(t.1, d.confirmed.programmer.get(&t)),
                fmt_value(t.1, d.confirmed.holds.get(&t)),
                t.1.display(r.value),
                t.1.display(r.final_value),
                source(&r.source)
            ),
            TEXT,
        );
    }
    inspector(s, d);
}
fn programmer(s: &mut Scene, d: &Surface) {
    s.panel(
        24,
        108,
        1164,
        804,
        "PROGRAMMER / RECORDABLE ATTRIBUTES / LIVE SIM",
    );
    s.text(
        48,
        168,
        "SELECTED is an edit scope. PROGRAMMER overrides may be 0%.",
        TEXT,
    );
    s.text(
        48,
        216,
        "Record stores touched values only. Clear to Hold keeps output.",
        DIM,
    );
    s.text(
        48,
        288,
        "FIXTURE   ATTR     PROGRAM      HOLD         PLAY/RESOLVED SOURCE",
        DIM,
    );
    let mut row = 0;
    for t in d.confirmed.targets() {
        if d.selected.contains(&t.0) && row < 17 {
            let r = &d.confirmed.resolved[&t];
            s.text(
                48,
                336 + row * 28,
                format!(
                    "F{:02}       {:<6}   {:<10}   {:<10}   {} {}",
                    t.0,
                    t.1.name(),
                    fmt_value(t.1, d.confirmed.programmer.get(&t)),
                    fmt_value(t.1, d.confirmed.holds.get(&t)),
                    t.1.display(r.value),
                    source(&r.source)
                ),
                TEXT,
            );
            row += 1;
        }
    }
    s.text(
        48,
        852,
        "First 17 selected attributes / full list: status. FX & timing unavailable.",
        DIM,
    );
    inspector(s, d);
}
fn library(s: &mut Scene, d: &Surface) {
    s.panel(24, 108, 552, 804, "GROUPS / SELECTION ONLY");
    for (i, g) in d.confirmed.groups.iter().enumerate() {
        s.text(
            48,
            180 + i as u32 * 96,
            format!("G{} {}", g.id, g.name),
            CYAN,
        );
        s.text(48, 216 + i as u32 * 96, format!("{:?}", g.members), TEXT);
    }
    s.text(48, 744, "group ID / layer groups", DIM);
    s.text(48, 792, "multi on: toggle membership", DIM);
    s.panel(600, 108, 588, 804, "PALETTES / FIXTURE-SPECIFIC SNAPSHOTS");
    for (i, p) in d.confirmed.palettes.values().take(12).enumerate() {
        s.text(
            624,
            180 + i as u32 * 48,
            format!("{:02}  {}  {} values", p.id, p.name, p.values.len()),
            TEXT,
        );
    }
    s.text(624, 792, "Store: palette record ID", DIM);
    s.text(624, 840, "Apply: palette recall ID", DIM);
    inspector(s, d);
}
fn playbacks(s: &mut Scene, d: &Surface) {
    s.panel(
        24,
        108,
        1164,
        384,
        "PLAYBACKS / TWO STATIC SIMULATION SLOTS",
    );
    for i in 0..2 {
        let x = 48 + i as u32 * 552;
        s.text(x, 168, format!("PB{} / INTENSITY HTP", i + 1), CYAN);
        if let Some(p) = &d.confirmed.playbacks[i] {
            s.text(
                x,
                216,
                format!("PLAYING Q{:02} / {:.1}%", p.cue, f64::from(p.level) / 10.0),
                TEXT,
            );
            s.text(
                x,
                264,
                format!("{} captured attribute values", p.values.len()),
                DIM,
            );
        } else {
            s.text(x, 216, "RELEASED", DIM);
        }
        s.text(x, 336, "Non-intensity: latest GO wins", TEXT);
        s.text(x, 384, "Programmer / HOLD override slot", AMBER);
    }
    s.panel(
        24,
        516,
        1164,
        396,
        "STORED CUES / RECORDING DOES NOT PLAY OR RELEASE",
    );
    for (i, q) in d.confirmed.cues.values().take(8).enumerate() {
        s.text(
            48,
            576 + i as u32 * 32,
            format!(
                "Q{:02}  {:<16}  {:02} attrs   stored, zero-time static look",
                q.id,
                q.name,
                q.values.len()
            ),
            TEXT,
        );
    }
    s.text(
        48,
        864,
        "Cue tracking / lists / fades / effects: UNAVAILABLE; owned by SHR Lux.",
        DIM,
    );
    inspector(s, d);
}
fn automation(s: &mut Scene, d: &Surface) {
    s.panel(
        24,
        108,
        1164,
        804,
        "AUTOMATION / EXPLICIT PER-TARGET BOUNDS",
    );
    s.text(
        48,
        168,
        format!(
            "MODE {:?} / MODE CHANGES PRESERVE CURRENT LOOK",
            d.confirmed.mode
        )
        .to_uppercase(),
        AMBER,
    );
    s.text(
        48,
        216,
        "MANUAL: operator. ASSIST: proposals. AUTO: bounded grants.",
        TEXT,
    );
    s.text(
        48,
        288,
        "TARGET      PROPOSED      GRANT           APPLIED CANDIDATE",
        DIM,
    );
    for (i, (t, v)) in d.confirmed.proposals.iter().take(12).enumerate() {
        s.text(
            48,
            336 + i as u32 * 36,
            format!(
                "F{:02} {:<5}  {:<12}  {:<14}  {}",
                t.0,
                t.1.name(),
                t.1.display(*v),
                d.confirmed
                    .grants
                    .get(t)
                    .map_or("NONE".into(), |(l, h)| format!(
                        "{:.1}..{:.1}",
                        f64::from(*l) / 10.0,
                        f64::from(*h) / 10.0
                    )),
                fmt_value(t.1, d.confirmed.automation.get(t))
            ),
            TEXT,
        );
    }
    s.text(
        48,
        816,
        "Analysis feed: UNAVAILABLE / injected proposals only",
        DIM,
    );
    s.text(
        48,
        864,
        "Hold has no timeout. A proposal cannot silently release it.",
        TEXT,
    );
    inspector(s, d);
}
fn patch(s: &mut Scene, d: &Surface) {
    s.panel(
        24,
        108,
        1872,
        804,
        "PATCH / DECLARED SYNTHETIC CAPABILITIES / NO REAL FIXTURE PERSONALITIES",
    );
    s.text(
        48,
        168,
        "ID    NAME             TYPE    INT RGB PAN TILT ZOOM    UNIVERSE / ADDRESS / LINK",
        DIM,
    );
    for (i, f) in d.confirmed.fixtures.iter().enumerate() {
        s.text(
            48,
            240 + i as u32 * 60,
            format!(
                "{:02}    {:<16} {:<7} YES {:<3} {:<3} {:<4} {:<4}    -- / -- / NO DRIVER",
                f.id,
                f.name,
                f.kind,
                if f.attributes.contains(&Attribute::Red) {
                    "YES"
                } else {
                    "--"
                },
                if f.attributes.contains(&Attribute::Pan) {
                    "YES"
                } else {
                    "--"
                },
                if f.attributes.contains(&Attribute::Tilt) {
                    "YES"
                } else {
                    "--"
                },
                if f.attributes.contains(&Attribute::Zoom) {
                    "YES"
                } else {
                    "--"
                }
            ),
            TEXT,
        );
    }
    s.text(
        48,
        792,
        "Unsupported = unavailable. Mixed selection edits reject atomically.",
        AMBER,
    );
    s.text(
        48,
        840,
        "Pan/tilt degrees are synthetic bounds, not calibrated fixture movement.",
        DIM,
    );
}
fn health(s: &mut Scene, d: &Surface) {
    s.panel(24, 108, 912, 804, "OUTPUT HEALTH / EVIDENCE LADDER");
    for (i, (label, value)) in [
        (
            "Surface connection",
            if d.connected {
                "IN-PROCESS SIM"
            } else {
                "DISCONNECTED / STALE"
            },
        ),
        ("Engine adapter", "SHR LUX NOT CONNECTED"),
        ("Resolved intent", "SYNTHETIC VALUES ONLY"),
        ("Encoded DMX frame", "UNAVAILABLE"),
        ("Transport submitted", "UNAVAILABLE"),
        ("Fixture response", "UNKNOWN"),
        ("Feedback owner", "light-sim / NO LED DRIVER"),
        ("Output arming", "UNAVAILABLE"),
    ]
    .iter()
    .enumerate()
    {
        s.text(48, 180 + i as u32 * 72, *label, DIM);
        s.text(408, 180 + i as u32 * 72, *value, TEXT);
    }
    s.text(
        48,
        840,
        "A fixture drawing is not proof of emitted light.",
        AMBER,
    );
    s.panel(960, 108, 936, 804, "SHOW / CONNECTION / RECOVERY");
    s.text(984, 180, &d.confirmed.show, CYAN);
    s.text(
        984,
        240,
        format!(
            "epoch {} / revision {}",
            d.confirmed.epoch, d.confirmed.revision
        ),
        TEXT,
    );
    s.text(
        984,
        312,
        "No saved show. Restart resets the offline demo.",
        AMBER,
    );
    s.text(
        984,
        384,
        "Disconnect discards pending intent, keeps stale state.",
        TEXT,
    );
    s.text(
        984,
        432,
        "Reconnect fetches current authority; no old replay.",
        TEXT,
    );
    s.text(
        984,
        504,
        "Live output-loss policy requires a validated patch",
        DIM,
    );
    s.text(984, 540, "and engine-owned fallback / explicit rearm.", DIM);
    s.text(
        984,
        624,
        "AUDIO DESK: separate process, focus and controller.",
        TEXT,
    );
    s.text(
        984,
        672,
        "Monitor/controller assignment: PLANNED, unverified.",
        DIM,
    );
    s.text(
        984,
        816,
        "Effects, movement paths, flash and physical I/O",
        DIM,
    );
    s.text(984, 852, "are unavailable in this implementation.", DIM);
}

/// Software raster review of the same primitives, not the intended GPU renderer.
pub fn ppm(scene: &Scene) -> Vec<u8> {
    let mut pixels = vec![0_u8; (WIDTH * HEIGHT * 3) as usize];
    let font = Font::default();
    let color = |s: &str| {
        [
            u8::from_str_radix(&s[1..3], 16).unwrap(),
            u8::from_str_radix(&s[3..5], 16).unwrap(),
            u8::from_str_radix(&s[5..7], 16).unwrap(),
        ]
    };
    let mut put = |x: u32, y: u32, c: [u8; 3]| {
        if x < WIDTH && y < HEIGHT {
            let i = ((y * WIDTH + x) * 3) as usize;
            pixels[i..i + 3].copy_from_slice(&c);
        }
    };
    for p in &scene.primitives {
        match p {
            Primitive::Rect { x, y, w, h, fill } => {
                let c = color(fill);
                for yy in *y..y + h {
                    for xx in *x..x + w {
                        put(xx, yy, c);
                    }
                }
            }
            Primitive::Line {
                x1,
                y1,
                x2,
                y2,
                color: c,
            } => {
                let c = color(c);
                let n = x1.abs_diff(*x2).max(y1.abs_diff(*y2)).max(1);
                for i in 0..=n {
                    let x = i64::from(*x1)
                        + (i64::from(*x2) - i64::from(*x1)) * i64::from(i) / i64::from(n);
                    let y = i64::from(*y1)
                        + (i64::from(*y2) - i64::from(*y1)) * i64::from(i) / i64::from(n);
                    put(x as u32, y as u32, c);
                }
            }
            Primitive::Text {
                x,
                y,
                value,
                color: c,
            } => {
                let c = color(c);
                for (i, ch) in value.chars().enumerate() {
                    if let Some((_, bits)) = font.glyphs.iter().find(|(c, _)| *c == ch) {
                        for yy in 0..24 {
                            for xx in 0..12 {
                                if bits[yy * 2 + xx / 8] & (0x80 >> (xx % 8)) != 0 {
                                    put(x + i as u32 * 12 + xx as u32, y + yy as u32, c);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let mut out = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    out.extend(pixels);
    out
}

/// Shared logical stage geometry for rendering and headless hit selection.
pub fn fixture_rect(f: &crate::model::Fixture) -> (u32, u32, u32, u32) {
    (
        60 + u32::from(f.position.0),
        156 + u32::from(f.position.1) * 3 / 5,
        204,
        64,
    )
}
pub fn hit_fixture(d: &Surface, x: u32, y: u32) -> Option<crate::model::FixtureId> {
    if d.page != Page::Stage {
        return None;
    }
    d.confirmed
        .fixtures
        .iter()
        .rev()
        .find(|f| {
            let (left, top, width, height) = fixture_rect(f);
            x >= left && y >= top && x - left < width && y - top < height
        })
        .map(|f| f.id)
}
