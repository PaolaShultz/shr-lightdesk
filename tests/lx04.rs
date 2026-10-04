use serde_json::{Value, json};
use shr_lightdesk::adapter::LuxClient;
fn page(inventory: &Value, sequence: u64) -> Value {
    json!({"contract":"C-LIGHT","version":1,"show_id":inventory["snapshot"]["show_id"],"module":"lighting","epoch":inventory["snapshot"]["epoch"],"writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{"page":0,"page_count":1,"encoding":"json_utf8_chunks","chunk":serde_json::to_string(inventory).unwrap()},"revision":inventory["snapshot"]["revision"],"sequence":sequence.to_string(),"effective_tick":"0","reason":null})
}
fn visit(v: &Value, count: &mut usize) {
    match v {
        Value::Object(o) if o.contains_key("snapshot") && o.contains_key("wire_schema") => {
            let epoch = v["snapshot"]["epoch"].as_str().unwrap().parse().unwrap();
            let mut c = LuxClient::new(v["snapshot"]["show_id"].as_str().unwrap(), epoch).unwrap();
            c.ingest(&serde_json::to_vec(&page(v, 1)).unwrap(), 0)
                .unwrap();
            assert_eq!(c.snapshot(), Some(v));
            assert!(c.presentation().unwrap().contains("engine transition"));
            *count += 1;
        }
        Value::Object(o) => {
            for x in o.values() {
                visit(x, count)
            }
        }
        Value::Array(a) => {
            for x in a {
                visit(x, count)
            }
        }
        _ => {}
    }
}
#[test]
fn every_actual_accepted_timed_durable_error_and_recovered_inventory_validates() {
    let mut count = 0;
    for text in [
        include_str!("fixtures/lx04/timing.json"),
        include_str!("fixtures/lx04/durable-commands.json"),
        include_str!("fixtures/lx04/recovery.json"),
    ] {
        visit(&serde_json::from_str(text).unwrap(), &mut count);
    }
    assert!(count > 50, "actual inventories {count}");
}
#[test]
fn timed_schema_transition_identity_and_static_durability_are_strict() {
    let corpus: Value = serde_json::from_str(include_str!("fixtures/lx04/timing.json")).unwrap();
    let good = &corpus["cases"][3]["after"];
    for n in 0..9 {
        let mut p = good.clone();
        match n {
            0 => p["wire_schema"] = json!("future"),
            1 => p["release"]["extra"] = json!(0),
            2 => p["release"]["transition"]["targets"][0]["current"] = json!(601),
            3 => p["release"]["transition"]["progress_ticks"] = json!(24),
            4 => p["release"]["transition"]["start_tick"] = json!("18446744073709551615"),
            5 => p["snapshot"]["fixtures"][0]["attributes"][0]["source"] = json!("hold"),
            6 => p["snapshot"]["fixtures"][0]["attributes"][0]["hold"] = json!(600),
            7 => p["snapshot"]["durability"] = json!("checkpointed"),
            _ => {
                p.as_object_mut().unwrap().remove("wire_schema");
            }
        }
        let mut c = LuxClient::new("11111111-1111-4111-8111-111111111111", 9).unwrap();
        c.ingest(&serde_json::to_vec(&page(good, 1)).unwrap(), 0)
            .unwrap();
        assert!(
            c.ingest(&serde_json::to_vec(&page(&p, 2)).unwrap(), 1)
                .is_err(),
            "mutation{n}"
        );
        assert_eq!(c.snapshot(), Some(good));
    }
}
