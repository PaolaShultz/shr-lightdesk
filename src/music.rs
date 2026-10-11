//! Bounded Lux music metadata validation and presentation; no audio analysis here.
use crate::adapter::{array, integer, level, schema, text};
use crate::codec::counter;
use serde_json::Value;

type Result<T> = std::result::Result<T, String>;
fn pitch(v: &Value) -> Result<i32> {
    let n = integer(v)?;
    if !(0..12).contains(&n) {
        return Err("music pitch class".into());
    }
    Ok(n)
}
fn mode(v: &Value) -> Result<&str> {
    let s = text(v)?;
    if !matches!(s, "major" | "minor") {
        return Err("music mode".into());
    }
    Ok(s)
}
pub(crate) fn validate(a: &Value) -> Result<()> {
    let m = &a["musical"];
    schema(
        m,
        &[
            "state",
            "source_frame",
            "chroma",
            "chord",
            "key",
            "movement",
            "section",
        ],
    )?;
    let state = text(&m["state"])?;
    if !matches!(state, "unknown" | "tracking" | "stale") {
        return Err("music state".into());
    }
    let chroma = array(&m["chroma"], 12)?;
    if chroma.len() != 12 {
        return Err("music chroma count".into());
    }
    for n in chroma {
        level(n)?;
    }
    if (state == "stale") != (a["state"] == "stale") {
        return Err("music parent state mismatch".into());
    }
    if state != "tracking" {
        if !m["source_frame"].is_null()
            || chroma.iter().any(|n| n != 0)
            || ["chord", "key", "movement", "section"]
                .iter()
                .any(|k| !m[*k].is_null())
        {
            return Err("unavailable music has estimates".into());
        }
        return Ok(());
    }
    let frame = counter(&m["source_frame"])?;
    let end = counter(&a["source_window_range"]["end_frame_exclusive"])?;
    let first = counter(&a["source_identity"]["first_frame"])?;
    if a["state"] != "ready" || frame < first || frame > end || end - frame >= 6144 {
        return Err("music freshness/source mismatch".into());
    }
    let chord = &m["chord"];
    if !chord.is_null() {
        schema(chord, &["root", "quality", "confidence"])?;
        pitch(&chord["root"])?;
        mode(&chord["quality"])?;
        if level(&chord["confidence"])? == 0 {
            return Err("music chord confidence".into());
        }
    }
    let key = &m["key"];
    if !key.is_null() {
        schema(
            key,
            &[
                "tonic",
                "mode",
                "camelot_number",
                "camelot_letter",
                "confidence",
            ],
        )?;
        let tonic = pitch(&key["tonic"])?;
        let minor = mode(&key["mode"])? == "minor";
        // Validate the redundant DJ-wheel identity, not an estimated key.
        let number = (tonic * 7 + if minor { 4 } else { 7 }) % 12 + 1;
        if integer(&key["camelot_number"])? != number
            || key["camelot_letter"] != if minor { "A" } else { "B" }
        {
            return Err("music Camelot identity".into());
        }
        if level(&key["confidence"])? == 0 {
            return Err("music key confidence".into());
        }
    }
    let movement = &m["movement"];
    if !movement.is_null() {
        schema(movement, &["fifths_steps", "kind", "tension", "resolution"])?;
        let steps = integer(&movement["fifths_steps"])?;
        if !(-6..=6).contains(&steps) {
            return Err("music fifths range".into());
        }
        let kind = text(&movement["kind"])?;
        if !matches!(
            kind,
            "stationary" | "fifths_forward" | "fifths_backward" | "other"
        ) {
            return Err("music movement kind".into());
        }
        let expected = match steps {
            0 => "stationary",
            1 => "fifths_forward",
            -1 => "fifths_backward",
            _ => "other",
        };
        if kind != expected {
            return Err("music movement direction".into());
        }
        let tension = level(&movement["tension"])?;
        let resolution = level(&movement["resolution"])?;
        if key.is_null() && (tension != 0 || resolution != 0) {
            return Err("music function without key".into());
        }
    }
    let section = &m["section"];
    if !section.is_null() {
        schema(
            section,
            &["id", "novelty", "recurrence", "label", "confidence"],
        )?;
        section["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("music section id")?;
        for k in ["novelty", "recurrence", "confidence"] {
            level(&section[k])?;
        }
        if !matches!(
            text(&section["label"])?,
            "unknown" | "verse_candidate" | "chorus_candidate"
        ) {
            return Err("music section label".into());
        }
        if (section["label"] == "unknown") != (section["confidence"] == 0) {
            return Err("music section label confidence".into());
        }
    }
    Ok(())
}
fn note(v: &Value) -> &'static str {
    const NOTES: [&str; 12] = [
        "C", "C#/Db", "D", "D#/Eb", "E", "F", "F#/Gb", "G", "G#/Ab", "A", "A#/Bb", "B",
    ];
    v.as_u64()
        .and_then(|n| NOTES.get(n as usize))
        .copied()
        .unwrap_or("unknown")
}
fn confidence(v: &Value) -> String {
    let n = v.as_u64().unwrap_or(0);
    format!("{}.{:01}%", n / 10, n % 10)
}
/// Human-readable, provider-owned estimates. Call only for a validated inventory.
pub fn presentation(a: &Value) -> Vec<String> {
    let Some(m) = a.get("musical") else {
        return vec!["beat downbeat harmony unavailable".into()];
    };
    if m["state"] != "tracking" {
        return vec![format!(
            "Music {} / key, chord and section unavailable; beat/downbeat unavailable",
            m["state"].as_str().unwrap_or("unknown")
        )];
    }
    let mut lines = vec![format!(
        "Music tracking / source end frame {} / beat/downbeat unavailable",
        m["source_frame"].as_str().unwrap_or("unknown")
    )];
    let key = &m["key"];
    lines.push(if key.is_null() {
        "Key unknown / Camelot unavailable".into()
    } else {
        format!(
            "Key estimate {} {} / Camelot {}{} / confidence {}",
            note(&key["tonic"]),
            key["mode"].as_str().unwrap_or("unknown"),
            key["camelot_number"],
            key["camelot_letter"].as_str().unwrap_or("?"),
            confidence(&key["confidence"])
        )
    });
    let chord = &m["chord"];
    lines.push(if chord.is_null() {
        "Chord unknown".into()
    } else {
        format!(
            "Chord estimate {} {} / confidence {}",
            note(&chord["root"]),
            chord["quality"].as_str().unwrap_or("unknown"),
            confidence(&chord["confidence"])
        )
    });
    let movement = &m["movement"];
    if !movement.is_null() {
        lines.push(format!(
            "Harmonic motion {} / fifths steps {} / tension {} / resolution {}",
            movement["kind"]
                .as_str()
                .unwrap_or("unknown")
                .replace('_', " "),
            movement["fifths_steps"],
            confidence(&movement["tension"]),
            confidence(&movement["resolution"])
        ));
    }
    let section = &m["section"];
    lines.push(if section.is_null() {
        "Section unknown".into()
    } else {
        let label = match section["label"].as_str() {
            Some("verse_candidate") => "tentative verse",
            Some("chorus_candidate") => "tentative chorus",
            _ => "unlabeled",
        };
        format!(
            "Section {} / {} / confidence {} / novelty {} / recurrence {}",
            section["id"],
            label,
            confidence(&section["confidence"]),
            confidence(&section["novelty"]),
            confidence(&section["recurrence"])
        )
    });
    lines
}
