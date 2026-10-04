use serde_json::{Value, json};
use shr_lightdesk::{
    adapter::{Freshness, LuxClient},
    codec,
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn corpus() -> Value {
    serde_json::from_str(include_str!("fixtures/lx03/commands.json")).unwrap()
}
fn page() -> Value {
    corpus()["cases"][0]["replies"][0].clone()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn payload(v: &Value) -> Value {
    serde_json::from_str(v["body"]["chunk"].as_str().unwrap()).unwrap()
}
fn set_payload(v: &mut Value, p: Value) {
    v["body"]["chunk"] = json!(serde_json::to_string(&p).unwrap());
}
fn client() -> LuxClient {
    LuxClient::new(SHOW, 9).unwrap()
}
fn bad(mutator: impl FnOnce(&mut Value)) {
    let mut c = client();
    let good = page();
    c.ingest(&bytes(&good), 0).unwrap();
    let old = c.snapshot().unwrap().clone();
    let mut v = good.clone();
    v["sequence"] = json!("99");
    mutator(&mut v);
    assert!(c.ingest(&bytes(&v), 50).is_err(), "accepted bad page: {v}");
    assert_eq!(c.snapshot(), Some(&old));
}
#[test]
fn actual_provider_snapshots_decode_and_present_full_ladder() {
    let mut c = client();
    let data = corpus();
    let mut count = 0;
    for case in data["cases"].as_array().unwrap() {
        for p in case["replies"].as_array().unwrap() {
            if p["kind"] == "snapshot" {
                c.ingest(&bytes(p), count * 10).unwrap();
                count += 1;
            }
        }
    }
    c.finish().unwrap();
    assert_eq!(count, 2);
    let out = c.presentation().unwrap();
    for s in [
        "LUX READ-ONLY",
        "fixture-11",
        "Hold 0",
        "programmer null",
        "stored",
        "playing",
        "proposal",
        "contributors",
        "inhibit \"blackout\"",
        "final_intent",
        "submitted UNKNOWN",
        "observed UNKNOWN",
        "physical UNKNOWN",
        "volatile",
    ] {
        assert!(out.contains(s), "missing {s}");
    }
}
#[test]
fn complete_accumulated_looks_use_patch_capacity_not_edit_capacity() {
    let mut inventory = payload(&page());
    let fixture = inventory["patch"]["fixtures"][0].clone();
    let metadata = inventory["capability_metadata"][0].clone();
    let snapshot = inventory["snapshot"]["fixtures"][0].clone();
    let mut fixtures = Vec::new();
    let mut capabilities = Vec::new();
    let mut snapshots = Vec::new();
    let mut ids = Vec::new();
    let mut values = Vec::new();
    for n in 0..10 {
        let fid = json!(format!("wide-{n}"));
        ids.push(fid.clone());
        let mut f = fixture.clone();
        f["id"] = fid.clone();
        f["address"] = json!(1 + n * 7);
        fixtures.push(f);
        let mut m = metadata.clone();
        m["fixture"] = fid.clone();
        capabilities.push(m);
        let mut s = snapshot.clone();
        s["fixture"] = fid.clone();
        for a in s["attributes"].as_array_mut().unwrap() {
            let value = a["resolved"].clone();
            values.push(json!({"fixture":fid,"attribute":a["attribute"],"value":value}));
            a["stored"] = json!([
                {"kind":"cue","id":"wide-cue","value":value},
                {"kind":"palette","id":"wide-palette","value":value}
            ]);
            a["playing"] = json!([{"id":"wide-playback","value":value}]);
            a["source"] = json!({"playback":"wide-playback"});
            a["contributors"] = json!([{"source":a["source"],"value":value,"winner":true}]);
        }
        snapshots.push(s);
    }
    assert_eq!(values.len(), 70);
    assert_eq!(inventory["limits"]["targets"], 64);
    inventory["patch"]["fixtures"] = json!(fixtures);
    inventory["capability_metadata"] = json!(capabilities);
    inventory["snapshot"]["fixtures"] = json!(snapshots);
    inventory["groups"] = json!([{"id":"all","fixtures":ids}]);
    inventory["authority_inventory"]["cues"] = json!([{"id":"wide-cue","values":values}]);
    inventory["authority_inventory"]["palettes"] = json!([{"id":"wide-palette","values":values}]);
    inventory["authority_inventory"]["playbacks"] =
        json!([{"id":"wide-playback","level":1000,"activation_order":"1","values":values}]);
    let decode = |inventory: &Value| {
        let text = serde_json::to_string(inventory).unwrap();
        let chunks: Vec<_> = text.as_bytes().chunks(40_000).collect();
        let mut c = client();
        for (n, chunk) in chunks.iter().enumerate() {
            let mut p = page();
            p["body"]["page"] = json!(n);
            p["body"]["page_count"] = json!(chunks.len());
            p["body"]["chunk"] = json!(std::str::from_utf8(chunk).unwrap());
            c.ingest(&bytes(&p), 0)?;
        }
        c.finish()?;
        Ok::<_, String>(c)
    };
    assert_eq!(decode(&inventory).unwrap().snapshot(), Some(&inventory));
    // Enlarging the look bound must retain per-target identity and uniqueness checks.
    for kind in ["cues", "palettes", "playbacks"] {
        let mut bad = inventory.clone();
        bad["authority_inventory"][kind][0]["values"][1] = values[0].clone();
        assert!(decode(&bad).is_err());
    }
}
#[test]
fn actual_provider_refusal_inventories_validate_without_partial_commit() {
    let data: Value = serde_json::from_str(include_str!("fixtures/lx03/refusals.json")).unwrap();
    for case in data["cases"].as_array().unwrap() {
        for key in ["prior", "after"] {
            let p = &case[key];
            let mut v = page();
            v["revision"] = p["snapshot"]["revision"].clone();
            set_payload(&mut v, p.clone());
            let mut c = client();
            assert!(c.ingest(&bytes(&v), 0).unwrap());
        }
    }
}
#[test]
fn wire_schema_identity_and_counters_are_strict() {
    for key in [
        "writer",
        "lease",
        "request_id",
        "expected_revision",
        "reason",
    ] {
        bad(|v| v[key] = json!("1"));
    }
    bad(|v| v["extra"] = json!(true));
    bad(|v| v["body"]["extra"] = json!(0));
    bad(|v| v["show_id"] = json!("22222222-2222-4222-8222-222222222222"));
    bad(|v| v["epoch"] = json!("8"));
    bad(|v| v["version"] = json!(2));
    for value in [
        json!(99),
        json!("099"),
        json!("+99"),
        json!("18446744073709551615"),
        json!("18446744073709551616"),
    ] {
        bad(|v| v["sequence"] = value);
    }
}
#[test]
fn inventory_unknown_range_duplicate_and_coherent_masks_refuse() {
    type Mutation = Box<dyn Fn(&mut Value)>;
    let changes: Vec<Mutation> = vec![
        Box::new(|p| p["patch"]["fixtures"][0]["address"] = json!(2147483647)),
        Box::new(|p| p["patch"]["fixtures"][0]["address"] = json!(-2147483648)),
        Box::new(|p| p["snapshot"]["extra"] = json!(0)),
        Box::new(|p| p["patch"]["fixtures"][0]["extra"] = json!(0)),
        Box::new(|p| p["snapshot"]["fixtures"][0]["attributes"][0]["resolved"] = json!(1001)),
        Box::new(|p| p["snapshot"]["fixtures"][0]["attributes"][0]["observed"] = json!(0)),
        Box::new(|p| {
            p["snapshot"]["fixtures"][0]["attributes"][0]["contributors"][0]["extra"] = json!(0)
        }),
        Box::new(|p| p["capability_metadata"][0]["attributes"][0]["max"] = json!(999)),
        Box::new(|p| p["capability_metadata"][0]["coherent_groups"] = json!([])),
        Box::new(|p| p["snapshot"]["fixtures"][0]["attributes"][1]["programmer"] = json!(25)),
        Box::new(|p| {
            p["snapshot"]["fixtures"][0]["attributes"][0]["contributors"][0]["source"] =
                json!({"playback":"missing"})
        }),
        Box::new(|p| {
            let f = p["patch"]["fixtures"][0].clone();
            p["patch"]["fixtures"].as_array_mut().unwrap().push(f);
        }),
        Box::new(|p| {
            p["snapshot"]["fixtures"][0]["attributes"]
                .as_array_mut()
                .unwrap()
                .pop()
                .map(|_| ())
                .unwrap()
        }),
    ];
    for change in changes {
        bad(|v| {
            let mut p = payload(v);
            change(&mut p);
            set_payload(v, p);
        });
    }
}
#[test]
fn framed_utf8_duplicates_depth_floats_and_trailing_content_refuse() {
    for b in [
        b"{\"a\":1,\"a\":2}".as_slice(),
        b"{\"a\":0.5}",
        b"{}{}",
        &[255],
        b"[[[[[[[[[[[[[0]]]]]]]]]]]]]",
    ] {
        assert!(codec::json(b, 65536).is_err());
    }
    let mut v = page();
    let raw = v["body"]["chunk"]
        .as_str()
        .unwrap()
        .replace("\"master\":1000", "\"master\":1000,\"master\":1000");
    v["body"]["chunk"] = json!(raw);
    assert!(client().ingest(&bytes(&v), 0).is_err());
    for data in [
        vec![0, 0, 0, 0],
        vec![0, 1, 0, 1],
        vec![0, 0],
        vec![0, 0, 0, 2, b'{'],
    ] {
        assert!(codec::read_frame(&mut data.as_slice()).is_err());
    }
    assert!(codec::json(&vec![b' '; 65537], 65536).is_err());
}
fn split() -> (Value, Value) {
    let v = page();
    let s = v["body"]["chunk"].as_str().unwrap();
    let n = s.len() / 2;
    let (mut a, mut b) = (v.clone(), v.clone());
    a["body"]["page_count"] = json!(2);
    b["body"]["page_count"] = json!(2);
    b["body"]["page"] = json!(1);
    a["body"]["chunk"] = json!(&s[..n]);
    b["body"]["chunk"] = json!(&s[n..]);
    (a, b)
}
#[test]
fn pages_commit_only_complete_once_within_deadline() {
    let (a, b) = split();
    let mut c = client();
    assert!(!c.ingest(&bytes(&b), 0).unwrap());
    assert!(c.snapshot().is_none());
    assert!(c.ingest(&bytes(&a), 1999).unwrap());
    c.finish().unwrap();
    for mutate in [0, 1, 2, 3] {
        let mut c = client();
        assert!(!c.ingest(&bytes(&a), 0).unwrap());
        let mut next = b.clone();
        if mutate == 1 {
            next["revision"] = json!("1");
        }
        if mutate == 2 {
            next["body"]["page_count"] = json!(3);
        }
        if mutate == 3 {
            next = a.clone();
        }
        assert!(
            c.ingest(&bytes(&next), if mutate == 0 { 2000 } else { 10 })
                .is_err()
        );
        assert!(c.snapshot().is_none());
    }
    let mut c = client();
    c.ingest(&bytes(&a), 0).unwrap();
    assert!(c.finish().is_err());
}
#[test]
fn older_sequences_revisions_and_clock_cannot_roll_back_display() {
    let mut c = client();
    let mut v = page();
    v["sequence"] = json!("10");
    c.ingest(&bytes(&v), 100).unwrap();
    let old = c.snapshot().unwrap().clone();
    assert!(c.ingest(&bytes(&page()), 110).is_err());
    assert_eq!(c.snapshot(), Some(&old));
    v["sequence"] = json!("11");
    assert!(c.ingest(&bytes(&v), 99).is_err());
    assert_eq!(c.snapshot(), Some(&old));
    bad(|v| {
        let mut p = payload(v);
        p["snapshot"]["epoch"] = json!("8");
        set_payload(v, p);
    });
}
#[test]
fn stable_ids_and_advertised_angular_ranges_replace_simulator_assumptions() {
    let mut v = page();
    let mut p = payload(&v);
    p["patch"]["fixtures"][0]["id"] = json!("long-stable-mover");
    p["capability_metadata"][0]["fixture"] = json!("long-stable-mover");
    p["groups"][0]["fixtures"][0] = json!("long-stable-mover");
    p["snapshot"]["fixtures"][0]["fixture"] = json!("long-stable-mover");
    p["patch"]["fixtures"][0]["capabilities"][4]["min"] = json!(-123);
    p["patch"]["fixtures"][0]["capabilities"][4]["max"] = json!(456);
    p["capability_metadata"][0]["attributes"][4]["min"] = json!(-123);
    p["capability_metadata"][0]["attributes"][4]["max"] = json!(456);
    set_payload(&mut v, p);
    let mut c = client();
    c.ingest(&bytes(&v), 0).unwrap();
    let out = c.presentation().unwrap();
    assert!(out.contains("long-stable-mover"));
    assert!(out.contains("-123..456"));
}

#[test]
fn complete_stored_and_playing_source_masks_are_validated_and_visible() {
    // Local schema extensions; the byte-exact real producer corpus remains untouched.
    let mut v = page();
    let mut p = payload(&v);
    let vals = json!([
      {"fixture":"fixture-11","attribute":"red","value":20},
      {"fixture":"fixture-11","attribute":"green","value":30},
      {"fixture":"fixture-11","attribute":"blue","value":40}]);
    p["authority_inventory"]["cues"] = json!([{"id":"cue-a","values":vals}]);
    p["authority_inventory"]["playbacks"] =
        json!([{"id":"playback-a","activation_order":"1","level":0,"values":vals}]);
    for (i, n) in [(1, 20), (2, 30), (3, 40)] {
        let a = &mut p["snapshot"]["fixtures"][0]["attributes"][i];
        a["stored"] = json!([{"kind":"cue","id":"cue-a","value":n}]);
        a["playing"] = json!([{"id":"playback-a","value":n}]);
        a["source"] = json!({"playback":"playback-a"});
        a["resolved"] = json!(n);
        a["final_intent"] = json!(n);
        a["contributors"] = json!([{"source":{"playback":"playback-a"},"value":n,"winner":true}]);
    }
    set_payload(&mut v, p.clone());
    let mut c = client();
    c.ingest(&bytes(&v), 0).unwrap();
    assert!(c.presentation().unwrap().contains("playback-a"));
    p["authority_inventory"]["playbacks"][0]["values"]
        .as_array_mut()
        .unwrap()
        .pop();
    set_payload(&mut v, p);
    v["sequence"] = json!("2");
    assert!(c.ingest(&bytes(&v), 10).is_err());
}

#[test]
fn cached_state_freshness_retains_values_and_requires_complete_recovery() {
    let mut c = client();
    assert_eq!(c.freshness(), Freshness::Unavailable);
    assert!(c.presentation().unwrap().contains("Unavailable"));
    let mut good = page();
    c.ingest(&bytes(&good), 0).unwrap();
    let trusted = c.snapshot().unwrap().clone();
    assert_eq!(c.freshness(), Freshness::Fresh);
    assert!(c.presentation().unwrap().contains("cached state Fresh"));
    c.ingest(b"{}", 1).unwrap_err();
    assert_eq!(c.freshness(), Freshness::Stale);
    assert_eq!(c.snapshot(), Some(&trusted));
    let (mut a, mut b) = split();
    a["sequence"] = json!("2");
    b["sequence"] = json!("2");
    assert!(!c.ingest(&bytes(&a), 10).unwrap());
    assert_eq!(c.freshness(), Freshness::Stale);
    assert!(c.finish().is_err());
    assert_eq!(c.snapshot(), Some(&trusted));
    c.ingest(&bytes(&a), 20).unwrap();
    assert!(c.ingest(&bytes(&b), 2020).is_err());
    assert_eq!(c.freshness(), Freshness::Stale);
    good["sequence"] = json!("3");
    c.ingest(&bytes(&good), 2021).unwrap();
    assert_eq!(c.freshness(), Freshness::Fresh);
    c.advance(4020).unwrap();
    assert_eq!(c.freshness(), Freshness::Fresh);
    c.advance(4021).unwrap();
    assert_eq!(c.freshness(), Freshness::Stale);
    assert_eq!(c.snapshot(), Some(&trusted));
    good["sequence"] = json!("4");
    c.ingest(&bytes(&good), 4022).unwrap();
    c.disconnect();
    assert_eq!(c.freshness(), Freshness::Stale);
    assert!(c.presentation().unwrap().contains("retained last-known"));
    good["sequence"] = json!("5");
    c.ingest(&bytes(&good), 4023).unwrap();
    c.timeout();
    assert_eq!(c.freshness(), Freshness::Stale);
    good["sequence"] = json!("6");
    c.ingest(&bytes(&good), 4024).unwrap();
    assert_eq!(c.freshness(), Freshness::Fresh);
    assert!(c.presentation().unwrap().contains("physical UNKNOWN"));
}

#[test]
fn contributor_inventory_and_winning_source_are_exact() {
    // A losing Hold remains advertised alongside a winning programmer.
    let mut v = page();
    let mut p = payload(&v);
    let a = &mut p["snapshot"]["fixtures"][0]["attributes"][0];
    let value = a["resolved"].clone();
    a["programmer"] = value.clone();
    a["hold"] = json!(42);
    a["source"] = json!("programmer");
    a["contributors"] = json!([
        {"source":"programmer","value":value,"winner":true},
        {"source":"hold","value":42,"winner":false}
    ]);
    set_payload(&mut v, p.clone());
    client().ingest(&bytes(&v), 0).unwrap();
    for mutation in 0..3 {
        let mut invalid = p.clone();
        let a = &mut invalid["snapshot"]["fixtures"][0]["attributes"][0];
        match mutation {
            0 => {
                a["contributors"].as_array_mut().unwrap().pop();
            }
            1 => a["source"] = json!("hold"),
            _ => a["contributors"]
                .as_array_mut()
                .unwrap()
                .push(json!({"source":"fixture_default","value":0,"winner":false})),
        }
        let mut bad = v.clone();
        set_payload(&mut bad, invalid);
        assert!(client().ingest(&bytes(&bad), 0).is_err());
    }
}

#[test]
fn malformed_extreme_addresses_never_panic_or_replace_cached_state() {
    for address in [i32::MIN, i32::MAX, 0, 513] {
        bad(|v| {
            let mut p = payload(v);
            p["patch"]["fixtures"][0]["address"] = json!(address);
            set_payload(v, p);
        });
    }
}

#[test]
fn playing_intensity_is_engine_supplied_level_adjusted_value() {
    let mut v = page();
    let mut p = payload(&v);
    p["authority_inventory"]["playbacks"] = json!([{"id":"playback-a","activation_order":"1","level":500,"values":[{"fixture":"fixture-11","attribute":"intensity","value":800}]}]);
    let a = &mut p["snapshot"]["fixtures"][0]["attributes"][0];
    a["programmer"] = Value::Null;
    a["hold"] = Value::Null;
    a["playing"] = json!([{"id":"playback-a","value":400}]);
    a["source"] = json!({"playback":"playback-a"});
    a["resolved"] = json!(400);
    a["final_intent"] = json!(400);
    a["contributors"] = json!([{"source":{"playback":"playback-a"},"value":400,"winner":true}]);
    set_payload(&mut v, p.clone());
    client().ingest(&bytes(&v), 0).unwrap();
    p["snapshot"]["fixtures"][0]["attributes"][0]["contributors"][0]["value"] = json!(800);
    set_payload(&mut v, p);
    assert!(client().ingest(&bytes(&v), 0).is_err());
}
