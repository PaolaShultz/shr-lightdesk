use serde_json::{Value, json};
use shr_lightdesk::{adapter::LuxClient, scenes};
fn fixture() -> Value {
    let mut i: Value =
        serde_json::from_str(include_str!("fixtures/musical/legacy-estimates.json")).unwrap();
    i = i["active"].clone();
    i["wire_schema"] = json!("lx06-scenes-v1");
    i["applied_auto"] = json!("explicit_bounded_scene_grant");
    let f = i["patch"]["fixtures"][0]["id"].clone();
    let s = json!({"id":"intimate","label":"Intimate","family":"intimate","mood":"warm","palette":"amber","lift":300,"roles":[{"role":"stage","rgb":[1000,300,100],"intensity":1000}],"conditions":{"energy":{"min":0,"max":1000},"density":{"min":0,"max":1000},"trend":{"min":-1000,"max":1000},"min_confidence":600},"compatible_from":[],"fade_ms":1200,"min_dwell_ms":2500,"confirm_ms":600,"variation":20,"repetition_penalty":80});
    i["scenes"] = json!({"map":{"version":1,"revision":"1","roles":[{"id":"stage","fixtures":[f]}],"scenes":[s]},"evidence":{"available":false,"energy":0,"density":0,"confidence":0,"section_id":null,"harmonic_relation":null},"interpretation":{"trend":0,"change":"unknown","uncertain":true},"current":null,"proposed":null,"reason":"waiting_for_evidence","transition":null,"intended_values":[],"variation":0,"grant":null,"automatic_layer":{"values":[]}});
    i
}
fn page(i: &Value, sequence: u64) -> Vec<u8> {
    serde_json::to_vec(&json!({"contract":"C-LIGHT","version":1,"show_id":i["snapshot"]["show_id"],"module":"lighting","epoch":i["snapshot"]["epoch"],"writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{"page":0,"page_count":1,"encoding":"json_utf8_chunks","chunk":serde_json::to_string(i).unwrap()},"revision":i["snapshot"]["revision"],"sequence":sequence.to_string(),"effective_tick":"0","reason":null})).unwrap()
}
#[test]
fn scene_version_preserves_legacy_and_is_explicit() {
    let i = fixture();
    let mut c = LuxClient::new(i["snapshot"]["show_id"].as_str().unwrap(), 9).unwrap();
    c.ingest(&page(&i, 1), 0).unwrap();
    assert_eq!(c.snapshot(), Some(&i));
    let mut old = i.clone();
    old["wire_schema"] = json!("lx05-music-v1");
    assert!(c.ingest(&page(&old, 2), 1).is_err());
    assert_eq!(c.snapshot(), Some(&i));
}
#[test]
fn malformed_scene_retains_last_known_and_marks_stale() {
    let i = fixture();
    for n in 0..12 {
        let mut c = LuxClient::new(i["snapshot"]["show_id"].as_str().unwrap(), 9).unwrap();
        c.ingest(&page(&i, 1), 0).unwrap();
        let mut bad = i.clone();
        match n {
            0 => bad["scenes"]["map"]["extra"] = json!(0),
            1 => bad["scenes"]["map"]["revision"] = json!("01"),
            2 => bad["scenes"]["map"]["scenes"][0]["lift"] = json!(1001),
            3 => bad["scenes"]["map"]["scenes"][0]["compatible_from"] = json!(["missing"]),
            4 => bad["scenes"]["map"]["roles"][0]["fixtures"] = json!(["missing"]),
            5 => bad["scenes"]["map"]["scenes"][0]["roles"][0]["rgb"] = json!([1, 2]),
            6 => {
                bad["scenes"]["map"]["scenes"][0]["conditions"]["energy"] =
                    json!({"min":500,"max":400})
            }
            7 => bad["scenes"]["current"] = json!("missing"),
            8 => bad["scenes"]["interpretation"]["change"] = json!("beat"),
            9 => bad["scenes"]["evidence"]["confidence"] = json!(-1),
            10 => bad["scenes"]["map"]["scenes"][0]["fade_ms"] = json!(0),
            _ => {
                bad["scenes"]["intended_values"] =
                    json!([{"fixture":"fixture-11","attribute":"strobe","value":1}])
            }
        };
        assert!(c.ingest(&page(&bad, 2), 1).is_err(), "case {n}");
        assert_eq!(c.snapshot(), Some(&i));
        assert_eq!(format!("{:?}", c.freshness()), "Stale");
    }
}
#[test]
fn detached_edit_validates_atomic_revision_and_independent_axes() {
    let i = fixture();
    let c = scenes::edit(&i, &["intimate", "mood", "cool", "calm"]).unwrap();
    assert_eq!(c["map"]["revision"], "2");
    assert_eq!(c["map"]["scenes"][0]["mood"], "cool calm");
    assert_eq!(c["map"]["scenes"][0]["lift"], 300);
    scenes::validate_command(&i, &c).unwrap();
    assert!(scenes::edit(&i, &["intimate", "lift", "1001"]).is_err());
    assert!(scenes::edit(&i, &["intimate", "rgb", "stage", "1", "2"]).is_err());
    let mut stale = c;
    stale["map"]["revision"] = json!("1");
    assert!(scenes::validate_command(&i, &stale).is_err());
}
#[test]
fn scene_page_has_evidence_actual_before_map_and_wraps() {
    let i = fixture();
    let v = shr_lightdesk::frontend::View {
        inventory: Some(i),
        page: "scenes".into(),
        freshness: "Fresh".into(),
        ..Default::default()
    };
    let svg = shr_lightdesk::render::svg(&shr_lightdesk::frontend::scene(&v, "", 0));
    assert!(svg.contains("Measured evidence"));
    assert!(svg.contains("Inferred change"));
    assert!(svg.contains("ACTUAL"));
    assert!(svg.contains("beat"));
}
#[test]
fn exact_changed_fields_review_and_context_recovery() {
    let i = fixture();
    let c = scenes::edit(&i, &["intimate", "lift", "850"]).unwrap();
    let lines = shr_lightdesk::frontend::material_review_lines(&c, &i, 9, 1, 30000).join("\n");
    assert!(lines.contains("map.scenes[intimate].lift: 300 -> 850"));
    assert!(lines.contains("map.revision"));
    assert!(!lines.contains("roles [{"));
    let mut input = shr_lightdesk::lux_operator::InputContext {
        confirmation: Some(shr_lightdesk::lux_operator::Confirmation {
            command: c.clone(),
            analysis_basis: None,
            material_lines: vec![],
            context: 0,
            revision: 1,
            lease: "1".into(),
            epoch: 9,
            expires: 30000,
        }),
        ..Default::default()
    };
    assert_eq!(input.press(1, Some("1"), 9, 3000).unwrap(), c);
    input.release();
    input.confirmation = Some(shr_lightdesk::lux_operator::Confirmation {
        command: c,
        analysis_basis: None,
        material_lines: vec![],
        context: 0,
        revision: 1,
        lease: "1".into(),
        epoch: 9,
        expires: 30000,
    });
    assert!(input.press(2, Some("1"), 9, 3001).is_err());
}
#[test]
fn actual_owner_initial_active_held_corpus_decodes() {
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/scenes/producer.json")).unwrap();
    for name in ["initial", "active", "held"] {
        let i = &corpus[name];
        let mut c = LuxClient::new(i["snapshot"]["show_id"].as_str().unwrap(), 9).unwrap();
        c.ingest(&page(i, 1), 0)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(c.snapshot(), Some(i));
    }
}
#[test]
fn owner_preview_ack_is_strict_and_nonmutating() {
    use shr_lightdesk::lux_control::{LightingAuthority, LuxAuthority};
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/scenes/producer.json")).unwrap();
    let i = &corpus["active"];
    for n in 0..5 {
        let mut c =
            LuxAuthority::new(i["snapshot"]["show_id"].as_str().unwrap(), 9, "clock-test").unwrap();
        c.observe(&page(i, 1), 0).unwrap();
        let mut g: Value = serde_json::from_slice(
            &c.begin("grant", json!({"scope":"lighting-control"}), 0)
                .unwrap(),
        )
        .unwrap();
        g["kind"] = json!("applied");
        g["body"] = json!({"scope":"lighting-control","lease":"1","remaining_ms":2000});
        g["revision"] = i["snapshot"]["revision"].clone();
        g["sequence"] = json!("2");
        g["effective_tick"] = json!("0");
        g["reason"] = Value::Null;
        c.acknowledge(&serde_json::to_vec(&g).unwrap(), 0).unwrap();
        let command = corpus["preview"]["request"]["body"]["command"].clone();
        let mut ack: Value = serde_json::from_slice(
            &c.begin(
                "command",
                json!({"scope":"lighting-control","command":command}),
                10,
            )
            .unwrap(),
        )
        .unwrap();
        ack["kind"] = json!("applied");
        ack["body"] = corpus["preview"]["replies"][0]["body"].clone();
        ack["revision"] = i["snapshot"]["revision"].clone();
        ack["sequence"] = json!("3");
        ack["effective_tick"] = json!("0");
        ack["reason"] = Value::Null;
        match n {
            0 => {}
            1 => ack["body"]["map_revision"] = json!("2"),
            2 => ack["body"]["values"][0]["value"] = json!(1001),
            3 => ack["body"]["physical"] = json!("observed"),
            _ => ack["body"]["unsupported"][0]["attributes"] = json!(["movement", "movement"]),
        }
        let result = c.acknowledge(&serde_json::to_vec(&ack).unwrap(), 11);
        assert_eq!(result.is_ok(), n == 0, "case{n}: {result:?}");
        assert_eq!(c.snapshot(), Some(i));
    }
}
#[test]
fn import_refuses_nonregular_symlink_duplicate_and_oversized_data() {
    use std::os::unix::fs::symlink;
    let dir = std::env::temp_dir().join(format!("lightdesk-scenes-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("map.json");
    std::fs::write(&path, b"{\"version\":1,\"version\":2}").unwrap();
    assert!(scenes::read_map(&path).is_err());
    std::fs::write(&path, vec![b' '; 65537]).unwrap();
    assert!(scenes::read_map(&path).is_err());
    let link = dir.join("link");
    symlink(&path, &link).unwrap();
    assert!(scenes::read_map(&link).is_err());
    assert!(scenes::read_map(&dir).is_err());
    std::fs::remove_file(link).unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn expired_view_fences_writer_and_retains_captured_preview_identity() {
    let mut i = fixture();
    i["scenes"]["map"]["revision"] = json!("2");
    let v = shr_lightdesk::frontend::View {
        inventory: Some(i),
        page: "scenes".into(),
        freshness: "Fresh".into(),
        lease: true,
        updated_at: Some(std::time::Instant::now() - std::time::Duration::from_millis(2001)),
        scene_preview: Some(
            json!({"scene_id":"intimate","map_revision":"1","values":[],"actual":[],"unsupported":[]}),
        ),
        ..Default::default()
    };
    let expired = v.current();
    assert_eq!(expired.freshness, "Stale");
    assert!(!expired.lease);
    let svg = shr_lightdesk::render::svg(&shr_lightdesk::frontend::scene(&v, "", 0));
    assert!(svg.contains("older map"));
    assert!(svg.contains("LAST OWNER PREVIEW"));
    assert!(svg.contains("PHYSICAL UNKNOWN"));
}
#[test]
fn retained_scene_provenance_survives_authored_identity_change() {
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/scenes/producer.json")).unwrap();
    let mut i = corpus["held"].clone();
    i["scenes"]["map"]["revision"] = json!("2");
    for s in i["scenes"]["map"]["scenes"].as_array_mut().unwrap() {
        if s["id"] == "driving" {
            s["id"] = json!("drive-new");
        }
    }
    for k in ["current", "proposed"] {
        if i["scenes"][k] == "driving" {
            i["scenes"][k] = json!("drive-new");
        }
    }
    scenes::validate_status(&i["scenes"], &i).unwrap();
    assert_eq!(
        i["scenes"]["automatic_layer"]["values"][0]["provenance"]["scene_id"],
        "driving"
    );
    let mut bad = i.clone();
    bad["scenes"]["automatic_layer"]["values"][0]["provenance"]["extra"] = json!(0);
    assert!(scenes::validate_status(&bad["scenes"], &bad).is_err());
}
#[test]
fn compact_scene_grant_ack_binds_map_scope_caps_lease_revision_and_expiry() {
    use shr_lightdesk::lux_control::{LightingAuthority, LuxAuthority};
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/scenes/producer.json")).unwrap();
    let i = &corpus["active"];
    for n in 0..7 {
        let mut c =
            LuxAuthority::new(i["snapshot"]["show_id"].as_str().unwrap(), 9, "clock-test").unwrap();
        c.observe(&page(i, 1), 0).unwrap();
        let mut lease: Value = serde_json::from_slice(
            &c.begin("grant", json!({"scope":"lighting-control"}), 0)
                .unwrap(),
        )
        .unwrap();
        lease["kind"] = json!("applied");
        lease["body"] = json!({"scope":"lighting-control","lease":"1","remaining_ms":2000});
        lease["revision"] = i["snapshot"]["revision"].clone();
        lease["sequence"] = json!("2");
        lease["effective_tick"] = json!("0");
        lease["reason"] = Value::Null;
        c.acknowledge(&serde_json::to_vec(&lease).unwrap(), 0)
            .unwrap();
        let cmd = json!({"action":"scene_grant","fixtures":["fixture-11"],"intensity_cap":700,"color_cap":650,"ttl_ms":1000});
        let mut ack: Value = serde_json::from_slice(
            &c.begin(
                "command",
                json!({"scope":"lighting-control","command":cmd}),
                10,
            )
            .unwrap(),
        )
        .unwrap();
        let mut grant = i["scenes"]["grant"].clone();
        grant["issue_revision"] = i["snapshot"]["revision"].clone();
        ack["kind"] = json!("applied");
        ack["revision"] = i["snapshot"]["revision"].clone();
        ack["sequence"] = json!("3");
        ack["effective_tick"] = json!("0");
        ack["reason"] = Value::Null;
        ack["body"] = json!({"application":"scene_granted","map_revision":"1","grant":grant});
        match n {
            0 => {}
            1 => ack["body"]["map_revision"] = json!("2"),
            2 => ack["body"]["grant"]["color_cap"] = json!(651),
            3 => ack["body"]["grant"]["lease"] = json!("alpha"),
            4 => ack["body"]["grant"]["issue_revision"] = json!("2"),
            5 => ack["body"]["grant"]["expiry_tick"] = json!("101"),
            _ => ack["body"]["grant"]["fixtures"] = json!([]),
        }
        let result = c.acknowledge(&serde_json::to_vec(&ack).unwrap(), 11);
        assert_eq!(result.is_ok(), n == 0, "case{n}: {result:?}");
    }
}
