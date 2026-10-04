use shr_lightdesk::{
    controller::{EncoderMode, Input},
    model::*,
    render,
    simulator::Simulator,
    surface::*,
};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};

const HELP: &str = r#"SHR LIGHTDESK - OFFLINE SIMULATION; no devices, sockets or physical output
simulate [preview.svg]       line-oriented keyboard loop; optional live SVG file
gallery DIRECTORY           seven full-HD state-driven SVG drafts and index.html
--help / --version

Loop (values use percent or degrees; decimals accepted):
  select ID... | group ID | deselect | multi on|off | layer fixtures|groups
  page stage|programmer|library|playbacks|automation|patch|health
  set INT|RED|GREEN|BLUE|PAN|TILT|ZOOM VALUE
  cue record ID | cue update ID | go ID SLOT | off SLOT | playback SLOT PERCENT
  palette record ID | palette update ID | palette recall ID
  clear hold                  remove recordable values, keep output in HOLD
  return playback|auto ATTR   preview selected attribute release (0 s simulation)
  confirm | cancel            commit/cancel that exact revision and scope
  mode manual|assist|auto      freeze current look; does not release holds
  grant ATTR LOW HIGH         selected targets; grant alone changes no look
  propose ID ATTR VALUE | accept ATTR (ASSIST only)
  master PERCENT | blackout on|off (latched, explicit release)
  pad 1..8 | key MIDI_NOTE     inject semantic press; not hardware I/O
  midi light-sim STATUS DATA1 DATA2   complete decimal MIDI bytes; other identity ignored
  encoder 1..16 absolute|twos|offset|signed | fine on|off
  autoack on|off | ack | reject | dropack | refresh | disconnect | reconnect
  status | render PATH.svg|PATH.ppm | help | quit
Synthetic profile: keys channel 1, anchor 48; pads channel 10 notes 36..43;
rotaries channel 1 CC16..31. Hardware maps must be learned independently.
"#;

struct App {
    desk: Surface,
    sim: Simulator,
    autoack: bool,
}
impl App {
    fn new() -> Self {
        let sim = Simulator::new();
        let desk = Surface::new(sim.snapshot());
        Self {
            desk,
            sim,
            autoack: true,
        }
    }
    fn send(&mut self, c: Command) -> Result<(), String> {
        let r = self.desk.queue(c)?;
        if self.autoack {
            self.desk.acknowledge(self.sim.request(&r));
        }
        Ok(())
    }
    fn status(&self) -> String {
        let d = &self.desk;
        let mut out = format!(
            "OFFLINE {:?} {:?} / {:?} / selected {:?}\n",
            d.page, d.confirmed.mode, d.result, d.selected
        );
        for t in d.confirmed.targets() {
            if d.selected.is_empty() || d.selected.contains(&t.0) {
                let r = &d.confirmed.resolved[&t];
                out.push_str(&format!(
                    "F{} {:<5} {} {:?} -> final {} (SIM; physical UNKNOWN)\n",
                    t.0,
                    t.1.name(),
                    t.1.display(r.value),
                    r.source,
                    t.1.display(r.final_value)
                ));
            }
        }
        if let Some(p) = &d.preview {
            out.push_str("RELEASE PREVIEW / 0 s SIMULATION ONLY\n");
            for t in &p.targets {
                out.push_str(&format!(
                    "F{} {} {} -> {}\n",
                    t.0,
                    t.1.name(),
                    t.1.display(d.confirmed.resolved[t].value),
                    t.1.display(p.after.resolved[t].value)
                ));
            }
        }
        out.push_str(&format!("{}\n", d.notice));
        out
    }
    fn input(&mut self, e: Input) -> Result<(), String> {
        if let Some(c) = self.desk.input(e, &self.sim)? {
            self.send(c)?;
        }
        Ok(())
    }
    fn line(&mut self, line: &str) -> Result<bool, String> {
        let w: Vec<_> = line.split_whitespace().collect();
        let command = match w.as_slice() {
            [] => return Ok(true),
            ["quit"] | ["exit"] => return Ok(false),
            ["help"] => {
                println!("{HELP}");
                return Ok(true);
            }
            ["status"] => {
                print!("{}", self.status());
                return Ok(true);
            }
            ["select", ids @ ..] if !ids.is_empty() => {
                let ids: Result<Vec<u16>, _> = ids.iter().map(|s| s.parse()).collect();
                self.desk.select(&ids.map_err(|_| "invalid fixture ID")?)?;
                None
            }
            ["group", id] => {
                let id = num(id)?;
                let members = self
                    .desk
                    .confirmed
                    .groups
                    .iter()
                    .find(|g| g.id == id)
                    .ok_or("unknown group")?
                    .members
                    .clone();
                self.desk.select(&members)?;
                None
            }
            ["deselect"] => {
                self.desk.deselect();
                None
            }
            ["multi", v] => {
                self.desk.additive = on(v)?;
                self.desk.context();
                None
            }
            ["layer", v] => {
                self.desk.layer = match *v {
                    "fixtures" => Layer::Fixtures,
                    "groups" => Layer::Groups,
                    _ => return Err("layer must be fixtures or groups".into()),
                };
                self.desk.context();
                None
            }
            ["page", p] => {
                self.desk.page = Page::parse(p).ok_or("unknown page")?;
                self.desk.context();
                None
            }
            ["set", a, v] => Some(Command::Set {
                targets: self.desk.targets(attr(a)?),
                value: value(v)?,
            }),
            ["clear", "hold"] => Some(Command::ClearToHold),
            [kind, op, id]
                if matches!(*kind, "cue" | "palette") && matches!(*op, "record" | "update") =>
            {
                Some(Command::Record {
                    id: num(id)?,
                    palette: *kind == "palette",
                    replace: *op == "update",
                })
            }
            ["palette", "recall", id] => Some(Command::RecallPalette {
                id: num(id)?,
                fixtures: self.desk.selected.iter().copied().collect(),
            }),
            ["go", cue, slot] => Some(Command::Go {
                cue: num(cue)?,
                slot: slot_index(slot)?,
            }),
            ["off", slot] => Some(Command::Off {
                slot: slot_index(slot)?,
            }),
            ["playback", slot, v] => Some(Command::PlaybackLevel {
                slot: slot_index(slot)?,
                value: value(v)?,
            }),
            ["master", v] => Some(Command::Master(value(v)?)),
            ["blackout", v] => Some(Command::Blackout(on(v)?)),
            ["mode", m] => Some(Command::Mode(match *m {
                "manual" => Mode::Manual,
                "assist" => Mode::Assist,
                "auto" => Mode::Auto,
                _ => return Err("unknown mode".into()),
            })),
            ["return", dest, a] => {
                self.desk.release_preview(
                    &self.sim,
                    attr(a)?,
                    match *dest {
                        "playback" => Destination::Playback,
                        "auto" => Destination::Automation,
                        _ => return Err("unknown return destination".into()),
                    },
                )?;
                print!("{}", self.status());
                None
            }
            ["confirm"] => Some(self.desk.confirm_release()?),
            ["cancel"] => {
                self.desk.preview = None;
                self.desk.menu = false;
                self.desk.notice = "Cancelled; state retained".into();
                None
            }
            ["grant", a, l, h] => Some(Command::Grant {
                targets: self.desk.targets(attr(a)?),
                low: value(l)?,
                high: value(h)?,
            }),
            ["propose", id, a, v] => Some(Command::Propose {
                target: (num(id)?, attr(a)?),
                value: value(v)?,
            }),
            ["accept", a] => Some(Command::Accept {
                targets: self.desk.targets(attr(a)?),
            }),
            ["pad", n] => {
                let n: usize = num(n)?;
                if !(1..=8).contains(&n) {
                    return Err("pad must be 1..8".into());
                }
                self.input(Input::Pad(n - 1))?;
                None
            }
            ["key", note] => {
                let note: u8 = num(note)?;
                if note > 127 {
                    return Err("MIDI note must be 0..127".into());
                }
                if let Some(n) = note.checked_sub(self.desk.controller.profile.anchor) {
                    self.input(Input::Key(usize::from(n)))?;
                }
                None
            }
            ["midi", identity, a, b, c] => {
                let bytes = [num(a)?, num(b)?, num(c)?];
                if let Some(e) = self.desk.controller.decode(identity, &bytes) {
                    self.input(e)?;
                }
                None
            }
            ["encoder", n, mode] => {
                let n: usize = num(n)?;
                if !(1..=16).contains(&n) {
                    return Err("encoder must be 1..16".into());
                }
                self.desk.controller.profile.modes[n - 1] = match *mode {
                    "absolute" => EncoderMode::Absolute,
                    "twos" => EncoderMode::TwosComplement,
                    "offset" => EncoderMode::BinaryOffset,
                    "signed" => EncoderMode::SignedBit,
                    _ => return Err("unknown encoding".into()),
                };
                self.desk.controller.context_changed();
                None
            }
            ["fine", v] => {
                self.desk.controller.fine = on(v)?;
                self.desk.controller.context_changed();
                None
            }
            ["autoack", v] => {
                self.autoack = on(v)?;
                None
            }
            ["ack"] | ["dropack"] => {
                let r = self.desk.pending.clone().ok_or("nothing pending")?;
                let reply = self.sim.request(&r);
                if w[0] == "ack" {
                    self.desk.acknowledge(reply);
                } else {
                    self.desk.disconnect();
                }
                None
            }
            ["reject"] => {
                let r = self.desk.pending.clone().ok_or("nothing pending")?;
                self.desk.acknowledge(Reply {
                    id: r.id,
                    epoch: r.epoch,
                    result: Err("injected refusal; authority unchanged".into()),
                });
                None
            }
            ["disconnect"] => {
                self.desk.disconnect();
                None
            }
            ["reconnect"] | ["refresh"] => {
                self.desk.reconnect(self.sim.snapshot());
                None
            }
            ["render", path] => {
                write_scene(&self.desk, Path::new(path))?;
                None
            }
            _ => return Err("unknown command or wrong arguments; type help".into()),
        };
        if let Some(c) = command {
            self.send(c)?;
        }
        Ok(true)
    }
}
fn num<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    s.parse().map_err(|_| "invalid number".into())
}
fn attr(s: &str) -> Result<Attribute, String> {
    Attribute::parse(s).ok_or("unknown attribute".into())
}
fn value(s: &str) -> Result<i32, String> {
    let v: f64 = num(s)?;
    if !v.is_finite() || v.abs() > 100000.0 {
        return Err("invalid value".into());
    }
    Ok((v * 10.0).round() as i32)
}
fn on(s: &str) -> Result<bool, String> {
    match s {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err("expected on or off".into()),
    }
}
fn slot_index(s: &str) -> Result<usize, String> {
    let n: usize = num(s)?;
    if (1..=2).contains(&n) {
        Ok(n - 1)
    } else {
        Err("slot must be 1 or 2".into())
    }
}
fn write_scene(d: &Surface, path: &Path) -> Result<(), String> {
    let s = render::scene(d);
    if !s.in_bounds() {
        return Err("layout exceeded viewport".into());
    }
    let data = if path.extension().is_some_and(|e| e == "ppm") {
        render::ppm(&s)
    } else {
        render::svg(&s).into_bytes()
    };
    std::fs::write(path, data).map_err(|e| e.to_string())
}
fn gallery(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    let mut app = App::new();
    for line in [
        "select 1 2",
        "set INT 60",
        "cue record 1",
        "go 1 1",
        "clear hold",
        "return playback INT",
        "confirm",
        "select 11 12 13 14",
        "set INT 35",
        "set RED 15",
        "set GREEN 30",
        "set BLUE 90",
        "palette record 1",
        "cue record 2",
        "go 2 2",
        "clear hold",
        "return playback INT",
        "confirm",
        "mode assist",
        "select 11",
        "propose 11 INT 55",
        "set INT 42",
    ] {
        app.line(line)?;
    }
    let mut html = String::from(
        "<!doctype html><meta charset='utf-8'><title>SHR Lightdesk offline screen drafts</title><style>body{background:#10151d;color:#e4e8e9;font:18px monospace;margin:24px}img{width:100%;image-rendering:pixelated}a{color:#66dfd3}</style><h1>SHR Lightdesk / OFFLINE ONLY</h1><p>1920 x 1080 state-driven drafts. No native window, physical DMX or photometric measurement.</p>",
    );
    for page in Page::ALL {
        app.desk.page = page;
        let name = page.name().to_lowercase();
        write_scene(&app.desk, &path.join(format!("{name}.svg")))?;
        html.push_str(&format!(
            "<h2>{}</h2><a href='{name}.svg'><img alt='{} offline draft' src='{name}.svg'></a>",
            page.name(),
            page.name()
        ));
    }
    std::fs::write(path.join("index.html"), html).map_err(|e| e.to_string())?;
    Ok(())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [v] if v == "--version" => println!("shr-lightdesk 0.1.0 / offline"),
        [] => println!("{HELP}"),
        [v] if v == "--help" => println!("{HELP}"),
        [cmd, path] if cmd == "gallery" => gallery(Path::new(path))?,
        [cmd, rest @ ..] if cmd == "simulate" && rest.len() <= 1 => {
            let mut app = App::new();
            println!("OFFLINE LIGHTDESK / type help / no state persists on exit");
            if let Some(path) = rest.first() {
                write_scene(&app.desk, Path::new(path))?;
            }
            let stdin = io::stdin();
            let mut lines = stdin.lock().lines();
            loop {
                print!("lightdesk> ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let Some(line) = lines.next() else {
                    break;
                };
                let line = line.map_err(|e| e.to_string())?;
                match app.line(&line) {
                    Ok(false) => break,
                    Ok(true) => println!("{:?}", app.desk.result),
                    Err(e) => {
                        app.desk.notice = e.clone();
                        println!("ERROR: {e}");
                    }
                }
                if let Some(path) = rest.first() {
                    write_scene(&app.desk, Path::new(path))?;
                }
            }
        }
        _ => return Err("unknown arguments; use --help".into()),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
