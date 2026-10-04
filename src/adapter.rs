//! Read-only Lux consumer. Trust is granted only to a complete validated inventory.
use crate::codec::{self, MAX_PAGES, MESSAGE_BYTES, counter};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;
type Target = (String, String);
type Look = BTreeMap<Target, i32>;
pub(crate) fn schema<'a>(
    v: &'a Value,
    keys: &[&str],
) -> Result<&'a serde_json::Map<String, Value>> {
    let o = v.as_object().ok_or("expected object")?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) {
        return Err(format!("unknown/missing fields: expected {keys:?}"));
    }
    Ok(o)
}
pub(crate) fn text(v: &Value) -> Result<&str> {
    v.as_str().ok_or_else(|| "expected string".into())
}
pub(crate) fn integer(v: &Value) -> Result<i32> {
    v.as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(|| "expected i32 integer".into())
}
pub(crate) fn boolean(v: &Value) -> Result<bool> {
    v.as_bool().ok_or_else(|| "expected bool".into())
}
pub(crate) fn array(v: &Value, max: usize) -> Result<&[Value]> {
    let a = v.as_array().ok_or("expected array")?;
    if a.len() > max {
        return Err("array capacity".into());
    }
    Ok(a)
}
pub(crate) fn id(v: &Value) -> Result<&str> {
    let s = text(v)?;
    if s.is_empty()
        || s.len() > 64
        || !s.as_bytes()[0].is_ascii_lowercase() && !s.as_bytes()[0].is_ascii_digit()
        || !s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
    {
        return Err("invalid stable ID".into());
    }
    Ok(s)
}
fn show(s: &str) -> Result<()> {
    if s.len() != 36
        || !s.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err("invalid show UUID".into());
    }
    Ok(())
}
pub(crate) fn exact(v: &Value, s: &str) -> Result<()> {
    if text(v)? != s {
        return Err(format!("expected {s}"));
    }
    Ok(())
}
pub(crate) fn level(v: &Value) -> Result<i32> {
    let n = integer(v)?;
    if !(0..=1000).contains(&n) {
        return Err("level range".into());
    }
    Ok(n)
}
#[derive(Clone)]
struct Capability {
    min: i32,
    max: i32,
    default: i32,
}
impl Capability {
    fn value(&self, v: &Value) -> Result<i32> {
        let n = integer(v)?;
        if n < self.min || n > self.max {
            return Err("advertised attribute range".into());
        }
        Ok(n)
    }
}
struct Patch {
    caps: BTreeMap<Target, Capability>,
    groups: BTreeMap<String, Vec<Vec<String>>>,
    fixtures: BTreeSet<String>,
    revision: u64,
}
fn patch(v: &Value, metadata: &Value, max: usize) -> Result<Patch> {
    schema(v, &["version", "patch_revision", "fixtures"])?;
    if integer(&v["version"])? != 1 {
        return Err("patch version".into());
    }
    let mut p = Patch {
        caps: BTreeMap::new(),
        groups: BTreeMap::new(),
        fixtures: BTreeSet::new(),
        revision: counter(&v["patch_revision"])?,
    };
    let mut addresses = BTreeSet::new();
    for f in array(&v["fixtures"], max)? {
        schema(f, &["address", "capabilities", "id", "label", "mode"])?;
        let fid = id(&f["id"])?;
        if !p.fixtures.insert(fid.into()) {
            return Err("duplicate fixture".into());
        }
        let label = text(&f["label"])?;
        if label.len() > 128 || label.chars().any(char::is_control) {
            return Err("invalid label".into());
        }
        let attrs: &[&str] = match text(&f["mode"])? {
            "dimmer" => &["intensity"],
            "rgb" => &["intensity", "red", "green", "blue"],
            "rgb_position" => &["intensity", "red", "green", "blue", "pan", "tilt", "zoom"],
            _ => return Err("unknown synthetic mode".into()),
        };
        let start = integer(&f["address"])?;
        if !(1..=512).contains(&start) || start + attrs.len() as i32 - 1 > 512 {
            return Err("patch address footprint".into());
        }
        for slot in start..start + attrs.len() as i32 {
            if !addresses.insert(slot) {
                return Err("patch overlap".into());
            }
        }
        let caps = array(&f["capabilities"], 7)?;
        if caps.len() != attrs.len() {
            return Err("incomplete capabilities".into());
        }
        for c in caps {
            schema(c, &["attribute", "min", "max", "default"])?;
            let a = text(&c["attribute"])?;
            if !attrs.contains(&a) {
                return Err("mode capability mismatch".into());
            }
            let c = Capability {
                min: integer(&c["min"])?,
                max: integer(&c["max"])?,
                default: integer(&c["default"])?,
            };
            if c.min > c.max
                || c.default < c.min
                || c.default > c.max
                || (matches!(a, "intensity" | "red" | "green" | "blue")
                    && (c.min != 0 || c.max != 1000))
            {
                return Err("capability range/default".into());
            }
            if p.caps.insert((fid.into(), a.into()), c).is_some() {
                return Err("duplicate capability".into());
            }
        }
        let groups = match attrs.len() {
            1 => vec![],
            4 => vec![vec!["red".into(), "green".into(), "blue".into()]],
            _ => vec![
                vec!["red".into(), "green".into(), "blue".into()],
                vec!["pan".into(), "tilt".into()],
            ],
        };
        p.groups.insert(fid.into(), groups);
    }
    let meta = array(metadata, max)?;
    if meta.len() != p.fixtures.len() {
        return Err("incomplete capability metadata".into());
    }
    let mut seen = BTreeSet::new();
    for m in meta {
        schema(m, &["fixture", "attributes", "coherent_groups"])?;
        let fid = id(&m["fixture"])?;
        if !p.fixtures.contains(fid) || !seen.insert(fid) {
            return Err("metadata fixture".into());
        }
        let mut attrs = BTreeSet::new();
        for c in array(&m["attributes"], 7)? {
            schema(c, &["attribute", "min", "max", "default", "step", "unit"])?;
            let a = text(&c["attribute"])?;
            let cap = p
                .caps
                .get(&(fid.into(), a.into()))
                .ok_or("metadata attribute")?;
            if !attrs.insert(a)
                || integer(&c["min"])? != cap.min
                || integer(&c["max"])? != cap.max
                || integer(&c["default"])? != cap.default
                || integer(&c["step"])? != 1
            {
                return Err("metadata mismatch".into());
            }
            exact(
                &c["unit"],
                if matches!(a, "pan" | "tilt" | "zoom") {
                    "tenth_degree"
                } else {
                    "tenth_percent"
                },
            )?;
        }
        if attrs.len() != p.caps.keys().filter(|t| t.0 == fid).count() {
            return Err("metadata missing attributes".into());
        }
        let mut groups = Vec::new();
        for g in array(&m["coherent_groups"], 2)? {
            groups.push(
                array(g, 3)?
                    .iter()
                    .map(|x| text(x).map(str::to_owned))
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        if groups != p.groups[fid] {
            return Err("coherent group mismatch".into());
        }
    }
    Ok(p)
}
fn coherent(look: &Look, p: &Patch) -> Result<()> {
    for (fid, groups) in &p.groups {
        for g in groups {
            let n = g
                .iter()
                .filter(|a| look.contains_key(&(fid.clone(), (*a).clone())))
                .count();
            if n != 0 && n != g.len() {
                return Err("partial coherent group".into());
            }
        }
    }
    Ok(())
}
fn values(v: &Value, p: &Patch, max: usize) -> Result<Look> {
    let mut out = Look::new();
    for x in array(v, max)? {
        schema(x, &["fixture", "attribute", "value"])?;
        let t = (id(&x["fixture"])?.into(), text(&x["attribute"])?.into());
        let c = p.caps.get(&t).ok_or("undeclared target")?;
        let value = c.value(&x["value"])?;
        if out.insert(t, value).is_some() {
            return Err("duplicate target".into());
        }
    }
    coherent(&out, p)?;
    Ok(out)
}
fn stores(v: &Value, p: &Patch, max: usize, targets: usize) -> Result<BTreeMap<String, Look>> {
    let mut out = BTreeMap::new();
    for s in array(v, max)? {
        schema(s, &["id", "values"])?;
        if out
            .insert(id(&s["id"])?.into(), values(&s["values"], p, targets)?)
            .is_some()
        {
            return Err("duplicate stored ID".into());
        }
    }
    Ok(out)
}
fn source(v: &Value, analysis: bool) -> Result<(String, Option<String>)> {
    if let Some(s) = v.as_str() {
        if !matches!(s, "fixture_default" | "hold" | "programmer" | "release") {
            return Err("unknown source".into());
        }
        return Ok((s.into(), None));
    }
    let o = v.as_object().ok_or("source schema")?;
    if analysis && o.len() == 1 && o.contains_key("auto") {
        auto_source(&o["auto"])?;
        return Ok(("auto".into(), Some(text(&o["auto"])?.into())));
    }
    if o.len() != 1 || !o.contains_key("playback") {
        return Err("unavailable/unknown source".into());
    }
    Ok(("playback".into(), Some(id(&o["playback"])?.into())))
}
fn auto_source(v: &Value) -> Result<()> {
    if !matches!(
        text(v)?,
        "mode-entry-continuity" | "analysis-active" | "analysis-held"
    ) {
        return Err("unknown AUTO source".into());
    }
    Ok(())
}
fn nullable_reason(v: &Value) -> Result<()> {
    if !v.is_null() {
        let s = text(v)?;
        if s.is_empty() || s.len() > 128 || !s.bytes().all(|b| b.is_ascii_graphic() || b == b' ') {
            return Err("analysis reason bounds".into());
        }
    }
    Ok(())
}
fn nullable_counter(v: &Value) -> Result<()> {
    if !v.is_null() {
        counter(v)?;
    }
    Ok(())
}
fn bounded(v: &Value, max: i32) -> Result<()> {
    if !(0..=max).contains(&integer(v)?) {
        return Err("analysis integer range".into());
    }
    Ok(())
}
fn analysis_descriptor(v: &Value) -> Result<()> {
    schema(
        v,
        &[
            "contract",
            "version",
            "subscription",
            "source_epoch",
            "stream",
            "sample_rate",
            "first_frame",
            "map_revision",
            "calibration_revision",
            "tap",
            "sources",
            "inputs",
            "clock",
        ],
    )?;
    for (k, value) in [
        ("contract", "C-ANALYSIS"),
        ("subscription", "lux.aux.v1"),
        ("tap", "raw-pre-fader"),
        ("clock", "linux-clock-monotonic-ms"),
    ] {
        exact(&v[k], value)?;
    }
    if integer(&v["version"])? != 1
        || integer(&v["sample_rate"])? != 48000
        || integer(&v["stream"])? != 3
    {
        return Err("analysis descriptor domain".into());
    }
    for k in [
        "source_epoch",
        "first_frame",
        "map_revision",
        "calibration_revision",
    ] {
        counter(&v[k])?;
    }
    if v["sources"] != serde_json::json!(["kick", "bass", "guitar-1", "guitar-2"]) {
        return Err("analysis ordered sources".into());
    }
    if v["inputs"] != serde_json::json!(["input-01", "input-02", "input-03", "input-04"])
        || counter(&v["source_epoch"])? == 0
    {
        return Err("analysis input/epoch identity".into());
    }
    Ok(())
}
fn analysis_window(v: &Value, descriptor: &Value) -> Result<()> {
    schema(v, &["first_frame", "end_frame_exclusive"])?;
    let first = counter(&v["first_frame"])?;
    let end = counter(&v["end_frame_exclusive"])?;
    if end.checked_sub(first) != Some(480) || first < counter(&descriptor["first_frame"])? {
        return Err("analysis window range".into());
    }
    Ok(())
}
fn analysis_status(a: &Value, p: &Patch) -> Result<BTreeMap<String, (i32, String)>> {
    schema(
        a,
        &[
            "analysis_version",
            "state",
            "reason",
            "source_identity",
            "source_epoch",
            "map_revision",
            "calibration_revision",
            "sources",
            "source_window_range",
            "source_age_ms",
            "losses",
            "generation",
            "calibration_generation",
            "calibration_windows",
            "confidence",
            "rms_millionths",
            "energy_millionths",
            "beat",
            "downbeat",
            "harmony",
            "proposal",
            "grant",
            "automatic_layer",
            "held_reason",
        ],
    )?;
    exact(&a["analysis_version"], "lux.analysis.v1")?;
    let state = text(&a["state"])?;
    if !matches!(
        state,
        "absent" | "invalid" | "calibrating" | "settling" | "ready" | "stale"
    ) {
        return Err("unknown analysis state".into());
    }
    nullable_reason(&a["reason"])?;
    nullable_reason(&a["held_reason"])?;
    for k in [
        "source_epoch",
        "map_revision",
        "calibration_revision",
        "calibration_generation",
    ] {
        nullable_counter(&a[k])?;
    }
    for k in ["generation", "losses"] {
        counter(&a[k])?;
    }
    bounded(&a["calibration_windows"], i32::MAX)?;
    level(&a["confidence"])?;
    if !a["source_age_ms"].is_null() {
        bounded(&a["source_age_ms"], i32::MAX)?;
    }
    let descriptor = &a["source_identity"];
    if !descriptor.is_null() {
        analysis_descriptor(descriptor)?;
        for k in [
            "source_epoch",
            "map_revision",
            "calibration_revision",
            "sources",
        ] {
            if a[k] != descriptor[k] {
                return Err("analysis source identity mismatch".into());
            }
        }
    } else if [
        "source_epoch",
        "map_revision",
        "calibration_revision",
        "sources",
        "source_window_range",
    ]
    .iter()
    .any(|k| !a[*k].is_null())
    {
        return Err("analysis metadata without source".into());
    }
    if !a["source_window_range"].is_null() {
        analysis_window(&a["source_window_range"], descriptor)?;
    }
    if !a["rms_millionths"].is_null() {
        let rms = array(&a["rms_millionths"], 4)?;
        if rms.len() != 4 {
            return Err("analysis RMS count".into());
        }
        for n in rms {
            bounded(n, 1_000_000)?;
        }
    }
    if !a["energy_millionths"].is_null() {
        bounded(&a["energy_millionths"], 1_000_000)?;
    }
    for k in ["beat", "downbeat", "harmony"] {
        if !a[k].is_null() {
            return Err("unavailable musical estimate".into());
        }
    }
    if !a["proposal"].is_null() {
        level(&a["proposal"])?;
    }
    if integer(&a["confidence"])? != if state == "ready" { 1000 } else { 0 } {
        return Err("analysis state/confidence mismatch".into());
    }
    if state == "ready"
        && (descriptor.is_null()
            || a["source_window_range"].is_null()
            || a["calibration_generation"].is_null()
            || a["source_age_ms"].is_null()
            || integer(&a["source_age_ms"])? > 100)
    {
        return Err("unqualified ready analysis".into());
    }
    if matches!(state, "absent" | "invalid" | "stale")
        && ["rms_millionths", "energy_millionths", "proposal"]
            .iter()
            .any(|k| !a[*k].is_null())
    {
        return Err("unavailable analysis features".into());
    }
    if ["rms_millionths", "energy_millionths", "proposal"]
        .iter()
        .any(|k| !a[*k].is_null())
        && (descriptor.is_null() || a["source_window_range"].is_null())
    {
        return Err("analysis features without source window".into());
    }
    let g = &a["grant"];
    if !g.is_null() {
        if state != "ready" || a["confidence"] != 1000 {
            return Err("unqualified analysis grant".into());
        }
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
                "cap",
            ],
        )?;
        id(&g["writer"])?;
        for k in [
            "lease",
            "issue_revision",
            "patch_revision",
            "analysis_generation",
            "expiry_tick",
        ] {
            counter(&g[k])?;
        }
        level(&g["cap"])?;
        if counter(&g["patch_revision"])? != p.revision
            || g["analysis_generation"] != a["generation"]
        {
            return Err("analysis grant identity".into());
        }
        let mut seen = BTreeSet::new();
        let fixtures = array(&g["fixtures"], 32)?;
        if fixtures.is_empty() {
            return Err("empty analysis grant".into());
        }
        for f in fixtures {
            let fid = id(f)?;
            if !p.caps.contains_key(&(fid.into(), "intensity".into())) || !seen.insert(fid) {
                return Err("analysis grant fixture".into());
            }
        }
    }
    schema(&a["automatic_layer"], &["values"])?;
    let mut layer = BTreeMap::new();
    for entry in array(&a["automatic_layer"]["values"], 32)? {
        schema(
            entry,
            &[
                "fixture",
                "intensity",
                "source",
                "held_reason",
                "provenance",
            ],
        )?;
        let fid = id(&entry["fixture"])?;
        let value = p
            .caps
            .get(&(fid.into(), "intensity".into()))
            .ok_or("AUTO unsupported fixture")?
            .value(&entry["intensity"])?;
        auto_source(&entry["source"])?;
        nullable_reason(&entry["held_reason"])?;
        let provenance = &entry["provenance"];
        if !provenance.is_null() {
            schema(
                provenance,
                &[
                    "analysis_version",
                    "source_identity",
                    "source_window_range",
                    "calibration_generation",
                    "confidence",
                ],
            )?;
            exact(&provenance["analysis_version"], "lux.analysis.v1")?;
            analysis_descriptor(&provenance["source_identity"])?;
            analysis_window(
                &provenance["source_window_range"],
                &provenance["source_identity"],
            )?;
            counter(&provenance["calibration_generation"])?;
            level(&provenance["confidence"])?;
        } else if entry["source"] != "mode-entry-continuity" {
            return Err("analysis AUTO missing provenance".into());
        }
        if layer
            .insert(fid.into(), (value, text(&entry["source"])?.into()))
            .is_some()
        {
            return Err("duplicate AUTO fixture".into());
        }
    }
    Ok(layer)
}

fn inventory(v: &Value, expected_show: &str, epoch: u64, revision: u64) -> Result<()> {
    let mut keys = vec![
        "applied_auto",
        "authority_inventory",
        "blackout",
        "capability_metadata",
        "grants",
        "groups",
        "limits",
        "master",
        "mode",
        "output",
        "patch",
        "physical",
        "scope",
        "snapshot",
        "snapshot_budget",
    ];
    let analysis = v.get("wire_schema").is_some_and(|s| s == "lx05-v1");
    let timed = match v.get("wire_schema") {
        None => false,
        Some(schema) if matches!(text(schema)?, "lx04-v1" | "lx04-durable-v1") => true,
        Some(schema) if text(schema)? == "lx05-v1" => v.get("release").is_some(),
        Some(_) => return Err("unknown wire schema".into()),
    };
    let durable = v.get("wire_schema").is_some_and(|s| s == "lx04-durable-v1")
        || (analysis && v.get("checkpoint").is_some());
    if timed || analysis {
        keys.push("wire_schema");
    }
    if timed {
        keys.push("release");
    }
    if durable && !timed {
        return Err("checkpoint requires timed release capability".into());
    }
    if durable {
        keys.push("checkpoint");
    }
    if analysis {
        keys.push("analysis");
    }
    schema(v, &keys)?;
    if durable {
        schema(
            &v["checkpoint"],
            &["available", "recovery", "active_transients_resumed"],
        )?;
        if !boolean(&v["checkpoint"]["available"])?
            || boolean(&v["checkpoint"]["active_transients_resumed"])?
        {
            return Err("checkpoint capability".into());
        }
        exact(
            &v["checkpoint"]["recovery"],
            "current_intended_look_frozen_into_hold",
        )?;
    }
    exact(
        &v["applied_auto"],
        if analysis {
            "explicit_bounded_intensity_grant"
        } else {
            "unavailable"
        },
    )?;
    exact(&v["output"], "null_disarmed")?;
    exact(&v["physical"], "unknown")?;
    exact(&v["scope"], "lighting-control")?;
    exact(
        &v["snapshot_budget"],
        "16 encoded pages of at most 65536 bytes; overbudget edits refused atomically",
    )?;
    if !matches!(text(&v["mode"])?, "manual" | "assist") && !(analysis && v["mode"] == "auto") {
        return Err("unavailable mode".into());
    }
    level(&v["master"])?;
    boolean(&v["blackout"])?;
    let l = &v["limits"];
    let limits = [
        ("fixtures", 32),
        ("playbacks", 8),
        ("cues", 32),
        ("palettes", 32),
        ("targets", 64),
        ("writers", 1),
        ("writer_history", 1024),
        ("cached_responses", 64),
        ("pages", 16),
        ("message_bytes", 65536),
        ("depth", 12),
        ("lease_ms", 2000),
        ("renew_ms", 500),
    ];
    schema(l, &limits.iter().map(|x| x.0).collect::<Vec<_>>())?;
    for (key, max) in limits {
        let n = integer(&l[key])?;
        if n <= 0 || n > max {
            return Err("negotiated limit".into());
        }
    }
    if integer(&l["renew_ms"])? >= integer(&l["lease_ms"])? {
        return Err("renew interval".into());
    }
    let p = patch(
        &v["patch"],
        &v["capability_metadata"],
        integer(&l["fixtures"])? as usize,
    )?;
    let automatic = if analysis {
        analysis_status(&v["analysis"], &p)?
    } else {
        BTreeMap::new()
    };
    if analysis && !v["analysis"]["grant"].is_null() && v["mode"] != "auto" {
        return Err("analysis grant outside AUTO mode".into());
    }
    let mut automatic_seen = BTreeSet::new();
    let mut group_ids = BTreeSet::new();
    let mut all_seen = false;
    for g in array(&v["groups"], 32)? {
        schema(g, &["id", "fixtures"])?;
        if !group_ids.insert(id(&g["id"])?) {
            return Err("duplicate group".into());
        }
        let mut seen = BTreeSet::new();
        for fid in array(&g["fixtures"], 32)? {
            let fid = id(fid)?;
            if !p.fixtures.contains(fid) || !seen.insert(fid) {
                return Err("group fixture".into());
            }
        }
        if id(&g["id"])? == "all" {
            all_seen = seen
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<BTreeSet<_>>()
                == p.fixtures;
        }
    }
    if !all_seen {
        return Err("incomplete all-fixtures group".into());
    }
    let mut writers = BTreeSet::new();
    for g in array(&v["grants"], integer(&l["writers"])? as usize)? {
        schema(g, &["writer", "scope", "remaining_ms"])?;
        if !writers.insert(id(&g["writer"])?) {
            return Err("duplicate grant".into());
        }
        exact(&g["scope"], "lighting-control")?;
        let n = integer(&g["remaining_ms"])?;
        if n < 0 || n > integer(&l["lease_ms"])? {
            return Err("grant remaining range".into());
        }
    }
    let inv = &v["authority_inventory"];
    schema(inv, &["cues", "palettes", "playbacks", "fixture_masters"])?;
    let target_limit = integer(&l["targets"])? as usize;
    let cues = stores(
        &inv["cues"],
        &p,
        integer(&l["cues"])? as usize,
        target_limit,
    )?;
    let palettes = stores(
        &inv["palettes"],
        &p,
        integer(&l["palettes"])? as usize,
        target_limit,
    )?;
    let mut playing = BTreeMap::new();
    let mut orders = BTreeSet::new();
    for s in array(&inv["playbacks"], integer(&l["playbacks"])? as usize)? {
        schema(s, &["id", "level", "activation_order", "values"])?;
        level(&s["level"])?;
        let order = counter(&s["activation_order"])?;
        if order == 0 || !orders.insert(order) {
            return Err("playback activation order".into());
        }
        if playing
            .insert(
                id(&s["id"])?.to_owned(),
                values(&s["values"], &p, target_limit)?,
            )
            .is_some()
        {
            return Err("duplicate playback".into());
        }
    }
    let mut masters = BTreeSet::new();
    for m in array(&inv["fixture_masters"], p.fixtures.len())? {
        schema(m, &["fixture", "level"])?;
        let fid = id(&m["fixture"])?;
        if !p.fixtures.contains(fid) || !masters.insert(fid) {
            return Err("fixture master".into());
        }
        level(&m["level"])?;
    }
    let s = &v["snapshot"];
    schema(
        s,
        &[
            "durability",
            "epoch",
            "fixtures",
            "patch_revision",
            "revision",
            "show_id",
            "version",
        ],
    )?;
    if text(&s["show_id"])? != expected_show
        || counter(&s["epoch"])? != epoch
        || counter(&s["revision"])? != revision
        || counter(&s["patch_revision"])? != p.revision
        || integer(&s["version"])? != 1
    {
        return Err("snapshot identity/revision/version".into());
    }
    if !matches!(
        text(&s["durability"])?,
        "volatile" | "checkpointed" | "error"
    ) {
        return Err("unknown durability".into());
    }
    if !durable {
        exact(&s["durability"], "volatile")?;
    }
    let mut transition_values = Look::new();
    if timed {
        schema(
            &v["release"],
            &[
                "transition_ms",
                "preview_validity_ms",
                "transition",
                "preview_available",
            ],
        )?;
        if integer(&v["release"]["transition_ms"])? != 500
            || integer(&v["release"]["preview_validity_ms"])? != 2000
        {
            return Err("release timing capability".into());
        }
        boolean(&v["release"]["preview_available"])?;
        if !v["release"]["transition"].is_null() {
            validate_transition(&v["release"]["transition"], v)?;
            for t in v["release"]["transition"]["targets"].as_array().unwrap() {
                transition_values.insert(
                    (text(&t["fixture"])?.into(), text(&t["attribute"])?.into()),
                    integer(&t["current"])?,
                );
            }
        }
    }
    let mut releases = Look::new();
    let mut fixtures = BTreeSet::new();
    let (mut programmer, mut holds, mut proposals) = (Look::new(), Look::new(), Look::new());
    for f in array(&s["fixtures"], p.fixtures.len())? {
        schema(f, &["fixture", "attributes"])?;
        let fid = id(&f["fixture"])?;
        if !p.fixtures.contains(fid) || !fixtures.insert(fid) {
            return Err("snapshot fixture".into());
        }
        let mut attrs = BTreeSet::new();
        for a in array(&f["attributes"], 7)? {
            let mut attribute_keys = vec![
                "attribute",
                "clamped",
                "contributors",
                "final_intent",
                "hold",
                "inhibit",
                "observed",
                "playing",
                "programmer",
                "proposal",
                "resolved",
                "source",
                "stored",
                "submitted",
            ];
            if a.get("release").is_some() {
                if !timed {
                    return Err("release unavailable in static schema".into());
                }
                attribute_keys.push("release");
            }
            if a.get("auto").is_some() {
                if !analysis || a["attribute"] != "intensity" {
                    return Err("AUTO outside LX05 intensity".into());
                }
                attribute_keys.push("auto");
            }
            schema(a, &attribute_keys)?;
            let name = text(&a["attribute"])?;
            let t = (fid.to_owned(), name.to_owned());
            let cap = p.caps.get(&t).ok_or("snapshot unsupported attribute")?;
            if !attrs.insert(name) {
                return Err("duplicate snapshot attribute".into());
            }
            for (key, map) in [
                ("programmer", &mut programmer),
                ("hold", &mut holds),
                ("proposal", &mut proposals),
            ] {
                if !a[key].is_null() {
                    map.insert(t.clone(), cap.value(&a[key])?);
                }
            }
            if a.get("release").is_some_and(|x| !x.is_null()) {
                releases.insert(t.clone(), cap.value(&a["release"])?);
            }
            let auto = automatic.get(fid);
            if name == "intensity" {
                match (a.get("auto"), auto) {
                    (Some(value), Some((n, _))) if cap.value(value)? == *n => {
                        automatic_seen.insert(fid.to_owned());
                    }
                    (None, None) => {}
                    _ => return Err("AUTO layer/value mismatch".into()),
                }
            }
            cap.value(&a["resolved"])?;
            cap.value(&a["final_intent"])?;
            boolean(&a["clamped"])?;
            if !matches!(
                text(&a["inhibit"])?,
                "none" | "blackout" | "protection" | "unavailable"
            ) {
                return Err("unknown inhibit".into());
            }
            if !a["submitted"].is_null() || !a["observed"].is_null() {
                return Err("null sink cannot claim submitted/observed".into());
            }
            let mut stored = BTreeMap::new();
            for sv in array(&a["stored"], 64)? {
                schema(sv, &["kind", "id", "value"])?;
                let kind = text(&sv["kind"])?;
                let sid = id(&sv["id"])?;
                let map = match kind {
                    "cue" => &cues,
                    "palette" => &palettes,
                    _ => return Err("stored kind".into()),
                };
                let n = cap.value(&sv["value"])?;
                if map.get(sid).and_then(|l| l.get(&t)) != Some(&n)
                    || stored.insert((kind, sid), n).is_some()
                {
                    return Err("stored provenance mismatch".into());
                }
            }
            let expected_stored = cues
                .values()
                .chain(palettes.values())
                .filter(|l| l.contains_key(&t))
                .count();
            if stored.len() != expected_stored {
                return Err("incomplete stored provenance".into());
            }
            let mut pb = BTreeMap::new();
            for x in array(&a["playing"], 8)? {
                schema(x, &["id", "value"])?;
                let pid = id(&x["id"])?;
                let n = cap.value(&x["value"])?;
                if pb.insert(pid, n).is_some()
                    || !playing.get(pid).is_some_and(|l| l.contains_key(&t))
                    || (name != "intensity" && playing.get(pid).and_then(|l| l.get(&t)) != Some(&n))
                {
                    return Err("playing provenance mismatch".into());
                }
            }
            if pb.len() != playing.values().filter(|l| l.contains_key(&t)).count() {
                return Err("incomplete playing provenance".into());
            }
            let chosen = source(&a["source"], analysis)?;
            let mut contributors = BTreeSet::new();
            let mut winners = 0;
            let mut chosen_wins = false;
            for c in array(&a["contributors"], if analysis { 12 } else { 11 })? {
                schema(c, &["source", "value", "winner"])?;
                let src = source(&c["source"], analysis)?;
                let value = cap.value(&c["value"])?;
                let exists = match src.0.as_str() {
                    "fixture_default" => value == cap.default,
                    "programmer" => programmer.get(&t) == Some(&value),
                    "hold" => holds.get(&t) == Some(&value),
                    "release" => releases.get(&t) == Some(&value),
                    "auto" => {
                        name == "intensity"
                            && auto.is_some_and(|(n, reason)| {
                                *n == value && src.1.as_deref() == Some(reason.as_str())
                            })
                    }
                    "playback" => {
                        src.1.as_ref().and_then(|pid| pb.get(pid.as_str())) == Some(&value)
                    }
                    _ => false,
                };
                if !exists || !contributors.insert(src.clone()) {
                    return Err("contributor reference/value".into());
                }
                if boolean(&c["winner"])? {
                    winners += 1;
                    chosen_wins |= src == chosen;
                    if value != integer(&a["resolved"])? {
                        return Err("winner resolved mismatch".into());
                    }
                }
            }
            let mut expected = BTreeSet::new();
            for (name, look) in [
                ("programmer", &programmer),
                ("hold", &holds),
                ("release", &releases),
            ] {
                if look.contains_key(&t) {
                    expected.insert((name.to_owned(), None));
                }
            }
            if name == "intensity"
                && let Some((_, reason)) = auto
            {
                expected.insert(("auto".into(), Some(reason.clone())));
            }
            for pid in pb.keys() {
                expected.insert(("playback".to_owned(), Some((*pid).to_owned())));
            }
            if expected.is_empty() {
                expected.insert(("fixture_default".to_owned(), None));
            }
            if contributors != expected {
                return Err("incomplete/unexpected contributors".into());
            }
            if winners == 0 || !chosen_wins || !contributors.contains(&chosen) {
                return Err("missing winning source".into());
            }
        }
        if attrs.len() != p.caps.keys().filter(|t| t.0 == fid).count() {
            return Err("incomplete snapshot attributes".into());
        }
    }
    if fixtures.len() != p.fixtures.len() {
        return Err("incomplete fixture snapshot".into());
    }
    if automatic_seen.len() != automatic.len() {
        return Err("incomplete AUTO layer".into());
    }
    if releases != transition_values {
        return Err("transition/release current mask mismatch".into());
    }
    coherent(&releases, &p)?;
    coherent(&programmer, &p)?;
    coherent(&holds, &p)?;
    coherent(&proposals, &p)?;
    Ok(())
}

struct Pending {
    sequence: u64,
    revision: u64,
    tick: u64,
    deadline: u64,
    chunks: Vec<Option<String>>,
}
/// Logical cached-state freshness, independent of physical output knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unavailable,
}
/// Complete snapshot receipts expire after two seconds without a validated refresh.
pub const SNAPSHOT_TTL_MS: u64 = 2000;

/// Last complete trusted read-only display. Invalid pages never replace it.
pub struct LuxClient {
    show: String,
    epoch: u64,
    pending: Option<Pending>,
    trusted: Option<Value>,
    sequence: u64,
    revision: u64,
    patch_revision: u64,
    last_ms: u64,
    freshness: Freshness,
    fresh_until: Option<u64>,
}
impl LuxClient {
    pub fn new(show_id: &str, epoch: u64) -> Result<Self> {
        show(show_id)?;
        if epoch == 0 || epoch == u64::MAX {
            return Err("invalid expected epoch".into());
        }
        Ok(Self {
            show: show_id.into(),
            epoch,
            pending: None,
            trusted: None,
            sequence: 0,
            revision: 0,
            patch_revision: 0,
            last_ms: 0,
            freshness: Freshness::Unavailable,
            fresh_until: None,
        })
    }
    pub fn snapshot(&self) -> Option<&Value> {
        self.trusted.as_ref()
    }
    pub fn freshness(&self) -> Freshness {
        self.freshness
    }
    /// Link loss and read timeouts retain the last complete values as stale.
    pub fn disconnect(&mut self) {
        self.pending = None;
        self.fresh_until = None;
        self.freshness = if self.trusted.is_some() {
            Freshness::Stale
        } else {
            Freshness::Unavailable
        };
    }
    pub fn timeout(&mut self) {
        self.disconnect();
    }
    /// Call from the client event clock even when no frames arrive.
    pub fn advance(&mut self, now_ms: u64) -> Result<()> {
        if now_ms < self.last_ms {
            self.disconnect();
            return Err("receipt clock regression".into());
        }
        self.last_ms = now_ms;
        if self.pending.as_ref().is_some_and(|p| now_ms >= p.deadline)
            || self.fresh_until.is_some_and(|deadline| now_ms >= deadline)
        {
            self.disconnect();
        }
        Ok(())
    }
    pub fn finish(&mut self) -> Result<()> {
        if self.pending.take().is_some() {
            self.disconnect();
            return Err("incomplete snapshot pages".into());
        }
        if self.trusted.is_none() {
            return Err("no trusted snapshot".into());
        }
        Ok(())
    }
    pub fn ingest(&mut self, bytes: &[u8], now_ms: u64) -> Result<bool> {
        let r = self.page(bytes, now_ms);
        if r.is_err() {
            self.disconnect();
        }
        r
    }
    fn page(&mut self, bytes: &[u8], now_ms: u64) -> Result<bool> {
        if self.pending.as_ref().is_some_and(|p| now_ms >= p.deadline) {
            return Err("expired snapshot pages".into());
        }
        self.advance(now_ms)?;
        let v = codec::json(bytes, MESSAGE_BYTES)?;
        schema(
            &v,
            &[
                "body",
                "contract",
                "effective_tick",
                "epoch",
                "expected_revision",
                "kind",
                "lease",
                "module",
                "reason",
                "request_id",
                "revision",
                "sequence",
                "show_id",
                "version",
                "writer",
            ],
        )?;
        exact(&v["contract"], "C-LIGHT")?;
        exact(&v["module"], "lighting")?;
        exact(&v["kind"], "snapshot")?;
        if integer(&v["version"])? != 1
            || text(&v["show_id"])? != self.show
            || counter(&v["epoch"])? != self.epoch
        {
            return Err("wrong show/epoch/version".into());
        }
        for k in [
            "expected_revision",
            "lease",
            "reason",
            "request_id",
            "writer",
        ] {
            if !v[k].is_null() {
                return Err("read-only observation correlation".into());
            }
        }
        let (sequence, revision, tick) = (
            counter(&v["sequence"])?,
            counter(&v["revision"])?,
            counter(&v["effective_tick"])?,
        );
        if sequence <= self.sequence || revision < self.revision {
            return Err("older snapshot sequence/revision".into());
        }
        let b = &v["body"];
        schema(b, &["chunk", "encoding", "page", "page_count"])?;
        exact(&b["encoding"], "json_utf8_chunks")?;
        let (page, count) = (integer(&b["page"])?, integer(&b["page_count"])?);
        if !(1..=MAX_PAGES as i32).contains(&count) || page < 0 || page >= count {
            return Err("page index/count".into());
        }
        let chunk = text(&b["chunk"])?;
        if let Some(p) = &self.pending {
            if now_ms >= p.deadline
                || p.sequence != sequence
                || p.revision != revision
                || p.tick != tick
                || p.chunks.len() != count as usize
            {
                return Err("expired/mixed snapshot pages".into());
            }
        } else {
            self.pending = Some(Pending {
                sequence,
                revision,
                tick,
                deadline: now_ms
                    .checked_add(2000)
                    .ok_or("receipt deadline exhausted")?,
                chunks: vec![None; count as usize],
            });
        }
        let p = self.pending.as_mut().unwrap();
        if p.chunks[page as usize].is_some() {
            return Err("duplicate snapshot page".into());
        }
        p.chunks[page as usize] = Some(chunk.into());
        if p.chunks.iter().any(Option::is_none) {
            self.freshness = if self.trusted.is_some() {
                Freshness::Stale
            } else {
                Freshness::Unavailable
            };
            self.fresh_until = None;
            return Ok(false);
        }
        let p = self.pending.take().unwrap();
        let joined = p.chunks.into_iter().map(Option::unwrap).collect::<String>();
        let candidate = codec::json(joined.as_bytes(), MAX_PAGES * MESSAGE_BYTES)?;
        inventory(&candidate, &self.show, self.epoch, revision)?;
        let patch_revision = counter(&candidate["patch"]["patch_revision"])?;
        if patch_revision < self.patch_revision {
            return Err("older patch revision".into());
        }
        if integer(&candidate["limits"]["pages"])? < count
            || integer(&candidate["limits"]["message_bytes"])? < (bytes.len() as i32)
        {
            return Err("advertised page capacity".into());
        }
        self.sequence = sequence;
        self.revision = revision;
        self.patch_revision = patch_revision;
        let fresh_until = now_ms
            .checked_add(SNAPSHOT_TTL_MS)
            .ok_or("freshness deadline exhausted")?;
        self.trusted = Some(candidate);
        self.fresh_until = Some(fresh_until);
        self.freshness = Freshness::Fresh;
        Ok(true)
    }
    pub fn presentation(&self) -> Result<String> {
        let Some(v) = self.trusted.as_ref() else {
            return Ok("LUX READ-ONLY / cached state Unavailable / physical UNKNOWN\nno complete compatible snapshot\n".into());
        };
        let mut out = format!(
            "LUX READ-ONLY / show {} / epoch {} / revision {} / patch {} / sequence {}\nmode {} / durability {} / output {} / physical UNKNOWN\n",
            self.show,
            self.epoch,
            self.revision,
            self.patch_revision,
            self.sequence,
            v["mode"],
            v["snapshot"]["durability"],
            v["output"]
        );
        out.push_str(&format!("cached state {:?} / {} / physical UNKNOWN\n", self.freshness,
            if self.freshness == Freshness::Fresh { "complete compatible snapshot at receipt; expires after 2000ms; no live subscription" }
            else { "retained last-known values; current complete state unavailable" }));
        if let Some(schema) = v.get("wire_schema") {
            out.push_str(&format!(
                "wire schema {} / engine transition {} / checkpoint capability {}\n",
                schema, v["release"]["transition"], v["checkpoint"]
            ));
        }
        if let Some(a) = v.get("analysis") {
            out.push_str(&format!("analysis {} / current confidence {} / proposal {} / retained AUTO {} / beat downbeat harmony unavailable\n", a["state"], a["confidence"], a["proposal"], a["automatic_layer"]));
        }
        out.push_str(&format!(
            "master {} / blackout {} / grants {}\nstored and playing inventory {}\n",
            v["master"], v["blackout"], v["grants"], v["authority_inventory"]
        ));
        for f in v["snapshot"]["fixtures"].as_array().unwrap() {
            for a in f["attributes"].as_array().unwrap() {
                let fid = f["fixture"].as_str().unwrap();
                let attr = a["attribute"].as_str().unwrap();
                let m = v["capability_metadata"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|m| m["fixture"] == f["fixture"])
                    .unwrap();
                let cap = m["attributes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["attribute"] == a["attribute"])
                    .unwrap();
                if let Some(release) = a.get("release") {
                    out.push_str(&format!(
                        "{fid} {attr} engine release current {release} / distinct from human Hold\n"
                    ));
                }
                out.push_str(&format!("{fid} {attr} [{} {}..{}] programmer {} / Hold {} / stored {} / playing {} / proposal {} / resolved {} source {} / contributors {} / inhibit {} clamp {} / final_intent {} / submitted UNKNOWN / observed UNKNOWN\n",cap["unit"],cap["min"],cap["max"],a["programmer"],a["hold"],a["stored"],a["playing"],a["proposal"],a["resolved"],a["source"],a["contributors"],a["inhibit"],a["clamped"],a["final_intent"]));
            }
        }
        Ok(out)
    }
}

/// Explicit read-only file mode: framed producer bytes, at most one bounded response.
pub fn snapshot_file(path: &std::path::Path, show: &str, epoch: u64) -> Result<String> {
    let mut reader = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut client = LuxClient::new(show, epoch)?;
    let start = std::time::Instant::now();
    let mut frames = 0;
    while let Some(bytes) = codec::read_frame(&mut reader)? {
        frames += 1;
        if frames > MAX_PAGES {
            return Err("file page capacity".into());
        }
        client.ingest(&bytes, start.elapsed().as_millis() as u64)?;
    }
    client.finish()?;
    client.presentation()
}
#[cfg(target_os = "linux")]
pub(crate) fn private_connect(path: &std::path::Path) -> Result<std::os::unix::net::UnixStream> {
    use std::os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            ffi::OsStrExt,
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    };
    let parent = path.parent().ok_or("socket parent")?;
    let canonical = parent.canonicalize().map_err(|e| e.to_string())?;
    if canonical != parent || !path.is_absolute() {
        return Err("socket needs canonical absolute private path".into());
    }
    // Linux credentials are queried only for this explicit private connection.
    let uid = unsafe { libc::geteuid() };
    let pm = std::fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    let sm = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !pm.is_dir()
        || pm.uid() != uid
        || pm.mode() & 0o777 != 0o700
        || !sm.file_type().is_socket()
        || sm.uid() != uid
        || sm.mode() & 0o777 != 0o600
    {
        return Err("socket private ownership/mode".into());
    }
    // Nonblocking connect refuses a full backlog rather than waiting indefinitely.
    let name = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if name.len() >= address.sun_path.len() || name.contains(&0) {
        return Err("socket path capacity".into());
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (out, byte) in address.sun_path.iter_mut().zip(name) {
        *out = *byte as libc::c_char;
    }
    let raw = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
        )
    };
    if raw < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    let connected = unsafe {
        libc::connect(
            fd.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as libc::socklen_t,
        )
    };
    if connected != 0 {
        return Err(format!(
            "private socket unavailable/busy: {}",
            std::io::Error::last_os_error()
        ));
    }
    let stream = UnixStream::from(fd);
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // Pointers refer to live, correctly sized local credential storage.
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut size,
        )
    };
    if rc != 0 || size as usize != std::mem::size_of::<libc::ucred>() || cred.uid != uid {
        return Err("socket peer UID".into());
    }
    let after = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if (after.dev(), after.ino()) != (sm.dev(), sm.ino()) {
        return Err("socket replaced during connection".into());
    }
    Ok(stream)
}
#[cfg(target_os = "linux")]
pub fn snapshot_socket(path: &std::path::Path, show: &str, epoch: u64) -> Result<String> {
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
        time::{Duration, Instant},
    };
    let mut stream = private_connect(path)?;
    stream
        .set_write_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| e.to_string())?;
    let mut client = LuxClient::new(show, epoch)?;
    let request=serde_json::to_vec(&serde_json::json!({"contract":"C-LIGHT","version":1,"show_id":show,"module":"lighting","epoch":epoch.to_string(),"writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{}})).map_err(|e|e.to_string())?;
    let mut framed = (request.len() as u32).to_be_bytes().to_vec();
    framed.extend_from_slice(&request);
    let write_deadline = Instant::now() + Duration::from_millis(500);
    let mut sent = 0;
    while sent < framed.len() {
        let remaining = write_deadline
            .checked_duration_since(Instant::now())
            .filter(|t| !t.is_zero())
            .ok_or("write frame deadline")?;
        stream
            .set_write_timeout(Some(remaining))
            .map_err(|e| e.to_string())?;
        let n = stream.write(&framed[sent..]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("incomplete provider request write".into());
        }
        sent += n;
    }
    struct Deadline<'a> {
        stream: &'a mut UnixStream,
        until: Instant,
    }
    impl Read for Deadline<'_> {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            let remaining = self
                .until
                .checked_duration_since(Instant::now())
                .filter(|t| !t.is_zero())
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::TimedOut, "frame deadline")
                })?;
            self.stream.set_read_timeout(Some(remaining))?;
            self.stream.read(b)
        }
    }
    let start = Instant::now();
    for _ in 0..MAX_PAGES {
        let mut bounded = Deadline {
            stream: &mut stream,
            until: Instant::now() + Duration::from_millis(500),
        };
        let bytes = codec::read_frame(&mut bounded)?.ok_or("incomplete provider response")?;
        if client.ingest(&bytes, start.elapsed().as_millis() as u64)? {
            client.finish()?;
            return client.presentation();
        }
    }
    Err("incomplete provider page capacity".into())
}

/// Validate engine transition metadata and target identities/ranges, without
/// computing interpolation or a release destination.
pub(crate) fn validate_transition(v: &Value, inventory: &Value) -> Result<()> {
    schema(
        v,
        &[
            "start_tick",
            "end_tick",
            "current_tick",
            "progress_ticks",
            "duration_ticks",
            "targets",
            "physical",
        ],
    )?;
    exact(&v["physical"], "unknown")?;
    let start = counter(&v["start_tick"])?;
    let end = counter(&v["end_tick"])?;
    let current = counter(&v["current_tick"])?;
    let duration = integer(&v["duration_ticks"])?;
    let progress = integer(&v["progress_ticks"])?;
    if duration != 50
        || end.checked_sub(start) != Some(50)
        || current < start
        || current >= end
        || current - start != progress as u64
        || !(0..50).contains(&progress)
    {
        return Err("transition tick identity/range".into());
    }
    let mut targets = BTreeSet::new();
    let ts = array(&v["targets"], 64)?;
    if ts.is_empty() {
        return Err("empty transition".into());
    }
    for t in ts {
        schema(t, &["fixture", "attribute", "current", "start", "target"])?;
        let fid = id(&t["fixture"])?;
        let attr = text(&t["attribute"])?;
        if !targets.insert((fid, attr)) {
            return Err("duplicate transition target".into());
        }
        for key in ["current", "start", "target"] {
            advertised_value(inventory, fid, attr, &t[key])?;
        }
    }
    Ok(())
}
pub(crate) fn advertised_value(
    inventory: &Value,
    fid: &str,
    attr: &str,
    value: &Value,
) -> Result<i32> {
    let metadata = array(&inventory["capability_metadata"], 32)?;
    let m = metadata
        .iter()
        .find(|m| m["fixture"] == fid)
        .ok_or("unknown fixture")?;
    let cs = array(&m["attributes"], 7)?;
    let c = cs
        .iter()
        .find(|c| c["attribute"] == attr)
        .ok_or("unknown attribute")?;
    let n = integer(value)?;
    if n < integer(&c["min"])? || n > integer(&c["max"])? {
        return Err("advertised value range".into());
    }
    Ok(n)
}
