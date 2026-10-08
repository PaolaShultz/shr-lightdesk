//! Native semantic workflow over the accepted Lux operator. No fixture authority.
use crate::{
    adapter,
    lux_control::LightingAuthority,
    lux_operator::Operator,
    model::Attribute,
    surface::{
        Page,
        actions::{Action, Draft, typed_value},
    },
};
use serde_json::json;
#[derive(Clone, Debug)]
pub enum Semantic {
    Action(Action),
    Grant,
    EnterUp,
    Preview,
    OpenGo,
    Mode,
    Calibrate(bool),
    AutoEnter,
    AutoGrant,
    AutoRevoke,
    Edit,
    Blackout,
    Bank(i32),
    Reviewed {
        context: u64,
        start: usize,
        through: usize,
    },
}
#[derive(Clone, Debug)]
pub enum Editor {
    Existing(Draft),
    Go(String),
    AutoGrant(String),
}
impl Editor {
    pub fn text(&self) -> String {
        match self {
            Self::Existing(Draft::Attribute { text, .. } | Draft::Record { text, .. })
            | Self::Go(text)
            | Self::AutoGrant(text) => text.clone(),
            Self::Existing(Draft::BlackoutOff) => String::new(),
        }
    }
}
#[derive(Default)]
pub struct Workflow {
    pub cursor: usize,
    pub attribute: usize,
    pub editor: Option<Editor>,
    revision: u64,
    lease: Option<String>,
    selection: Vec<String>,
    progress: ReviewProgress,
}
pub fn attribute_name(a: Attribute) -> &'static str {
    match a {
        Attribute::Intensity => "intensity",
        Attribute::Red => "red",
        Attribute::Green => "green",
        Attribute::Blue => "blue",
        Attribute::Pan => "pan",
        Attribute::Tilt => "tilt",
        Attribute::Zoom => "zoom",
    }
}
#[derive(Default)]
pub struct ReviewProgress {
    context: Option<u64>,
    seen: Vec<bool>,
}
impl ReviewProgress {
    pub fn see(&mut self, context: u64, start: usize, end: usize, total: usize) {
        if total == 0 || total > 4096 {
            return;
        }
        if self.context != Some(context) || self.seen.len() != total {
            self.context = Some(context);
            self.seen = vec![false; total];
        }
        if start >= end || end > total {
            return;
        }
        self.seen[start..end].fill(true);
    }
    pub fn complete(&self, context: u64, total: usize) -> bool {
        self.context == Some(context)
            && self.seen.len() == total
            && !self.seen.is_empty()
            && self.seen.iter().all(|seen| *seen)
    }
}
impl Workflow {
    pub fn lost(&mut self) {
        self.lease = None;
        self.progress = ReviewProgress::default();
    }
    fn begin(&mut self, op: &mut Operator, editor: Editor) -> Result<String, String> {
        op.client.maintain()?;
        if !op.client.authority.writable(op.client.now()) || op.input.release_required {
            return Err("fresh grant and released input required".into());
        }
        op.input.invalidate()?;
        self.revision = op.client.authority.revision();
        self.lease = op.client.authority.lease_id().map(str::to_owned);
        self.selection = op.selected.iter().cloned().collect();
        self.editor = Some(editor);
        Ok("Detached draft; Enter reviews exact targets; release then Enter confirms".into())
    }
    fn discard(&mut self, op: &mut Operator) -> Result<(), String> {
        self.editor = None;
        self.lost();
        op.input.invalidate()
    }
    fn check_draft(&self, op: &Operator) -> Result<(), String> {
        op.client.authority.check_review(
            self.revision,
            self.lease.as_deref().ok_or("draft writer unavailable")?,
            op.client.now(),
        )?;
        if self.selection != op.selected.iter().cloned().collect::<Vec<_>>() {
            return Err("draft selection changed".into());
        }
        Ok(())
    }
    pub fn dispatch(&mut self, op: &mut Operator, event: Semantic) -> Result<String, String> {
        match event {
            Semantic::Reviewed {
                context,
                start,
                through,
            } => {
                if let Some(c) = &op.input.confirmation
                    && c.context == context
                {
                    self.progress.see(
                        context,
                        start,
                        through,
                        crate::frontend::operator_review_lines(op).len(),
                    );
                }
                Ok("Review viewport acknowledged".into())
            }
            Semantic::Bank(delta) => {
                self.discard(op)?;
                let count = op
                    .client
                    .authority
                    .snapshot()
                    .and_then(|s| s["patch"]["fixtures"].as_array())
                    .map_or(0, Vec::len);
                if count == 0 {
                    return Err("fixture bank unavailable".into());
                }
                let banks = count.div_ceil(12);
                self.cursor = ((self.cursor / 12) as i64 + i64::from(delta))
                    .rem_euclid(banks as i64) as usize
                    * 12;
                Ok("Fixture bank focus moved; Space selects exact slot".into())
            }
            Semantic::Edit => self.dispatch(
                op,
                Semantic::Action(Action::Edit(Attribute::ALL[self.attribute])),
            ),
            Semantic::Blackout => {
                let enabled = op
                    .client
                    .authority
                    .snapshot()
                    .is_some_and(|s| s["blackout"] == false);
                self.dispatch(op, Semantic::Action(Action::Blackout(enabled)))
            }
            Semantic::EnterUp => {
                op.input.release();
                Ok("Enter released".into())
            }
            Semantic::Grant => {
                self.lost();
                op.line("grant").map(|(_, s)| s)
            }
            Semantic::OpenGo => self.begin(op, Editor::Go(String::new())),
            Semantic::Calibrate(finish) => {
                self.discard(op)?;
                let command = op.analysis_command(if finish { "finish" } else { "start" }, 0, 0)?;
                op.review(command)
            }
            Semantic::AutoEnter | Semantic::AutoRevoke => {
                let phase = if matches!(event, Semantic::AutoEnter) {
                    "auto"
                } else {
                    "revoke"
                };
                self.discard(op)?;
                let command = op.analysis_command(phase, 0, 0)?;
                op.review(command)
            }
            Semantic::AutoGrant => self.begin(op, Editor::AutoGrant(String::new())),
            Semantic::Mode => {
                self.discard(op)?;
                let mode = if op
                    .client
                    .authority
                    .snapshot()
                    .is_some_and(|s| s["mode"] == "manual")
                {
                    "assist"
                } else {
                    "manual"
                };
                op.review(json!({"action":"mode","mode":mode}))
            }
            Semantic::Preview => {
                self.discard(op)?;
                let a = attribute_name(Attribute::ALL[self.attribute]);
                op.line(&format!("preview {a}")).map(|(_, s)| s)
            }
            Semantic::Action(action) => match action {
                Action::Page(page) => {
                    self.discard(op)?;
                    let page = match page {
                        Page::Stage => "stage",
                        Page::Programmer => "programmer",
                        Page::Library => "library",
                        Page::Playbacks => "playbacks",
                        Page::Health => "health",
                        _ => {
                            return Err(
                                "page capability unavailable in accepted Lux provider".into()
                            );
                        }
                    };
                    op.line(&format!("page {page}")).map(|(_, s)| s)
                }
                Action::Navigate(delta) => {
                    self.discard(op)?;
                    let count = op
                        .client
                        .authority
                        .snapshot()
                        .and_then(|s| s["patch"]["fixtures"].as_array())
                        .map_or(0, Vec::len);
                    if count == 0 {
                        return Err("no fixture inventory".into());
                    }
                    self.cursor =
                        (self.cursor as i64 + i64::from(delta)).rem_euclid(count as i64) as usize;
                    Ok("Fixture focus moved; Space selects".into())
                }
                Action::Select(slots) => {
                    self.discard(op)?;
                    let snapshot = op.client.authority.snapshot().ok_or("no inventory")?;
                    let fs = snapshot["patch"]["fixtures"]
                        .as_array()
                        .ok_or("no fixtures")?;
                    let slots = if slots.is_empty() {
                        vec![(self.cursor + 1) as u16]
                    } else {
                        slots
                    };
                    let mut ids = Vec::new();
                    for slot in slots {
                        let index = usize::from(slot).checked_sub(1).ok_or("slot starts1")?;
                        let id = fs
                            .get(index)
                            .and_then(|f| f["id"].as_str())
                            .ok_or("fixture slot unavailable")?;
                        ids.push(id.to_owned());
                    }
                    op.line(&format!("select {}", ids.join(" ")))
                        .map(|(_, s)| s)
                }
                Action::Deselect => {
                    self.discard(op)?;
                    op.selected.clear();
                    Ok("Deselected; Lux look unchanged".into())
                }
                Action::Attribute(delta) => {
                    self.discard(op)?;
                    self.attribute =
                        (self.attribute as i64 + i64::from(delta)).rem_euclid(7) as usize;
                    Ok(format!(
                        "Attribute focus {}",
                        attribute_name(Attribute::ALL[self.attribute])
                    ))
                }
                Action::Edit(attribute) => self.begin(
                    op,
                    Editor::Existing(Draft::Attribute {
                        attribute,
                        text: String::new(),
                    }),
                ),
                Action::Record {
                    palette,
                    replace,
                    id,
                } => self.begin(
                    op,
                    Editor::Existing(Draft::Record {
                        palette,
                        replace,
                        text: if id == 0 {
                            String::new()
                        } else {
                            format!("{}-{id}", if palette { "palette" } else { "cue" })
                        },
                    }),
                ),
                Action::Text(text) => {
                    if text.len() > 128 || !text.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
                        return Err("draft text bound".into());
                    }
                    let field = match self.editor.as_mut() {
                        Some(Editor::Existing(
                            Draft::Attribute { text, .. } | Draft::Record { text, .. },
                        ))
                        | Some(Editor::Go(text))
                        | Some(Editor::AutoGrant(text)) => text,
                        _ => return Err("no detached draft".into()),
                    };
                    if field.len() + text.len() > 128 {
                        return Err("draft capacity128".into());
                    }
                    field.push_str(&text);
                    Ok("Draft edited; no provider write".into())
                }
                Action::Backspace => {
                    match self.editor.as_mut() {
                        Some(Editor::Existing(
                            Draft::Attribute { text, .. } | Draft::Record { text, .. },
                        ))
                        | Some(Editor::Go(text))
                        | Some(Editor::AutoGrant(text)) => {
                            text.pop();
                        }
                        _ => {}
                    }
                    Ok("Draft edited".into())
                }
                Action::ClearHold => {
                    self.discard(op)?;
                    op.review(json!({"action":"clear_to_hold"}))
                }
                Action::Blackout(enabled) => {
                    self.discard(op)?;
                    op.review(json!({"action":"blackout","enabled":enabled}))
                }
                Action::Confirm => {
                    if let Some(editor) = self.editor.clone() {
                        if op.input.held_enter || op.input.release_required {
                            return Err("held input: release before review".into());
                        }
                        op.input.held_enter = true;
                        op.client.maintain()?;
                        if !op.client.authority.writable(op.client.now()) {
                            return Err("fresh writer required to review retained draft".into());
                        }
                        if self.selection != op.selected.iter().cloned().collect::<Vec<_>>() {
                            return Err(
                                "draft selection changed; restore original selection or cancel"
                                    .into(),
                            );
                        }
                        self.revision = op.client.authority.revision();
                        self.lease = op.client.authority.lease_id().map(str::to_owned);
                        self.check_draft(op)?;
                        let command = match editor {
                            Editor::Existing(Draft::Attribute { attribute, text }) => {
                                let fields: Vec<_> = text.split_whitespace().collect();
                                let attrs: Vec<(&str, i32)> = match attribute {
                                    Attribute::Red | Attribute::Green | Attribute::Blue => {
                                        if fields.len() != 3 {
                                            return Err(
                                                "coherent RGB: enter three percent values R G B"
                                                    .into(),
                                            );
                                        }
                                        vec![
                                            ("red", typed_value(fields[0])?),
                                            ("green", typed_value(fields[1])?),
                                            ("blue", typed_value(fields[2])?),
                                        ]
                                    }
                                    Attribute::Pan | Attribute::Tilt => {
                                        if fields.len() != 2 {
                                            return Err(
                                                "coherent position: enter PAN TILT degrees".into(),
                                            );
                                        }
                                        vec![
                                            ("pan", typed_value(fields[0])?),
                                            ("tilt", typed_value(fields[1])?),
                                        ]
                                    }
                                    _ => vec![(attribute_name(attribute), typed_value(&text)?)],
                                };
                                json!({"action":"touch","values":op.targets(&attrs)?})
                            }
                            Editor::Existing(Draft::Record {
                                palette,
                                replace,
                                text,
                            }) => {
                                adapter::id(&json!(text))?;
                                json!({"action":if replace{"update"}else{"record"},"kind":if palette{"palette"}else{"cue"},"id":text})
                            }
                            Editor::AutoGrant(text) => {
                                let fields: Vec<_> = text.split_whitespace().collect();
                                if fields.len() != 2 {
                                    return Err("AUTO draft: enter CAP_PERCENT TTL_MS (10..2000ms in 10ms steps)".into());
                                }
                                op.analysis_command(
                                    "grant",
                                    typed_value(fields[0])?,
                                    fields[1].parse().map_err(|_| "TTL integer milliseconds")?,
                                )?
                            }
                            Editor::Go(text) => {
                                let fields: Vec<_> = text.split_whitespace().collect();
                                if fields.len() != 2 {
                                    return Err("GO draft: enter CUE_ID PLAYBACK_ID".into());
                                }
                                adapter::id(&json!(fields[0]))?;
                                adapter::id(&json!(fields[1]))?;
                                json!({"action":"go","cue":fields[0],"playback":fields[1]})
                            }
                            Editor::Existing(Draft::BlackoutOff) => {
                                json!({"action":"blackout","enabled":false})
                            }
                        };
                        let result = op.review(command)?;
                        self.editor = None;
                        Ok(result)
                    } else {
                        if !op.input.confirmation.as_ref().is_some_and(|c| {
                            self.progress.complete(
                                c.context,
                                crate::frontend::operator_review_lines(op).len(),
                            )
                        }) {
                            return Err(
                                "review all exact targets before confirmation; scroll to end"
                                    .into(),
                            );
                        }
                        op.line("confirm").map(|(_, s)| s)
                    }
                }
                Action::Cancel | Action::Back => {
                    self.editor = None;
                    self.lost();
                    op.line("cancel").map(|(_, s)| s)
                }
                Action::ContextLost => {
                    self.lost();
                    op.line("inputlost").map(|(_, s)| s)
                }
                Action::Layer(_) | Action::Multi(_) => {
                    Err("group/multi semantic binding not advertised by native workflow".into())
                }
            },
        }
    }
}
