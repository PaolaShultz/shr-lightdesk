#![cfg(target_os = "linux")]
//! Bounded private synthetic transport; no Lux executable, role broker or hardware.
use serde_json::{Value, json};
use shr_lightdesk::{
    adapter::Freshness,
    codec,
    frontend::Worker,
    lux_control::{LightingAuthority, local::LocalClient},
    native_actions::Semantic,
    surface::actions::Action,
};
use std::{
    io::Write,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
struct Observer {
    path: PathBuf,
    reads: Arc<AtomicUsize>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Observer {
    fn start() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "ld-observe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = dir.join("lux.sock");
        let listener = UnixListener::bind(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let reads = Arc::new(AtomicUsize::new(0));
        let count = reads.clone();
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let corpus: Value =
                serde_json::from_str(include_str!("fixtures/lx03/commands.json")).unwrap();
            while let Ok(Some(bytes)) = codec::read_frame(&mut stream) {
                let request: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(
                    request["kind"], "snapshot",
                    "no grant or mutation may reach provider"
                );
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                let mut reply = corpus["cases"][0]["replies"][0].clone();
                reply["sequence"] = json!(n.to_string());
                let bytes = serde_json::to_vec(&reply).unwrap();
                stream
                    .write_all(&(bytes.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&bytes).unwrap();
            }
        });
        Self {
            path,
            reads,
            thread: Some(thread),
        }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.thread.take().unwrap().join().unwrap();
        std::fs::remove_dir_all(self.path.parent().unwrap()).unwrap();
    }
}
fn wait(mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(4);
    while !condition() {
        assert!(Instant::now() < until, "bounded observer deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn unbound_worker_observes_automatically_but_refuses_writes_and_respects_disconnect() {
    let server = Observer::start();
    let worker = Worker::spawn(server.path.clone(), SHOW.into(), 9);
    wait(|| server.reads.load(Ordering::SeqCst) >= 6);
    wait(|| worker.view.lock().unwrap().freshness == "Fresh");
    for action in [Semantic::Grant, Semantic::Action(Action::Blackout(true))] {
        let before = worker.view.lock().unwrap().completed;
        worker.action(action).unwrap();
        wait(|| worker.view.lock().unwrap().completed > before);
        let view = worker.view.lock().unwrap();
        assert!(view.command_error.as_ref().unwrap().contains("GP09"));
        assert!(!view.lease);
    }
    let before = worker.view.lock().unwrap().completed;
    worker.enqueue("disconnect".into()).unwrap();
    wait(|| worker.view.lock().unwrap().completed > before);
    let reads = server.reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(server.reads.load(Ordering::SeqCst), reads);
    assert_ne!(worker.view.lock().unwrap().freshness, "Fresh");
    drop(worker);
}
#[test]
fn role_loss_allows_repeated_reads_without_reauthorizing_writer() {
    let server = Observer::start();
    let mut client = LocalClient::connect(&server.path, SHOW, 9, "reader").unwrap();
    client.set_role_guard(Arc::new(|| false));
    client.authority.disconnect();
    for _ in 0..3 {
        client.refresh().unwrap();
        assert_eq!(client.authority.freshness(), Freshness::Fresh);
        assert!(!client.authority.writable(client.now()));
    }
    assert!(client.grant().is_err());
    drop(client);
}

#[test]
fn semantic_draft_survives_context_and_revision_loss_but_needs_new_review() {
    use shr_lightdesk::{
        lux_control::LuxAuthority, lux_operator::Operator, native_actions::Workflow,
    };
    let server = Observer::start();
    let mut op = Operator::connect(&server.path, SHOW, 9).unwrap();
    let corpus: Value = serde_json::from_str(include_str!("fixtures/lx03/commands.json")).unwrap();
    let mut authority = LuxAuthority::new(SHOW, 9, "writer-a").unwrap();
    authority
        .observe(
            &serde_json::to_vec(&corpus["cases"][0]["replies"][0]).unwrap(),
            0,
        )
        .unwrap();
    authority
        .begin("grant", json!({"scope":"lighting-control"}), 0)
        .unwrap();
    authority
        .acknowledge(
            &serde_json::to_vec(&corpus["cases"][1]["replies"][0]).unwrap(),
            0,
        )
        .unwrap();
    op.client.authority = authority;
    op.selected.insert("fixture-11".into());
    op.input.release();
    let mut flow = Workflow::default();
    flow.dispatch(&mut op, Semantic::Edit).unwrap();
    flow.dispatch(&mut op, Semantic::Action(Action::Text("12.3".into())))
        .unwrap();
    flow.dispatch(&mut op, Semantic::Action(Action::ContextLost))
        .unwrap();
    assert_eq!(flow.editor.as_ref().unwrap().text(), "12.3");
    assert!(op.input.confirmation.is_none());
    assert!(
        flow.dispatch(&mut op, Semantic::Action(Action::Confirm))
            .is_err()
    );
    let mut page = corpus["cases"][0]["replies"][0].clone();
    page["sequence"] = json!("2");
    page["revision"] = json!("1");
    let mut inventory: Value =
        serde_json::from_str(page["body"]["chunk"].as_str().unwrap()).unwrap();
    inventory["snapshot"]["revision"] = json!("1");
    page["body"]["chunk"] = json!(serde_json::to_string(&inventory).unwrap());
    op.client
        .authority
        .observe(&serde_json::to_vec(&page).unwrap(), op.client.now())
        .unwrap();
    flow.dispatch(&mut op, Semantic::EnterUp).unwrap();
    flow.dispatch(&mut op, Semantic::Action(Action::Confirm))
        .unwrap();
    let review = op.input.confirmation.as_ref().unwrap();
    assert_eq!(review.revision, 1);
    assert_eq!(review.command["values"][0]["value"], 123);
    // A second press cannot send without release and the entire new review being seen.
    assert!(
        flow.dispatch(&mut op, Semantic::Action(Action::Confirm))
            .is_err()
    );
    assert_eq!(server.reads.load(Ordering::SeqCst), 1);
    drop(op);
}

#[test]
fn read_only_io_exception_never_authorizes_a_grant_on_a_fresh_connection() {
    let server = Observer::start();
    let mut client = LocalClient::connect(&server.path, SHOW, 9, "reader").unwrap();
    client.set_role_guard(Arc::new(|| false));
    client.refresh().unwrap();
    assert_eq!(client.authority.freshness(), Freshness::Fresh);
    assert!(client.grant().unwrap_err().contains("cancelled"));
    assert_eq!(server.reads.load(Ordering::SeqCst), 2);
    assert!(!client.authority.writable(client.now()));
    drop(client);
}
