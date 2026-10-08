use serde_json::{Value, json};
use shr_lightdesk::{frontend::material_review_lines, lux_operator::validate_analysis_command};
fn active() -> Value {
    serde_json::from_str::<Value>(include_str!("fixtures/lx05/features.json")).unwrap()["active"]
        .clone()
}
#[test]
fn bounded_analysis_controls_require_capability_scope_calibration_and_units() {
    let good = active();
    let command =
        json!({"action":"analysis_grant","fixtures":["fixture-11"],"cap":500,"ttl_ms":1000});
    validate_analysis_command(&good, &command).unwrap();
    for n in 0..9 {
        let mut i = good.clone();
        let mut c = command.clone();
        match n {
            0 => i["wire_schema"] = json!("lx04-v1"),
            1 => i["mode"] = json!("assist"),
            2 => i["analysis"]["state"] = json!("stale"),
            3 => c["fixtures"] = json!([]),
            4 => c["fixtures"] = json!(["fixture-11", "fixture-11"]),
            5 => c["fixtures"] = json!(["missing"]),
            6 => c["cap"] = json!(1001),
            7 => c["ttl_ms"] = json!(2001),
            _ => c["ttl_ms"] = json!(11),
        }
        assert!(validate_analysis_command(&i, &c).is_err(), "case {n}");
    }
}
#[test]
fn calibration_finish_refuses_insufficient_windows_and_missing_source() {
    let mut i = active();
    let start = json!({"action":"analysis_calibrate","phase":"start"});
    validate_analysis_command(&i, &start).unwrap();
    let finish = json!({"action":"analysis_calibrate","phase":"finish"});
    assert!(validate_analysis_command(&i, &finish).is_err());
    i["analysis"]["state"] = json!("calibrating");
    i["analysis"]["calibration_windows"] = json!(50);
    validate_analysis_command(&i, &finish).unwrap();
    i["analysis"]["source_age_ms"] = json!(101);
    assert!(validate_analysis_command(&i, &start).is_err());
}
#[test]
fn material_review_includes_every_fixture_cap_expiry_and_source_identity() {
    let i = active();
    let c = json!({"action":"analysis_grant","fixtures":["fixture-11","fixture-12"],"cap":375,"ttl_ms":1500});
    let text = material_review_lines(&c, &i, 9, 2, 2000).join("\n");
    for expected in [
        "fixture-11",
        "fixture-12",
        "37.5",
        "1500",
        "Source epoch",
        "calibration",
        "no automatic renewal",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn analysis_review_identity_changes_without_relying_on_engine_revision() {
    use shr_lightdesk::lux_operator::analysis_review_basis;
    let i = active();
    let command = json!({"action":"analysis_grant"});
    let basis = analysis_review_basis(&i, &command);
    for field in ["generation", "calibration_generation"] {
        let mut changed = i.clone();
        changed["analysis"][field] = json!("999");
        assert_ne!(analysis_review_basis(&changed, &command), basis);
    }
    let mut changed = i.clone();
    changed["analysis"]["source_identity"]["source_epoch"] = json!("10");
    assert_ne!(analysis_review_basis(&changed, &command), basis);
    let mut advancing = i.clone();
    advancing["analysis"]["source_age_ms"] = json!(12);
    advancing["analysis"]["source_window_range"]["first_frame"] = json!("999");
    assert_eq!(analysis_review_basis(&advancing, &command), basis);
}
#[test]
fn analysis_ack_rejects_expiry_source_scope_revision_and_phase_corruption() {
    use shr_lightdesk::lux_control::{LightingAuthority, LuxAuthority};
    fn setup(command: Value) -> (LuxAuthority, Value, Value) {
        let i = active();
        let mut c =
            LuxAuthority::new("11111111-1111-4111-8111-111111111111", 9, "clock-test").unwrap();
        let p = json!({"contract":"C-LIGHT","version":1,"show_id":i["snapshot"]["show_id"],"module":"lighting","epoch":"9","writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{"page":0,"page_count":1,"encoding":"json_utf8_chunks","chunk":serde_json::to_string(&i).unwrap()},"revision":"2","sequence":"1","effective_tick":"0","reason":null});
        c.observe(&serde_json::to_vec(&p).unwrap(), 0).unwrap();
        let mut grant: Value = serde_json::from_slice(
            &c.begin("grant", json!({"scope":"lighting-control"}), 0)
                .unwrap(),
        )
        .unwrap();
        grant["kind"] = json!("applied");
        grant["body"] = json!({"scope":"lighting-control","lease":"1","remaining_ms":2000});
        grant["revision"] = json!("2");
        grant["sequence"] = json!("2");
        grant["effective_tick"] = json!("0");
        grant["reason"] = Value::Null;
        c.acknowledge(&serde_json::to_vec(&grant).unwrap(), 0)
            .unwrap();
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
        ack["revision"] = json!("2");
        ack["sequence"] = json!("3");
        ack["effective_tick"] = json!("0");
        ack["reason"] = Value::Null;
        (c, ack, i)
    }
    let command =
        json!({"action":"analysis_grant","fixtures":["fixture-11"],"cap":500,"ttl_ms":1000});
    for mutation in 0..8 {
        let (mut c, mut ack, i) = setup(command.clone());
        let mut a = i["analysis"].clone();
        a["grant"]["issue_revision"] = json!("2");
        match mutation {
            0 => {}
            1 => a["grant"]["expiry_tick"] = json!("101"),
            2 => a["grant"]["issue_revision"] = json!("1"),
            3 => a["grant"]["writer"] = json!("other"),
            4 => a["grant"]["cap"] = json!(501),
            5 => {
                a["source_identity"]["source_epoch"] = json!("10");
                a["source_epoch"] = json!("10");
            }
            6 => {
                a["generation"] = json!("3");
                a["grant"]["analysis_generation"] = json!("3");
            }
            _ => a["grant"]["fixtures"] = json!([]),
        }
        ack["body"] = json!({"analysis":a});
        let result = c.acknowledge(&serde_json::to_vec(&ack).unwrap(), 11);
        assert_eq!(result.is_ok(), mutation == 0, "{mutation}: {result:?}");
    }
    let (mut c, mut ack, i) = setup(json!({"action":"analysis_calibrate","phase":"start"}));
    ack["body"] = json!({"analysis":i["analysis"]});
    assert!(
        c.acknowledge(&serde_json::to_vec(&ack).unwrap(), 11)
            .is_err()
    );
}
