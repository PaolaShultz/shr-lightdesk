#![cfg(target_os = "linux")]
use shr_lightdesk::{
    frontend::{View, scene},
    render::{self, Primitive},
};
fn accepted() -> serde_json::Value {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/lx04/durable-commands.json")).unwrap();
    corpus["after_late_tick"]["inventory"].clone()
}
fn text(s: &render::Scene) -> String {
    s.primitives
        .iter()
        .filter_map(|p| {
            if let Primitive::Text { value, .. } = p {
                Some(value.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn real_inventory_raster_retains_hold_and_release_sources() {
    let mut v = View {
        inventory: Some(accepted()),
        freshness: "Fresh".into(),
        page: "stage".into(),
        selected: vec!["fixture-11".into()],
        ..Default::default()
    };
    let s = scene(&v, "touch intensity 0", 0);
    assert!(s.in_bounds());
    let t = text(&s);
    for expected in [
        "REAL LUX",
        "PHYSICAL UNKNOWN",
        "programmer",
        "Hold",
        "playing",
        "engine current",
        "final intent",
        "intensity",
        "fixture-11",
    ] {
        assert!(t.contains(expected), "{expected}: {t}");
    }
    let ppm = render::ppm(&s);
    assert!(ppm.starts_with(b"P6\n1920 1080\n255\n"));
    assert_eq!(ppm.len(), 1920 * 1080 * 3 + 17);
    v.freshness = "Stale".into();
    let stale = scene(&v, "", 0);
    assert!(text(&stale).contains("Logical Stale"));
    assert!(text(&stale).contains("fixture-11"));
}
#[test]
fn unavailable_and_all_real_pages_scroll_with_font_bounds() {
    let mut v = View {
        freshness: "Unavailable".into(),
        ..Default::default()
    };
    assert!(text(&scene(&v, "", 0)).contains("No complete compatible"));
    v.inventory = Some(accepted());
    for page in ["stage", "programmer", "library", "playbacks", "health"] {
        v.page = page.into();
        for scroll in [0, 20, usize::MAX] {
            let s = scene(&v, &"x".repeat(256), scroll);
            assert!(s.in_bounds());
            let font = render::Font::default();
            for p in s.primitives {
                if let Primitive::Text { value, .. } = p {
                    assert!(value.chars().all(|c| font.supports(c)));
                }
            }
        }
    }
}
#[test]
fn stalled_provider_view_expires_review_and_writer_but_keeps_values() {
    let v = View {
        inventory: Some(accepted()),
        freshness: "Fresh".into(),
        lease: true,
        review: Some(serde_json::json!({"action":"blackout","enabled":false})),
        updated_at: Some(std::time::Instant::now() - std::time::Duration::from_millis(2001)),
        ..Default::default()
    };
    let current = v.current();
    assert_eq!(current.freshness, "Stale");
    assert!(!current.lease);
    assert!(current.review.is_none());
    assert_eq!(current.inventory, v.inventory);
}
#[test]
fn provider_identity_and_large_exact_modal_are_visible_without_truncation() {
    let inventory = accepted();
    let show = inventory["snapshot"]["show_id"].as_str().unwrap();
    let mut v = View {
        inventory: Some(inventory.clone()),
        freshness: "Fresh".into(),
        page: "stage".into(),
        ..Default::default()
    };
    let t = text(&scene(&v, "", 0));
    assert!(t.contains(show));
    assert!(t.contains(&format!(
        "epoch {}",
        inventory["snapshot"]["epoch"].as_str().unwrap()
    )));
    let values:Vec<_>=(0..64).map(|n|serde_json::json!({"fixture":format!("fixture-{n}"),"attribute":"intensity","current":0,"destination":700})).collect();
    v.review =
        Some(serde_json::json!({"action":"release_commit","token":{"preview":{"values":values}}}));
    let all = shr_lightdesk::frontend::review_lines(v.review.as_ref().unwrap());
    assert!(all.len() > 26);
    let mut visible = String::new();
    for start in (0..all.len()).step_by(20) {
        visible.push_str(&text(&scene(&v, "", start)));
    }
    for n in 0..64 {
        assert!(
            visible.contains(&format!("fixture-{n}")),
            "missing target{n}"
        );
    }
    assert!(visible.contains("destination"));
}
#[cfg(feature = "native")]
#[test]
fn native_viewport_preserves_aspect_portrait_narrow_minimized_and_integer_scale() {
    for (w, h) in [
        (1920, 1080),
        (3840, 2160),
        (1080, 1920),
        (240, 720),
        (1280, 720),
    ] {
        let (x, y, vw, vh) = shr_lightdesk::native::viewport(w, h).unwrap();
        assert!(x >= 0. && y >= 0. && vw + x <= w as f32 + 0.1 && vh + y <= h as f32 + 0.1);
        assert!((vw / vh - 16. / 9.).abs() < 0.001);
    }
    assert!(shr_lightdesk::native::viewport(0, 720).is_none());
    assert!(shr_lightdesk::native::viewport(1280, 0).is_none());
    assert_eq!(
        shr_lightdesk::native::viewport(3840, 2160).unwrap(),
        (0., 0., 3840., 2160.)
    );
}
#[test]
fn review_requires_every_contiguous_target_page_for_exact_context() {
    use shr_lightdesk::native_actions::ReviewProgress;
    let mut progress = ReviewProgress::default();
    progress.see(1, 90, 100, 100);
    assert!(!progress.complete(1, 100));
    progress.see(1, 0, 26, 100);
    progress.see(1, 20, 46, 100);
    progress.see(1, 60, 86, 100);
    assert!(!progress.complete(1, 100));
    progress.see(1, 40, 66, 100);
    progress.see(1, 80, 100, 100);
    assert!(progress.complete(1, 100));
    progress.see(2, 90, 100, 100);
    assert!(!progress.complete(2, 100));
    assert!(!progress.complete(1, 100));
    // Failed/timeout/suspended frames never invoke see, so cannot advance review.
    let failed = ReviewProgress::default();
    assert!(!failed.complete(3, 10));
}

#[test]
fn material_review_keeps_every_target_units_bounds_context_and_hides_opaque_tokens() {
    use shr_lightdesk::{frontend::material_review_lines, native_actions::ReviewProgress};
    let inventory = accepted();
    let values: Vec<_> = (0..64)
        .map(|n| {
            serde_json::json!({
                "fixture": format!("fixture-{n}"), "attribute": "pan", "current": -155,
                "destination": 2700
            })
        })
        .collect();
    let command = serde_json::json!({"action":"release_commit", "token":{
        "scope":"lighting-control", "lease":"PRIVATE-LEASE", "nonce":"PRIVATE-NONCE",
        "preview":{"show_id":inventory["snapshot"]["show_id"],"epoch":"9","revision":"55",
            "patch_revision":"2","transition_ms":500,"validity_ms":2000,"expiry_tick":"123",
            "values":values}
    }});
    let lines = material_review_lines(&command, &inventory, 9, 55, 4000);
    let all = lines.join("\n");
    for expected in [
        "lighting-control",
        "Epoch 9 / revision 55",
        "expires at session 4000 ms",
        "Transition 500 ms",
        "expiry engine tick 123",
        "-15.5 deg -> destination 270 deg",
    ] {
        assert!(all.contains(expected), "{expected}: {all}");
    }
    assert!(!all.contains("PRIVATE-NONCE"));
    assert!(!all.contains("PRIVATE-LEASE"));
    let view = View {
        review: Some(command),
        review_context: Some(7),
        review_text: lines.clone(),
        ..Default::default()
    };
    let mut visible = String::new();
    let mut progress = ReviewProgress::default();
    // Seeing the last page alone must not certify any omitted context or target.
    progress.see(7, lines.len() - 1, lines.len(), lines.len());
    assert!(!progress.complete(7, lines.len()));
    for start in (0..lines.len()).step_by(20) {
        visible.push_str(&text(&scene(&view, "", start)));
        progress.see(7, start, (start + 26).min(lines.len()), lines.len());
    }
    for line in &lines {
        assert!(visible.contains(line), "unpresented material: {line}");
    }
    for n in 0..64 {
        assert!(visible.contains(&format!("fixture-{n} pan:")));
    }
    assert!(progress.complete(7, lines.len()));
    assert!(!progress.complete(8, lines.len()));
    let touch = serde_json::json!({"action":"touch","values":[{"fixture":"fixture-11","attribute":"intensity","value":0}]});
    let touch_lines = material_review_lines(&touch, &inventory, 9, 55, 4000).join("\n");
    assert!(touch_lines.contains("destination 0%"));
    assert!(touch_lines.contains("allowed 0%..100%"));
}

#[test]
fn material_review_names_overwrites_global_scope_and_complete_notices() {
    use shr_lightdesk::frontend::material_review_lines;
    let mut inventory = accepted();
    inventory["authority_inventory"]["cues"] = serde_json::json!([{"id":"look-a","values":[{"fixture":"fixture-11","attribute":"intensity","value":700}]}]);
    let update = serde_json::json!({"action":"update","kind":"cue","id":"look-a"});
    let lines = material_review_lines(&update, &inventory, 9, 5, 2000).join("\n");
    assert!(lines.contains("Update cue look-a"));
    assert!(lines.contains("fixture selection does not limit"));
    assert!(lines.contains("fixture-11 intensity: stored 70% / retained unchanged"));
    let blackout = material_review_lines(
        &serde_json::json!({"action":"blackout","enabled":false}),
        &inventory,
        9,
        5,
        2000,
    )
    .join("\n");
    assert!(blackout.contains("OFF - output intent resumes"));
    assert!(blackout.contains("Scope: all engine fixture intensities"));
    let notice = format!("{} END-OF-NOTICE", "long operator notice ".repeat(40));
    let view = View {
        notice,
        ..Default::default()
    };
    let visible: String = (0..15).map(|p| text(&scene(&view, "", p * 20))).collect();
    assert!(visible.contains("END-OF-NOTICE"));
}

#[test]
fn review_expiry_and_notice_only_page_never_count_as_presented_material() {
    use shr_lightdesk::frontend::visible_review_range;
    let mut view = View {
        review: Some(serde_json::json!({"action":"blackout","enabled":false})),
        review_context: Some(42),
        review_text: vec!["Material context".into(), "Blackout OFF".into()],
        notice: format!("{} END-OF-NOTICE", "long notice ".repeat(80)),
        ..Default::default()
    };
    assert_eq!(visible_review_range(&view, 0), Some((42, 0, 2)));
    assert!(visible_review_range(&view, 20).is_none());
    let visible: String = (0..10).map(|p| text(&scene(&view, "", p * 20))).collect();
    assert!(visible.contains("END-OF-NOTICE"));
    view.review_deadline = Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
    let expired = view.current();
    assert!(expired.review.is_none());
    assert!(visible_review_range(&expired, 0).is_none());
}

#[test]
fn go_review_covers_replaced_playback_union_and_update_copies_only_programmer() {
    use serde_json::json;
    use shr_lightdesk::frontend::material_review_lines;
    let mut inventory = accepted();
    let attrs = inventory["snapshot"]["fixtures"][0]["attributes"]
        .as_array_mut()
        .unwrap();
    if !attrs.iter().any(|a| a["attribute"] == "red") {
        let mut red = attrs[0].clone();
        red["attribute"] = json!("red");
        attrs.push(red);
    }
    let intensity = attrs
        .iter_mut()
        .find(|a| a["attribute"] == "intensity")
        .unwrap();
    intensity["resolved"] = json!(0);
    intensity["source"] = json!("hold");
    intensity["programmer"] = json!(400);
    intensity["hold"] = json!(900);
    intensity["playing"] = json!([{"id":"playback-a","value":500}]);
    let red = attrs.iter_mut().find(|a| a["attribute"] == "red").unwrap();
    red["programmer"] = json!(null);
    red["playing"] = json!([{"id":"playback-a","value":150}]);
    inventory["authority_inventory"]["cues"] = json!([{"id":"look-a","values":[{"fixture":"fixture-11","attribute":"intensity","value":700}]}]);
    inventory["authority_inventory"]["playbacks"] = json!([{"id":"playback-a","level":500,"values":[{"fixture":"fixture-11","attribute":"intensity","value":500},{"fixture":"fixture-11","attribute":"red","value":300}]}]);
    let go = material_review_lines(
        &json!({"action":"go","cue":"look-a","playback":"playback-a"}),
        &inventory,
        9,
        5,
        2000,
    )
    .join("\n");
    for expected in [
        "Replace playback playback-a / existing present",
        "current 0% from Hold -> requested cue 70%",
        "replaced playback stored 50% / current contribution 50%",
        "fixture-11 red:",
        "requested removal from playback",
        "removed stored 30% / current contribution 15%",
        "effective result awaits Lux",
    ] {
        assert!(go.contains(expected), "{expected}: {go}");
    }
    inventory["authority_inventory"]["cues"][0]["values"]
        .as_array_mut()
        .unwrap()
        .push(json!({"fixture":"fixture-11","attribute":"red","value":300}));
    let update = material_review_lines(
        &json!({"action":"update","kind":"cue","id":"look-a"}),
        &inventory,
        9,
        5,
        2000,
    )
    .join("\n");
    assert!(update.contains("stored 70% -> requested programmer 40%"));
    assert!(update.contains("red: stored 30% / retained unchanged"));
    assert!(!update.contains("requested programmer 90%"));
    let record = material_review_lines(
        &json!({"action":"record","kind":"cue","id":"new-look"}),
        &inventory,
        9,
        5,
        2000,
    )
    .join("\n");
    assert!(record.contains("stored none -> requested programmer 40%"));
    assert!(record.contains("Hold, playing and resolved values are not stored"));
    inventory["master"] = json!(1000);
    let master = material_review_lines(
        &json!({"action":"master","level":500}),
        &inventory,
        9,
        5,
        2000,
    )
    .join("\n");
    assert!(master.contains("Grand master 100% -> requested 50%"));
    assert!(master.contains("effective values await provider resolution"));
}

#[test]
fn operator_state_notice_stays_human_readable_outside_health_details() {
    let view = View {
        inventory: Some(accepted()),
        notice: "selection {\"fixture-11\"} / confirmation Some(Debug-token) / input release required falseLUX RAW JSON".into(),
        ..Default::default()
    };
    let visible = text(&scene(&view, "", 0));
    assert!(visible.contains("raw protocol details are on Health"));
    assert!(!visible.contains("Debug-token"));
    assert!(!visible.contains("falseLUX"));
}
#[test]
fn lx05_retained_contribution_uses_original_provenance_and_human_units() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/lx05/features.json")).unwrap();
    let mut inventory = corpus["aged_loss"].clone();
    inventory["analysis"]["source_epoch"] = serde_json::json!("10");
    inventory["analysis"]["source_identity"]["source_epoch"] = serde_json::json!("10");
    let v = View {
        inventory: Some(inventory),
        freshness: "Fresh".into(),
        page: "health".into(),
        ..Default::default()
    };
    let s = scene(&v, "", 0);
    assert!(s.in_bounds());
    let t = text(&s);
    for expected in [
        "Analysis stale",
        "current confidence 0%",
        "retained contribution",
        "Current source epoch 10",
        "Contribution source epoch 9",
        "frames 100800..101280",
        "calibration 2",
        "confidence 100%",
        "Beat, downbeat and harmony unavailable",
    ] {
        assert!(t.contains(expected), "missing {expected}: {t}");
    }
    let stage = View {
        page: "stage".into(),
        ..v
    };
    let t = text(&scene(&stage, "", 0));
    assert!(!t.contains("rms_millionths"));
    assert!(!t.contains("source_identity"));
    assert!(!t.contains("frames 100800"));
    assert!(t.contains("AUTO analysis held"));
}
