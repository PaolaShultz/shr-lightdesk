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
