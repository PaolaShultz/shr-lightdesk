#![cfg(target_os = "linux")]
use serde_json::{Value, json};
use shr_lightdesk::role_client::{validate_grant, validate_verified};
fn request() -> Value {
    serde_json::from_str(include_str!("fixtures/gp09/acquire-b.json")).unwrap()
}
#[test]
fn accepted_role_binding_exact_generation_and_global_cas_are_distinct() {
    let req = request();
    let grant = json!({"format":"gigpies-role-lease","version":1,"generation":"2","binding":req["binding"]});
    assert_eq!(validate_grant(&grant, &req).unwrap(), 2);
    assert!(validate_verified(&json!({"format":"gigpies-role-verified","version":1,"lease":grant,"registry_generation":"3"}),&grant).is_ok());
    for generation in ["0", "1", "02", "18446744073709551615"] {
        let mut bad = grant.clone();
        bad["generation"] = json!(generation);
        assert!(validate_grant(&bad, &req).is_err());
    }
    let mut bad = grant.clone();
    bad["binding"]["profile"] = json!("different");
    assert!(validate_grant(&bad, &req).is_err());
    let audio: Value = serde_json::from_str(include_str!("fixtures/gp09/grant-a.json")).unwrap();
    assert!(validate_grant(&audio, &req).is_err());
}
#[test]
fn role_reply_wrong_lease_extra_fields_and_stale_registry_refuse() {
    let req = request();
    let grant = json!({"format":"gigpies-role-lease","version":1,"generation":"2","binding":req["binding"]});
    for bad in [
        json!({"format":"gigpies-role-verified","version":1,"lease":grant,"registry_generation":"1"}),
        json!({"format":"gigpies-role-verified","version":1,"lease":grant,"registry_generation":"2","extra":true}),
        json!({"format":"gigpies-role-verified","version":2,"lease":grant,"registry_generation":"2"}),
    ] {
        assert!(validate_verified(&bad, &grant).is_err());
    }
}
