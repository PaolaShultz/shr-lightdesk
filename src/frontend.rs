//! Provider-backed presentation and independent bounded queues. No lighting authority.
use crate::{
    lux_control::LightingAuthority,
    lux_operator::Operator,
    native_actions::{Editor, Semantic, Workflow},
    render::{self, Scene},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const INPUT_BOUND: usize = 16;
pub const TEXT_BOUND: usize = 256;
#[derive(Clone, Default)]
pub struct View {
    pub inventory: Option<Value>,
    pub freshness: String,
    pub selected: Vec<String>,
    pub page: String,
    pub review: Option<Value>,
    pub review_context: Option<u64>,
    pub review_text: Vec<String>,
    pub review_deadline: Option<Instant>,
    pub notice: String,
    pub lease: bool,
    pub updated_at: Option<Instant>,
    pub completed: u64,
    pub command_error: Option<String>,
    pub editor: Option<Editor>,
    pub cursor: usize,
    pub attribute: usize,
    pub role: Option<Value>,
    pub role_live: bool,
    pub role_notice: String,
    pub role_verified: Option<Instant>,
}
impl View {
    pub fn capture(op: &Operator, notice: String) -> Self {
        Self {
            inventory: op.client.authority.snapshot().cloned(),
            freshness: format!("{:?}", op.client.authority.freshness()),
            selected: op.selected.iter().cloned().collect(),
            page: op.page.clone(),
            review: op
                .input
                .confirmation
                .as_ref()
                .filter(|c| {
                    c.context == op.input.generation
                        && op.client.now() < c.expires
                        && op
                            .client
                            .authority
                            .check_review(c.revision, &c.lease, op.client.now())
                            .is_ok()
                })
                .map(|c| c.command.clone()),
            review_context: op.input.confirmation.as_ref().map(|c| c.context),
            review_text: operator_review_lines(op),
            review_deadline: op.input.confirmation.as_ref().map(|c| {
                Instant::now() + Duration::from_millis(c.expires.saturating_sub(op.client.now()))
            }),
            notice,
            lease: op.client.authority.writable(op.client.now()),
            updated_at: Some(Instant::now()),
            completed: 0,
            command_error: None,
            editor: None,
            cursor: 0,
            attribute: 0,
            role: None,
            role_live: false,
            role_notice: "Role unassigned / keyboard read-only".into(),
            role_verified: None,
        }
    }
}
impl View {
    pub fn current(&self) -> Self {
        let mut view = self.clone();
        if view
            .updated_at
            .is_some_and(|at| at.elapsed() >= Duration::from_millis(2000))
        {
            view.freshness = if view.inventory.is_some() {
                "Stale"
            } else {
                "Unavailable"
            }
            .into();
            view.lease = false;
            view.review = None;
            view.review_context = None;
        }
        if view.role_live
            && view
                .role_verified
                .is_none_or(|at| at.elapsed() >= Duration::from_millis(1000))
        {
            view.role_live = false;
            view.lease = false;
            view.review = None;
            view.review_context = None;
        }
        if view
            .review_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            view.review = None;
            view.review_context = None;
        }
        view
    }
}
fn string(v: &Value) -> String {
    v.as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string())
}
fn clipped(s: &str, n: usize) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .take(n)
        .collect()
}
/// All values are received from Lux. No scaling, blending or destination calculation.
pub fn scene(v: &View, draft: &str, scroll: usize) -> Scene {
    let current = v.current();
    let v = &current;
    let mut s = Scene::default();
    s.rect(0, 0, render::WIDTH, render::HEIGHT, "#10151d");
    s.panel(
        12,
        12,
        1896,
        110,
        "SHR LIGHTDESK / REAL LUX / PHYSICAL UNKNOWN",
    );
    s.text(
        24,
        60,
        format!(
            "Logical {} / writer {} / page {} / role {}",
            v.freshness,
            if v.lease { "granted" } else { "disabled" },
            v.page,
            if v.role_live {
                "lighting-desk live (injected)"
            } else {
                "unassigned/lost; read-only"
            }
        ),
        "#66dfd3",
    );
    s.text(
        24,
        90,
        "No physical output evidence. Semantic controls / F12 help / PgUp and PgDn exact review.",
        "#9caebc",
    );
    let lines = display_lines(v);
    let page = scroll / 20 + 1;
    let pages = lines.len().saturating_sub(26).div_ceil(20) + 1;
    s.panel(
        12,
        134,
        1896,
        706,
        &format!(
            "{} / PAGE {} OF {}{}",
            if v.review.is_some() {
                "REVIEW EXACT COMMAND"
            } else {
                "PROVIDER STATE / LAST KNOWN WHEN STALE"
            },
            page.min(pages),
            pages,
            v.review_deadline
                .filter(|_| v.review.is_some())
                .map(|at| format!(
                    " / expires in {} ms",
                    at.saturating_duration_since(Instant::now()).as_millis()
                ))
                .unwrap_or_default()
        ),
    );
    let total = lines.len();
    for (row, line) in lines
        .into_iter()
        .skip(scroll.min(total.saturating_sub(1)))
        .take(26)
        .enumerate()
    {
        s.text(24, 182 + row as u32 * 24, clipped(&line, 154), "#e4e8e9");
    }
    s.panel(12, 852, 1896, 216, "OPERATOR / REVIEW BEFORE CONFIRM");
    let notice = operator_notice(v);
    for (row, line) in wrapped(notice, 154).into_iter().take(2).enumerate() {
        s.text(24, 888 + row as u32 * 24, line, "#f1bd6b");
    }
    let editor = v
        .editor
        .as_ref()
        .map(|e| {
            format!(
                "DRAFT {} > {}",
                match e {
                    Editor::Existing(crate::surface::actions::Draft::Attribute {
                        attribute,
                        ..
                    }) => match attribute {
                        crate::model::Attribute::Red
                        | crate::model::Attribute::Green
                        | crate::model::Attribute::Blue => "RGB: three percentages R G B",
                        crate::model::Attribute::Pan | crate::model::Attribute::Tilt =>
                            "POSITION: PAN TILT degrees",
                        crate::model::Attribute::Zoom => "ZOOM: degrees",
                        crate::model::Attribute::Intensity => "INTENSITY: percent",
                    },
                    Editor::Existing(_) => "RECORD ID",
                    Editor::Go(_) => "GO: CUE PLAYBACK",
                    Editor::AutoGrant(_) => "AUTO: CAP_PERCENT TTL_MS (10..2000)",
                },
                e.text()
            )
        })
        .unwrap_or_else(|| format!("Command > {draft}"));
    s.text(24, 960, clipped(&editor, 154), "#66dfd3");
    s.text(
        24,
        1000,
        "Arrows fixture/attribute focus | Space select | E edit | R record | G GO | C Hold | P preview",
        "#9caebc",
    );
    s.text(24,1030,"F1..F5 pages | F6 grant | B blackout | M mode | Enter review/confirm | Esc cancel | : command", "#9caebc");
    s
}
fn operator_notice(v: &View) -> &str {
    let notice = if !v.role_live && !v.role_notice.is_empty() {
        &v.role_notice
    } else {
        &v.notice
    };
    if v.review.is_some()
        && (notice.starts_with("REVIEW ") || notice.starts_with("ENGINE PREVIEW "))
    {
        "Review all material lines above; release Enter, then confirm. Context or expiry changes refuse."
    } else if notice.starts_with("selection ") {
        "Current selection/page and Lux state are shown above; raw protocol details are on Health."
    } else {
        notice
    }
}
fn display_lines(v: &View) -> Vec<String> {
    let mut lines = if v.page == "help" {
        wrapped(crate::lux_operator::CONTROL_HELP, 154)
    } else {
        state_lines(v)
    };
    if let Some(review) = &v.review {
        lines = if v.review_text.is_empty() {
            review_lines(review)
        } else {
            v.review_text.clone()
        };
    }
    let notice = operator_notice(v);
    if wrapped(notice, 154).len() > 2 {
        let mut notice_lines = vec!["OPERATOR NOTICE (complete; scroll to inspect)".into()];
        notice_lines.extend(wrapped(notice, 154));
        if v.review.is_some() {
            lines.extend(notice_lines)
        } else {
            notice_lines.extend(lines);
            lines = notice_lines;
        }
    }
    lines
}
/// Coverage matches the actual material rows, never a later notice-only page.
pub fn visible_review_range(v: &View, scroll: usize) -> Option<(u64, usize, usize)> {
    v.review.as_ref()?;
    let context = v.review_context?;
    let start = scroll.min(display_lines(v).len().saturating_sub(1));
    let count = v.review_text.len();
    if start >= count {
        return None;
    }
    Some((context, start, start.saturating_add(26).min(count)))
}
fn wrapped(text: &str, width: usize) -> Vec<String> {
    text.lines()
        .flat_map(|line| {
            let chars: Vec<_> = line
                .chars()
                .map(|c| {
                    if c.is_ascii_graphic() || c == ' ' {
                        c
                    } else {
                        '?'
                    }
                })
                .collect();
            if chars.is_empty() {
                vec![String::new()]
            } else {
                chars.chunks(width).map(|c| c.iter().collect()).collect()
            }
        })
        .collect()
}
fn scalar(value: &Value, attribute: &str) -> String {
    let Some(n) = value.as_i64() else {
        return "none".into();
    };
    let suffix = if matches!(attribute, "pan" | "tilt" | "zoom") {
        " deg"
    } else {
        "%"
    };
    let sign = if n < 0 { "-" } else { "" };
    let n = n.unsigned_abs();
    if n % 10 == 0 {
        format!("{sign}{}{suffix}", n / 10)
    } else {
        format!("{sign}{}.{}{suffix}", n / 10, n % 10)
    }
}
fn source(value: &Value) -> String {
    if let Some(reason) = value.get("auto").and_then(Value::as_str) {
        return format!("AUTO {}", reason.replace('-', " "));
    }
    if let Some(id) = value.get("playback").and_then(Value::as_str) {
        return format!("playback {id}");
    }
    match value.as_str() {
        Some("fixture_default") => "fixture default".into(),
        Some("hold") => "Hold".into(),
        Some("release") => "engine release".into(),
        Some(name) => name.replace('_', " "),
        None => "unavailable".into(),
    }
}
fn capability<'a>(inventory: &'a Value, fixture: &Value, attribute: &Value) -> Option<&'a Value> {
    inventory["capability_metadata"]
        .as_array()?
        .iter()
        .find(|m| m["fixture"] == *fixture)?["attributes"]
        .as_array()?
        .iter()
        .find(|c| c["attribute"] == *attribute)
}
fn current_attribute<'a>(
    inventory: &'a Value,
    fixture: &Value,
    attribute: &Value,
) -> Option<&'a Value> {
    inventory["snapshot"]["fixtures"]
        .as_array()?
        .iter()
        .find(|f| f["fixture"] == *fixture)?["attributes"]
        .as_array()?
        .iter()
        .find(|a| a["attribute"] == *attribute)
}
fn state_lines(v: &View) -> Vec<String> {
    let Some(i) = &v.inventory else {
        return vec!["No complete compatible Lux snapshot. Explicit reconnect required.".into()];
    };
    let mut lines = vec![
        format!(
            "Show {} / epoch {} / revision {}",
            string(&i["snapshot"]["show_id"]),
            string(&i["snapshot"]["epoch"]),
            string(&i["snapshot"]["revision"])
        ),
        format!(
            "Mode {} / grand master {} / blackout {} / physical output unknown",
            string(&i["mode"]).to_ascii_uppercase(),
            scalar(&i["master"], "intensity"),
            if i["blackout"] == true { "ON" } else { "OFF" }
        ),
        format!(
            "Output {} / durability {}",
            string(&i["output"]).replace('_', " "),
            string(&i["snapshot"]["durability"])
        ),
        format!(
            "Selected {}",
            if v.selected.is_empty() {
                "none".into()
            } else {
                v.selected.join(", ")
            }
        ),
    ];
    let focused = i["patch"]["fixtures"]
        .as_array()
        .and_then(|fs| fs.get(v.cursor))
        .map(|f| string(&f["id"]))
        .unwrap_or_else(|| "unavailable".into());
    lines.push(format!(
        "FOCUS slot {} {} / attribute {}",
        v.cursor + 1,
        focused,
        crate::native_actions::attribute_name(crate::model::Attribute::ALL[v.attribute.min(6)])
    ));
    if let Some(a) = i.get("analysis") {
        lines.push(format!(
            "Analysis {} / current confidence {} / intensity proposal {}",
            string(&a["state"]),
            scalar(&a["confidence"], "intensity"),
            scalar(&a["proposal"], "intensity")
        ));
        lines.push(
            "Beat, downbeat and harmony unavailable. K/L calibrate start/finish; A enter AUTO; T grant cap/TTL; X revoke.".into(),
        );
        lines.push(format!(
            "Calibration {} windows / generation {} / source {} / refusal {}",
            string(&a["calibration_windows"]),
            string(&a["calibration_generation"]),
            string(&a["state"]),
            string(&a["reason"])
        ));
        if !a["grant"].is_null() {
            lines.push(format!(
                "AUTO grant fixtures {} / cap {} / at most 2 seconds, never renewed / expiry engine tick {}",
                a["grant"]["fixtures"],
                scalar(&a["grant"]["cap"], "intensity"),
                string(&a["grant"]["expiry_tick"])
            ));
        } else {
            lines.push("AUTO grant: none; explicit fresh reviewed grant required".into());
        }
        if v.page == "health" {
            lines.push(format!(
                "Current source epoch {} / frames {}..{} / calibration {} / age {} ms / losses {}",
                string(&a["source_epoch"]),
                string(&a["source_window_range"]["first_frame"]),
                string(&a["source_window_range"]["end_frame_exclusive"]),
                string(&a["calibration_generation"]),
                string(&a["source_age_ms"]),
                string(&a["losses"])
            ));
        }
        if let Some(values) = a["automatic_layer"]["values"].as_array() {
            for value in values {
                let provenance = &value["provenance"];
                let retained = value["source"] == "analysis-held";
                lines.push(format!(
                    "{} AUTO {} / {}{}",
                    string(&value["fixture"]),
                    scalar(&value["intensity"], "intensity"),
                    if retained {
                        "retained contribution"
                    } else {
                        "current contribution"
                    },
                    value["held_reason"]
                        .as_str()
                        .map(|s| format!(" / {}", s.replace('_', " ")))
                        .unwrap_or_default()
                ));
                if !provenance.is_null() && v.page == "health" {
                    lines.push(format!("  Contribution source epoch {} / frames {}..{} / calibration {} / confidence {}", string(&provenance["source_identity"]["source_epoch"]), string(&provenance["source_window_range"]["first_frame"]), string(&provenance["source_window_range"]["end_frame_exclusive"]), string(&provenance["calibration_generation"]), scalar(&provenance["confidence"], "intensity")));
                } else if provenance.is_null() {
                    lines.push("  Mode-entry continuity; analysis provenance unavailable.".into());
                }
            }
        }
    }
    if v.page == "health" {
        if let Ok(pretty) = serde_json::to_string_pretty(i) {
            lines.extend(wrapped(&pretty, 154));
        }
    } else if matches!(v.page.as_str(), "library" | "playbacks") {
        for (key, label) in [
            ("cues", "Cue"),
            ("palettes", "Palette"),
            ("playbacks", "Playback"),
        ] {
            if let Some(entries) = i["authority_inventory"][key].as_array() {
                if entries.is_empty() {
                    lines.push(format!("{label}s: none"));
                }
                for entry in entries {
                    lines.push(format!(
                        "{label} {}{}",
                        string(&entry["id"]),
                        entry
                            .get("level")
                            .map(|l| format!(" / level {}", scalar(l, "intensity")))
                            .unwrap_or_default()
                    ));
                    if let Some(values) = entry["values"].as_array() {
                        for value in values {
                            lines.push(format!(
                                "  {} {} {}",
                                string(&value["fixture"]),
                                string(&value["attribute"]),
                                scalar(&value["value"], value["attribute"].as_str().unwrap_or(""))
                            ));
                        }
                    }
                }
            }
        }
    } else if let Some(fs) = i["snapshot"]["fixtures"].as_array() {
        for f in fs {
            let selected = v
                .selected
                .iter()
                .any(|id| Some(id.as_str()) == f["fixture"].as_str());
            let label = i["patch"]["fixtures"]
                .as_array()
                .and_then(|fs| fs.iter().find(|p| p["id"] == f["fixture"]))
                .map(|p| string(&p["label"]))
                .unwrap_or_default();
            lines.push(format!(
                "{} {} {}",
                if selected { ">" } else { " " },
                string(&f["fixture"]),
                label
            ));
            if let Some(attrs) = f["attributes"].as_array() {
                for a in attrs {
                    let name = a["attribute"].as_str().unwrap_or("");
                    let bounds = capability(i, &f["fixture"], &a["attribute"])
                        .map(|c| {
                            format!("{}..{}", scalar(&c["min"], name), scalar(&c["max"], name))
                        })
                        .unwrap_or_else(|| "range unavailable".into());
                    let playing = a["playing"]
                        .as_array()
                        .map(|ps| {
                            ps.iter()
                                .map(|p| {
                                    format!("{} {}", string(&p["id"]), scalar(&p["value"], name))
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "none".into());
                    lines.push(format!(
                        "  {name} [{bounds}] programmer {} / Hold {} / playing {playing}",
                        scalar(&a["programmer"], name),
                        scalar(&a["hold"], name)
                    ));
                    lines.push(format!(
                        "    engine current {} from {} / final intent {}{}",
                        scalar(&a["resolved"], name),
                        source(&a["source"]),
                        scalar(&a["final_intent"], name),
                        if a["clamped"] == true {
                            " / CLAMPED"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }
        lines.push(if i["release"]["transition"].is_null() {
            "Engine transition: none".into()
        } else {
            "Engine transition active; values supplied by Lux".into()
        });
    }
    lines
        .into_iter()
        .flat_map(|line| wrapped(&line, 154))
        .collect()
}
/// Material command lines only. Opaque capability tokens stay internal.
pub fn review_lines(review: &Value) -> Vec<String> {
    let action = review["action"].as_str().unwrap_or("unavailable");
    let mut out = vec![format!(
        "REVIEW {} / physical output unknown",
        action.replace('_', " ").to_ascii_uppercase()
    )];
    match action {
        "release_commit" => {
            let preview = &review["token"]["preview"];
            out.push(format!("Scope {} / show {}",string(&review["token"]["scope"]),string(&preview["show_id"])));
            out.push(format!("Epoch {} / revision {} / patch {}",string(&preview["epoch"]),string(&preview["revision"]),string(&preview["patch_revision"])));
            out.push(format!("Transition {} ms / preview validity {} ms / expiry engine tick {}",preview["transition_ms"],preview["validity_ms"],string(&preview["expiry_tick"])));
            if let Some(values) = preview["values"].as_array() { for value in values { out.push(target_line(value, &value["current"], &value["destination"])); } }
        }
        "touch" => { if let Some(values)=review["values"].as_array() { for value in values { out.push(target_line(value,&Value::Null,&value["value"])); } } }
        "record" | "update" => out.push(format!("{} {} {} / {}",if action=="update" {"Update"} else {"Create"},string(&review["kind"]),string(&review["id"]),if action=="update" {"overwrite matching programmer targets; retain other stored values"} else {"existing identity will be refused"})),
        "go" => out.push(format!("GO cue {} into playback {}",string(&review["cue"]),string(&review["playback"]))),
        "blackout" => out.push(format!("Blackout {}",if review["enabled"]==true {"ON"} else {"OFF - output intent resumes"})),
        "analysis_calibrate" => out.push(format!("Calibration {} / existing analysis grant revoked; source windows measured by Lux", string(&review["phase"]))),
        "analysis_grant" => {
            out.push(format!("Intensity only / cap {} / expires after {} ms / no automatic renewal", scalar(&review["cap"], "intensity"), review["ttl_ms"]));
            if let Some(fixtures) = review["fixtures"].as_array() { for fixture in fixtures { out.push(format!("Fixture {} / programmer and Hold retain priority", string(fixture))); } }
        }
        "mode" => out.push(format!("Mode {} / existing look retained",string(&review["mode"]).to_ascii_uppercase())),
        "master" => out.push(format!("Grand master destination {}",scalar(&review["level"],"intensity"))),
        "checkpoint" => out.push("Checkpoint current engine intended look for disarmed recovery; playback/grants/transitions do not resume".into()),
        "clear_to_hold" => out.push("Move all programmer values to persistent Hold; existing Hold values on those targets replaced".into()),
        _ => { if let Ok(pretty)=serde_json::to_string_pretty(review) { out.extend(wrapped(&pretty,154)); } }
    }
    out.push("Release Enter then confirm after reviewing EVERY line; Escape cancels.".into());
    out.into_iter()
        .flat_map(|line| wrapped(&line, 154))
        .collect()
}
fn target_line(value: &Value, current: &Value, destination: &Value) -> String {
    let name = value["attribute"].as_str().unwrap_or("");
    format!(
        "{} {}: current {} -> destination {}",
        string(&value["fixture"]),
        name,
        scalar(current, name),
        scalar(destination, name)
    )
}
/// Shared by presentation and confirmation coverage; never used as a wire command.
pub fn material_review_lines(
    command: &Value,
    inventory: &Value,
    epoch: u64,
    revision: u64,
    expires: u64,
) -> Vec<String> {
    let mut out = vec![
        format!(
            "Scope {} / show {}",
            string(&inventory["scope"]),
            string(&inventory["snapshot"]["show_id"])
        ),
        format!(
            "Epoch {epoch} / revision {revision} / patch {} / expires at session {expires} ms",
            string(&inventory["patch"]["patch_revision"])
        ),
    ];
    let action = command["action"].as_str().unwrap_or("");
    let command_lines = review_lines(command);
    if action == "touch" {
        out.extend(command_lines.first().cloned());
        out.extend(command_lines.last().cloned());
    } else {
        out.extend(command_lines);
    }
    if matches!(action, "analysis_calibrate" | "analysis_grant")
        || (action == "mode" && command["mode"] == "auto")
    {
        let a = &inventory["analysis"];
        out.push(format!(
            "Source epoch {} / map {} / calibration {} / confidence {}",
            string(&a["source_epoch"]),
            string(&a["map_revision"]),
            string(&a["calibration_generation"]),
            scalar(&a["confidence"], "intensity")
        ));
        out.push(format!(
            "Ordered source bindings {}",
            a["source_identity"]["inputs"]
        ));
        out.push("Source loss, expiry, mode exit or changed calibration revokes the grant; retained look may remain. Physical output unknown.".into());
    }
    if matches!(action, "mode" | "master" | "blackout") {
        match action {
            "master" => out.push(format!(
                "Grand master {} -> requested {}",
                scalar(&inventory["master"], "intensity"),
                scalar(&command["level"], "intensity")
            )),
            "blackout" => out.push(format!(
                "Blackout {} -> requested {}",
                if inventory["blackout"] == true {
                    "ON"
                } else {
                    "OFF"
                },
                if command["enabled"] == true {
                    "ON"
                } else {
                    "OFF"
                }
            )),
            "mode" => out.push(format!(
                "Mode {} -> requested {}",
                string(&inventory["mode"]).to_ascii_uppercase(),
                string(&command["mode"]).to_ascii_uppercase()
            )),
            _ => {}
        }
        if matches!(action, "master" | "blackout") {
            out.push("Resulting effective values await provider resolution; no consumer output calculation".into());
        }
        out.push(format!(
            "Current mode {} / grand master {} / blackout {}",
            string(&inventory["mode"]).to_ascii_uppercase(),
            scalar(&inventory["master"], "intensity"),
            if inventory["blackout"] == true {
                "ON"
            } else {
                "OFF"
            }
        ));
        if action == "master" {
            out.push("Allowed grand master 0%..100%; intensity only".into());
        }
        if matches!(action, "master" | "blackout") {
            out.push("Scope: all engine fixture intensities; other attributes unchanged".into());
            if let Some(fs) = inventory["snapshot"]["fixtures"].as_array() {
                for f in fs {
                    if let Some(a) = f["attributes"]
                        .as_array()
                        .and_then(|attrs| attrs.iter().find(|a| a["attribute"] == "intensity"))
                    {
                        out.push(format!(
                            "{} intensity: engine current {} / final intent {} from {}",
                            string(&f["fixture"]),
                            scalar(&a["resolved"], "intensity"),
                            scalar(&a["final_intent"], "intensity"),
                            source(&a["source"])
                        ));
                    }
                }
            }
        }
    }
    if action == "touch"
        && let Some(values) = command["values"].as_array()
    {
        for value in values {
            let current = current_attribute(inventory, &value["fixture"], &value["attribute"])
                .map(|a| &a["resolved"])
                .unwrap_or(&Value::Null);
            out.push(target_line(value, current, &value["value"]));
        }
    }
    if matches!(action, "touch" | "release_commit") {
        let values = if action == "touch" {
            &command["values"]
        } else {
            &command["token"]["preview"]["values"]
        };
        if let Some(values) = values.as_array() {
            for value in values {
                if let Some(cap) = capability(inventory, &value["fixture"], &value["attribute"]) {
                    let name = value["attribute"].as_str().unwrap_or("");
                    out.push(format!(
                        "{} {} allowed {}..{}",
                        string(&value["fixture"]),
                        name,
                        scalar(&cap["min"], name),
                        scalar(&cap["max"], name)
                    ));
                }
            }
        }
    }
    if matches!(action, "record" | "update" | "clear_to_hold") {
        out.push(
            "Scope: entire engine programmer; fixture selection does not limit this operation"
                .into(),
        );
        if action != "clear_to_hold" {
            out.push("Only programmer values are copied; Hold, playing and resolved values are not stored".into());
        }
        let key = if command["kind"] == "palette" {
            "palettes"
        } else {
            "cues"
        };
        let existing = inventory["authority_inventory"][key]
            .as_array()
            .and_then(|ss| ss.iter().find(|s| s["id"] == command["id"]));
        let mut programmer_count = 0;
        if let Some(fs) = inventory["snapshot"]["fixtures"].as_array() {
            for f in fs {
                if let Some(attrs) = f["attributes"].as_array() {
                    for a in attrs {
                        if !a["programmer"].is_null() {
                            programmer_count += 1;
                            let name = a["attribute"].as_str().unwrap_or("");
                            if action == "clear_to_hold" {
                                out.push(format!(
                                    "{} {name}: Hold {} -> requested Hold {} / programmer cleared",
                                    string(&f["fixture"]),
                                    scalar(&a["hold"], name),
                                    scalar(&a["programmer"], name)
                                ));
                            } else {
                                let old = existing
                                    .and_then(|e| e["values"].as_array())
                                    .and_then(|vs| {
                                        vs.iter().find(|v| {
                                            v["fixture"] == f["fixture"]
                                                && v["attribute"] == a["attribute"]
                                        })
                                    })
                                    .map(|v| &v["value"])
                                    .unwrap_or(&Value::Null);
                                out.push(format!(
                                    "{} {name}: stored {} -> requested programmer {}",
                                    string(&f["fixture"]),
                                    scalar(old, name),
                                    scalar(&a["programmer"], name)
                                ));
                            }
                        }
                    }
                }
            }
        }
        if programmer_count == 0 {
            out.push("Programmer is empty; no programmer targets requested".into());
        }
        if action != "clear_to_hold" {
            out.push(format!(
                "Existing {} {}: {}",
                string(&command["kind"]),
                string(&command["id"]),
                if existing.is_some() {
                    "present"
                } else {
                    "absent"
                }
            ));
            if let Some(existing) = existing
                && let Some(values) = existing["values"].as_array()
            {
                for value in values {
                    let programmer =
                        current_attribute(inventory, &value["fixture"], &value["attribute"])
                            .map(|a| &a["programmer"])
                            .unwrap_or(&Value::Null);
                    let name = value["attribute"].as_str().unwrap_or("");
                    if programmer.is_null() || action == "record" {
                        out.push(format!(
                            "{} {name}: stored {} / {}",
                            string(&value["fixture"]),
                            scalar(&value["value"], name),
                            if action == "update" {
                                "retained unchanged"
                            } else {
                                "existing identity makes record refuse"
                            }
                        ));
                    }
                }
            }
        }
    }
    if action == "go" {
        let cue = inventory["authority_inventory"]["cues"]
            .as_array()
            .and_then(|cs| cs.iter().find(|c| c["id"] == command["cue"]));
        let playback = inventory["authority_inventory"]["playbacks"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| p["id"] == command["playback"]));
        out.push(format!(
            "Replace playback {} / existing {} / requested cue {}",
            string(&command["playback"]),
            if playback.is_some() {
                "present"
            } else {
                "absent"
            },
            string(&command["cue"])
        ));
        if let Some(playback) = playback {
            out.push(format!(
                "Existing playback level {} / stored values listed below",
                scalar(&playback["level"], "intensity")
            ));
        }
        out.push("Requested cue values are stored inputs; effective result awaits Lux resolution (Hold/programmer may mask)".into());
        if let Some(cue) = cue {
            if let Some(values) = cue["values"].as_array() {
                for value in values {
                    let current =
                        current_attribute(inventory, &value["fixture"], &value["attribute"]);
                    let name = value["attribute"].as_str().unwrap_or("");
                    let resolved = current.map(|a| &a["resolved"]).unwrap_or(&Value::Null);
                    let current_source = current
                        .map(|a| source(&a["source"]))
                        .unwrap_or_else(|| "unavailable".into());
                    let old = playback
                        .and_then(|p| p["values"].as_array())
                        .and_then(|vs| {
                            vs.iter().find(|v| {
                                v["fixture"] == value["fixture"]
                                    && v["attribute"] == value["attribute"]
                            })
                        })
                        .map(|v| &v["value"])
                        .unwrap_or(&Value::Null);
                    let playing = current
                        .and_then(|a| a["playing"].as_array())
                        .and_then(|ps| ps.iter().find(|p| p["id"] == command["playback"]))
                        .map(|p| &p["value"])
                        .unwrap_or(&Value::Null);
                    out.push(format!(
                        "{} {name}: current {} from {current_source} -> requested cue {}",
                        string(&value["fixture"]),
                        scalar(resolved, name),
                        scalar(&value["value"], name)
                    ));
                    out.push(format!(
                        "  replaced playback stored {} / current contribution {}",
                        scalar(old, name),
                        scalar(playing, name)
                    ));
                }
            }
        } else {
            out.push(format!(
                "Cue {} unavailable; provider will refuse",
                string(&command["cue"])
            ));
        }
        if let Some(old_values) = playback.and_then(|p| p["values"].as_array()) {
            for old in old_values {
                if !cue.and_then(|c| c["values"].as_array()).is_some_and(|vs| {
                    vs.iter().any(|v| {
                        v["fixture"] == old["fixture"] && v["attribute"] == old["attribute"]
                    })
                }) {
                    let name = old["attribute"].as_str().unwrap_or("");
                    let current = current_attribute(inventory, &old["fixture"], &old["attribute"]);
                    let resolved = current.map(|a| &a["resolved"]).unwrap_or(&Value::Null);
                    let current_source = current
                        .map(|a| source(&a["source"]))
                        .unwrap_or_else(|| "unavailable".into());
                    let playing = current
                        .and_then(|a| a["playing"].as_array())
                        .and_then(|ps| ps.iter().find(|p| p["id"] == command["playback"]))
                        .map(|p| &p["value"])
                        .unwrap_or(&Value::Null);
                    out.push(format!("{} {name}: current {} from {current_source} -> requested removal from playback",string(&old["fixture"]),scalar(resolved,name)));
                    out.push(format!("  removed stored {} / current contribution {} / effective result awaits Lux",scalar(&old["value"],name),scalar(playing,name)));
                }
            }
        }
    }
    out.into_iter()
        .flat_map(|line| wrapped(&line, 154))
        .collect()
}
pub fn operator_review_lines(op: &Operator) -> Vec<String> {
    op.input
        .confirmation
        .as_ref()
        .map(|c| c.material_lines.clone())
        .unwrap_or_default()
}
pub fn role_args(args: &[String]) -> Result<Option<crate::role_client::Config>, String> {
    match args { []=>Ok(None),[provider,exe,dir_arg,dir,request_arg,request] if provider=="--role-provider" && dir_arg=="--role-dir" && request_arg=="--role-request" => crate::role_client::Config::load(exe.into(),dir.into(),request.into()).map(Some),_=>Err("role binding requires --role-provider ABSOLUTE_EXE --role-dir ABSOLUTE_PRIVATE_DIR --role-request JSON_PATH".into()) }
}
/// CPU raster is available without enabling native dependencies or discovering adapters.
pub fn offscreen(
    path: &Path,
    show: &str,
    epoch: u64,
    script: &Path,
    out: &Path,
    role: Option<crate::role_client::Config>,
) -> Result<(), String> {
    let bytes = std::fs::read(script).map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("script bound64KiB".into());
    }
    let script = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let lines: Vec<_> = script.lines().collect();
    if lines.len() > 256 {
        return Err("script bound256 lines".into());
    }
    let worker = Worker::spawn_with_role(path.into(), show.into(), epoch, role);
    let deadline = Instant::now() + Duration::from_millis(crate::lux_control::SESSION_MS);
    loop {
        let view = worker.view.lock().unwrap().clone();
        if !view.freshness.is_empty() {
            if view.inventory.is_none() {
                return Err(view.notice);
            }
            if worker.role.is_some() && !view.role_live {
                if view.role_notice.starts_with("ROLE LOST") {
                    return Err(view.role_notice);
                }
            } else {
                break;
            }
        }
        if Instant::now() >= deadline {
            return Err("provider attach deadline".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    for line in lines {
        if line.trim() == "quit" {
            break;
        }
        let before = worker.view.lock().unwrap().completed;
        if line.starts_with('@') {
            let view = worker.view.lock().unwrap().clone();
            worker.action(script_action(line, &view)?)?;
        } else {
            worker.enqueue(line.into())?;
        }
        loop {
            let view = worker.view.lock().unwrap().clone();
            if view.completed > before {
                if let Some(error) = view.command_error {
                    return Err(error);
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err("offscreen session25s deadline".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    let v = worker.view.lock().unwrap().current();
    let scene = scene(&v, "", 0);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    std::fs::write(out.join("lux.svg"), render::svg(&scene)).map_err(|e| e.to_string())?;
    std::fs::write(out.join("lux.ppm"), render::ppm(&scene)).map_err(|e| e.to_string())?;
    if v.review.is_some() {
        for (page, start) in (0..display_lines(&v).len()).step_by(20).enumerate() {
            std::fs::write(
                out.join(format!("lux-review-{page}.svg")),
                render::svg(&self::scene(&v, "", start)),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(out.join("lux.json"),serde_json::to_vec_pretty(&serde_json::json!({"inventory":v.inventory,"freshness":v.freshness,"physical":"unknown","review":v.review,"review_lines":v.review_text})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}
/// Explicit injected semantic script for device-free real-provider acceptance.
pub fn script_action(line: &str, view: &View) -> Result<Semantic, String> {
    use crate::surface::{Page, actions::Action};
    let words: Vec<_> = line.split_whitespace().collect();
    Ok(match words.as_slice() {
        ["@grant"] => Semantic::Grant,
        ["@up"] => Semantic::EnterUp,
        ["@edit"] => Semantic::Edit,
        ["@go"] => Semantic::OpenGo,
        ["@mode"] => Semantic::Mode,
        ["@calibrate", "start"] => Semantic::Calibrate(false),
        ["@calibrate", "finish"] => Semantic::Calibrate(true),
        ["@auto", "enter"] => Semantic::AutoEnter,
        ["@auto", "grant"] => Semantic::AutoGrant,
        ["@auto", "revoke"] => Semantic::AutoRevoke,
        ["@blackout"] => Semantic::Blackout,
        ["@preview"] => Semantic::Preview,
        ["@select", slot] => Semantic::Action(Action::Select(vec![
            slot.parse().map_err(|_| "slot integer")?,
        ])),
        ["@bank", delta] => Semantic::Bank(delta.parse().map_err(|_| "bank delta")?),
        ["@fixture", delta] => Semantic::Action(Action::Navigate(
            delta.parse().map_err(|_| "fixture delta")?,
        )),
        ["@attribute", delta] => Semantic::Action(Action::Attribute(
            delta.parse().map_err(|_| "attribute delta")?,
        )),
        ["@page", page] => Semantic::Action(Action::Page(Page::parse(page).ok_or("page")?)),
        ["@record", kind] => Semantic::Action(Action::Record {
            palette: *kind == "palette",
            replace: false,
            id: 0,
        }),
        ["@update", kind] => Semantic::Action(Action::Record {
            palette: *kind == "palette",
            replace: true,
            id: 0,
        }),
        ["@text", ..] => Semantic::Action(Action::Text(
            line.strip_prefix("@text ").ok_or("text")?.into(),
        )),
        ["@confirm"] => Semantic::Action(Action::Confirm),
        ["@cancel"] => Semantic::Action(Action::Cancel),
        ["@clear"] => Semantic::Action(Action::ClearHold),
        ["@focus-lost"] => Semantic::Action(Action::ContextLost),
        ["@review-all"] => Semantic::Reviewed {
            context: view.review_context.ok_or("no review")?,
            start: 0,
            through: view.review_text.len(),
        },
        _ => return Err("unknown injected semantic action".into()),
    })
}
struct Intent {
    generation: u64,
    payload: Payload,
}
enum Payload {
    Line(String),
    Semantic(Semantic),
}
/// Lifecycle invalidation bypasses the saturated input queue. Provider views coalesce
/// in a separate latest-value slot. LED delivery remains disabled pending GP09.
pub struct Worker {
    input: SyncSender<Intent>,
    generation: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    pub view: Arc<Mutex<View>>,
    thread: Option<JoinHandle<()>>,
    pub role: Option<crate::role_client::Monitor>,
}
impl Worker {
    pub fn spawn(path: PathBuf, show: String, epoch: u64) -> Self {
        Self::spawn_with_role(path, show, epoch, None)
    }
    pub fn spawn_with_role(
        path: PathBuf,
        show: String,
        epoch: u64,
        role_config: Option<crate::role_client::Config>,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Intent>(INPUT_BOUND);
        let generation = Arc::new(AtomicU64::new(1));
        let stop = Arc::new(AtomicBool::new(false));
        let view = Arc::new(Mutex::new(View::default()));
        let role = role_config
            .map(|config| crate::role_client::Monitor::spawn(config, generation.clone()));
        let role_state = role.as_ref().map(|r| r.state.clone());
        let role_live = role.as_ref().map(|r| r.live.clone());
        let role_leds = role.as_ref().map(|r| r.leds.clone());
        let role_guard: Arc<dyn Fn() -> bool + Send + Sync> = {
            let state = role_state.clone();
            let live = role_live.clone();
            Arc::new(move || {
                live.as_ref().is_some_and(|l| l.load(Ordering::Acquire))
                    && state.as_ref().is_some_and(|s| s.lock().unwrap().live())
            })
        };
        let (g, quit, state) = (generation.clone(), stop.clone(), view.clone());
        let handle = thread::spawn(move || {
            if let Some(initial_role) = &role_state {
                let deadline = Instant::now() + Duration::from_millis(1000);
                while !role_guard() && Instant::now() < deadline {
                    if initial_role.lock().unwrap().notice.starts_with("ROLE LOST") {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
            let initial = Operator::connect(&path, &show, epoch);
            let mut notice = match &initial {
                Ok(_) => "Connected read-only; type grant to acquire writer".into(),
                Err(e) => format!("Unavailable: {e}; explicit reconnect EPOCH required"),
            };
            let mut op = initial.ok();
            let mut seen = g.load(Ordering::Acquire);
            let mut refreshed = Instant::now();
            let mut completed = 0_u64;
            let mut command_error = None;
            let mut workflow = Workflow::default();
            let mut had_role = role_guard();
            let mut observing = op.is_some();
            while !quit.load(Ordering::Acquire) {
                let current = g.load(Ordering::Acquire);
                if seen != current {
                    if let Some(op) = &mut op {
                        let _ = op.input.lost();
                        workflow.lost();
                    }
                    seen = current;
                    notice =
                        "Input context lost; intents discarded; release Enter before edits".into();
                }
                match rx.recv_timeout(Duration::from_millis(25)) {
                    Ok(intent)
                        if intent.generation == seen && seen == g.load(Ordering::Acquire) =>
                    {
                        completed = completed.checked_add(1).expect("bounded input counter");
                        command_error = None;
                        if let Some(operator) = &mut op {
                            operator.client.set_input_context(g.clone(), seen);
                            let writable_role = role_guard();
                            let readonly = match &intent.payload {
                                Payload::Line(line) => matches!(
                                    line.split_whitespace().next(),
                                    Some(
                                        "status"
                                            | "status-json"
                                            | "help"
                                            | "refresh"
                                            | "reconnect"
                                            | "select"
                                            | "page"
                                            | "enter"
                                            | "release-input"
                                            | "disconnect"
                                    )
                                ),
                                Payload::Semantic(
                                    Semantic::EnterUp
                                    | Semantic::Reviewed { .. }
                                    | Semantic::Bank(_)
                                    | Semantic::Action(
                                        crate::surface::actions::Action::Page(_)
                                        | crate::surface::actions::Action::Navigate(_)
                                        | crate::surface::actions::Action::Select(_)
                                        | crate::surface::actions::Action::Deselect
                                        | crate::surface::actions::Action::Attribute(_)
                                        | crate::surface::actions::Action::ContextLost
                                        | crate::surface::actions::Action::Cancel
                                        | crate::surface::actions::Action::Back,
                                    ),
                                ) => true,
                                _ => false,
                            };
                            if writable_role {
                                operator.client.set_role_guard(role_guard.clone());
                            }

                            let disconnect_requested = matches!(&intent.payload,
                                Payload::Line(line) if line.trim() == "disconnect");
                            let reconnect_requested = matches!(&intent.payload,
                                Payload::Line(line) if line.split_whitespace().next() == Some("reconnect"));
                            let result = if !writable_role && !readonly {
                                Err("live GP09 lighting lease required; keyboard read-only".into())
                            } else {
                                match intent.payload {
                                    Payload::Line(line) => match line
                                        .split_whitespace()
                                        .collect::<Vec<_>>()
                                        .as_slice()
                                    {
                                        ["confirm"] | ["release"] | ["enter", "down"] => workflow
                                            .dispatch(
                                                operator,
                                                Semantic::Action(
                                                    crate::surface::actions::Action::Confirm,
                                                ),
                                            )
                                            .map(|s| (true, s)),
                                        ["enter", "up"] | ["release-input"] => workflow
                                            .dispatch(operator, Semantic::EnterUp)
                                            .map(|s| (true, s)),
                                        ["cancel"] => workflow
                                            .dispatch(
                                                operator,
                                                Semantic::Action(
                                                    crate::surface::actions::Action::Cancel,
                                                ),
                                            )
                                            .map(|s| (true, s)),
                                        _ => {
                                            workflow.lost();
                                            operator.reviewed_line(&line)
                                        }
                                    },
                                    Payload::Semantic(action) => {
                                        workflow.dispatch(operator, action).map(|text| (true, text))
                                    }
                                }
                            };
                            match result {
                                Ok((running, text)) => {
                                    if disconnect_requested {
                                        observing = false;
                                    }
                                    if reconnect_requested {
                                        observing = true;
                                    }
                                    notice = text;
                                    if !running {
                                        break;
                                    }
                                }
                                Err(e) => {
                                    notice = format!("REFUSED: {e}");
                                    command_error = Some(e);
                                }
                            }
                        } else {
                            let line = match intent.payload {
                                Payload::Line(line) => line,
                                _ => String::new(),
                            };
                            let words: Vec<_> = line.split_whitespace().collect();
                            match words.as_slice() {
                                ["reconnect", epoch] => {
                                    match crate::codec::counter(&serde_json::json!(epoch))
                                        .and_then(|epoch| Operator::connect(&path, &show, epoch))
                                    {
                                        Ok(mut connected) => {
                                            let _ = connected.input.lost();
                                            op = Some(connected);
                                            observing = true;
                                            notice = "Reconnected; release Enter then grant; old intents discarded".into();
                                        }
                                        Err(e) => notice = format!("Reconnect refused: {e}"),
                                    }
                                }
                                _ => {
                                    notice = "Unavailable: explicit reconnect EPOCH required".into()
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if let Some(operator) = &mut op {
                    operator.client.set_input_context(g.clone(), seen);
                    let live_now = role_guard();
                    if had_role && !live_now {
                        operator.client.authority.disconnect();
                        let _ = operator.input.lost();
                        workflow.lost();
                    }
                    had_role = live_now;
                    let _ = operator.client.authority.advance(operator.client.now());
                    if observing && refreshed.elapsed() >= Duration::from_millis(500) {
                        // Observation is independent of GP09 write authority. Failed transport
                        // still requires explicit reconnect; role loss alone does not stop reads.
                        let maintenance = if live_now {
                            operator.client.maintain()
                        } else {
                            Ok(())
                        };
                        let observation = operator.client.refresh();
                        if let Err(e) = observation {
                            observing = false;
                            let _ = operator.input.lost();
                            workflow.lost();
                            notice =
                                format!("Provider unavailable: {e}; explicit reconnect required");
                        }
                        if let Err(e) = maintenance {
                            let _ = operator.input.lost();
                            workflow.lost();
                            if observing {
                                notice = format!("Writer unavailable: {e}; observing read-only");
                            }
                        }
                        refreshed = Instant::now();
                    }
                    // A lifecycle event during a bounded provider operation cannot
                    // retract an already submitted mutation, but must hide its old review.
                    if seen != g.load(Ordering::Acquire) {
                        let _ = operator.input.lost();
                    }
                    let mut next = View::capture(operator, notice.clone());
                    next.completed = completed;
                    next.command_error = command_error.clone();
                    next.editor = workflow.editor.clone();
                    next.cursor = workflow.cursor;
                    next.attribute = workflow.attribute;
                    next.role_live = role_guard();
                    if let Some(role) = &role_state {
                        let role = role.lock().unwrap();
                        next.role = role.grant.clone();
                        next.role_notice = role.notice.clone();
                        next.role_verified = role.verified;
                    }
                    if let Some(leds) = &role_leds {
                        if next.role_live {
                            if let Some(grant) = &next.role {
                                *leds.lock().unwrap() = Some(crate::role_client::LedFrame {
                                    generation: crate::codec::counter(&grant["generation"])
                                        .unwrap(),
                                    binding: grant["binding"].clone(),
                                    states: [if next.review.is_some() {
                                        "pending"
                                    } else {
                                        "available"
                                    }; 8],
                                });
                            }
                        } else {
                            *leds.lock().unwrap() = None;
                        }
                    }

                    *state.lock().unwrap() = next;
                } else {
                    *state.lock().unwrap() = View {
                        freshness: "Unavailable".into(),
                        notice: notice.clone(),
                        completed,
                        command_error: Some(notice.clone()),
                        ..Default::default()
                    };
                }
            }
        });
        Self {
            input: tx,
            generation,
            stop,
            view,
            thread: Some(handle),
            role,
        }
    }
    pub fn invalidate(&self) {
        let _ = self
            .generation
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1));
    }
    pub fn enqueue(&self, line: String) -> Result<(), String> {
        if self.generation.load(Ordering::Acquire) == u64::MAX {
            return Err("input generation exhausted; restart frontend".into());
        }
        if line.len() > TEXT_BOUND {
            return Err("typed input bound256 bytes".into());
        }
        let intent = Intent {
            generation: self.generation.load(Ordering::Acquire),
            payload: Payload::Line(line),
        };
        self.submit(intent)
    }
    pub fn action(&self, action: Semantic) -> Result<(), String> {
        if matches!(&action,Semantic::Action(crate::surface::actions::Action::Text(text)) if text.len()>128)
            || matches!(&action,Semantic::Action(crate::surface::actions::Action::Select(slots)) if slots.len()>32)
        {
            return Err("semantic input payload capacity".into());
        }
        if self.generation.load(Ordering::Acquire) == u64::MAX {
            return Err("input generation exhausted".into());
        }
        self.submit(Intent {
            generation: self.generation.load(Ordering::Acquire),
            payload: Payload::Semantic(action),
        })
    }
    fn submit(&self, intent: Intent) -> Result<(), String> {
        match self.input.try_send(intent) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.invalidate();
                Err("Input busy; overflow invalidated queued intent".into())
            }
            Err(TrySendError::Disconnected(_)) => Err("Provider worker stopped".into()),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.invalidate();
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

/// Native keyboard edge state, exercised without a window or keyboard device.
#[derive(Default)]
pub struct Keyboard {
    pub draft: String,
    pub scroll: usize,
    pub focused: bool,
    held_enter: bool,
}
impl Keyboard {
    pub fn context_lost(&mut self) {
        self.held_enter = true;
    }
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
        self.context_lost();
    }
    pub fn release_enter(&mut self) -> &'static str {
        self.held_enter = false;
        "enter up"
    }
    pub fn text(&mut self, text: &str) {
        if self.focused
            && self.draft.len() + text.len() <= TEXT_BOUND
            && text.chars().all(|c| c.is_ascii_graphic() || c == ' ')
        {
            self.draft.push_str(text);
        }
    }
    pub fn enter(&mut self) -> Option<String> {
        if !self.focused || self.held_enter {
            return None;
        }
        self.held_enter = true;
        let line = std::mem::take(&mut self.draft);
        Some(if line.trim().is_empty() {
            "confirm".into()
        } else {
            line
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_discards_generation_without_blocking_lifecycle() {
        let (tx, rx) = mpsc::sync_channel(INPUT_BOUND);
        let worker = Worker {
            input: tx,
            generation: Arc::new(AtomicU64::new(1)),
            stop: Arc::new(AtomicBool::new(false)),
            view: Arc::new(Mutex::new(View::default())),
            thread: None,
            role: None,
        };
        for _ in 0..INPUT_BOUND {
            worker.enqueue("confirm".into()).unwrap();
        }
        assert!(
            worker
                .enqueue("go cue playback".into())
                .unwrap_err()
                .contains("overflow")
        );
        assert_eq!(worker.generation.load(Ordering::Acquire), 2);
        for _ in 0..INPUT_BOUND {
            assert_eq!(rx.try_recv().unwrap().generation, 1);
        }
        worker.enqueue("enter up".into()).unwrap();
        assert_eq!(rx.try_recv().unwrap().generation, 2);
    }
    #[test]
    fn keyboard_focus_resize_repeat_and_text_bound() {
        let mut k = Keyboard::default();
        k.text("grant");
        assert!(k.draft.is_empty());
        k.focus(true);
        k.text("grant");
        assert!(k.enter().is_none());
        assert_eq!(k.release_enter(), "enter up");
        assert_eq!(k.enter().unwrap(), "grant");
        assert!(k.enter().is_none());
        k.release_enter();
        k.text("touch intensity 0");
        k.context_lost();
        assert_eq!(k.draft, "touch intensity 0");
        assert!(k.enter().is_none());
        k.focus(false);
        k.focus(true);
        assert_eq!(k.draft, "touch intensity 0");
        assert!(k.enter().is_none());
        k.release_enter();
        assert_eq!(k.enter().unwrap(), "touch intensity 0");
        k.text(&"a".repeat(TEXT_BOUND));
        k.text("b");
        assert_eq!(k.draft.len(), TEXT_BOUND);
        k.focus(false);
        assert!(k.enter().is_none());
    }
}
