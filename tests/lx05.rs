use serde_json::{Value, json};
use shr_lightdesk::adapter::LuxClient;
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn page(i: &Value, sequence: u64) -> Value {
    json!({"contract":"C-LIGHT","version":1,"show_id":i["snapshot"]["show_id"],"module":"lighting","epoch":i["snapshot"]["epoch"],"writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{"page":0,"page_count":1,"encoding":"json_utf8_chunks","chunk":serde_json::to_string(i).unwrap()},"revision":i["snapshot"]["revision"],"sequence":sequence.to_string(),"effective_tick":"0","reason":null})
}
fn decode(i: &Value) -> LuxClient {
    let mut c = LuxClient::new(SHOW, 9).unwrap();
    c.ingest(&serde_json::to_vec(&page(i, 1)).unwrap(), 0)
        .unwrap();
    assert_eq!(c.snapshot(), Some(i));
    c
}
fn features() -> Value {
    serde_json::from_str(include_str!("fixtures/lx05/features.json")).unwrap()
}
fn visit(v: &Value, n: &mut usize) {
    if v.get("snapshot").is_some() && v["wire_schema"] == "lx05-v1" {
        decode(v);
        *n += 1;
    } else {
        match v {
            Value::Object(o) => {
                for x in o.values() {
                    visit(x, n)
                }
            }
            Value::Array(a) => {
                for x in a {
                    visit(x, n)
                }
            }
            _ => {}
        }
    }
}
#[test]
fn all_actual_owner_command_and_feature_inventories_decode() {
    let mut n = 0;
    visit(
        &serde_json::from_str(include_str!("fixtures/lx05/commands.json")).unwrap(),
        &mut n,
    );
    visit(&features(), &mut n);
    assert_eq!(n, 7);
}
#[test]
fn retained_provenance_is_independent_of_current_reattached_source() {
    let mut i = features()["aged_loss"].clone();
    let old = i["analysis"]["automatic_layer"].clone();
    i["analysis"]["source_epoch"] = json!("10");
    i["analysis"]["source_identity"]["source_epoch"] = json!("10");
    let c = decode(&i);
    assert_eq!(c.snapshot().unwrap()["analysis"]["automatic_layer"], old);
    let text = c.presentation().unwrap();
    assert!(text.contains("retained AUTO"));
    assert!(text.contains("beat downbeat harmony unavailable"));
}
#[test]
fn malformed_analysis_and_auto_never_replace_trusted_state() {
    let good = features()["active"].clone();
    for n in 0..34 {
        let mut i = good.clone();
        match n {
            0 => i["analysis"]["extra"] = json!(0),
            1 => i["analysis"]["confidence"] = json!(1001),
            2 => i["analysis"]["generation"] = json!("02"),
            3 => i["analysis"]["losses"] = json!("18446744073709551616"),
            4 => i["analysis"]["state"] = json!("future"),
            5 => i["analysis"]["beat"] = json!(true),
            6 => i["analysis"]["source_epoch"] = json!("10"),
            7 => i["analysis"]["source_identity"]["sample_rate"] = json!(44100),
            8 => i["analysis"]["source_window_range"]["end_frame_exclusive"] = json!("101281"),
            9 => i["analysis"]["rms_millionths"] = json!([0, 0, 0]),
            10 => i["analysis"]["rms_millionths"][0] = json!(2147483648u64),
            11 => i["analysis"]["energy_millionths"] = json!(-1),
            12 => i["analysis"]["proposal"] = json!(1001),
            13 => i["analysis"]["automatic_layer"]["values"][0]["intensity"] = json!(21),
            14 => i["analysis"]["automatic_layer"]["values"][0]["fixture"] = json!("missing"),
            15 => i["analysis"]["automatic_layer"]["values"][0]["source"] = json!("future"),
            16 => i["analysis"]["automatic_layer"]["values"][0]["provenance"] = Value::Null,
            17 => {
                i["analysis"]["automatic_layer"]["values"][0]["provenance"]["confidence"] =
                    json!(1001)
            }
            18 => i["analysis"]["grant"]["analysis_generation"] = json!("8"),
            19 => i["analysis"]["grant"]["fixtures"] = json!(["fixture-11", "fixture-11"]),
            20 => i["wire_schema"] = json!("lx04-v1"),
            21 => {
                i["snapshot"]["fixtures"][0]["attributes"][0]["contributors"][0]["source"] =
                    json!({"auto":"analysis-held"})
            }
            22 => i["analysis"]["automatic_layer"]["values"]
                .as_array_mut()
                .unwrap()
                .push(good["analysis"]["automatic_layer"]["values"][0].clone()),
            23 => {
                i["analysis"]["source_identity"]["sources"] =
                    json!(["bass", "kick", "guitar-1", "guitar-2"])
            }
            24 => {
                i["analysis"]["source_identity"]["inputs"] =
                    json!(["input-02", "input-01", "input-03", "input-04"])
            }
            25 => i["analysis"]["source_identity"]["stream"] = json!(4),
            26 => i["analysis"]["calibration_windows"] = json!(-1),
            27 => i["analysis"]["source_age_ms"] = json!(101),
            28 => {
                i["analysis"]["automatic_layer"]["values"][0]["provenance"]["source_identity"]["extra"] =
                    json!(0)
            }
            29 => {
                i["analysis"]["automatic_layer"]["values"][0]["provenance"]["source_window_range"]
                    ["first_frame"] = json!("18446744073709551615")
            }
            30 => i["analysis"]["grant"]["cap"] = json!(-1),
            31 => i["snapshot"]["fixtures"][0]["attributes"][0]["attribute"] = json!("red"),
            32 => i["analysis"]["confidence"] = json!(0),
            _ => i["analysis"]["state"] = json!("stale"),
        }
        let mut c = decode(&good);
        assert!(
            c.ingest(&serde_json::to_vec(&page(&i, 2)).unwrap(), 1)
                .is_err(),
            "mutation {n}"
        );
        assert_eq!(c.snapshot(), Some(&good));
    }
}
#[test]
fn duplicate_analysis_keys_refuse_before_trust() {
    let i = features()["active"].clone();
    let mut p = page(&i, 1);
    let chunk = p["body"]["chunk"].as_str().unwrap().replace(
        "\"confidence\":1000",
        "\"confidence\":1000,\"confidence\":1000",
    );
    p["body"]["chunk"] = json!(chunk);
    let mut c = LuxClient::new(SHOW, 9).unwrap();
    assert!(c.ingest(&serde_json::to_vec(&p).unwrap(), 0).is_err());
    assert!(c.snapshot().is_none());
}
#[test]
fn lx05_durable_capability_preserves_recovery_contract() {
    let mut i = features()["aged_loss"].clone();
    i["checkpoint"] = json!({"available":true,"recovery":"current_intended_look_frozen_into_hold","active_transients_resumed":false});
    i["snapshot"]["durability"] = json!("checkpointed");
    decode(&i);
    i["checkpoint"]["active_transients_resumed"] = json!(true);
    let mut c = LuxClient::new(SHOW, 9).unwrap();
    assert!(
        c.ingest(&serde_json::to_vec(&page(&i, 1)).unwrap(), 0)
            .is_err()
    );
}

#[test]
fn confidence_matches_current_state_and_grants_require_explicit_auto_mode() {
    for (base, confidence, mode) in [
        ("aged_loss", 1000, "auto"),
        ("active", 0, "auto"),
        ("active", 1000, "assist"),
        ("active", 1000, "manual"),
    ] {
        let mut i = features()[base].clone();
        i["analysis"]["confidence"] = json!(confidence);
        i["mode"] = json!(mode);
        if confidence == 0 {
            i["analysis"]["grant"] = Value::Null;
        }
        let mut c = LuxClient::new(SHOW, 9).unwrap();
        assert!(
            c.ingest(&serde_json::to_vec(&page(&i, 1)).unwrap(), 0)
                .is_err(),
            "{base}/{confidence}/{mode}"
        );
    }
}

#[test]
fn untimed_lx05_keeps_strict_static_and_durable_capability_bounds() {
    let i: Value =
        serde_json::from_str(include_str!("fixtures/lx05-untimed/untimed-absent.json")).unwrap();
    let epoch = i["snapshot"]["epoch"].as_str().unwrap().parse().unwrap();
    let mut c = LuxClient::new(SHOW, epoch).unwrap();
    c.ingest(&serde_json::to_vec(&page(&i, 1)).unwrap(), 0)
        .unwrap();
    assert_eq!(c.snapshot(), Some(&i));
    assert!(c.snapshot().unwrap().get("release").is_none());
    let mut timed = i.clone();
    timed["release"] = features()["active"]["release"].clone();
    let mut c = LuxClient::new(SHOW, epoch).unwrap();
    c.ingest(&serde_json::to_vec(&page(&timed, 1)).unwrap(), 0)
        .unwrap();
    for n in 0..5 {
        let mut bad = i.clone();
        match n {
            0 => {
                bad["checkpoint"] = json!({"available":true,"recovery":"current_intended_look_frozen_into_hold","active_transients_resumed":false})
            }
            1 => bad["snapshot"]["fixtures"][0]["attributes"][0]["release"] = json!(0),
            2 => bad["snapshot"]["durability"] = json!("checkpointed"),
            3 => bad["release"] = json!({}),
            _ => {
                bad["release"] = timed["release"].clone();
                bad["release"]["transition_ms"] = json!(1);
            }
        }
        let mut c = LuxClient::new(SHOW, epoch).unwrap();
        assert!(
            c.ingest(&serde_json::to_vec(&page(&bad, 1)).unwrap(), 0)
                .is_err(),
            "mutation {n}"
        );
    }
}
