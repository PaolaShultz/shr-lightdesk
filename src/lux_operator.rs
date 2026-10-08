//! Headless real-provider operator context; no lighting algorithms.
use crate::{
    adapter::{self, Freshness},
    lux_control::{LightingAuthority, local::LocalClient},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug)]
pub struct Confirmation {
    pub command: Value,
    pub analysis_basis: Option<Value>,
    /// Frozen operator material; the exact command/token remains untouched.
    pub material_lines: Vec<String>,
    pub context: u64,
    pub revision: u64,
    pub lease: String,
    pub epoch: u64,
    pub expires: u64,
}
#[derive(Default)]
pub struct InputContext {
    pub generation: u64,
    pub held_enter: bool,
    pub release_required: bool,
    pub confirmation: Option<Confirmation>,
}
impl InputContext {
    pub fn invalidate(&mut self) -> Result<()> {
        self.confirmation = None;
        self.generation = self
            .generation
            .checked_add(1)
            .filter(|n| *n < u64::MAX)
            .ok_or("context exhausted")?;
        Ok(())
    }
    pub fn lost(&mut self) -> Result<()> {
        self.release_required = true;
        self.invalidate()
    }
    pub fn release(&mut self) {
        self.held_enter = false;
        self.release_required = false;
    }
    pub fn press(
        &mut self,
        revision: u64,
        lease: Option<&str>,
        epoch: u64,
        now: u64,
    ) -> Result<Value> {
        if self.held_enter || self.release_required {
            return Err("held input: release/pickup required".into());
        }
        self.held_enter = true;
        let c = self.confirmation.take().ok_or("no reviewed confirmation")?;
        if c.context != self.generation
            || c.revision != revision
            || Some(c.lease.as_str()) != lease
            || c.epoch != epoch
            || now >= c.expires
        {
            return Err("confirmation invalidated/expired".into());
        }
        Ok(c.command)
    }
}
pub struct Operator {
    pub client: LocalClient,
    pub input: InputContext,
    pub selected: BTreeSet<String>,
    path: PathBuf,
    show: String,
    epoch: u64,
    pub page: String,
    review_commands: bool,
}
fn writer() -> String {
    format!(
        "lightdesk-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}
impl Operator {
    pub fn connect(path: &Path, show: &str, epoch: u64) -> Result<Self> {
        Ok(Self {
            client: LocalClient::connect(path, show, epoch, &writer())?,
            input: InputContext::default(),
            selected: BTreeSet::new(),
            path: path.into(),
            show: show.into(),
            epoch,
            page: "stage".into(),
            review_commands: false,
        })
    }
    fn check_reply(reply: Value) -> Result<String> {
        if reply["kind"] != "applied" {
            return Err(format!(
                "Lux refused: {} {}",
                reply["kind"], reply["reason"]
            ));
        }
        Ok(format!(
            "ACK {} revision {} / logical only; physical UNKNOWN\n",
            reply["request_id"], reply["revision"]
        ))
    }
    fn command(&mut self, command: Value) -> Result<String> {
        if self.review_commands && command["action"] != "release_cancel" {
            return self.review(command);
        }
        if self.input.release_required {
            return Err("release/pickup required after input loss/reconnect".into());
        }
        self.input.invalidate()?;
        let result = self.client.command(command);
        if result.is_err() {
            self.input.lost()?;
        }
        Self::check_reply(result?)
    }
    pub(crate) fn review(&mut self, command: Value) -> Result<String> {
        self.client.maintain()?;
        if !self.client.authority.writable(self.client.now()) {
            return Err("fresh snapshot/grant required".into());
        }
        self.input.invalidate()?;
        self.input.confirmation = Some(Confirmation {
            command: command.clone(),
            analysis_basis: analysis_review_basis(
                self.client.authority.snapshot().unwrap(),
                &command,
            ),
            material_lines: Vec::new(),
            context: self.input.generation,
            revision: self.client.authority.revision(),
            lease: self.client.authority.lease_id().unwrap().into(),
            epoch: self.epoch,
            expires: self
                .client
                .now()
                .checked_add(2000)
                .ok_or("confirmation deadline exhausted")?,
        });
        self.freeze_material_review();
        Ok(format!(
            "REVIEW {} / current revision {} / physical UNKNOWN / Enter edge then release-input\n",
            command,
            self.client.authority.revision()
        ))
    }
    fn freeze_material_review(&mut self) {
        if let (Some(c), Some(inventory)) = (
            self.input.confirmation.as_mut(),
            self.client.authority.snapshot(),
        ) {
            c.material_lines = crate::frontend::material_review_lines(
                &c.command, inventory, c.epoch, c.revision, c.expires,
            );
        }
    }
    pub(crate) fn targets(&self, attributes: &[(&str, i32)]) -> Result<Value> {
        if self.selected.is_empty() {
            return Err("select fixtures first".into());
        }
        let snapshot = self.client.authority.snapshot().ok_or("no snapshot")?;
        let mut out = Vec::new();
        for fid in &self.selected {
            let m = snapshot["capability_metadata"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["fixture"] == *fid)
                .ok_or("selection unavailable")?;
            for &(attribute, value) in attributes {
                let c = m["attributes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["attribute"] == attribute)
                    .ok_or("unsupported attribute")?;
                if value < adapter::integer(&c["min"])? || value > adapter::integer(&c["max"])? {
                    return Err("advertised range".into());
                }
                out.push(json!({"fixture":fid,"attribute":attribute,"value":value}));
            }
        }
        if out.len() > 64 {
            return Err("target capacity".into());
        }
        Ok(out.into())
    }
    pub(crate) fn analysis_command(&self, phase: &str, cap: i32, ttl_ms: u64) -> Result<Value> {
        let snapshot = self.client.authority.snapshot().ok_or("no snapshot")?;
        let command = match phase {
            "start" | "finish" => json!({"action":"analysis_calibrate","phase":phase}),
            "grant" => {
                json!({"action":"analysis_grant","fixtures":self.selected,"cap":cap,"ttl_ms":ttl_ms})
            }
            "auto" => json!({"action":"mode","mode":"auto"}),
            "revoke" => json!({"action":"mode","mode":"assist"}),
            _ => return Err("analysis operation".into()),
        };
        validate_analysis_command(snapshot, &command)?;
        if snapshot.get("analysis").is_none() {
            return Err("analysis capability unavailable".into());
        }
        Ok(command)
    }
    fn preview(&mut self, attributes: &[&str]) -> Result<String> {
        if self.input.release_required {
            return Err("release/pickup required".into());
        }
        let snapshot = self.client.authority.snapshot().ok_or("no snapshot")?;
        if !matches!(
            snapshot["wire_schema"].as_str(),
            Some("lx04-v1" | "lx04-durable-v1" | "lx05-v1" | "lx05-v2")
        ) || snapshot.get("release").is_none()
        {
            return Err("timed preview capability unavailable".into());
        }
        if self.selected.is_empty() || attributes.is_empty() {
            return Err("select exact preview scope".into());
        }
        let mut values = Vec::new();
        for fid in &self.selected {
            let f = snapshot["snapshot"]["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["fixture"] == *fid)
                .ok_or("preview fixture")?;
            for attr in attributes {
                let a = f["attributes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["attribute"] == *attr)
                    .ok_or("preview attribute")?;
                values.push(json!({"fixture":fid,"attribute":attr,"value":a["resolved"]}));
            }
        }
        if values.len() > 64 {
            return Err("preview target capacity".into());
        }
        self.input.invalidate()?;
        let reply = self
            .client
            .command(json!({"action":"release_preview","values":values}))?;
        Self::check_reply(reply)?;
        let token = self
            .client
            .authority
            .preview
            .clone()
            .ok_or("no validated engine preview")?;
        let deadline = self
            .client
            .authority
            .preview_until
            .ok_or("preview expired")?;
        let out = self.review(json!({"action":"release_commit","token":token}))?;
        self.input.confirmation.as_mut().unwrap().expires = deadline;
        self.freeze_material_review();
        Ok(format!(
            "ENGINE PREVIEW {} / finite500ms / no physical observation\n{}",
            token["preview"]["values"], out
        ))
    }
    pub fn status(&self) -> Result<String> {
        Ok(format!(
            "selection {:?} / page {} / confirmation {:?} / input release required {}\n{}",
            self.selected,
            self.page,
            self.input.confirmation.as_ref().map(|c| &c.command),
            self.input.release_required,
            self.client.authority.status()?
        ))
    }
    /// Native console uses the same material-review path as semantic controls.
    /// Headless CLI retains its accepted immediate-command behavior.
    pub fn reviewed_line(&mut self, line: &str) -> Result<(bool, String)> {
        self.review_commands = true;
        let result = self.line(line);
        self.review_commands = false;
        result
    }
    pub fn line(&mut self, line: &str) -> Result<(bool, String)> {
        if line.len() > 4096 {
            return Err("line capacity 4096 bytes".into());
        }
        let w: Vec<_> = line.split_whitespace().collect();
        let output = match w.as_slice() {
            [] => String::new(),
            ["quit"] => return Ok((false, String::new())),
            ["status"] => self.status()?,
            ["status-json", path] => {
                let report = json!({"cached_state":format!("{:?}",self.client.authority.freshness()),"physical":"unknown","submitted":null,"observed":null,"uncertain":self.client.authority.uncertain,"last_ack":self.client.authority.last_reply,"inventory":self.client.authority.snapshot(),"confirmation":self.input.confirmation.as_ref().map(|c| &c.command)});
                std::fs::write(
                    path,
                    serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                "status exported; logical metadata separate from physical unknown\n".into()
            }
            ["grant"] => {
                self.input.invalidate()?;
                Self::check_reply(self.client.grant()?)?
            }
            ["refresh"] => {
                self.input.invalidate()?;
                self.client.refresh()?;
                self.status()?
            }
            ["reconnect", epoch] => {
                let epoch = crate::codec::counter(&json!(epoch))?;
                self.input.lost()?;
                self.client.authority.disconnect();
                self.client = LocalClient::connect(&self.path, &self.show, epoch, &writer())?;
                self.epoch = epoch;
                self.status()?
            }
            ["disconnect"] => {
                self.input.lost()?;
                self.client.authority.disconnect();
                self.status()?
            }
            ["select", ids @ ..] if !ids.is_empty() && ids.len() <= 32 => {
                let snapshot = self.client.authority.snapshot().ok_or("no inventory")?;
                let mut selected = BTreeSet::new();
                for fid in ids {
                    adapter::id(&json!(fid))?;
                    if !snapshot["patch"]["fixtures"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["id"] == *fid)
                    {
                        return Err("unknown fixture".into());
                    }
                    selected.insert((*fid).into());
                }
                self.input.invalidate()?;
                self.selected = selected;
                self.status()?
            }
            ["page", page]
                if ["stage", "programmer", "library", "playbacks", "health"].contains(page) =>
            {
                self.input.invalidate()?;
                self.page = (*page).into();
                self.status()?
            }
            ["focus", "lost"] | ["inputlost"] => {
                self.input.lost()?;
                self.status()?
            }
            ["release-input"] | ["enter", "up"] => {
                self.input.release();
                "input released; absolute controls require fresh pickup\n".into()
            }
            ["cancel"] => {
                self.input.invalidate()?;
                if self.client.authority.preview.is_some() {
                    self.command(json!({"action":"release_cancel"}))?
                } else {
                    "confirmation cancelled\n".into()
                }
            }
            ["confirm"] | ["release"] | ["enter", "down"] => {
                let reviewed = self.input.confirmation.clone();
                let command = self.input.press(
                    self.client.authority.revision(),
                    self.client.authority.lease_id(),
                    self.epoch,
                    self.client.now(),
                )?;
                let reviewed = reviewed.ok_or("no reviewed confirmation")?;
                self.input.invalidate()?;
                let reply = self.client.confirmed_command(
                    command,
                    reviewed.revision,
                    &reviewed.lease,
                    reviewed.analysis_basis.as_ref(),
                )?;
                Self::check_reply(reply)?
            }
            ["touch", attribute, value] => {
                if self.input.release_required {
                    return Err("release/pickup required".into());
                }
                let value = value
                    .parse::<i32>()
                    .map_err(|_| "integer tenths required")?;
                let values = self.targets(&[(attribute, value)])?;
                self.command(json!({"action":"touch","values":values}))?
            }
            ["touch-rgb", r, g, b] => {
                if self.input.release_required {
                    return Err("release/pickup required".into());
                }
                let parse = |s: &str| s.parse::<i32>().map_err(|_| "integer tenths required");
                let values = self.targets(&[
                    ("red", parse(r)?),
                    ("green", parse(g)?),
                    ("blue", parse(b)?),
                ])?;
                self.command(json!({"action":"touch","values":values}))?
            }
            ["touch-position", pan, tilt] => {
                if self.input.release_required {
                    return Err("release/pickup required".into());
                }
                let parse = |s: &str| s.parse::<i32>().map_err(|_| "integer tenths required");
                let values = self.targets(&[("pan", parse(pan)?), ("tilt", parse(tilt)?)])?;
                self.command(json!({"action":"touch","values":values}))?
            }
            ["preview", attrs @ ..] if !attrs.is_empty() && attrs.len() <= 7 => {
                self.preview(attrs)?
            }
            ["checkpoint"] => self.command(json!({"action":"checkpoint"}))?,
            ["clearHold"] => self.command(json!({"action":"clear_to_hold"}))?,
            [
                action @ ("record" | "update"),
                kind @ ("cue" | "palette"),
                destination,
            ] => {
                adapter::id(&json!(destination))?;
                let command = json!({"action":action,"kind":kind,"id":destination});
                if *action == "update" {
                    self.review(command)?
                } else {
                    self.command(command)?
                }
            }
            ["go", cue, playback] => {
                adapter::id(&json!(cue))?;
                adapter::id(&json!(playback))?;
                self.command(json!({"action":"go","cue":cue,"playback":playback}))?
            }
            ["master", level] => {
                let level = level.parse::<i32>().map_err(|_| "integer level")?;
                adapter::level(&json!(level))?;
                self.command(json!({"action":"master","level":level}))?
            }
            ["blackout", "on"] => self.command(json!({"action":"blackout","enabled":true}))?,
            ["blackout", "off"] => self.review(json!({"action":"blackout","enabled":false}))?,
            ["calibrate", phase @ ("start" | "finish")] => {
                let command = self.analysis_command(phase, 0, 0)?;
                self.review(command)?
            }
            ["auto", "grant", cap, ttl] => {
                let command = self.analysis_command(
                    "grant",
                    cap.parse().map_err(|_| "cap integer tenths")?,
                    ttl.parse().map_err(|_| "TTL integer milliseconds")?,
                )?;
                self.review(command)?
            }
            ["auto", phase @ ("enter" | "revoke")] => {
                let command =
                    self.analysis_command(if *phase == "enter" { "auto" } else { "revoke" }, 0, 0)?;
                self.review(command)?
            }
            ["mode", mode @ ("manual" | "assist")] => {
                self.command(json!({"action":"mode","mode":mode}))?
            }
            ["wait", ms] => {
                self.client.wait(ms.parse().map_err(|_| "wait integer")?)?;
                self.status()?
            }
            ["help"] => CONTROL_HELP.into(),
            _ => return Err("unknown real Lux command; help lists accepted operations".into()),
        };
        Ok((true, output))
    }
}
pub const CONTROL_HELP: &str = "REAL LUX / logical null output; physical UNKNOWN\nstatus | grant | select STABLE_ID... | touch ATTRIBUTE INTEGER_TENTHS | touch-rgb R G B | touch-position PAN TILT\nrecord cue|palette ID | update cue|palette ID (review) | go CUE PLAYBACK | clearHold\npreview ATTRIBUTE... | release (exact reviewed engine token) | cancel | checkpoint (accepted durable capability only)\nNative K/L calibration start/finish; A AUTO entry; T cap-percent/TTL editor; X revoke to ASSIST\nmaster 0..1000 | blackout on|off (off review) | mode manual|assist\ncalibrate start|finish (review) | auto enter (review) | auto grant CAP_TENTHS TTL_MS (review selected fixtures, 10..2000ms) | auto revoke (review ASSIST exit)\nconfirm (Enter down) | enter up | release-input | cancel | page stage|programmer|library|playbacks|health | focus lost | inputlost\nrefresh | disconnect | reconnect EPOCH | wait 0..2000 | quit\n";

/// Bounded script or stdin session. Poll stdin so lease renewal continues while idle.
pub fn run(path: &Path, show: &str, epoch: u64, script: Option<&Path>) -> Result<()> {
    use std::io::{Read, Write};
    let mut op = Operator::connect(path, show, epoch)?;
    print!("{}", op.status()?);
    let mut failed = false;
    if let Some(script) = script {
        let mut bytes = Vec::new();
        std::fs::File::open(script)
            .map_err(|e| e.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 65536 {
            return Err("script capacity 64KiB".into());
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| "script UTF8")?;
        if text.lines().count() > 256 {
            return Err("script command capacity256".into());
        }
        for line in text.lines() {
            crate::lux_control::session_guard(op.client.now())?;
            match op.line(line) {
                Ok((more, output)) => {
                    print!("{output}");
                    if !more {
                        break;
                    }
                }
                Err(e) => {
                    failed = true;
                    println!("ERROR {e}");
                }
            }
        }
    } else {
        let mut line = Vec::new();
        let mut count = 0;
        loop {
            crate::lux_control::session_guard(op.client.now())?;
            if op.client.authority.lease_id().is_some() {
                if let Err(e) = op.client.maintain() {
                    op.input.lost()?;
                    println!("ERROR {e}");
                }
            } else {
                op.client.authority.advance(op.client.now())?;
            }
            std::io::stdout().flush().map_err(|e| e.to_string())?;
            let mut fd = libc::pollfd {
                fd: 0,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut fd, 1, 50) };
            if ready < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if ready == 0 {
                continue;
            }
            let mut byte = [0];
            if std::io::stdin()
                .read(&mut byte)
                .map_err(|e| e.to_string())?
                == 0
            {
                break;
            }
            if byte[0] != b'\n' {
                if line.len() >= 4096 {
                    return Err("stdin line capacity4096".into());
                }
                line.push(byte[0]);
                continue;
            }
            count += 1;
            if count > 256 {
                return Err("stdin command capacity256".into());
            }
            let text = std::str::from_utf8(&line).map_err(|_| "stdin UTF8")?;
            match op.line(text) {
                Ok((more, output)) => {
                    print!("{output}");
                    if !more {
                        break;
                    }
                }
                Err(e) => {
                    failed = true;
                    println!("ERROR {e}");
                }
            }
            line.clear();
        }
    }
    if op.client.authority.freshness() == Freshness::Stale {
        print!("{}", op.status()?);
    }
    if failed {
        Err("one or more operator commands refused".into())
    } else {
        Ok(())
    }
}

/// Validate analysis capabilities and exact bounded scope before preparing or sending intent.
pub fn validate_analysis_command(snapshot: &Value, command: &Value) -> Result<()> {
    let action = command["action"].as_str().unwrap_or("");
    if !matches!(action, "analysis_calibrate" | "analysis_grant")
        && !(action == "mode" && command["mode"] == "auto")
    {
        return Ok(());
    }
    if !matches!(
        snapshot["wire_schema"].as_str(),
        Some("lx05-v1" | "lx05-v2")
    ) || snapshot["applied_auto"] != "explicit_bounded_intensity_grant"
    {
        return Err("analysis capability unavailable".into());
    }
    let a = &snapshot["analysis"];
    if action == "analysis_calibrate" {
        adapter::schema(command, &["action", "phase"])?;
        if !matches!(command["phase"].as_str(), Some("start" | "finish")) {
            return Err("calibration phase".into());
        }
        if a["source_identity"].is_null() || a["source_age_ms"].as_u64().is_none_or(|age| age > 100)
        {
            return Err("fresh analysis source required".into());
        }
        if command["phase"] == "finish"
            && (a["state"] != "calibrating" || a["calibration_windows"].as_u64().unwrap_or(0) < 50)
        {
            return Err("calibration not ready: at least 50 source windows required".into());
        }
    } else if action == "analysis_grant" {
        adapter::schema(command, &["action", "fixtures", "cap", "ttl_ms"])?;
        if snapshot["mode"] != "auto" || a["state"] != "ready" || a["confidence"] != 1000 {
            return Err("AUTO mode and ready calibrated analysis required".into());
        }
        adapter::level(&command["cap"])?;
        let ttl = command["ttl_ms"].as_u64().ok_or("TTL milliseconds")?;
        if ttl == 0 || ttl > 2000 || !ttl.is_multiple_of(10) {
            return Err("TTL must be 10..2000ms in 10ms steps".into());
        }
        let fixtures = command["fixtures"].as_array().ok_or("fixture scope")?;
        if fixtures.is_empty() || fixtures.len() > 32 {
            return Err("select 1..32 fixtures".into());
        }
        let mut seen = BTreeSet::new();
        for fixture in fixtures {
            let fid = fixture.as_str().ok_or("fixture identity")?;
            if !seen.insert(fid) {
                return Err("duplicate fixture".into());
            }
            let capability = snapshot["capability_metadata"]
                .as_array()
                .and_then(|all| all.iter().find(|f| f["fixture"] == fid))
                .and_then(|f| f["attributes"].as_array())
                .and_then(|attrs| attrs.iter().find(|a| a["attribute"] == "intensity"))
                .ok_or("selected fixture intensity unavailable")?;
            let cap = adapter::integer(&command["cap"])?;
            if cap < adapter::integer(&capability["min"])?
                || cap > adapter::integer(&capability["max"])?
            {
                return Err("cap outside fixture range".into());
            }
        }
    }
    Ok(())
}

/// Identity relevant to a reviewed analysis operation; moving frame/age counters are not identities.
pub fn analysis_review_basis(inventory: &Value, command: &Value) -> Option<Value> {
    if !matches!(
        command["action"].as_str(),
        Some("analysis_calibrate" | "analysis_grant")
    ) && !(command["action"] == "mode" && command["mode"] == "auto")
    {
        return None;
    }
    let a = &inventory["analysis"];
    Some(
        json!({"source_identity":a["source_identity"],"generation":a["generation"],"calibration_generation":a["calibration_generation"],"binding":a["analysis_binding"]}),
    )
}
