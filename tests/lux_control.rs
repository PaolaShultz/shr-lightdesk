use serde_json::{Value, json};
use shr_lightdesk::{
    adapter::Freshness,
    lux_control::{LightingAuthority, LuxAuthority},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn corpus() -> Value {
    serde_json::from_str(include_str!("fixtures/lx03/commands.json")).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap()
}
fn scope() -> Value {
    json!({"scope":"lighting-control"})
}
fn fresh(c: &mut LuxAuthority, now: u64, sequence: u64) {
    let mut page = corpus()["cases"][0]["replies"][0].clone();
    page["sequence"] = json!(sequence.to_string());
    assert!(c.observe(&bytes(&page), now).unwrap());
}
fn client() -> LuxAuthority {
    let mut c = LuxAuthority::new(SHOW, 9, "writer-a").unwrap();
    fresh(&mut c, 0, 1);
    c
}
fn grant(c: &mut LuxAuthority, first: u64, receipt: u64) {
    c.begin("grant", scope(), first).unwrap();
    c.acknowledge(&bytes(&corpus()["cases"][1]["replies"][0]), receipt)
        .unwrap();
}
#[test]
fn grant_requires_complete_fresh_read_and_delayed_reply_deadline_uses_first_send() {
    let mut c = LuxAuthority::new(SHOW, 9, "writer-a").unwrap();
    assert!(c.begin("grant", scope(), 0).is_err());
    fresh(&mut c, 1000, 1);
    grant(&mut c, 1000, 1500);
    assert_eq!(c.lease_deadline(), Some(3000));
    assert!(c.writable(2999));
    c.advance(3000).unwrap();
    assert!(!c.writable(3000));
    assert_eq!(c.freshness(), Freshness::Stale);
}
#[test]
fn fixed_retry_keeps_exact_envelope_id_and_payload_then_bounded_recovery_drops_intent() {
    let mut c = client();
    grant(&mut c, 0, 0);
    let body = json!({"scope":"lighting-control","command":{"action":"touch","values":[{"fixture":"fixture-11","attribute":"intensity","value":0}]}});
    let original = c.begin("command", body, 10).unwrap();
    assert!(c.begin("command", json!({}), 10).is_err());
    assert_eq!(c.retry(109).unwrap(), None);
    for now in [110, 260, 510] {
        assert_eq!(c.retry(now).unwrap(), Some(original.clone()));
    }
    assert_eq!(c.retry(1000).unwrap(), None);
    assert!(c.retry(1510).is_err());
    assert!(c.pending.is_none());
    assert!(c.uncertain);
    fresh(&mut c, 1511, 2);
    assert!(!c.uncertain);
    assert!(c.pending.is_none());
    assert!(!c.writable(1511));
    assert!(c.begin("grant", scope(), 1511).is_err());
    assert!(c.begin("command", json!({}), 1511).is_err());
}
#[test]
fn unsolicited_fresh_snapshot_resolves_current_look_without_fake_ack_or_pending_deadlock() {
    let mut c = client();
    grant(&mut c, 0, 0);
    c.begin(
        "command",
        json!({"scope":"lighting-control","command":{"action":"blackout","enabled":true}}),
        10,
    )
    .unwrap();
    fresh(&mut c, 20, 2);
    assert!(c.pending.is_none());
    assert!(!c.writable(20));
    assert!(c.notice.contains("unconfirmed"));
    assert_ne!(c.last_reply.as_ref().unwrap()["request_id"], "2");
}
#[test]
fn cached_old_ack_never_rolls_back_newer_state_and_wrong_identity_fails_closed() {
    let mut c = client();
    grant(&mut c, 0, 0);
    let data = corpus();
    let input = &data["cases"][2]["input"];
    c.begin("command", input["body"].clone(), 10).unwrap();
    let ack = bytes(&data["cases"][2]["replies"][0]);
    assert!(c.acknowledge(&ack, 20).unwrap());
    assert_eq!(c.revision(), 1);
    assert!(
        !c.acknowledge(&bytes(&data["cases"][1]["replies"][0]), 21)
            .unwrap()
    );
    assert_eq!(c.revision(), 1);
    assert!(!c.acknowledge(&ack, 22).unwrap());
    let mut wrong = data["cases"][1]["replies"][0].clone();
    wrong["epoch"] = json!("8");
    assert!(c.acknowledge(&bytes(&wrong), 23).is_err());
    assert!(c.lease_id().is_none());
}
#[test]
fn grant_reply_schema_mutations_refuse_unknown_missing_float_overflow_and_identity() {
    for mutation in 0..11 {
        let mut c = client();
        c.begin("grant", scope(), 0).unwrap();
        let mut ack = corpus()["cases"][1]["replies"][0].clone();
        match mutation {
            0 => ack["extra"] = json!(true),
            1 => {
                ack.as_object_mut().unwrap().remove("lease");
            }
            2 => ack["request_id"] = json!("2"),
            3 => ack["writer"] = json!("wrong"),
            4 => ack["expected_revision"] = json!("1"),
            5 => ack["body"]["remaining_ms"] = json!(2000.0),
            6 => ack["body"]["lease"] = json!("18446744073709551615"),
            7 => ack["body"]["extra"] = json!(0),
            8 => ack["version"] = json!(2),
            9 => ack["show_id"] = json!("22222222-2222-4222-8222-222222222222"),
            _ => ack["lease"] = json!("1"),
        }
        assert!(
            c.acknowledge(&bytes(&ack), 100).is_err(),
            "mutation {mutation}"
        );
        assert!(c.pending.is_none());
        assert!(c.lease_id().is_none());
        assert_eq!(c.freshness(), Freshness::Stale);
    }
}
#[cfg(target_os = "linux")]
#[test]
fn context_changes_and_held_enter_cannot_commit_another_target() {
    use shr_lightdesk::lux_operator::{Confirmation, InputContext};
    let mut c = InputContext::default();
    let confirmation = Confirmation {
        command: json!({"action":"blackout","enabled":false}),
        material_lines: Vec::new(),
        context: 0,
        revision: 5,
        lease: "1".into(),
        epoch: 9,
        expires: 2000,
    };
    c.confirmation = Some(confirmation.clone());
    assert_eq!(c.press(5, Some("1"), 9, 0).unwrap(), confirmation.command);
    c.confirmation = Some(confirmation.clone());
    assert!(c.press(5, Some("1"), 9, 1).is_err());
    c.release();
    c.invalidate().unwrap();
    assert!(c.press(5, Some("1"), 9, 2).is_err());
    c.release();
    c.confirmation = Some(Confirmation {
        context: c.generation,
        ..confirmation.clone()
    });
    assert!(c.press(6, Some("1"), 9, 3).is_err());
    c.release();
    c.confirmation = Some(Confirmation {
        context: c.generation,
        ..confirmation
    });
    c.lost().unwrap();
    assert!(c.press(5, Some("1"), 9, 4).is_err());
}

#[test]
fn delayed_cached_renew_uses_original_first_send_and_expires_fail_closed() {
    let mut c = client();
    grant(&mut c, 0, 0);
    c.begin("renew", scope(), 500).unwrap();
    let mut ack = corpus()["cases"][1]["replies"][0].clone();
    ack["request_id"] = json!("2");
    ack["lease"] = json!("1");
    ack["sequence"] = json!("3");
    ack["body"] = json!({"remaining_ms":2000});
    c.acknowledge(&bytes(&ack), 1000).unwrap();
    assert_eq!(c.lease_deadline(), Some(2500));
    assert!(!c.acknowledge(&bytes(&ack), 1400).unwrap());
    assert_eq!(c.lease_deadline(), Some(2500));
    c.advance(2500).unwrap();
    assert!(!c.writable(2500));
    let mut c = client();
    c.begin("grant", scope(), 0).unwrap();
    assert!(
        c.acknowledge(&bytes(&corpus()["cases"][1]["replies"][0]), 2000)
            .is_err()
    );
    assert!(c.lease_id().is_none());
}
fn timed() -> Value {
    serde_json::from_str(include_str!("fixtures/lx04/timing.json")).unwrap()
}
fn observation(inventory: &Value, sequence: u64) -> Vec<u8> {
    let mut p = corpus()["cases"][0]["replies"][0].clone();
    p["sequence"] = json!(sequence.to_string());
    p["revision"] = inventory["snapshot"]["revision"].clone();
    p["body"]["chunk"] = json!(serde_json::to_string(inventory).unwrap());
    bytes(&p)
}
fn timed_client() -> LuxAuthority {
    let data = timed();
    let mut c = LuxAuthority::new(SHOW, 9, "writer-a").unwrap();
    c.observe(&observation(&data["cases"][0]["prior"], 1), 0)
        .unwrap();
    c.begin("grant", scope(), 0).unwrap();
    c.acknowledge(&bytes(&data["cases"][0]["replies"][0]), 0)
        .unwrap();
    c
}
#[test]
fn actual_engine_preview_token_and_commit_transition_validate_without_destination_calculation() {
    let data = timed();
    let mut c = timed_client();
    c.begin("command", data["cases"][1]["input"]["body"].clone(), 0)
        .unwrap();
    c.acknowledge(&bytes(&data["cases"][1]["replies"][0]), 0)
        .unwrap();
    assert_eq!(
        c.preview.as_ref(),
        Some(&data["cases"][1]["replies"][0]["body"]["token"])
    );
    c.observe(&observation(&data["cases"][1]["after"], 2), 1)
        .unwrap();
    c.begin("command", data["cases"][2]["input"]["body"].clone(), 1)
        .unwrap();
    c.acknowledge(&bytes(&data["cases"][2]["replies"][0]), 1)
        .unwrap();
    assert!(c.preview.is_none());
    assert_eq!(c.revision(), 4);
    c.observe(&observation(&data["cases"][3]["after"], 3), 250)
        .unwrap();
    let current = c.snapshot().unwrap().clone();
    let revision = c.revision();
    assert!(
        !c.acknowledge(&bytes(&data["cases"][2]["replies"][0]), 251)
            .unwrap()
    );
    assert_eq!(c.revision(), revision);
    assert_eq!(c.snapshot(), Some(&current));
}
#[test]
fn preview_identity_selection_schema_nonce_tick_and_range_mutations_fail_closed() {
    let data = timed();
    for n in 0..10 {
        let mut c = timed_client();
        c.begin("command", data["cases"][1]["input"]["body"].clone(), 0)
            .unwrap();
        let mut ack = data["cases"][1]["replies"][0].clone();
        let t = &mut ack["body"]["token"];
        match n {
            0 => t["writer"] = json!("wrong"),
            1 => t["lease"] = json!("2"),
            2 => t["nonce"] = json!("nope"),
            3 => t["preview"]["revision"] = json!("4"),
            4 => t["preview"]["epoch"] = json!("8"),
            5 => t["preview"]["expiry_tick"] = json!("201"),
            6 => t["preview"]["values"][0]["fixture"] = json!("f2"),
            7 => t["preview"]["values"][0]["current"] = json!(701),
            8 => t["preview"]["values"][0]["destination"] = json!(1001),
            _ => t["extra"] = json!(true),
        }
        assert!(c.acknowledge(&bytes(&ack), 0).is_err(), "mutation{n}");
        assert!(c.preview.is_none());
        assert!(!c.writable(0));
    }
}
#[test]
fn snapshot_older_than_confirmed_ack_is_stale_and_never_replaces_cached_values() {
    let data = corpus();
    let mut c = client();
    grant(&mut c, 0, 0);
    c.begin("command", data["cases"][2]["input"]["body"].clone(), 0)
        .unwrap();
    c.acknowledge(&bytes(&data["cases"][2]["replies"][0]), 0)
        .unwrap();
    let old = c.snapshot().unwrap().clone();
    let mut page = data["cases"][0]["replies"][0].clone();
    page["sequence"] = json!("99");
    assert!(c.observe(&bytes(&page), 1).is_err());
    assert_eq!(c.snapshot(), Some(&old));
    assert_eq!(c.freshness(), Freshness::Stale);
}
#[test]
fn correlated_renew_conflict_allows_bounded_fresh_new_id_but_uncertain_does_not() {
    let mut c = client();
    grant(&mut c, 0, 0);
    c.begin("renew", scope(), 500).unwrap();
    let mut refusal = corpus()["cases"][1]["replies"][0].clone();
    refusal["request_id"] = json!("2");
    refusal["lease"] = json!("1");
    refusal["expected_revision"] = json!("0");
    refusal["revision"] = json!("1");
    refusal["kind"] = json!("conflict");
    refusal["reason"] = json!("stale_revision");
    refusal["body"] = Value::Null;
    c.acknowledge(&bytes(&refusal), 510).unwrap();
    assert!(!c.uncertain);
    assert!(c.pending.is_none());
    let mut p = corpus()["cases"][0]["replies"][0].clone();
    p["sequence"] = json!("10");
    p["revision"] = json!("1");
    let mut inventory: Value = serde_json::from_str(p["body"]["chunk"].as_str().unwrap()).unwrap();
    inventory["snapshot"]["revision"] = json!("1");
    p["body"]["chunk"] = json!(serde_json::to_string(&inventory).unwrap());
    c.observe(&bytes(&p), 511).unwrap();
    let request: Value = serde_json::from_slice(&c.begin("renew", scope(), 512).unwrap()).unwrap();
    assert_eq!(request["request_id"], "3");
    assert_eq!(request["expected_revision"], "1");
    assert!(c.begin("renew", scope(), 513).is_err());
}
#[test]
fn protected_review_refuses_changed_revision_after_maintenance_before_any_send() {
    let mut c = client();
    grant(&mut c, 0, 0);
    c.check_review(0, "1", 0).unwrap();
    let mut p = corpus()["cases"][0]["replies"][0].clone();
    p["sequence"] = json!("2");
    p["revision"] = json!("1");
    let mut inventory: Value = serde_json::from_str(p["body"]["chunk"].as_str().unwrap()).unwrap();
    inventory["snapshot"]["revision"] = json!("1");
    p["body"]["chunk"] = json!(serde_json::to_string(&inventory).unwrap());
    c.observe(&bytes(&p), 500).unwrap();
    assert!(c.check_review(0, "1", 500).is_err());
    assert!(c.pending.is_none());
}
#[cfg(target_os = "linux")]
#[test]
fn real_transport_valid_ack_then_snapshot_failure_retains_confirmed_outcome_stale() {
    use shr_lightdesk::lux_control::local::LocalClient;
    use std::{
        io::{Read, Write},
        os::unix::{fs::PermissionsExt, net::UnixListener},
        time::Duration,
    };
    let dir = std::env::temp_dir().join(format!(
        "ld-ack-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let dir = dir.canonicalize().unwrap();
    let path = dir.join("lux.sock");
    let listener = UnixListener::bind(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let read = |s: &mut std::os::unix::net::UnixStream| {
            let mut h = [0; 4];
            s.read_exact(&mut h).unwrap();
            let mut b = vec![0; u32::from_be_bytes(h) as usize];
            s.read_exact(&mut b).unwrap();
        };
        let send = |s: &mut std::os::unix::net::UnixStream, v: &Value| {
            let b = bytes(v);
            s.write_all(&(b.len() as u32).to_be_bytes()).unwrap();
            s.write_all(&b).unwrap();
        };
        read(&mut stream);
        send(&mut stream, &corpus()["cases"][0]["replies"][0]);
        read(&mut stream);
        send(&mut stream, &corpus()["cases"][1]["replies"][0]);
        read(&mut stream); // Close instead of replying to post-ACK snapshot.
    });
    let mut c = LocalClient::connect(&path, SHOW, 9, "writer-a").unwrap();
    let reply = c.grant().unwrap();
    assert_eq!(reply["kind"], "applied");
    assert_eq!(c.authority.last_reply.as_ref().unwrap(), &reply);
    assert!(!c.authority.uncertain);
    assert_eq!(c.authority.freshness(), Freshness::Stale);
    assert!(!c.authority.writable(c.now()));
    assert!(c.authority.notice.contains("ACK"));
    drop(c);
    thread.join().unwrap();
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
#[test]
fn session_time_bound_and_enter_without_confirmation_fail_closed() {
    use shr_lightdesk::lux_control::{SESSION_MS, session_guard};
    assert!(session_guard(SESSION_MS - 1).is_ok());
    assert!(session_guard(SESSION_MS).is_err());
    assert!(session_guard(u64::MAX).is_err());
    #[cfg(target_os = "linux")]
    {
        use shr_lightdesk::lux_operator::{Confirmation, InputContext};
        let mut input = InputContext::default();
        assert!(input.press(0, Some("1"), 9, 0).is_err());
        assert!(input.held_enter);
        input.confirmation = Some(Confirmation {
            command: json!({"action":"blackout","enabled":false}),
            material_lines: Vec::new(),
            context: 0,
            revision: 0,
            lease: "1".into(),
            epoch: 9,
            expires: 2000,
        });
        assert!(input.press(0, Some("1"), 9, 1).is_err());
        input.release();
        assert!(input.press(0, Some("1"), 9, 2).is_ok());
    }
}
