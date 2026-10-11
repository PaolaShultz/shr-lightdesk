//! Strict bounded scene metadata and detached edits; Lux alone evaluates scenes.
use crate::{
    adapter::{array, id, integer, level, schema, text},
    codec::counter,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
type Result<T> = std::result::Result<T, String>;
pub fn available(i: &Value) -> bool {
    matches!(
        i["wire_schema"].as_str(),
        Some("lx06-scenes-v1" | "lx06-scenes-v2")
    )
}
fn bounded(v: &Value, min: i32, max: i32) -> Result<i32> {
    let n = integer(v)?;
    if !(min..=max).contains(&n) {
        return Err("scene numeric range".into());
    }
    Ok(n)
}
fn label(v: &Value) -> Result<()> {
    let s = text(v)?;
    if s.is_empty() || s.len() > 64 || s.chars().any(char::is_control) {
        return Err("scene label".into());
    }
    Ok(())
}
fn fixtures(i: &Value) -> Result<BTreeSet<&str>> {
    i["patch"]["fixtures"]
        .as_array()
        .ok_or("scene patch unavailable")?
        .iter()
        .map(|f| id(&f["id"]))
        .collect()
}
pub fn validate_map(m: &Value, i: &Value) -> Result<()> {
    schema(m, &["version", "revision", "roles", "scenes"])?;
    if m["version"] != 1 || counter(&m["revision"])? == 0 {
        return Err("scene map version/revision".into());
    }
    let fs = fixtures(i)?;
    let mut roles = BTreeSet::new();
    let mut assigned = BTreeSet::new();
    let rs = array(&m["roles"], 16)?;
    let ss = array(&m["scenes"], 32)?;
    if rs.is_empty() || ss.is_empty() {
        return Err("empty scene map".into());
    }
    for r in rs {
        schema(r, &["id", "fixtures"])?;
        if !roles.insert(id(&r["id"])?) {
            return Err("duplicate scene role".into());
        }
        let targets = array(&r["fixtures"], 32)?;
        if targets.is_empty() {
            return Err("empty scene role".into());
        }
        for f in targets {
            let f = id(f)?;
            if !fs.contains(f) || !assigned.insert(f) {
                return Err("scene role fixture".into());
            }
        }
    }
    let mut ids = BTreeSet::new();
    for s in ss {
        schema(
            s,
            &[
                "id",
                "label",
                "family",
                "mood",
                "palette",
                "lift",
                "roles",
                "conditions",
                "compatible_from",
                "fade_ms",
                "min_dwell_ms",
                "confirm_ms",
                "variation",
                "repetition_penalty",
            ],
        )?;
        if !ids.insert(id(&s["id"])?) {
            return Err("duplicate scene id".into());
        }
        for k in ["label", "mood", "palette"] {
            label(&s[k])?;
        }
        if !matches!(
            text(&s["family"])?,
            "intimate" | "floating" | "driving" | "building" | "open_lift" | "release"
        ) {
            return Err("scene family".into());
        }
        level(&s["lift"])?;
        bounded(&s["variation"], 0, 100)?;
        level(&s["repetition_penalty"])?;
        bounded(&s["fade_ms"], 100, 10000)?;
        bounded(&s["min_dwell_ms"], 500, 60000)?;
        bounded(&s["confirm_ms"], 100, 10000)?;
        let c = &s["conditions"];
        schema(c, &["energy", "density", "trend", "min_confidence"])?;
        level(&c["min_confidence"])?;
        for k in ["energy", "density", "trend"] {
            let r = &c[k];
            schema(r, &["min", "max"])?;
            let lo = bounded(&r["min"], if k == "trend" { -1000 } else { 0 }, 1000)?;
            let hi = bounded(&r["max"], if k == "trend" { -1000 } else { 0 }, 1000)?;
            if lo > hi {
                return Err("scene inverted condition".into());
            }
        }
        let looks = array(&s["roles"], 16)?;
        if looks.is_empty() {
            return Err("empty scene look".into());
        }
        let mut used = BTreeSet::new();
        for l in looks {
            schema(l, &["role", "rgb", "intensity"])?;
            let role = id(&l["role"])?;
            if !roles.contains(role) || !used.insert(role) {
                return Err("scene look role".into());
            }
            level(&l["intensity"])?;
            let rgb = array(&l["rgb"], 3)?;
            if rgb.len() != 3 {
                return Err("scene RGB count".into());
            }
            for v in rgb {
                level(v)?;
            }
        }
    }
    for s in ss {
        let mut seen = BTreeSet::new();
        for x in array(&s["compatible_from"], 32)? {
            let x = id(x)?;
            if !ids.contains(x) || !seen.insert(x) {
                return Err("scene compatibility".into());
            }
        }
    }
    Ok(())
}
pub fn map_lines(m: &Value) -> Vec<String> {
    let mut out = vec![format!(
        "Scene map v{} revision {} / movement and strobe unsupported",
        m["version"], m["revision"]
    )];
    if let Some(rs) = m["roles"].as_array() {
        for r in rs {
            out.push(format!("Role {} fixtures {}", r["id"], r["fixtures"]));
        }
    }
    if let Some(ss) = m["scenes"].as_array() {
        for s in ss {
            out.push(format!(
                "{} / {} / mood {} / palette {} / lift {}",
                s["id"], s["family"], s["mood"], s["palette"], s["lift"]
            ));
            out.push(format!("  roles {}", s["roles"]));
            out.push(format!(
                "  conditions {} / compatible {}",
                s["conditions"], s["compatible_from"]
            ));
            out.push(format!(
                "  fade {}ms / dwell {}ms / confirm {}ms / variation {} / repetition {}",
                s["fade_ms"],
                s["min_dwell_ms"],
                s["confirm_ms"],
                s["variation"],
                s["repetition_penalty"]
            ));
        }
    }
    out
}
/// Edits authored data only. Integer levels are logical tenths-percent.
pub fn edit(i: &Value, words: &[&str]) -> Result<Value> {
    if !available(i) {
        return Err("scene capability unavailable".into());
    }
    if words.len() < 3 {
        return Err("scene edit: ID FIELD VALUE...".into());
    }
    let mut m = i["scenes"]["map"].clone();
    let next = counter(&m["revision"])?
        .checked_add(1)
        .filter(|n| *n < u64::MAX)
        .ok_or("scene revision exhausted")?;
    m["revision"] = json!(next.to_string());
    let ss = m["scenes"].as_array_mut().ok_or("scene map unavailable")?;
    let s = ss
        .iter_mut()
        .find(|s| s["id"] == words[0])
        .ok_or("unknown scene")?;
    let values = &words[2..];
    let n = |v: &str| {
        v.parse::<i32>()
            .map(Value::from)
            .map_err(|_| "scene integer required".to_owned())
    };
    match words[1] {
        "label" | "mood" | "palette" => s[words[1]] = json!(values.join(" ")),
        "family" if values.len() == 1 => s["family"] = json!(values[0]),
        "lift" | "fade_ms" | "min_dwell_ms" | "confirm_ms" | "variation" | "repetition_penalty"
            if values.len() == 1 =>
        {
            s[words[1]] = n(values[0])?
        }
        "min_confidence" if values.len() == 1 => s["conditions"]["min_confidence"] = n(values[0])?,
        "energy" | "density" | "trend" if values.len() == 2 => {
            s["conditions"][words[1]] = json!({"min":n(values[0])?,"max":n(values[1])?})
        }
        "compatible_from" => {
            s["compatible_from"] = json!(if values == ["none"] {
                Vec::<&str>::new()
            } else {
                values.to_vec()
            })
        }
        "rgb" | "intensity" => {
            let expected = if words[1] == "rgb" { 4 } else { 2 };
            if values.len() != expected {
                return Err("scene role edit: ROLE RGB_VALUES or ROLE INTENSITY".into());
            }
            let l = s["roles"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|l| l["role"] == values[0])
                .ok_or("unknown scene role")?;
            l[words[1]] = if expected == 4 {
                json!([n(values[1])?, n(values[2])?, n(values[3])?])
            } else {
                n(values[1])?
            };
        }
        _ => {
            return Err(
                "unknown scene field or value count; use bounded full-map import for role bindings"
                    .into(),
            );
        }
    }
    validate_map(&m, i)?;
    Ok(json!({"action":"scene_map_replace","map":m}))
}
pub fn presentation(i: &Value) -> Vec<String> {
    if !available(i) {
        return vec!["Scene capability unavailable in this provider version".into()];
    }
    let s = &i["scenes"];
    let e = &s["evidence"];
    let p = &s["interpretation"];
    let mut out = vec![
        format!(
            "Scene intent current {} / proposed {} / reason {}",
            s["current"], s["proposed"], s["reason"]
        ),
        if e["available"] == true {
            format!(
                "Measured evidence: energy {} / density {} / confidence {} / section {}",
                e["energy"], e["density"], e["confidence"], e["section_id"]
            )
        } else {
            "Measured evidence unavailable / energy, density and section unqualified".into()
        },
        format!(
            "Inferred change {} / trend {} / uncertain {} / harmonic relation {}",
            p["change"], p["trend"], p["uncertain"], e["harmonic_relation"]
        ),
    ];
    if s["transition"].is_null() {
        out.push("Transition idle / beat and downbeat unavailable".into())
    } else {
        let t = &s["transition"];
        out.push(format!(
            "Fade {} -> {} / progress {} of 1000 / duration {}ms / beat/downbeat unavailable",
            t["from"], t["to"], t["progress"], t["duration_ms"]
        ));
    }
    let g = &s["grant"];
    out.push(if g.is_null() {
        "Scene authority absent / programmer and Hold retain priority".into()
    } else {
        format!(
            "Scene authority scoped {} / intensity cap {} / RGB cap {} / expiry tick {}",
            g["fixtures"], g["intensity_cap"], g["color_cap"], g["expiry_tick"]
        )
    });
    out.push("Scene selection is intent; actual resolved values and winners follow.".into());
    out
}
pub fn validate_command(i: &Value, c: &Value) -> Result<()> {
    match c["action"].as_str() {
        Some("scene_map_replace") => {
            if !available(i) {
                return Err("scene capability unavailable".into());
            }
            schema(c, &["action", "map"])?;
            validate_map(&c["map"], i)?;
            if counter(&c["map"]["revision"])?
                != counter(&i["scenes"]["map"]["revision"])?
                    .checked_add(1)
                    .ok_or("scene revision exhausted")?
            {
                return Err("scene map revision mismatch".into());
            }
        }
        Some("scene_preview") => {
            if !available(i) {
                return Err("scene capability unavailable".into());
            }
            schema(c, &["action", "scene_id"])?;
            id(&c["scene_id"])?;
            if !i["scenes"]["map"]["scenes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["id"] == c["scene_id"])
            {
                return Err("unknown scene preview".into());
            }
        }
        Some("scene_grant") => {
            if !available(i) {
                return Err("scene capability unavailable".into());
            }
            schema(
                c,
                &["action", "fixtures", "intensity_cap", "color_cap", "ttl_ms"],
            )?;
            crate::lux_operator::validate_analysis_command(
                i,
                &json!({"action":"analysis_grant","fixtures":c["fixtures"],"cap":c["intensity_cap"],"ttl_ms":c["ttl_ms"]}),
            )?;
            level(&c["intensity_cap"])?;
            level(&c["color_cap"])?;
            let ttl = bounded(&c["ttl_ms"], 10, 2000)?;
            if ttl % 10 != 0 {
                return Err("scene grant TTL".into());
            }
            let fs = fixtures(i)?;
            let mut seen = BTreeSet::new();
            let scope = array(&c["fixtures"], 32)?;
            if scope.is_empty() {
                return Err("empty scene grant".into());
            }
            for f in scope {
                let f = id(f)?;
                if !fs.contains(f) || !seen.insert(f) {
                    return Err("scene grant fixture".into());
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn optional_scene(v: &Value, ids: &BTreeSet<&str>) -> Result<()> {
    if !v.is_null() && !ids.contains(id(v)?) {
        return Err("scene status identity".into());
    }
    Ok(())
}
pub fn validate_status(s: &Value, i: &Value) -> Result<()> {
    schema(
        s,
        &[
            "map",
            "evidence",
            "interpretation",
            "current",
            "proposed",
            "reason",
            "transition",
            "intended_values",
            "variation",
            "grant",
            "automatic_layer",
        ],
    )?;
    validate_map(&s["map"], i)?;
    let ids = s["map"]["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| id(&s["id"]))
        .collect::<Result<BTreeSet<_>>>()?;
    optional_scene(&s["current"], &ids)?;
    optional_scene(&s["proposed"], &ids)?;
    label(&s["reason"])?;
    bounded(&s["variation"], 0, 100)?;
    let e = &s["evidence"];
    schema(
        e,
        &[
            "available",
            "energy",
            "density",
            "confidence",
            "section_id",
            "harmonic_relation",
        ],
    )?;
    e["available"]
        .as_bool()
        .ok_or("scene evidence availability")?;
    for k in ["energy", "density", "confidence"] {
        level(&e[k])?;
    }
    if !e["section_id"].is_null() {
        e["section_id"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= u32::MAX as u64)
            .ok_or("scene section id")?;
    }
    if !e["harmonic_relation"].is_null() {
        label(&e["harmonic_relation"])?;
    }
    let p = &s["interpretation"];
    schema(p, &["trend", "change", "uncertain"])?;
    bounded(&p["trend"], -1000, 1000)?;
    if !matches!(
        text(&p["change"])?,
        "unknown" | "stable" | "building" | "release"
    ) {
        return Err("scene interpretation".into());
    }
    p["uncertain"].as_bool().ok_or("scene uncertainty")?;
    let t = &s["transition"];
    if !t.is_null() {
        schema(t, &["from", "to", "start_ms", "duration_ms", "progress"])?;
        optional_scene(&t["from"], &ids)?;
        optional_scene(&t["to"], &ids)?;
        if t["to"].is_null() || t["to"] != s["current"] {
            return Err("scene transition target".into());
        }
        t["start_ms"]
            .as_u64()
            .filter(|n| *n <= u64::MAX / 2)
            .ok_or("scene elapsed time")?;
        bounded(&t["duration_ms"], 100, 10000)?;
        level(&t["progress"])?;
    }
    validate_values(&s["intended_values"], i)?;
    let g = &s["grant"];
    if !g.is_null() {
        schema(
            g,
            &[
                "writer",
                "lease",
                "issue_revision",
                "patch_revision",
                "analysis_generation",
                "expiry_tick",
                "fixtures",
                "intensity_cap",
                "color_cap",
            ],
        )?;
        id(&g["writer"])?;
        counter(&g["lease"])?;
        for k in [
            "issue_revision",
            "patch_revision",
            "analysis_generation",
            "expiry_tick",
        ] {
            counter(&g[k])?;
        }
        let c = json!({"action":"scene_grant","fixtures":g["fixtures"],"intensity_cap":g["intensity_cap"],"color_cap":g["color_cap"],"ttl_ms":10});
        validate_command(i, &c)?;
        if i["mode"] != "auto"
            || g["patch_revision"] != i["patch"]["patch_revision"]
            || g["analysis_generation"] != i["analysis"]["generation"]
        {
            return Err("scene grant authority identity".into());
        }
    }
    schema(&s["automatic_layer"], &["values"])?;
    let mut seen = BTreeSet::new();
    for a in array(&s["automatic_layer"]["values"], 128)? {
        schema(
            a,
            &[
                "fixture",
                "attribute",
                "value",
                "source",
                "held_reason",
                "provenance",
            ],
        )?;
        validate_values(
            &json!([{"fixture":a["fixture"],"attribute":a["attribute"],"value":a["value"]}]),
            i,
        )?;
        if !seen.insert((id(&a["fixture"])?, text(&a["attribute"])?)) {
            return Err("duplicate scene layer target".into());
        }
        if !matches!(text(&a["source"])?, "scene-active" | "scene-held") {
            return Err("scene layer source".into());
        }
        if !a["held_reason"].is_null() {
            label(&a["held_reason"])?;
        }
        let p = &a["provenance"];
        schema(
            p,
            &[
                "map_revision",
                "scene_id",
                "analysis_generation",
                "section_id",
                "source_identity",
                "source_window_range",
            ],
        )?;
        counter(&p["map_revision"])?;
        counter(&p["analysis_generation"])?;
        if p["map_revision"] == s["map"]["revision"] {
            optional_scene(&p["scene_id"], &ids)?;
        } else if !p["scene_id"].is_null() {
            id(&p["scene_id"])?;
        }
        if !p["section_id"].is_null() {
            p["section_id"]
                .as_u64()
                .filter(|n| *n > 0 && *n <= u32::MAX as u64)
                .ok_or("scene retained section id")?;
        }
        crate::adapter::validate_scene_source(p, i)?;
    }
    Ok(())
}
pub fn validate_values(v: &Value, i: &Value) -> Result<()> {
    let mut seen = BTreeSet::new();
    for x in array(v, 128)? {
        schema(x, &["fixture", "attribute", "value"])?;
        let f = id(&x["fixture"])?;
        let a = text(&x["attribute"])?;
        if !matches!(a, "intensity" | "red" | "green" | "blue") {
            return Err("unsupported scene attribute".into());
        }
        let cap = i["capability_metadata"]
            .as_array()
            .ok_or("scene capabilities")?
            .iter()
            .find(|c| c["fixture"] == f)
            .and_then(|c| c["attributes"].as_array())
            .and_then(|cs| cs.iter().find(|c| c["attribute"] == a))
            .ok_or("unsupported scene fixture target")?;
        bounded(&x["value"], integer(&cap["min"])?, integer(&cap["max"])?)?;
        if !seen.insert((f, a)) {
            return Err("duplicate scene target".into());
        }
    }
    Ok(())
}
pub fn preview_lines(p: &Value) -> Vec<String> {
    let mut out = vec![
        format!(
            "Owner preview scene {} / map revision {} / nonmutating logical targets",
            p["scene_id"], p["map_revision"]
        ),
        "Targets below are authored effect; actual state remains controlled by Lux authority."
            .into(),
    ];
    if let Some(values) = p["values"].as_array() {
        for v in values {
            let actual = p["actual"].as_array().and_then(|values| {
                values
                    .iter()
                    .find(|a| a["fixture"] == v["fixture"] && a["attribute"] == v["attribute"])
            });
            out.push(format!(
                "{} {}: preview {} / actual resolved {} / winning source {}",
                v["fixture"],
                v["attribute"],
                display_level(&v["value"]),
                actual.map_or_else(|| "unknown".into(), |a| display_level(&a["value"])),
                actual.map_or(Value::Null, |a| a["source"].clone())
            ));
        }
    }
    if let Some(us) = p["unsupported"].as_array() {
        for u in us {
            out.push(format!("{} unsupported {}", u["fixture"], u["attributes"]));
        }
    }
    out.push("No physical light observation; beat/downbeat alignment unavailable.".into());
    out
}
/// Review exact replacement changes without burying edits among unchanged rows.
pub fn changes(old: &Value, new: &Value) -> Vec<String> {
    fn walk(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
        if a == b {
            return;
        }
        match (a, b) {
            (Value::Object(aa), Value::Object(bb)) => {
                let keys = aa.keys().chain(bb.keys()).collect::<BTreeSet<_>>();
                for k in keys {
                    walk(
                        &format!("{path}.{k}"),
                        aa.get(k).unwrap_or(&Value::Null),
                        bb.get(k).unwrap_or(&Value::Null),
                        out,
                    )
                }
            }
            (Value::Array(aa), Value::Array(bb)) if aa.len() == bb.len() => {
                for (n, (a, b)) in aa.iter().zip(bb).enumerate() {
                    let identity = b
                        .get("id")
                        .or_else(|| b.get("role"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| n.to_string());
                    walk(&format!("{path}[{identity}]"), a, b, out)
                }
            }
            _ => out.push(format!("{path}: {a} -> {b}")),
        }
    }
    let mut out = vec![
        "Replace authored map atomically; changed fields below / unchanged fields retained.".into(),
    ];
    if let Some(roles) = new["roles"].as_array() {
        for r in roles {
            out.push(format!(
                "Role context {} / fixtures {}",
                r["id"], r["fixtures"]
            ));
        }
    }
    walk("map", old, new, &mut out);
    out.push(
        "Map edit revokes automation; no scene GO or new grant. Release Enter, then confirm."
            .into(),
    );
    out
}
/// Regular-file import stays bounded and never waits on a FIFO or follows symlinks.
pub fn read_map(path: &std::path::Path) -> Result<Value> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("scene import requires regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("scene map capacity65536 bytes".into());
    }
    crate::codec::json(&bytes, 65536)
}

fn display_level(v: &Value) -> String {
    v.as_u64().map_or_else(
        || "unknown".into(),
        |n| format!("{}.{:01}%", n / 10, n % 10),
    )
}
