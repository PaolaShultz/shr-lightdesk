#![cfg(target_os = "linux")]
//! Explicit producer integration only; no discovery, windows or physical endpoints.
use serde_json::{Value, json};
use shr_lightdesk::{
    frontend::{View, Worker},
    native_actions::Semantic,
    role_client::{Config, Monitor},
    surface::actions::Action,
};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, atomic::AtomicU64},
    time::{Duration, Instant},
};
struct Scope {
    directory: PathBuf,
    children: Vec<Child>,
}
impl Scope {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "ld-native-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            children: Vec::new(),
        }
    }
    fn dir(&self, name: &str) -> PathBuf {
        let p = self.directory.join(name);
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.kill();
            let _ = child.wait();
        }
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}
fn wait(mut predicate: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(4);
    while !predicate() {
        assert!(Instant::now() < end, "bounded integration deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn send(worker: &Worker, action: Semantic) -> View {
    let before = worker.view.lock().unwrap().completed;
    worker.action(action).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    let v = worker.view.lock().unwrap().clone();
    assert!(v.command_error.is_none(), "{}", v.notice);
    v
}
fn confirm(worker: &Worker) {
    let v = worker.view.lock().unwrap().clone();
    assert!(v.review.is_some(), "reviewed command");
    send(
        worker,
        Semantic::Reviewed {
            context: v.review_context.unwrap(),
            start: 0,
            through: v.review_text.len(),
        },
    );
    send(worker, Semantic::EnterUp);
    send(worker, Semantic::Action(Action::Confirm));
}
fn edit(worker: &Worker, text: &str) {
    send(worker, Semantic::Edit);
    send(worker, Semantic::Action(Action::Text(text.into())));
    send(worker, Semantic::EnterUp);
    send(worker, Semantic::Action(Action::Confirm));
    confirm(worker);
}
fn lighting_config(broker: PathBuf, dir: PathBuf, expected: &str) -> Config {
    let mut request: Value =
        serde_json::from_str(include_str!("fixtures/gp09/acquire-b.json")).unwrap();
    request["expected_generation"] = json!(expected);
    Config {
        executable: broker,
        directory: dir,
        request,
    }
}
#[test]
#[ignore = "explicit accepted Lux executable in GP_LUX_PROVIDER"]
fn accumulated_cue_palette_and_playback_exceed_one_edit_without_losing_state() {
    use shr_lightdesk::{
        adapter::Freshness,
        lux_control::{LightingAuthority, local::LocalClient},
    };
    let lux = PathBuf::from(std::env::var("GP_LUX_PROVIDER").expect("explicit accepted Lux"));
    let mut scope = Scope::new();
    let directory = scope.dir("large-look");
    scope.children.push(
        Command::new(lux)
            .args(["--synthetic-private-dir", directory.to_str().unwrap()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let socket = directory.join("lux.sock");
    wait(|| socket.exists());
    let mut client = LocalClient::connect(
        &socket,
        "11111111-1111-4111-8111-111111111111",
        1,
        "large-look-review",
    )
    .unwrap();
    assert_eq!(client.grant().unwrap()["kind"], "applied");
    // Use the provider's own advertised seven-attribute synthetic personality.
    let template = client.authority.snapshot().unwrap()["patch"]["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["mode"] == "rgb_position")
        .expect("synthetic RGB position fixture")
        .clone();
    let mut fixtures = Vec::new();
    let mut values = Vec::new();
    for n in 0..10 {
        let mut fixture = template.clone();
        fixture["id"] = json!(format!("large-{n}"));
        fixture["address"] = json!(1 + n * 7);
        for cap in fixture["capabilities"].as_array().unwrap() {
            values.push(json!({"fixture":fixture["id"],"attribute":cap["attribute"],"value":cap["default"]}));
        }
        fixtures.push(fixture);
    }
    let patch = json!({"version":1,"patch_revision":"2","fixtures":fixtures});
    assert_eq!(
        client
            .command(json!({"action":"replace_patch","patch":patch}))
            .unwrap()["kind"],
        "applied"
    );
    assert_eq!(values.len(), 70);
    // The provider still refuses an oversized single edit.
    assert_eq!(
        client
            .command(json!({"action":"touch","values":values}))
            .unwrap()["reason"],
        "capacity"
    );
    for batch in values.chunks(35) {
        assert_eq!(
            client
                .command(json!({"action":"touch","values":batch}))
                .unwrap()["kind"],
            "applied"
        );
    }
    for action in [
        json!({"action":"record","kind":"cue","id":"large-cue"}),
        json!({"action":"record","kind":"palette","id":"large-palette"}),
        json!({"action":"go","cue":"large-cue","playback":"large-playback"}),
    ] {
        assert_eq!(client.command(action).unwrap()["kind"], "applied");
        assert_eq!(
            client.authority.freshness(),
            Freshness::Fresh,
            "{}",
            client.authority.notice
        );
    }
    for kind in ["cues", "palettes", "playbacks"] {
        assert_eq!(
            client.authority.snapshot().unwrap()["authority_inventory"][kind][0]["values"]
                .as_array()
                .unwrap()
                .len(),
            70
        );
    }
}
#[test]
#[ignore = "explicit accepted role/Lux executables in GP_ROLE_BROKER and GP_LUX_PROVIDER"]
fn actual_role_lux_semantic_render_and_loss_fence() {
    let broker = PathBuf::from(std::env::var("GP_ROLE_BROKER").expect("explicit accepted broker"));
    let lux = PathBuf::from(std::env::var("GP_LUX_PROVIDER").expect("explicit accepted Lux"));
    let mut scope = Scope::new();
    let role_dir = scope.dir("roles");
    let lux_dir = scope.dir("lux");
    let child = Command::new(lux)
        .args([
            "--synthetic-private-dir",
            lux_dir.to_str().unwrap(),
            "--durable",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    scope.children.push(child);
    wait(|| lux_dir.join("lux.sock").exists());
    let worker = Worker::spawn_with_role(
        lux_dir.join("lux.sock"),
        "11111111-1111-4111-8111-111111111111".into(),
        1,
        Some(lighting_config(broker, role_dir, "0")),
    );
    wait(|| {
        let v = worker.view.lock().unwrap();
        v.inventory.is_some() && v.role_live
    });
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Grant);
    send(&worker, Semantic::Action(Action::Select(vec![1])));
    // Parsed console mutations cannot bypass the semantic full-review gate.
    let before_revision =
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["snapshot"]["revision"].clone();
    let before = worker.view.lock().unwrap().completed;
    worker.enqueue("touch\tintensity  700".into()).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    let view = worker.view.lock().unwrap().clone();
    assert!(view.review.is_some());
    assert!(
        view.review_text
            .iter()
            .any(|l| l.contains("destination 70%"))
    );
    assert_eq!(
        view.inventory.as_ref().unwrap()["snapshot"]["revision"],
        before_revision
    );
    let before = view.completed;
    worker.enqueue("enter\t down".into()).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    assert!(
        worker
            .view
            .lock()
            .unwrap()
            .command_error
            .as_ref()
            .unwrap()
            .contains("review all")
    );
    assert_eq!(
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["snapshot"]["revision"],
        before_revision
    );
    send(&worker, Semantic::Action(Action::Cancel));
    edit(&worker, "70");
    let v = worker.view.lock().unwrap().clone();
    assert_eq!(
        v.inventory.as_ref().unwrap()["snapshot"]["fixtures"][0]["attributes"][0]["programmer"],
        700
    );
    send(
        &worker,
        Semantic::Action(Action::Record {
            palette: false,
            replace: false,
            id: 0,
        }),
    );
    send(&worker, Semantic::Action(Action::Text("look-a".into())));
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Action(Action::Confirm));
    confirm(&worker);
    send(&worker, Semantic::OpenGo);
    send(
        &worker,
        Semantic::Action(Action::Text("look-a playback-a".into())),
    );
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Action(Action::Confirm));
    confirm(&worker);
    edit(&worker, "0");
    send(&worker, Semantic::Action(Action::ClearHold));
    confirm(&worker);
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Preview);
    let v = worker.view.lock().unwrap().clone();
    assert_eq!(
        v.review.as_ref().unwrap()["token"]["preview"]["values"][0]["destination"],
        700
    );
    confirm(&worker);
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["snapshot"]["fixtures"][0]["attributes"]
            [0]["final_intent"]
            == 700
    });
    let v = worker.view.lock().unwrap().clone();
    assert!(shr_lightdesk::frontend::scene(&v, "", 0).in_bounds());
    send(&worker, Semantic::Mode);
    confirm(&worker);
    send(&worker, Semantic::Blackout);
    confirm(&worker);
    send(&worker, Semantic::Blackout);
    confirm(&worker);
    // A console confirmation uses the same full-review gate as semantic Enter.
    send(&worker, Semantic::Action(Action::ClearHold));
    let before = worker.view.lock().unwrap().completed;
    worker.enqueue("confirm".into()).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    assert!(
        worker
            .view
            .lock()
            .unwrap()
            .command_error
            .as_ref()
            .unwrap()
            .contains("review all")
    );
    for text in ["enter  down", "enter\tdown", "  confirm  ", "enter\n down"] {
        let before = worker.view.lock().unwrap().completed;
        worker.enqueue(text.into()).unwrap();
        wait(|| worker.view.lock().unwrap().completed > before);
        assert!(
            worker
                .view
                .lock()
                .unwrap()
                .command_error
                .as_ref()
                .unwrap()
                .contains("review all")
        );
    }
    send(&worker, Semantic::Action(Action::ContextLost));
    let before = worker.view.lock().unwrap().completed;
    worker.action(Semantic::Action(Action::Confirm)).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    assert!(worker.view.lock().unwrap().command_error.is_some());
    let role = worker.role.as_ref().unwrap();
    role.publish_leds(["selected"; 8]);
    assert!(role.take_leds().is_some());
    let pid = role.state.lock().unwrap().child_pid.unwrap();
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
    wait(|| !worker.view.lock().unwrap().role_live);
    assert!(role.take_leds().is_none());
    let before = worker.view.lock().unwrap().completed;
    worker.action(Semantic::Grant).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    assert!(
        worker
            .view
            .lock()
            .unwrap()
            .command_error
            .as_ref()
            .unwrap()
            .contains("GP09")
    );
    let retained = worker.view.lock().unwrap().inventory.clone().unwrap();
    let lux_socket = lux_dir.join("lux.sock");
    let broker_path = PathBuf::from(std::env::var("GP_ROLE_BROKER").unwrap());
    let role_path = worker
        .role
        .as_ref()
        .unwrap()
        .state
        .lock()
        .unwrap()
        .grant
        .as_ref()
        .unwrap()["generation"]
        .as_str()
        .unwrap()
        .to_owned();
    drop(worker);
    let recovered = Worker::spawn_with_role(
        lux_socket,
        "11111111-1111-4111-8111-111111111111".into(),
        1,
        Some(lighting_config(
            broker_path,
            scope.directory.join("roles"),
            &role_path,
        )),
    );
    wait(|| {
        let v = recovered.view.lock().unwrap();
        v.inventory.is_some() && v.role_live
    });
    let view = recovered.view.lock().unwrap().clone();
    assert!(!view.lease);
    assert!(view.review.is_none());
    assert!(view.editor.is_none());
    let current = view.inventory.unwrap();
    assert_eq!(
        current["snapshot"]["revision"],
        retained["snapshot"]["revision"]
    );
    assert_eq!(current["blackout"], retained["blackout"]);
    assert_eq!(
        current["snapshot"]["fixtures"],
        retained["snapshot"]["fixtures"]
    );
    drop(recovered);
}
#[test]
#[ignore = "explicit accepted role executable in GP_ROLE_BROKER"]
fn actual_two_role_global_generation_does_not_invalidate_live_lighting_lease() {
    let broker = PathBuf::from(std::env::var("GP_ROLE_BROKER").unwrap());
    let mut scope = Scope::new();
    let role_dir = scope.dir("roles");
    let mut audio = Command::new(&broker)
        .arg(&role_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = audio.stdin.take().unwrap();
    let mut stdout = BufReader::new(audio.stdout.take().unwrap());
    stdin
        .write_all(include_bytes!("fixtures/gp09/acquire-a.json"))
        .unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap()["generation"],
        "1"
    );
    let monitor = Monitor::spawn(
        lighting_config(broker.clone(), role_dir.clone(), "1"),
        Arc::new(AtomicU64::new(1)),
    );
    wait(|| monitor.live.load(std::sync::atomic::Ordering::Acquire));
    audio.kill().unwrap();
    audio.wait().unwrap();
    drop(stdin);
    drop(stdout);
    let mut audio = Command::new(&broker)
        .arg(&role_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = audio.stdin.take().unwrap();
    let mut stdout = BufReader::new(audio.stdout.take().unwrap());
    stdin
        .write_all(include_bytes!("fixtures/gp09/reclaim-a.json"))
        .unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap()["generation"],
        "3"
    );
    std::thread::sleep(Duration::from_millis(650));
    assert!(monitor.state.lock().unwrap().live());
    assert_eq!(
        monitor.state.lock().unwrap().grant.as_ref().unwrap()["generation"],
        "2"
    );
    drop(monitor);
    drop(stdin);
    audio.wait().unwrap();
    scope.children.push(audio);
}
#[test]
#[ignore = "explicit accepted role executable; own synthetic child SIGSTOP only"]
fn actual_role_timeout_fences_generation_and_led_delivery() {
    let broker = PathBuf::from(std::env::var("GP_ROLE_BROKER").unwrap());
    let scope = Scope::new();
    let role_dir = scope.dir("roles");
    let fence = Arc::new(AtomicU64::new(1));
    let monitor = Monitor::spawn(lighting_config(broker, role_dir, "0"), fence.clone());
    wait(|| monitor.state.lock().unwrap().live());
    monitor.publish_leds(["available"; 8]);
    let pid = monitor.state.lock().unwrap().child_pid.unwrap();
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
    let generation = fence.load(std::sync::atomic::Ordering::Acquire);
    wait(|| !monitor.live.load(std::sync::atomic::Ordering::Acquire));
    assert!(fence.load(std::sync::atomic::Ordering::Acquire) > generation);
    assert!(monitor.take_leds().is_none());
    assert!(monitor.state.lock().unwrap().notice.contains("500ms"));
    drop(monitor);
}

#[test]
#[ignore = "explicit configured GigPies/Lux/topology/role artifacts; synthetic null output only"]
fn configured_analysis_calibration_scoped_auto_and_source_loss() {
    let source_bin =
        PathBuf::from(std::env::var("GP_ANALYSIS_PROVIDER").expect("configured producer"));
    let topology = PathBuf::from(std::env::var("GP_ANALYSIS_TOPOLOGY").expect("owner topology"));
    let lux = PathBuf::from(std::env::var("GP_LUX_PROVIDER").expect("configured Lux"));
    let broker = PathBuf::from(std::env::var("GP_ROLE_BROKER").expect("role broker"));
    let mut scope = Scope::new();
    let source = scope.dir("source");
    let owner = scope.dir("lux");
    let roles = scope.dir("roles");
    let corpus: Value =
        serde_json::from_str(include_str!("fixtures/lx05/v2/configured.json")).unwrap();
    let config = scope.directory.join("config.json");
    let mapping = scope.directory.join("mapping.json");
    std::fs::write(&config, serde_json::to_vec(&corpus["config"]).unwrap()).unwrap();
    std::fs::write(
        &mapping,
        serde_json::to_vec(&json!({"version":1,"inputs":corpus["config"]["analysis"]["inputs"]}))
            .unwrap(),
    )
    .unwrap();
    let show = corpus["config"]["show_id"].as_str().unwrap();
    let start_source = |epoch: &str| {
        Command::new(&source_bin)
            .args([
                "--directory",
                source.to_str().unwrap(),
                "--show",
                show,
                "--epoch",
                epoch,
                "--synthetic-source",
                "fouraux",
                "--topology",
                topology.to_str().unwrap(),
                "--analysis-map",
                mapping.to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    };
    scope.children.push(start_source("9"));
    wait(|| source.join("analysis.sock").exists());
    scope.children.push(
        Command::new(&lux)
            .args([
                "--synthetic-private-dir",
                owner.to_str().unwrap(),
                "--durable",
                "--config",
                config.to_str().unwrap(),
                "--analysis",
                source.join("analysis.sock").to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    wait(|| owner.join("lux.sock").exists());
    let worker = Worker::spawn_with_role(
        owner.join("lux.sock"),
        show.into(),
        1,
        Some(lighting_config(broker, roles, "0")),
    );
    wait(|| {
        let v = worker.view.lock().unwrap();
        v.role_live
            && v.inventory
                .as_ref()
                .is_some_and(|i| !i["analysis"]["source_identity"].is_null())
    });
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Grant);
    send(&worker, Semantic::Action(Action::Select(vec![1])));
    let desk_index = if let Ok(executable) = std::env::var("GP_DESK_READONLY_EXECUTABLE") {
        let expected =
            std::env::var("GP_DESK_READONLY_SHA256").expect("exact Desk executable hash");
        let sum = Command::new("sha256sum").arg(&executable).output().unwrap();
        assert!(sum.status.success());
        assert_eq!(
            String::from_utf8(sum.stdout)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap(),
            expected
        );
        let output = scope.directory.join("coexist.ppm");
        let index = scope.children.len();
        scope.children.push(
            Command::new(executable)
                .args([
                    "--headless",
                    source.join("audio.sock").to_str().unwrap(),
                    show,
                    "9",
                    "coexist-readonly",
                    "foh",
                    output.to_str().unwrap(),
                    "--dynamic",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        Some(index)
    } else {
        None
    };
    send(&worker, Semantic::Calibrate(false));
    confirm(&worker);
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["calibration_windows"]
            .as_u64()
            .unwrap_or(0)
            >= 60
    });
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Calibrate(true));
    confirm(&worker);
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["state"] == "ready"
    });
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::AutoEnter);
    confirm(&worker);
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::AutoGrant);
    send(&worker, Semantic::Action(Action::Text("50 2000".into())));
    send(&worker, Semantic::Action(Action::Confirm));
    let reviewed = worker.view.lock().unwrap().clone();
    assert!(
        reviewed
            .review_text
            .iter()
            .any(|line| line.contains("front"))
    );
    assert!(
        reviewed
            .review_text
            .iter()
            .any(|line| line.contains("2000"))
    );
    let review_scene = shr_lightdesk::frontend::scene(&reviewed, "", 0);
    assert!(review_scene.in_bounds());
    let review_ppm = shr_lightdesk::render::ppm(&review_scene);
    assert!(review_ppm.starts_with(b"P6\n1920 1080\n255\n"));
    confirm(&worker);
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["automatic_layer"]["values"].as_array().is_some_and(|vs|vs.iter().any(|v|v["fixture"]=="front" && v["source"]=="analysis-active"))
    });
    let active = worker.view.lock().unwrap().inventory.clone().unwrap();
    assert_eq!(active["analysis"]["grant"]["fixtures"], json!(["front"]));
    let back = active["snapshot"]["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["fixture"] == "back")
        .unwrap();
    assert!(
        back["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["resolved"] == 0)
    );
    // The bounded grant expires without automatic renewal while the source stays live.
    wait(|| worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["grant"].is_null());
    assert_eq!(
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["state"],
        "ready"
    );
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::AutoGrant);
    send(&worker, Semantic::Action(Action::Text("50 2000".into())));
    send(&worker, Semantic::Action(Action::Confirm));
    confirm(&worker);
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["automatic_layer"]["values"].as_array().is_some_and(|vs| vs.iter().any(|v| v["fixture"] == "front" && v["source"] == "analysis-active"))
    });
    scope.children[0].kill().unwrap();
    scope.children[0].wait().unwrap();
    wait(|| {
        let v = worker.view.lock().unwrap();
        let a = &v.inventory.as_ref().unwrap()["analysis"];
        a["state"] == "absent" && a["grant"].is_null()
    });
    let retained =
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["automatic_layer"]
            .clone();
    assert!(
        retained["values"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["fixture"] == "front"
                && v["provenance"]["source_identity"]["source_epoch"] == "9")
    );
    // These private endpoints belong to the exited child; never unlink a live owner.
    for name in ["audio.sock", "analysis.sock"] {
        let path = source.join(name);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }
    }
    scope.children.push(start_source("10"));
    wait(|| {
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["analysis"]["source_epoch"] == "10"
    });
    let mut health_view = worker.view.lock().unwrap().clone();
    health_view.page = "health".into();
    let health_scene = shr_lightdesk::frontend::scene(&health_view, "", 0);
    assert!(health_scene.in_bounds());
    assert!(shr_lightdesk::render::svg(&health_scene).contains("<svg"));
    let rebound = worker.view.lock().unwrap().inventory.clone().unwrap();
    assert!(rebound["analysis"]["grant"].is_null());
    assert_ne!(rebound["analysis"]["state"], "ready");
    assert_eq!(rebound["analysis"]["automatic_layer"], retained);
    send(&worker, Semantic::EnterUp);
    edit(&worker, "0");
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::Action(Action::ClearHold));
    confirm(&worker);
    let held = worker.view.lock().unwrap().inventory.clone().unwrap();
    let front = held["snapshot"]["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["fixture"] == "front")
        .unwrap();
    assert_eq!(front["attributes"][0]["resolved"], 0);
    assert_eq!(front["attributes"][0]["source"], "hold");
    send(&worker, Semantic::EnterUp);
    send(&worker, Semantic::AutoRevoke);
    confirm(&worker);
    assert_eq!(
        worker.view.lock().unwrap().inventory.as_ref().unwrap()["mode"],
        "assist"
    );
    send(&worker, Semantic::EnterUp);
    let before = worker.view.lock().unwrap().completed;
    worker.enqueue("checkpoint".into()).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    assert!(worker.view.lock().unwrap().command_error.is_none());
    confirm(&worker);
    if let Some(index) = desk_index {
        use std::io::Read;
        wait(|| scope.children[index].try_wait().unwrap().is_some());
        assert!(scope.children[index].wait().unwrap().success());
        let mut stdout = String::new();
        scope.children[index]
            .stdout
            .take()
            .unwrap()
            .take(1_048_576)
            .read_to_string(&mut stdout)
            .unwrap();
        assert!(stdout.contains("fresh=true"), "{stdout}");
        let output = scope.directory.join("coexist.ppm");
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.starts_with(b"P6\n1920 1080\n255\n"));
        let hash = Command::new("sha256sum").arg(&output).output().unwrap();
        println!(
            "Desk coexistence fresh=true PPM {}",
            String::from_utf8(hash.stdout)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
        );
    }
    drop(worker);
    scope.children[1].kill().unwrap();
    scope.children[1].wait().unwrap();
    std::fs::remove_file(owner.join("lux.sock")).unwrap();
    scope.children.push(
        Command::new(&lux)
            .args([
                "--synthetic-private-dir",
                owner.to_str().unwrap(),
                "--durable",
                "--config",
                config.to_str().unwrap(),
                "--analysis",
                source.join("analysis.sock").to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let mut recovered = None;
    wait(|| {
        recovered =
            shr_lightdesk::lux_operator::Operator::connect(&owner.join("lux.sock"), show, 2).ok();
        recovered.is_some()
    });
    let recovered = recovered.unwrap();
    use shr_lightdesk::lux_control::LightingAuthority;
    let i = recovered.client.authority.snapshot().unwrap();
    assert_eq!(i["snapshot"]["epoch"], "2");
    assert!(i["analysis"]["grant"].is_null());
    assert_eq!(i["mode"], "manual");
    assert_eq!(i["snapshot"]["fixtures"][0]["attributes"][0]["resolved"], 0);
}
