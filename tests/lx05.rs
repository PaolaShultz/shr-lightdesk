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

#[test]
fn configured_owner_corpus_decodes_and_rejects_binding_or_provenance_drift() {
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/lx05/v2/configured.json")).unwrap();
    fn check(v: &Value, count: &mut usize) {
        if v["wire_schema"] == "lx05-v2" && v.get("snapshot").is_some() {
            let mut c = LuxClient::new(
                v["snapshot"]["show_id"].as_str().unwrap(),
                v["snapshot"]["epoch"].as_str().unwrap().parse().unwrap(),
            )
            .unwrap();
            c.ingest(&serde_json::to_vec(&page(v, 1)).unwrap(), 0)
                .unwrap();
            *count += 1;
            for mutation in 0..5 {
                let mut bad = v.clone();
                match mutation {
                    0 => bad["analysis_binding"]["inputs"][0] = json!("input-017"),
                    1 => bad["analysis"]["analysis_binding"]["inputs"][0] = json!("input-16"),
                    2 => bad["analysis_binding"]["inputs"][0] = json!("input-03"),
                    3 => bad["wire_schema"] = json!("lx05-v1"),
                    _ => bad["analysis_binding"]["extra"] = json!(true),
                }
                assert!(
                    c.ingest(&serde_json::to_vec(&page(&bad, 2)).unwrap(), 1)
                        .is_err(),
                    "mutation {mutation}"
                );
            }
        } else if let Some(o) = v.as_object() {
            for value in o.values() {
                check(value, count);
            }
        }
    }
    let mut count = 0;
    check(&corpus, &mut count);
    assert!(count >= 4, "{count}");
    // Each actual corpus entry above is validated, including source-absent provenance.
}

#[test]
fn configured_retained_provenance_checks_mapping_without_current_source() {
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/lx05/v2/configured.json")).unwrap();
    let good = corpus["source_absent_retained"].clone();
    decode(&good);
    for n in 0..4 {
        let mut bad = good.clone();
        let d =
            &mut bad["analysis"]["automatic_layer"]["values"][0]["provenance"]["source_identity"];
        match n {
            0 => d["inputs"][0] = json!("input-16"),
            1 => d["version"] = json!(1),
            2 => d["subscription"] = json!("lux.aux.v1"),
            _ => d["map_revision"] = json!("0"),
        }
        let mut c = LuxClient::new(SHOW, 9).unwrap();
        assert!(
            c.ingest(&serde_json::to_vec(&page(&bad, 1)).unwrap(), 0)
                .is_err()
        );
    }
}

fn music_inventory() -> Value {
    let mut i = features()["active"].clone();
    i["wire_schema"] = json!("lx05-music-v1");
    let a = &mut i["analysis"];
    a["analysis_version"] = json!("lux.analysis.v2");
    for v in a["automatic_layer"]["values"].as_array_mut().unwrap() {
        if !v["provenance"].is_null() {
            v["provenance"]["analysis_version"] = json!("lux.analysis.v2");
        }
    }
    a["musical"] = json!({"state":"tracking", "source_frame":a["source_window_range"]["end_frame_exclusive"], "chroma":[1000,0,0,0,700,0,0,800,0,0,0,0], "chord":{"root":0,"quality":"major","confidence":800}, "key":{"tonic":0,"mode":"major","camelot_number":8,"camelot_letter":"B","confidence":700}, "movement":{"fifths_steps":-1,"kind":"fifths_backward","tension":200,"resolution":800}, "section":{"id":1,"novelty":100,"recurrence":800,"label":"chorus_candidate","confidence":600}});
    i
}
#[test]
fn music_estimates_are_readable_and_candidate_labels_stay_tentative() {
    let i = music_inventory();
    let text = decode(&i).presentation().unwrap();
    assert!(text.contains("C major / Camelot 8B"));
    assert!(text.contains("tentative chorus"));
    assert!(text.contains("confidence 60.0%"));
    assert!(text.contains("fifths backward"));
    let mut stale = features()["aged_loss"].clone();
    stale["wire_schema"] = json!("lx05-music-v1");
    stale["analysis"]["analysis_version"] = json!("lux.analysis.v2");
    for v in stale["analysis"]["automatic_layer"]["values"]
        .as_array_mut()
        .unwrap()
    {
        if !v["provenance"].is_null() {
            v["provenance"]["analysis_version"] = json!("lux.analysis.v2");
        }
    }
    stale["analysis"]["musical"] = json!({"state":"stale","source_frame":null,"chroma":vec![0;12],"chord":null,"key":null,"movement":null,"section":null});
    assert!(
        decode(&stale)
            .presentation()
            .unwrap()
            .contains("key, chord and section unavailable")
    );
}
#[test]
fn music_schema_and_semantics_refuse_invalid_estimates_without_replacing_trust() {
    let good = music_inventory();
    for n in 0..32 {
        let mut i = good.clone();
        let a = &mut i["analysis"];
        match n {
            0 => a["musical"]["chroma"] = json!(vec![0; 11]),
            1 => a["musical"]["chroma"][0] = json!(1001),
            2 => a["musical"]["chord"]["root"] = json!(12),
            3 => a["musical"]["chord"]["confidence"] = json!(-1),
            4 => a["musical"]["key"]["camelot_number"] = json!(9),
            5 => a["musical"]["key"]["camelot_letter"] = json!("A"),
            6 => a["musical"]["key"]["mode"] = json!("dorian"),
            7 => a["musical"]["movement"]["fifths_steps"] = json!(7),
            8 => a["musical"]["movement"]["kind"] = json!("regression"),
            9 => a["musical"]["movement"]["tension"] = json!(1001),
            10 => a["musical"]["section"]["id"] = json!(4294967296u64),
            11 => a["musical"]["section"]["label"] = json!("chorus"),
            12 => a["musical"]["section"]["confidence"] = json!(1.5),
            13 => a["musical"]["source_frame"] = json!("01"),
            14 => a["musical"]["source_frame"] = json!("101281"),
            15 => a["musical"]["state"] = json!("stale"),
            16 => a["musical"]["extra"] = json!(0),
            17 => a["analysis_version"] = json!("lux.analysis.v1"),
            18 => i["wire_schema"] = json!("lx05-v1"),
            19 => a["musical"]["key"]["extra"] = json!(0),
            21 => a["musical"]["movement"]["kind"] = json!("fifths_forward"),
            22 => a["musical"]["chord"]["confidence"] = json!(0),
            23 => a["musical"]["key"]["confidence"] = json!(0),
            24 => a["musical"]["source_frame"] = json!("95136"),
            25 => a["musical"]["section"]["id"] = json!(0),
            26 => {
                a["musical"] = json!({"state":"stale","source_frame":null,"chroma":vec![0;12],"chord":null,"key":null,"movement":null,"section":null})
            }
            28 => a["musical"]["section"]["label"] = json!("unknown"),
            29 => a["musical"]["source_frame"] = json!("47999"),
            30 => a["musical"]["key"] = Value::Null,
            31 => a["musical"]["section"]["confidence"] = json!(0),
            _ => {
                a["state"] = json!("stale");
                a["confidence"] = json!(0);
                a["proposal"] = Value::Null;
                a["grant"] = Value::Null;
                a["rms_millionths"] = Value::Null;
                a["energy_millionths"] = Value::Null;
            }
        }
        let mut c = decode(&good);
        assert!(
            c.ingest(&serde_json::to_vec(&page(&i, 2)).unwrap(), 1)
                .is_err(),
            "music mutation {n}"
        );
        assert_eq!(c.snapshot(), Some(&good));
    }
}

#[test]
fn music_accepts_bounded_spectral_lag_and_unknown_without_inventing_estimates() {
    let mut i = music_inventory();
    let end = i["analysis"]["source_window_range"]["end_frame_exclusive"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    i["analysis"]["musical"]["source_frame"] = json!((end - 6143).to_string());
    decode(&i);
    i["analysis"]["musical"] = json!({"state":"unknown","source_frame":null,"chroma":vec![0;12],"chord":null,"key":null,"movement":null,"section":null});
    let c = decode(&i);
    assert!(c.presentation().unwrap().contains("Music unknown"));
}

#[cfg(target_os = "linux")]
#[test]
fn musical_native_pages_keep_candidate_text_and_all_rows_accessible() {
    use shr_lightdesk::{
        frontend::{View, scene},
        render::{Font, Primitive},
    };
    let i = music_inventory();
    decode(&i);
    for page in ["stage", "programmer", "library", "playbacks", "health"] {
        let view = View {
            inventory: Some(i.clone()),
            freshness: "Fresh".into(),
            page: page.into(),
            ..Default::default()
        };
        let mut text = String::new();
        for scroll in [0, 20, 40, usize::MAX] {
            let scene = scene(&view, "", scroll);
            assert!(scene.in_bounds());
            for primitive in scene.primitives {
                if let Primitive::Text { value, .. } = primitive {
                    assert!(value.chars().all(|c| Font::default().supports(c)));
                    text.push_str(&value);
                    text.push('\n');
                }
            }
        }
        for expected in [
            "Camelot 8B",
            "tentative chorus",
            "fifths backward",
            "confidence 60.0%",
            "beat/downbeat unavailable",
            "PHYSICAL UNKNOWN",
        ] {
            assert!(text.contains(expected), "{page}: {expected}: {text}");
        }
        assert!(!text.contains("harmony unavailable"));
    }
}

#[test]
fn music_camelot_matches_all_known_major_and_minor_identities() {
    let major = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
    let minor = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];
    for (mode, letter, table) in [("major", "B", major), ("minor", "A", minor)] {
        for (tonic, number) in table.into_iter().enumerate() {
            let mut i = music_inventory();
            i["analysis"]["musical"]["key"] = json!({"tonic":tonic,"mode":mode,"camelot_number":number,"camelot_letter":letter,"confidence":700});
            decode(&i);
            i["analysis"]["musical"]["key"]["camelot_number"] = json!(number % 12 + 1);
            let mut c = LuxClient::new(SHOW, 9).unwrap();
            assert!(
                c.ingest(&serde_json::to_vec(&page(&i, 1)).unwrap(), 0)
                    .is_err()
            );
        }
    }
}

#[test]
fn actual_lux_music_producer_corpora_decode_both_bindings_and_stale_loss() {
    for raw in [
        include_str!("fixtures/musical/configured.json"),
        include_str!("fixtures/musical/legacy-binding.json"),
        include_str!("fixtures/musical/configured-estimates.json"),
        include_str!("fixtures/musical/legacy-estimates.json"),
    ] {
        let corpus: Value = serde_json::from_str(raw).unwrap();
        for inventory in corpus.as_object().unwrap().values() {
            let mut client = LuxClient::new(
                inventory["snapshot"]["show_id"].as_str().unwrap(),
                inventory["snapshot"]["epoch"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
            )
            .unwrap();
            client
                .ingest(&serde_json::to_vec(&page(inventory, 1)).unwrap(), 0)
                .unwrap();
            assert_eq!(client.snapshot(), Some(inventory));
            assert!(client.presentation().unwrap().contains("Music"));
        }
    }
}

#[test]
fn music_geometry_without_key_does_not_claim_functional_harmony() {
    let mut i = music_inventory();
    i["analysis"]["musical"]["key"] = Value::Null;
    i["analysis"]["musical"]["movement"]["tension"] = json!(0);
    i["analysis"]["musical"]["movement"]["resolution"] = json!(0);
    decode(&i);
    for field in ["tension", "resolution"] {
        let mut bad = i.clone();
        bad["analysis"]["musical"]["movement"][field] = json!(1);
        let mut c = LuxClient::new(SHOW, 9).unwrap();
        assert!(
            c.ingest(&serde_json::to_vec(&page(&bad, 1)).unwrap(), 0)
                .is_err()
        );
    }
}
