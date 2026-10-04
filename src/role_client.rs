//! Consumer of accepted GP09 executable protocol. No descriptor matching/role registry.
use crate::{adapter, codec};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    os::fd::AsRawFd,
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Config {
    pub executable: PathBuf,
    pub directory: PathBuf,
    pub request: Value,
}
impl Config {
    pub fn load(executable: PathBuf, directory: PathBuf, request: PathBuf) -> Result<Self, String> {
        if !executable.is_absolute() || !directory.is_absolute() {
            return Err("role executable/directory must be absolute".into());
        }
        let bytes = std::fs::read(request).map_err(|e| e.to_string())?;
        let request = codec::json(&bytes, 65535)?;
        schema(
            &request,
            &[
                "format",
                "version",
                "expected_generation",
                "binding",
                "inventory",
            ],
        )?;
        if request["format"] != "gigpies-role-acquire" || request["version"] != 1 {
            return Err("role acquire protocol1".into());
        }
        codec::counter(&request["expected_generation"])?;
        validate_binding(&request["binding"])?;
        Ok(Self {
            executable,
            directory,
            request,
        })
    }
}
fn schema(v: &Value, keys: &[&str]) -> Result<(), String> {
    let o = v.as_object().ok_or("role object")?;
    if o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)) {
        return Err("role unknown/missing field".into());
    }
    Ok(())
}
fn validate_binding(v: &Value) -> Result<(), String> {
    schema(
        v,
        &[
            "role",
            "display_connector",
            "display_edid",
            "controller",
            "profile",
            "operator_label",
        ],
    )?;
    if v["role"] != "lighting-desk" {
        return Err("Lightdesk requires lighting-desk role".into());
    }
    for key in ["display_connector", "display_edid", "controller", "profile"] {
        adapter::id(&v[key])?;
    }
    if !v["operator_label"].is_null() {
        let label = v["operator_label"].as_str().ok_or("role label")?;
        if label.is_empty() || label.len() > 128 || label.chars().any(char::is_control) {
            return Err("role label bound".into());
        }
    }
    Ok(())
}
pub fn validate_grant(value: &Value, request: &Value) -> Result<u64, String> {
    schema(value, &["format", "version", "generation", "binding"])?;
    if value["format"] != "gigpies-role-lease"
        || value["version"] != 1
        || value["binding"] != request["binding"]
    {
        return Err("role grant identity".into());
    }
    validate_binding(&value["binding"])?;
    let generation = codec::counter(&value["generation"])?;
    if generation <= codec::counter(&request["expected_generation"])? {
        return Err("role acquisition generation not advanced".into());
    }
    Ok(generation)
}
pub fn validate_verified(value: &Value, grant: &Value) -> Result<(), String> {
    schema(
        value,
        &["format", "version", "lease", "registry_generation"],
    )?;
    if value["format"] != "gigpies-role-verified"
        || value["version"] != 1
        || value["lease"] != *grant
    {
        return Err("role verify exact live lease mismatch".into());
    }
    if codec::counter(&value["registry_generation"])? < codec::counter(&grant["generation"])? {
        return Err("registry generation older than live acquisition".into());
    }
    Ok(())
}
#[derive(Clone, Default)]
pub struct State {
    pub grant: Option<Value>,
    pub notice: String,
    pub verified: Option<Instant>,
    pub child_pid: Option<u32>,
}
impl State {
    pub fn live(&self) -> bool {
        self.grant.is_some()
            && self
                .verified
                .is_some_and(|t| t.elapsed() < Duration::from_millis(1000))
    }
}
struct Pipes {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
}
impl Pipes {
    fn start(config: &Config) -> Result<Self, String> {
        let mut child = Command::new(&config.executable)
            .arg(&config.directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        for fd in [input.as_raw_fd(), output.as_raw_fd()] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err("role pipe nonblocking setup".into());
            }
        }
        Ok(Self {
            child,
            input,
            output,
        })
    }
    fn exchange(&mut self, request: &Value) -> Result<Value, String> {
        let mut bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        if bytes.len() > 65536 {
            return Err("role request64KiB".into());
        }
        let until = Instant::now() + Duration::from_millis(500);
        let mut pos = 0;
        while pos < bytes.len() {
            ready(self.input.as_raw_fd(), libc::POLLOUT, until)?;
            match self.input.write(&bytes[pos..]) {
                Ok(0) => return Err("role pipe closed".into()),
                Ok(n) => pos += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        let mut response = Vec::new();
        loop {
            ready(self.output.as_raw_fd(), libc::POLLIN, until)?;
            let mut buf = [0; 4096];
            match self.output.read(&mut buf) {
                Ok(0) => return Err("role provider EOF".into()),
                Ok(n) => {
                    response.extend_from_slice(&buf[..n]);
                    if response.len() > 4096 {
                        return Err("role reply exceeds PIPE_BUF".into());
                    }
                    if let Some(end) = response.iter().position(|b| *b == b'\n') {
                        if end + 1 != response.len() {
                            return Err("role unsolicited reply bytes".into());
                        }
                        return codec::json(&response[..end], 4096);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}
fn ready(fd: i32, events: i16, until: Instant) -> Result<(), String> {
    let ms = until
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(500) as i32;
    if ms == 0 {
        return Err("role reply500ms deadline".into());
    }
    let mut p = libc::pollfd {
        fd,
        events,
        revents: 0,
    };
    let rc = unsafe { libc::poll(&mut p, 1, ms) };
    if rc <= 0 {
        return Err("role reply500ms deadline/poll".into());
    }
    if p.revents & events == 0 {
        return Err("role pipe lost".into());
    }
    Ok(())
}
impl Drop for Pipes {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
/// Independent role verification and LED mailbox, never on the input/provider hot path.
pub struct Monitor {
    pub state: Arc<Mutex<State>>,
    pub live: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub leds: Arc<Mutex<Option<LedFrame>>>,
}
#[derive(Clone, Debug)]
pub struct LedFrame {
    pub generation: u64,
    pub binding: Value,
    pub states: [&'static str; 8],
}
impl Monitor {
    pub fn spawn(config: Config, fence: Arc<AtomicU64>) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let live = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let leds = Arc::new(Mutex::new(None));
        let (s, l, q, led) = (state.clone(), live.clone(), stop.clone(), leds.clone());
        let thread = thread::spawn(move || {
            let run = || -> Result<(), String> {
                let mut pipes = Pipes::start(&config)?;
                let grant = pipes.exchange(&config.request)?;
                validate_grant(&grant, &config.request)?;
                *s.lock().unwrap()=State{grant:Some(grant.clone()),notice:"GP09 injected descriptors / live process-held lease / physical identity unverified".into(),verified:Some(Instant::now()),child_pid:Some(pipes.child.id())};
                let _ =
                    fence.fetch_update(Ordering::AcqRel, Ordering::Acquire, |g| g.checked_add(1));
                l.store(true, Ordering::Release);
                while !q.load(Ordering::Acquire) {
                    let until = Instant::now() + Duration::from_millis(500);
                    while Instant::now() < until && !q.load(Ordering::Acquire) {
                        thread::sleep(Duration::from_millis(10));
                    }
                    if q.load(Ordering::Acquire) {
                        break;
                    }
                    let reply=pipes.exchange(&json!({"format":"gigpies-role-command","version":1,"operation":"verify","generation":grant["generation"],"binding":grant["binding"]}))?;
                    validate_verified(&reply, &grant)?;
                    s.lock().unwrap().verified = Some(Instant::now());
                }
                Ok(())
            };
            let result = run();
            l.store(false, Ordering::Release);
            let _ = fence.fetch_update(Ordering::AcqRel, Ordering::Acquire, |g| g.checked_add(1));
            *led.lock().unwrap() = None;
            let mut state = s.lock().unwrap();
            state.verified = None;
            state.notice = match result {
                Ok(()) => "Role stopped; saved intent retained".into(),
                Err(e) => format!("ROLE LOST: {e}; explicit fresh request/new process required"),
            };
        });
        Self {
            state,
            live,
            stop,
            thread: Some(thread),
            leds,
        }
    }
    pub fn take_leds(&self) -> Option<LedFrame> {
        let state = self.state.lock().unwrap();
        let mut mailbox = self.leds.lock().unwrap();
        if !self.live.load(Ordering::Acquire) || !state.live() {
            *mailbox = None;
            return None;
        }
        let grant = state.grant.as_ref()?;
        let frame = mailbox.take()?;
        if frame.generation != codec::counter(&grant["generation"]).ok()?
            || frame.binding != grant["binding"]
        {
            return None;
        }
        Some(frame)
    }
    /// Desired injected LED states coalesce independently; no MIDI bytes are sent.
    pub fn publish_leds(&self, states: [&'static str; 8]) {
        let state = self.state.lock().unwrap();
        if !self.live.load(Ordering::Acquire) || !state.live() {
            *self.leds.lock().unwrap() = None;
            return;
        }
        let grant = state.grant.as_ref().unwrap();
        *self.leds.lock().unwrap() = Some(LedFrame {
            generation: codec::counter(&grant["generation"]).unwrap(),
            binding: grant["binding"].clone(),
            states,
        });
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.live.store(false, Ordering::Release);
        self.stop.store(true, Ordering::Release);
        *self.leds.lock().unwrap() = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
