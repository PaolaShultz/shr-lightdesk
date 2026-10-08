//! Real C-LIGHT client lifecycle. Lux supplies every look and release destination.
use crate::{
    adapter::{self, Freshness, LuxClient},
    codec,
};
use adapter::{exact, id, integer, schema, text};
use serde_json::{Value, json};
use std::collections::VecDeque;
type Result<T> = std::result::Result<T, String>;
pub const SESSION_MS: u64 = 25_000;
pub fn session_guard(now_ms: u64) -> Result<()> {
    if now_ms >= SESSION_MS {
        Err("bounded session25s expired; fresh attach required".into())
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Pending {
    pub envelope: Value,
    pub bytes: Vec<u8>,
    pub first_send_ms: u64,
    retry: usize,
}
#[derive(Clone, Debug)]
struct Lease {
    id: String,
    until: u64,
    renew_at: u64,
}
/// Stable-ID authority seam for the real wire provider; the original in-process
/// u16 Simulator Authority remains unchanged.
pub trait LightingAuthority {
    fn snapshot(&self) -> Option<&Value>;
    fn freshness(&self) -> Freshness;
    fn begin(&mut self, kind: &str, body: Value, now: u64) -> Result<Vec<u8>>;
    fn acknowledge(&mut self, bytes: &[u8], now: u64) -> Result<bool>;
    fn disconnect(&mut self);
}
pub struct LuxAuthority {
    pub cache: LuxClient,
    show: String,
    epoch: u64,
    writer: String,
    next_id: u64,
    revision: u64,
    lease: Option<Lease>,
    pub pending: Option<Pending>,
    history: VecDeque<(Value, Value)>,
    pub notice: String,
    pub uncertain: bool,
    pub last_reply: Option<Value>,
    connected: bool,
    pub preview: Option<Value>,
    pub preview_until: Option<u64>,
}
impl LuxAuthority {
    pub fn new(show: &str, epoch: u64, writer: &str) -> Result<Self> {
        id(&json!(writer))?;
        Ok(Self {
            cache: LuxClient::new(show, epoch)?,
            show: show.into(),
            epoch,
            writer: writer.into(),
            next_id: 1,
            revision: 0,
            lease: None,
            pending: None,
            history: VecDeque::new(),
            notice: "fresh snapshot required".into(),
            uncertain: false,
            last_reply: None,
            connected: true,
            preview: None,
            preview_until: None,
        })
    }
    pub fn observe(&mut self, bytes: &[u8], now: u64) -> Result<bool> {
        let header = match codec::json(bytes, codec::MESSAGE_BYTES) {
            Ok(header) => header,
            Err(e) => {
                self.disconnect();
                return Err(e);
            }
        };
        let revision = match codec::counter(&header["revision"]) {
            Ok(revision) => revision,
            Err(e) => {
                self.disconnect();
                return Err(e);
            }
        };
        if revision < self.revision {
            self.disconnect();
            return Err("snapshot predates confirmed authority revision".into());
        }
        let result = self.cache.ingest(bytes, now);
        if let Ok(true) = result {
            self.revision = self.revision.max(codec::counter(
                &self.cache.snapshot().unwrap()["snapshot"]["revision"],
            )?);
            if self.pending.take().is_some() {
                self.connected = false;
                self.lease = None;
                self.notice = "fresh look recovered; prior operation unconfirmed, intent discarded; new writer/input release required".into();
            }
            if self.preview.as_ref().is_some_and(|token| {
                token["preview"]["revision"]
                    != self.cache.snapshot().unwrap()["snapshot"]["revision"]
                    || token["preview"]["patch_revision"]
                        != self.cache.snapshot().unwrap()["patch"]["patch_revision"]
            }) {
                self.preview = None;
                self.preview_until = None;
            }
            self.uncertain = false;
        } else if result.is_err() {
            self.disconnect();
        }
        result
    }
    pub fn read_envelope(&self) -> Value {
        json!({"contract":"C-LIGHT","version":1,"show_id":self.show,"module":"lighting","epoch":self.epoch.to_string(),"writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"snapshot","body":{}})
    }
    pub fn writable(&self, now: u64) -> bool {
        self.connected
            && self.cache.freshness() == Freshness::Fresh
            && !self.uncertain
            && self.lease.as_ref().is_some_and(|l| now < l.until)
    }
    pub fn advance(&mut self, now: u64) -> Result<()> {
        self.cache.advance(now)?;
        if self.preview_until.is_some_and(|until| now >= until) {
            self.preview = None;
            self.preview_until = None;
        }
        if self.lease.as_ref().is_some_and(|l| now >= l.until) {
            self.disconnect();
            self.notice = "lease expired; fresh snapshot and new writer grant required".into();
        }
        Ok(())
    }
    pub fn needs_renew(&self, now: u64) -> bool {
        self.lease
            .as_ref()
            .is_some_and(|l| now >= l.renew_at && now < l.until)
            && self.pending.is_none()
    }
    pub fn retry(&mut self, now: u64) -> Result<Option<Vec<u8>>> {
        self.advance(now)?;
        let p = self.pending.as_mut().ok_or("no pending request")?;
        let age = now.checked_sub(p.first_send_ms).ok_or("clock regression")?;
        let until = self
            .lease
            .as_ref()
            .map_or(p.first_send_ms.saturating_add(2000), |l| l.until);
        if now >= until {
            self.disconnect();
            return Err("uncertain request expired; no replay".into());
        }
        if age >= 1500 {
            self.disconnect();
            return Err("bounded ACK wait exhausted; query fresh state, never replay".into());
        }
        let schedule = [100, 250, 500];
        if p.retry < schedule.len() && age >= schedule[p.retry] {
            p.retry += 1;
            return Ok(Some(p.bytes.clone()));
        }
        Ok(None)
    }
    pub fn check_review(&self, revision: u64, lease: &str, now: u64) -> Result<()> {
        if !self.writable(now) || self.revision != revision || self.lease_id() != Some(lease) {
            return Err(
                "reviewed revision/lease changed during maintenance; confirmation invalidated"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn lease_id(&self) -> Option<&str> {
        self.lease.as_ref().map(|l| l.id.as_str())
    }
    pub fn lease_deadline(&self) -> Option<u64> {
        self.lease.as_ref().map(|l| l.until)
    }
    pub fn status(&self) -> Result<String> {
        Ok(format!(
            "LUX CONTROL / {} / pending {} / uncertain {} / lease {:?} / logical revision {}\n{}",
            self.notice,
            self.pending.is_some(),
            self.uncertain,
            self.lease_deadline(),
            self.revision,
            self.cache.presentation()?
        ))
    }
    fn reply(&mut self, bytes: &[u8], now: u64) -> Result<bool> {
        let v = codec::json(bytes, codec::MESSAGE_BYTES)?;
        validate_reply(&v, &self.show, self.epoch)?;
        // Only byte-equivalent cached old outcomes may be ignored; no revision rollback.
        if self.history.iter().any(|(_, old)| old == &v) {
            return Ok(false);
        }
        let p = self.pending.as_ref().ok_or("unsolicited reply")?;
        let until = self
            .lease
            .as_ref()
            .map_or(p.first_send_ms.saturating_add(2000), |l| l.until);
        if now < p.first_send_ms || now >= until {
            return Err("reply after conservative lease/deadline".into());
        }
        for k in ["writer", "lease", "request_id", "expected_revision"] {
            if v[k] != p.envelope[k] {
                return Err(format!("wrong reply {k}"));
            }
        }
        let revision = codec::counter(&v["revision"])?;
        let kind = text(&v["kind"])?.to_owned();
        if kind == "applied" {
            if !v["reason"].is_null() {
                return Err("applied reason".into());
            }
            match text(&p.envelope["kind"])? {
                "grant" | "renew" => {
                    let lease = if p.envelope["kind"] == "grant" {
                        schema(&v["body"], &["lease", "remaining_ms", "scope"])?;
                        exact(&v["body"]["scope"], "lighting-control")?;
                        let lease = codec::counter(&v["body"]["lease"])?;
                        if lease == 0 {
                            return Err("zero lease".into());
                        }
                        lease.to_string()
                    } else {
                        schema(&v["body"], &["remaining_ms"])?;
                        text(&p.envelope["lease"])?.to_owned()
                    };
                    let remaining = integer(&v["body"]["remaining_ms"])?;
                    if !(1..=2000).contains(&remaining) {
                        return Err("lease duration".into());
                    }
                    let until = p
                        .first_send_ms
                        .checked_add(remaining as u64)
                        .ok_or("lease deadline exhausted")?;
                    if now >= until {
                        return Err("delayed lease expired".into());
                    }
                    self.lease = Some(Lease {
                        id: lease,
                        until,
                        renew_at: p
                            .first_send_ms
                            .checked_add(500)
                            .ok_or("renew deadline exhausted")?,
                    });
                }
                "retire" => {
                    schema(&v["body"], &["retired"])?;
                    if v["body"]["retired"] != true {
                        return Err("retire body".into());
                    }
                    self.lease = None;
                }
                "command" => {
                    let command = &p.envelope["body"]["command"];
                    let snapshot = self.cache.snapshot().ok_or("no trusted capabilities")?;
                    match text(&command["action"])? {
                        "release_preview" => {
                            schema(&v["body"], &["application", "physical", "token"])?;
                            exact(&v["body"]["application"], "preview_only")?;
                            exact(&v["body"]["physical"], "unknown")?;
                            validate_preview(&v["body"]["token"], &p.envelope, &v, snapshot)?;
                            self.preview = Some(v["body"]["token"].clone());
                            self.preview_until = Some(
                                p.first_send_ms
                                    .checked_add(2000)
                                    .ok_or("preview deadline exhausted")?,
                            );
                        }
                        "release_commit" => {
                            schema(&v["body"], &["application", "physical", "transition"])?;
                            exact(&v["body"]["application"], "logical_timed")?;
                            exact(&v["body"]["physical"], "unknown")?;
                            adapter::validate_transition(&v["body"]["transition"], snapshot)?;
                            let ts = v["body"]["transition"]["targets"].as_array().unwrap();
                            let expected = command["token"]["preview"]["values"]
                                .as_array()
                                .ok_or("commit token values")?;
                            if ts.len() != expected.len()
                                || v["body"]["transition"]["start_tick"] != v["effective_tick"]
                            {
                                return Err("commit transition identity".into());
                            }
                            for t in ts {
                                let e = expected
                                    .iter()
                                    .find(|e| {
                                        e["fixture"] == t["fixture"]
                                            && e["attribute"] == t["attribute"]
                                    })
                                    .ok_or("commit selection identity")?;
                                if t["start"] != e["current"]
                                    || t["target"] != e["destination"]
                                    || t["current"] != t["start"]
                                {
                                    return Err("commit reviewed values identity".into());
                                }
                            }
                            self.preview = None;
                            self.preview_until = None;
                        }
                        "release_cancel" => {
                            schema(&v["body"], &["application", "physical"])?;
                            exact(&v["body"]["application"], "preview_cancelled")?;
                            exact(&v["body"]["physical"], "unknown")?;
                            self.preview = None;
                            self.preview_until = None;
                        }
                        "analysis_calibrate" | "analysis_grant" => {
                            schema(&v["body"], &["analysis"])?;
                            adapter::validate_analysis_result(&v["body"]["analysis"], snapshot)?;
                            let a = &v["body"]["analysis"];
                            if command["action"] == "analysis_grant" {
                                let expected_expiry = codec::counter(&v["effective_tick"])?
                                    .checked_add(
                                        command["ttl_ms"].as_u64().ok_or("grant TTL")? / 10,
                                    )
                                    .ok_or("grant expiry overflow")?;
                                if a["grant"]["fixtures"] != command["fixtures"]
                                    || a["grant"]["cap"] != command["cap"]
                                    || a["grant"]["writer"] != p.envelope["writer"]
                                    || a["grant"]["lease"] != p.envelope["lease"]
                                    || codec::counter(&a["grant"]["expiry_tick"])?
                                        != expected_expiry
                                    || a["grant"]["issue_revision"] != v["revision"]
                                    || crate::lux_operator::analysis_review_basis(
                                        &json!({"analysis":a}),
                                        command,
                                    ) != crate::lux_operator::analysis_review_basis(
                                        snapshot, command,
                                    )
                                {
                                    return Err("analysis grant acknowledgment identity".into());
                                }
                            } else {
                                let expected_state = if command["phase"] == "start" {
                                    "calibrating"
                                } else {
                                    "settling"
                                };
                                let old_generation =
                                    codec::counter(&snapshot["analysis"]["generation"])?;
                                let expected_generation = if command["phase"] == "start" {
                                    old_generation.saturating_add(1)
                                } else {
                                    old_generation
                                };
                                if codec::counter(&a["generation"])? != expected_generation
                                    || (command["phase"] == "start"
                                        && (a["calibration_windows"] != 0
                                            || !a["calibration_generation"].is_null()))
                                    || (command["phase"] == "finish"
                                        && a["calibration_generation"] != a["generation"])
                                {
                                    return Err("calibration acknowledgment generation".into());
                                }
                                if a["state"] != expected_state
                                    || !a["grant"].is_null()
                                    || a["source_identity"]
                                        != snapshot["analysis"]["source_identity"]
                                {
                                    return Err("calibration acknowledgment identity".into());
                                }
                            }
                        }
                        "checkpoint" => {
                            schema(&v["body"], &["application", "durability", "physical"])?;
                            exact(&v["body"]["application"], "checkpointed")?;
                            exact(&v["body"]["durability"], "checkpointed")?;
                            exact(&v["body"]["physical"], "unknown")?;
                        }
                        _ => {
                            schema(&v["body"], &["application", "output", "physical"])?;
                            exact(&v["body"]["application"], "logical_static")?;
                            exact(&v["body"]["output"], "null_disarmed")?;
                            exact(&v["body"]["physical"], "unknown")?;
                            self.preview = None;
                            self.preview_until = None;
                        }
                    }
                }
                _ => return Err("unavailable reply operation".into()),
            }
            if revision < self.revision {
                return Err("uncached reply revision regression".into());
            }
        } else {
            if !matches!(kind.as_str(), "conflict" | "rejected" | "busy") || !v["body"].is_null() {
                return Err("unknown/refusal reply".into());
            }
            let reason = text(&v["reason"])?;
            if reason.is_empty() || reason.len() > 128 {
                return Err("refusal reason".into());
            }
        }
        self.revision = self.revision.max(revision);
        self.notice = format!("{} {} / logical only; physical UNKNOWN", kind, v["reason"]);
        let p = self.pending.take().unwrap();
        self.history.push_back((p.envelope, v.clone()));
        if self.history.len() > 64 {
            self.history.pop_front();
        }
        self.last_reply = Some(v);
        self.uncertain = false;
        // An ACK is not a complete current look. Refresh is required after writes.
        if kind != "applied" || self.last_reply.as_ref().unwrap()["request_id"] != "1" {
            self.cache.disconnect();
        }
        Ok(true)
    }
}
impl LightingAuthority for LuxAuthority {
    fn snapshot(&self) -> Option<&Value> {
        self.cache.snapshot()
    }
    fn freshness(&self) -> Freshness {
        self.cache.freshness()
    }
    fn begin(&mut self, kind: &str, body: Value, now: u64) -> Result<Vec<u8>> {
        self.advance(now)?;
        if !self.connected || self.pending.is_some() || self.uncertain {
            return Err("disconnected/busy/uncertain".into());
        }
        if self.cache.freshness() != Freshness::Fresh {
            return Err("fresh complete snapshot required".into());
        }
        if kind == "grant" {
            if self.next_id != 1 || self.lease.is_some() {
                return Err("grant requires new writer".into());
            }
        } else if !self.writable(now) {
            return Err("valid writer lease required".into());
        }
        if !matches!(kind, "grant" | "renew" | "retire" | "command") {
            return Err("unavailable operation".into());
        }
        if kind == "command" {
            schema(&body, &["scope", "command"])?;
            exact(&body["scope"], "lighting-control")?;
            crate::lux_operator::validate_analysis_command(
                self.cache.snapshot().unwrap(),
                &body["command"],
            )?;
            if body["command"]["action"] == "release_commit"
                && (self.preview.as_ref() != Some(&body["command"]["token"])
                    || self.preview_until.is_none_or(|until| now >= until))
            {
                return Err("exact unexpired engine preview required".into());
            }
            if body["command"]["action"] == "checkpoint"
                && self.cache.snapshot().unwrap()["checkpoint"]["available"] != true
            {
                return Err("checkpoint capability unavailable".into());
            }
        } else {
            schema(&body, &["scope"])?;
            exact(&body["scope"], "lighting-control")?;
        }
        let next = self
            .next_id
            .checked_add(1)
            .filter(|n| *n < u64::MAX)
            .ok_or("request counter exhausted")?;
        let envelope = json!({"contract":"C-LIGHT","version":1,"show_id":self.show,"module":"lighting","epoch":self.epoch.to_string(),"writer":self.writer,"lease":self.lease_id(),"request_id":self.next_id.to_string(),"expected_revision":self.revision.to_string(),"kind":kind,"body":body});
        let bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
        codec::json(&bytes, codec::MESSAGE_BYTES)?;
        self.next_id = next;
        self.pending = Some(Pending {
            envelope,
            bytes: bytes.clone(),
            first_send_ms: now,
            retry: 0,
        });
        self.uncertain = true;
        self.notice = "pending; submitted packet is not confirmation".into();
        Ok(bytes)
    }
    fn acknowledge(&mut self, bytes: &[u8], now: u64) -> Result<bool> {
        let result = self.reply(bytes, now);
        if result.is_err() {
            self.disconnect();
        }
        result
    }
    fn disconnect(&mut self) {
        self.connected = false;
        self.cache.disconnect();
        self.lease = None;
        self.pending = None;
        self.preview = None;
        self.preview_until = None;
        // Preserve uncertainty until a real fresh snapshot, never replay intents.
        self.notice =
            "connection lost; cached look retained, fresh attach/new writer required".into();
    }
}
fn validate_reply(v: &Value, show: &str, epoch: u64) -> Result<()> {
    schema(
        v,
        &[
            "body",
            "contract",
            "effective_tick",
            "epoch",
            "expected_revision",
            "kind",
            "lease",
            "module",
            "reason",
            "request_id",
            "revision",
            "sequence",
            "show_id",
            "version",
            "writer",
        ],
    )?;
    exact(&v["contract"], "C-LIGHT")?;
    exact(&v["module"], "lighting")?;
    if integer(&v["version"])? != 1
        || text(&v["show_id"])? != show
        || codec::counter(&v["epoch"])? != epoch
    {
        return Err("wrong version/show/epoch".into());
    }
    for key in ["effective_tick", "revision", "sequence"] {
        codec::counter(&v[key])?;
    }
    id(&v["writer"])?;
    codec::counter(&v["request_id"])?;
    codec::counter(&v["expected_revision"])?;
    if !v["lease"].is_null() {
        codec::counter(&v["lease"])?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub mod local {
    use super::*;
    use std::{
        io::{Read, Write},
        os::{fd::AsRawFd, unix::net::UnixStream},
        path::Path,
        time::{Duration, Instant},
    };
    pub struct LocalClient {
        pub authority: LuxAuthority,
        stream: UnixStream,
        start: Instant,
        incoming: Vec<u8>,
        incoming_deadline: Option<Instant>,
        cancellation: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>,
        transport_cancelled: bool,
        role_guard: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
        read_only_io: bool,
    }
    impl LocalClient {
        pub fn connect(path: &Path, show: &str, epoch: u64, writer: &str) -> Result<Self> {
            let stream = adapter::private_connect(path)?;
            stream.set_nonblocking(true).map_err(|e| e.to_string())?;
            let mut c = Self {
                authority: LuxAuthority::new(show, epoch, writer)?,
                stream,
                start: Instant::now(),
                incoming: Vec::new(),
                incoming_deadline: None,
                cancellation: None,
                transport_cancelled: false,
                role_guard: None,
                read_only_io: false,
            };
            c.refresh()?;
            Ok(c)
        }
        pub fn set_role_guard(&mut self, guard: std::sync::Arc<dyn Fn() -> bool + Send + Sync>) {
            self.role_guard = Some(guard);
        }
        pub fn set_input_context(
            &mut self,
            generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
            expected: u64,
        ) {
            self.cancellation = Some((generation, expected));
        }
        fn check_cancellation(&mut self) -> Result<()> {
            if self.transport_cancelled
                || (!self.read_only_io && self.role_guard.as_ref().is_some_and(|guard| !guard()))
                || self.cancellation.as_ref().is_some_and(|(g, expected)| {
                    g.load(std::sync::atomic::Ordering::Acquire) != *expected
                })
            {
                self.transport_cancelled = true;
                self.authority.disconnect();
                let _ = self.stream.shutdown(std::net::Shutdown::Both);
                return Err("input context cancelled; submitted outcome may be uncertain; no further sends or retries; reconnect required".into());
            }
            Ok(())
        }
        pub fn now(&self) -> u64 {
            self.start.elapsed().as_millis() as u64
        }
        fn ready(&self, events: i16, until: Instant) -> Result<bool> {
            let ms = until
                .saturating_duration_since(Instant::now())
                .as_millis()
                .min(500) as i32;
            if ms == 0 {
                return Ok(false);
            }
            let mut fd = libc::pollfd {
                fd: self.stream.as_raw_fd(),
                events,
                revents: 0,
            };
            let rc = unsafe { libc::poll(&mut fd, 1, ms) };
            if rc < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0
                && fd.revents & events == 0
            {
                return Err("provider link lost".into());
            }
            Ok(rc > 0 && fd.revents & events != 0)
        }
        fn send(&mut self, bytes: &[u8]) -> Result<()> {
            self.check_cancellation()?;
            if bytes.is_empty() || bytes.len() > codec::MESSAGE_BYTES {
                return Err("request capacity".into());
            }
            let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
            framed.extend(bytes);
            let until = Instant::now() + Duration::from_millis(500);
            let mut n = 0;
            while n < framed.len() {
                self.check_cancellation()?;
                if !self.ready(libc::POLLOUT, until)? {
                    return Err("write deadline".into());
                }
                self.check_cancellation()?;
                match self.stream.write(&framed[n..]) {
                    Ok(0) => return Err("provider write closed".into()),
                    Ok(m) => n += m,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        }
        fn receive(&mut self, until: Instant) -> Result<Option<Vec<u8>>> {
            loop {
                // Expiry precedes completion: scheduling after the last read must
                // never turn an expired buffered frame into a valid response.
                if self
                    .incoming_deadline
                    .is_some_and(|deadline| Instant::now() >= deadline)
                {
                    return Err("partial response frame deadline500ms".into());
                }
                if self.incoming.len() >= 4 {
                    let n = u32::from_be_bytes(self.incoming[..4].try_into().unwrap()) as usize;
                    if n == 0 || n > codec::MESSAGE_BYTES {
                        return Err("response capacity".into());
                    }
                    if self.incoming.len() == n + 4 {
                        let bytes = self.incoming.split_off(4);
                        self.incoming.clear();
                        self.incoming_deadline = None;
                        return Ok(Some(bytes));
                    }
                }
                if self
                    .incoming_deadline
                    .is_some_and(|deadline| Instant::now() >= deadline)
                {
                    return Err("partial response frame deadline500ms".into());
                }
                let frame_until = self
                    .incoming_deadline
                    .map_or(until, |deadline| deadline.min(until));
                if !self.ready(libc::POLLIN, frame_until)? {
                    if self
                        .incoming_deadline
                        .is_some_and(|deadline| Instant::now() >= deadline)
                    {
                        return Err("partial response frame deadline500ms".into());
                    }
                    return Ok(None);
                }
                let needed = if self.incoming.len() < 4 {
                    4 - self.incoming.len()
                } else {
                    u32::from_be_bytes(self.incoming[..4].try_into().unwrap()) as usize + 4
                        - self.incoming.len()
                };
                let mut b = [0; 4096];
                let bound = needed.min(b.len());
                match self.stream.read(&mut b[..bound]) {
                    Ok(0) => return Err("provider disconnected".into()),
                    Ok(n) => {
                        if self.incoming.is_empty() {
                            self.incoming_deadline =
                                Some(Instant::now() + Duration::from_millis(500));
                        }
                        self.incoming.extend_from_slice(&b[..n]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
        }
        pub fn refresh(&mut self) -> Result<()> {
            let result = self.refresh_inner();
            if result.is_err() {
                self.authority.disconnect();
            }
            result
        }
        fn refresh_inner(&mut self) -> Result<()> {
            self.read_only_io = true;
            let result = self.read_snapshot();
            self.read_only_io = false;
            result
        }
        fn read_snapshot(&mut self) -> Result<()> {
            session_guard(self.now())?;
            let bytes =
                serde_json::to_vec(&self.authority.read_envelope()).map_err(|e| e.to_string())?;
            self.send(&bytes)?;
            let until = Instant::now() + Duration::from_millis(2000);
            for _ in 0..codec::MAX_PAGES + 64 {
                let bytes = self.receive(until)?.ok_or("snapshot timeout")?;
                let v = codec::json(&bytes, codec::MESSAGE_BYTES)?;
                if v["kind"] != "snapshot" {
                    // Exact delayed cached old ACK is harmless; any other response fails closed.
                    if self.authority.history.iter().any(|(_, old)| old == &v) {
                        continue;
                    }
                    return Err("snapshot refused/uncorrelated response".into());
                }
                if self.authority.observe(&bytes, self.now())? {
                    return Ok(());
                }
            }
            Err("snapshot page capacity".into())
        }
        pub fn request(&mut self, kind: &str, body: Value) -> Result<Value> {
            let result = self.request_inner(kind, body);
            if self.now() >= SESSION_MS {
                self.authority.disconnect();
            }
            if result.is_err() && (self.authority.pending.is_some() || self.authority.uncertain) {
                self.authority.disconnect();
                // A bounded read recovers authoritative current state, not an ACK.
                // The prior writer remains unusable and no operation is replayed.
                let _ = self.refresh_inner();
                self.authority.notice =
                    "operation unconfirmed; recovered look if available; new attach/grant required"
                        .into();
            }
            result
        }
        fn request_inner(&mut self, kind: &str, body: Value) -> Result<Value> {
            session_guard(self.now())?;
            let first = self.now();
            let bytes = self.authority.begin(kind, body, first)?;
            self.send(&bytes)?;
            let mut received = 0;
            for age in [100, 250, 500, 1500] {
                let deadline = first.checked_add(age).ok_or("request deadline exhausted")?;
                let until_ms = self
                    .authority
                    .lease_deadline()
                    .map_or(deadline, |l| l.min(deadline));
                let until = self.start + Duration::from_millis(until_ms);
                while let Some(bytes) = self.receive(until)? {
                    received += 1;
                    if received > 64 {
                        return Err("reply queue capacity".into());
                    }
                    if self.authority.acknowledge(&bytes, self.now())? {
                        let reply = self.authority.last_reply.clone().unwrap();
                        if let Err(e) = self.refresh() {
                            self.authority.notice = format!(
                                "correlated ACK {} {} retained; current look stale, refresh failed: {}",
                                reply["request_id"], reply["kind"], e
                            );
                            self.authority.uncertain = false;
                        }
                        return Ok(reply);
                    }
                }
                if let Some(retry) = self.authority.retry(self.now())? {
                    self.send(&retry)?;
                }
            }
            Err("ACK uncertain; reconnect without replay".into())
        }
        pub fn grant(&mut self) -> Result<Value> {
            self.request("grant", json!({"scope":"lighting-control"}))
        }
        pub fn command(&mut self, command: Value) -> Result<Value> {
            self.maintain()?;
            self.request(
                "command",
                json!({"scope":"lighting-control","command":command}),
            )
        }
        /// Maintenance may discover a later revision. Confirmed operations retain
        /// the revision/lease reviewed by the human and refuse after any change.
        pub fn confirmed_command(
            &mut self,
            command: Value,
            revision: u64,
            lease: &str,
            analysis_basis: Option<&Value>,
        ) -> Result<Value> {
            self.maintain()?;
            if analysis_basis.is_some() {
                self.refresh()?;
            }
            self.authority.check_review(revision, lease, self.now())?;
            if crate::lux_operator::analysis_review_basis(
                self.authority
                    .snapshot()
                    .ok_or("no fresh analysis snapshot")?,
                &command,
            )
            .as_ref()
                != analysis_basis
            {
                return Err("analysis source/calibration changed since review".into());
            }
            self.request(
                "command",
                json!({"scope":"lighting-control","command":command}),
            )
        }
        pub fn maintain(&mut self) -> Result<()> {
            let result = self.maintain_inner();
            if result.is_err() {
                self.authority.disconnect();
            }
            result
        }
        fn maintain_inner(&mut self) -> Result<()> {
            session_guard(self.now())?;
            let now = self.now();
            self.authority.advance(now)?;
            if self.authority.needs_renew(now) {
                let mut renewed = false;
                for _ in 0..2 {
                    // A fade can change the engine revision within a Fresh TTL.
                    self.refresh()?;
                    let reply = self.request("renew", json!({"scope":"lighting-control"}))?;
                    if reply["kind"] == "applied" {
                        renewed = true;
                        break;
                    }
                    if reply["kind"] != "conflict" || reply["reason"] != "stale_revision" {
                        break;
                    }
                    // This was a correlated refusal, not an uncertain delivery;
                    // a fresh observation permits a new request ID, bounded once.
                }
                if !renewed {
                    return Err("lease renewal refused after bounded revision refresh".into());
                }
            }
            if self.authority.freshness() != Freshness::Fresh && self.authority.lease.is_some() {
                self.refresh()?;
            }
            Ok(())
        }
        pub fn wait(&mut self, ms: u64) -> Result<()> {
            if ms > 2000 {
                return Err("wait bound 2000ms".into());
            }
            let until = Instant::now() + Duration::from_millis(ms);
            while Instant::now() < until {
                self.maintain()?;
                std::thread::sleep(
                    until
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(50)),
                );
            }
            self.maintain()
        }
    }
    #[cfg(test)]
    mod frame_tests {
        use super::*;
        use std::{
            os::unix::{fs::PermissionsExt, net::UnixListener},
            sync::mpsc,
        };
        #[test]
        fn input_loss_during_lost_ack_fences_every_retry_and_recovery_send() {
            use std::sync::{
                Arc,
                atomic::{AtomicU64, Ordering},
            };
            let (stream, mut peer) = UnixStream::pair().unwrap();
            stream.set_nonblocking(true).unwrap();
            peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut authority =
                LuxAuthority::new("11111111-1111-4111-8111-111111111111", 9, "writer-a").unwrap();
            let corpus: Value =
                serde_json::from_str(include_str!("../tests/fixtures/lx03/commands.json")).unwrap();
            authority
                .observe(
                    &serde_json::to_vec(&corpus["cases"][0]["replies"][0]).unwrap(),
                    0,
                )
                .unwrap();
            authority.lease = Some(Lease {
                id: "1".into(),
                until: 2000,
                renew_at: 500,
            });
            let generation = Arc::new(AtomicU64::new(1));
            let mut client = LocalClient {
                authority,
                stream,
                start: Instant::now(),
                incoming: Vec::new(),
                incoming_deadline: None,
                cancellation: None,
                transport_cancelled: false,
                role_guard: None,
                read_only_io: false,
            };
            client.set_input_context(generation.clone(), 1);
            let thread = std::thread::spawn(move || {
                let error=client.request("command",json!({"scope":"lighting-control","command":{"action":"touch","values":[{"fixture":"fixture-11","attribute":"intensity","value":0}]}})).unwrap_err();
                assert!(error.contains("cancelled"));
                assert!(client.authority.uncertain);
                client
            });
            let first = codec::read_frame(&mut peer).unwrap().unwrap();
            assert_eq!(codec::json(&first, 65536).unwrap()["kind"], "command");
            generation.store(2, Ordering::Release);
            let client = thread.join().unwrap();
            let mut tail = Vec::new();
            peer.read_to_end(&mut tail).unwrap();
            assert!(tail.is_empty(), "no bytes after initial submitted request");
            assert!(client.transport_cancelled);
        }
        #[test]
        fn complete_buffer_after_frame_deadline_is_rejected_before_delivery() {
            let (stream, _peer) = UnixStream::pair().unwrap();
            let mut c = LocalClient {
                authority: LuxAuthority::new("11111111-1111-4111-8111-111111111111", 9, "writer-a")
                    .unwrap(),
                stream,
                start: Instant::now(),
                incoming: vec![0, 0, 0, 2, b'{', b'}'],
                incoming_deadline: Some(Instant::now() - Duration::from_millis(1)),
                cancellation: None,
                transport_cancelled: false,
                role_guard: None,
                read_only_io: false,
            };
            let error = c
                .receive(Instant::now() + Duration::from_millis(100))
                .unwrap_err();
            assert!(error.contains("500ms"));
            assert_eq!(c.incoming, vec![0, 0, 0, 2, b'{', b'}']);
        }
        #[test]
        fn partial_response_deadline_is_not_reset_by_ack_retry_windows() {
            let dir = std::env::temp_dir().join(format!(
                "ld-frame-{}-{}",
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
            let (tx, rx) = mpsc::channel();
            let worker = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                codec::read_frame(&mut stream).unwrap();
                let corpus: Value =
                    serde_json::from_str(include_str!("../tests/fixtures/lx03/commands.json"))
                        .unwrap();
                let bytes = serde_json::to_vec(&corpus["cases"][0]["replies"][0]).unwrap();
                stream
                    .write_all(&(bytes.len() as u32).to_be_bytes())
                    .unwrap();
                stream.write_all(&bytes).unwrap();
                rx.recv_timeout(Duration::from_secs(2)).unwrap();
                stream.write_all(&[0]).unwrap();
                std::thread::sleep(Duration::from_millis(600));
            });
            let mut c =
                LocalClient::connect(&path, "11111111-1111-4111-8111-111111111111", 9, "writer-a")
                    .unwrap();
            tx.send(()).unwrap();
            assert!(
                c.receive(Instant::now() + Duration::from_millis(30))
                    .unwrap()
                    .is_none()
            );
            let first = c.incoming_deadline;
            assert!(first.is_some());
            std::thread::sleep(Duration::from_millis(510));
            assert!(
                c.receive(Instant::now() + Duration::from_millis(100))
                    .unwrap_err()
                    .contains("500ms")
            );
            assert_eq!(c.incoming_deadline, first);
            drop(c);
            worker.join().unwrap();
            std::fs::remove_file(path).unwrap();
            std::fs::remove_dir(dir).unwrap();
        }
    }
}

fn validate_preview(
    token: &Value,
    request: &Value,
    reply: &Value,
    inventory: &Value,
) -> Result<()> {
    schema(
        token,
        &["schema", "nonce", "writer", "lease", "scope", "preview"],
    )?;
    exact(&token["schema"], "lx04-release-v1")?;
    exact(&token["scope"], "lighting-control")?;
    if token["writer"] != request["writer"] || token["lease"] != request["lease"] {
        return Err("preview writer/lease identity".into());
    }
    let nonce = text(&token["nonce"])?;
    if nonce.len() != 32
        || !nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("preview nonce".into());
    }
    let p = &token["preview"];
    schema(
        p,
        &[
            "show_id",
            "epoch",
            "patch_revision",
            "revision",
            "issued_tick",
            "expiry_tick",
            "transition_ms",
            "validity_ms",
            "values",
        ],
    )?;
    for k in [
        "epoch",
        "patch_revision",
        "revision",
        "issued_tick",
        "expiry_tick",
    ] {
        codec::counter(&p[k])?;
    }
    if p["show_id"] != request["show_id"]
        || p["epoch"] != request["epoch"]
        || p["revision"] != request["expected_revision"]
        || p["revision"] != reply["revision"]
        || p["patch_revision"] != inventory["patch"]["patch_revision"]
        || p["issued_tick"] != reply["effective_tick"]
    {
        return Err("preview show/epoch/revision/patch/tick identity".into());
    }
    if integer(&p["transition_ms"])? != 500
        || integer(&p["validity_ms"])? != 2000
        || codec::counter(&p["expiry_tick"])?.checked_sub(codec::counter(&p["issued_tick"])?)
            != Some(200)
    {
        return Err("preview timing".into());
    }
    let expected = adapter::array(&request["body"]["command"]["values"], 64)?;
    let values = adapter::array(&p["values"], 64)?;
    if values.len() != expected.len() || values.is_empty() {
        return Err("preview target mask".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for t in values {
        schema(t, &["fixture", "attribute", "current", "destination"])?;
        let fid = id(&t["fixture"])?;
        let attr = text(&t["attribute"])?;
        if !seen.insert((fid, attr))
            || !expected
                .iter()
                .any(|e| e["fixture"] == fid && e["attribute"] == attr)
        {
            return Err("preview selection identity".into());
        }
        adapter::advertised_value(inventory, fid, attr, &t["current"])?;
        adapter::advertised_value(inventory, fid, attr, &t["destination"])?;
        let fs = inventory["snapshot"]["fixtures"]
            .as_array()
            .ok_or("fixture inventory")?;
        let f = fs
            .iter()
            .find(|f| f["fixture"] == fid)
            .ok_or("preview fixture")?;
        let attrs = f["attributes"].as_array().ok_or("attribute inventory")?;
        let a = attrs
            .iter()
            .find(|a| a["attribute"] == attr)
            .ok_or("preview attribute")?;
        if a["resolved"] != t["current"] {
            return Err("preview reviewed current value".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counter_exhaustion_does_not_create_pending_or_reuse_id() {
        let mut c =
            LuxAuthority::new("11111111-1111-4111-8111-111111111111", 9, "writer-a").unwrap();
        let corpus: Value =
            serde_json::from_str(include_str!("../tests/fixtures/lx03/commands.json")).unwrap();
        c.observe(
            &serde_json::to_vec(&corpus["cases"][0]["replies"][0]).unwrap(),
            0,
        )
        .unwrap();
        c.next_id = u64::MAX - 1;
        c.lease = Some(Lease {
            id: "1".into(),
            until: 2000,
            renew_at: 500,
        });
        assert!(c.begin("command", json!({}), 0).is_err());
        assert!(c.pending.is_none());
        assert_eq!(c.next_id, u64::MAX - 1);
    }
}
