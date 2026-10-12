//! Ratatui adapter using the same typed Lux actions and review coverage as native.
use crate::{
    frontend::{self, Worker},
    native_actions::Semantic,
    render::{Primitive, Scene},
    surface::{Page, actions::Action},
};
use shr_instrument_panel::{
    scene,
    terminal::{
        Console,
        ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    },
};
use std::{path::PathBuf, time::Duration};
struct App {
    worker: Worker,
    keyboard: frontend::Keyboard,
    notice: String,
    help: bool,
    command_mode: bool,
    semantic_editing: bool,
    review_seen: Option<(u64, usize, usize)>,
    review_context: Option<u64>,
}
impl App {
    fn send(&mut self, line: String) {
        self.notice.clear();
        self.help = false;
        if let Err(e) = self.worker.enqueue(line) {
            self.notice = e;
            self.keyboard.draft.clear();
        }
    }
    fn action(&mut self, action: Semantic) {
        self.notice.clear();
        self.help = false;
        if let Err(e) = self.worker.action(action) {
            self.notice = e;
            self.lost();
        }
    }
    fn lost(&mut self) {
        self.worker.invalidate();
        self.keyboard.context_lost();
        self.review_seen = None;
        self.review_context = None;
    }
    fn key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                if let Some(line) = self.keyboard.enter() {
                    if self.command_mode {
                        self.send(line);
                        self.command_mode = false;
                    } else {
                        self.action(Semantic::Action(Action::Confirm));
                        self.semantic_editing = false;
                    }
                }
            }
            KeyCode::Esc => {
                self.keyboard.draft.clear();
                self.command_mode = false;
                self.semantic_editing = false;
                self.action(Semantic::Action(Action::Cancel));
            }
            KeyCode::Backspace => {
                if self.command_mode {
                    self.keyboard.draft.pop();
                } else {
                    self.action(Semantic::Action(Action::Backspace));
                }
            }
            KeyCode::PageDown => self.keyboard.scroll = self.keyboard.scroll.saturating_add(20),
            KeyCode::PageUp => self.keyboard.scroll = self.keyboard.scroll.saturating_sub(20),
            KeyCode::F(12) => {
                self.help = !self.help;
                self.keyboard.scroll = 0;
            }
            KeyCode::F(1) => self.action(Semantic::Action(Action::Page(Page::Stage))),
            KeyCode::F(2) => self.action(Semantic::Action(Action::Page(Page::Programmer))),
            KeyCode::F(3) => self.action(Semantic::Action(Action::Page(Page::Library))),
            KeyCode::F(4) => self.action(Semantic::Action(Action::Page(Page::Playbacks))),
            KeyCode::F(5) => self.action(Semantic::Action(Action::Page(Page::Health))),
            KeyCode::F(6) => self.action(Semantic::Grant),
            KeyCode::F(7) => self.action(Semantic::Scenes),
            KeyCode::Home => self.action(Semantic::Bank(-1)),
            KeyCode::End => self.action(Semantic::Bank(1)),
            KeyCode::Right => self.action(Semantic::Action(Action::Navigate(1))),
            KeyCode::Left => self.action(Semantic::Action(Action::Navigate(-1))),
            KeyCode::Up => self.action(Semantic::Action(Action::Attribute(-1))),
            KeyCode::Down => self.action(Semantic::Action(Action::Attribute(1))),
            KeyCode::Char(' ') => {
                if self.command_mode {
                    self.keyboard.text(" ");
                } else if self.semantic_editing || self.worker.view.lock().unwrap().editor.is_some()
                {
                    self.action(Semantic::Action(Action::Text(" ".into())));
                } else {
                    self.action(Semantic::Action(Action::Select(vec![])));
                }
            }
            KeyCode::Char(character) => {
                let text = character.to_string();
                if self.command_mode {
                    self.keyboard.text(&text);
                } else if self.semantic_editing || self.worker.view.lock().unwrap().editor.is_some()
                {
                    self.action(Semantic::Action(Action::Text(text.to_string())));
                } else if text.as_str() == "R" || text.as_str() == "u" || text.as_str() == "U" {
                    self.semantic_editing = true;
                    self.action(Semantic::Action(Action::Record {
                        palette: text.as_str() == "R" || text.as_str() == "U",
                        replace: text.as_str() == "u" || text.as_str() == "U",
                        id: 0,
                    }));
                } else {
                    match text.to_ascii_lowercase().as_str() {
                        ":" => {
                            self.command_mode = true;
                            self.keyboard.draft.clear();
                        }
                        "e" => {
                            self.semantic_editing = true;
                            self.action(Semantic::Edit);
                        }
                        "r" => {
                            self.semantic_editing = true;
                            self.action(Semantic::Action(Action::Record {
                                palette: false,
                                replace: false,
                                id: 0,
                            }));
                        }
                        "g" => {
                            self.semantic_editing = true;
                            self.action(Semantic::OpenGo);
                        }
                        "c" => self.action(Semantic::Action(Action::ClearHold)),
                        "p" => self.action(Semantic::Preview),
                        "b" => self.action(Semantic::Blackout),
                        "m" => self.action(Semantic::Mode),
                        "k" => self.action(Semantic::Calibrate(false)),
                        "l" => self.action(Semantic::Calibrate(true)),
                        "a" => self.action(Semantic::AutoEnter),
                        "t" => {
                            self.semantic_editing = true;
                            self.action(Semantic::AutoGrant);
                        }
                        "s" => self.action(Semantic::SceneEdit),
                        "v" => self.action(Semantic::ScenePreview),
                        "x" => self.action(Semantic::AutoRevoke),
                        "d" => self.action(Semantic::Action(Action::Deselect)),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
pub fn primitives(s: &Scene) -> Vec<scene::Primitive> {
    s.primitives
        .iter()
        .map(|p| match p {
            Primitive::Rect { x, y, w, h, fill } => scene::Primitive::Rect {
                x: *x,
                y: *y,
                w: *w,
                h: *h,
                fill: fill.clone(),
            },
            Primitive::Text { x, y, value, color } => scene::Primitive::Text {
                x: *x,
                y: *y,
                value: value.clone(),
                color: (*color).into(),
            },
            Primitive::Line {
                x1,
                y1,
                x2,
                y2,
                color,
            } => scene::Primitive::Line {
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                color: (*color).into(),
            },
        })
        .collect()
}

pub fn run(
    path: PathBuf,
    show: String,
    epoch: u64,
    role: Option<crate::role_client::Config>,
) -> Result<(), String> {
    let mut console = Console::open().map_err(|e| e.to_string())?;
    let mut app = App {
        worker: Worker::spawn_with_role(path, show, epoch, role),
        keyboard: frontend::Keyboard::default(),
        notice: String::new(),
        help: false,
        command_mode: false,
        semantic_editing: false,
        review_seen: None,
        review_context: None,
    };
    app.keyboard.focus(true);
    app.keyboard.release_enter();
    let result = (|| -> Result<(), String> {
        loop {
            let mut view = app.worker.view.lock().unwrap().current();
            if !app.notice.is_empty() {
                view.notice = app.notice.clone();
            }
            if app.help {
                view.page = "help".into();
            }
            if view.review_context != app.review_context {
                app.keyboard.scroll = 0;
                app.review_seen = None;
                app.review_context = view.review_context;
            }
            let s = primitives(&frontend::scene(
                &view,
                &app.keyboard.draft,
                app.keyboard.scroll,
            ));
            let mut visible = false;
            let material = console.material;
            console
                .terminal
                .draw(|f| {
                    visible = if view.page != "scenes" {
                        scene::instrument_text(
                            &s,
                            f.buffer_mut(),
                            material,
                            view.freshness == "Fresh",
                            view.lease,
                            &[
                                "F1 STAGE",
                                "F2 PROGRAM",
                                "F3 LIBRARY",
                                "F4 PLAYBACK",
                                "F5 HEALTH",
                                "F7 SCENES",
                                "F6 GRANT",
                            ],
                        )
                    } else {
                        scene::draw(&s, f.buffer_mut(), material)
                    };
                })
                .map_err(|e| e.to_string())?;
            if visible
                && let Some((context, start, through)) =
                    frontend::visible_review_range(&view, app.keyboard.scroll)
                && app.review_seen != Some((context, start, through))
                && app
                    .worker
                    .action(Semantic::Reviewed {
                        context,
                        start,
                        through,
                    })
                    .is_ok()
            {
                app.review_seen = Some((context, start, through));
            }
            if event::poll(Duration::from_millis(40)).map_err(|e| e.to_string())? {
                match event::read().map_err(|e| e.to_string())? {
                    Event::Key(k)
                        if k.code == KeyCode::Char('c')
                            && k.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        break;
                    }
                    Event::Key(k)
                        if k.code == KeyCode::Enter && k.kind == KeyEventKind::Release =>
                    {
                        app.keyboard.release_enter();
                        app.action(Semantic::EnterUp);
                    }
                    Event::Key(k)
                        if visible && app.keyboard.focused && k.kind == KeyEventKind::Press =>
                    {
                        // No legacy repeat can masquerade as a fresh confirmation edge.
                        if k.code == KeyCode::Enter && !console.release_events {
                            app.notice =
                                "Confirmation unavailable: terminal lacks release events".into();
                        } else {
                            app.key(k.code);
                        }
                    }
                    Event::FocusLost => {
                        app.keyboard.focus(false);
                        app.lost();
                    }
                    Event::FocusGained => {
                        app.keyboard.focus(true);
                        app.lost();
                    }
                    Event::Resize(_, _) => app.lost(),
                    _ => {}
                }
            }
        }
        Ok(())
    })();
    app.lost();
    result
}
