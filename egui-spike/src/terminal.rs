//! RMP Terminal (mac-terminal.h): the Linux skin's terminal window, driven by the engine's
//! ap_terminal_* functions. Rust owns command parsing, completion and the working directory.

use crate::theme::Skin;
use crate::App;
use eframe::egui;
use std::ffi::{c_char, c_void, CStr, CString};

extern "C" {
    fn ap_terminal_command(p: *mut c_void, line: *const c_char, action: *mut i32) -> *mut c_char;
    fn ap_terminal_prompt(p: *mut c_void) -> *mut c_char;
    fn ap_terminal_complete(p: *mut c_void, line: *const c_char) -> *mut c_char;
    fn ap_terminal_volume(p: *mut c_void) -> i32;
    fn ap_terminal_free(text: *mut c_char);
}

const INK: egui::Color32 = egui::Color32::from_rgb(0xdc, 0xe2, 0xdd);
const PROMPT_INK: egui::Color32 = egui::Color32::from_rgb(0x8b, 0xdc, 0x9e);
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(0x11, 0x14, 0x16);
/// appendTerminal trims the transcript above 200000 characters.
const TRANSCRIPT_MAX: usize = 200_000;

pub struct Terminal {
    pub hidden: bool,
    lines: Vec<(String, egui::Color32)>,
    input: String,
    history: Vec<String>,
    history_index: usize,
    draft: String,
    prompt: String,
    started: bool,
}

impl Terminal {
    /// The transcript as plain text (used by the test hook).
    pub fn transcript(&self) -> String {
        self.lines.iter().map(|l| l.0.as_str()).collect::<String>()
    }
    pub fn set_input(&mut self, text: &str) {
        self.input = text.to_string();
    }
    pub fn new() -> Self {
        Self { hidden: false, lines: Vec::new(), input: String::new(), history: Vec::new(), history_index: 0, draft: String::new(), prompt: String::new(), started: false }
    }
}

unsafe fn take(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
    ap_terminal_free(raw);
    s
}

impl App {
    /// showTerminal / the terminal window's content.
    pub fn terminal_window(&mut self, ctx: &egui::Context) {
        let Some(raw) = self.engine.as_ref().map(|e| e.raw()) else { return };
        let builder = egui::ViewportBuilder::default().with_title("RMP Terminal").with_inner_size([650.0, 300.0]).with_min_inner_size([400.0, 180.0]);
        ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("rmp-terminal"), builder, |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                // windowShouldClose: hide the terminal and bring the player forward.
                self.term.hidden = true;
                return;
            }
            if !self.term.started {
                self.term.started = true;
                self.term.prompt = unsafe { take(ap_terminal_prompt(raw)) };
                self.term.lines.push(("RMP Terminal\nType help to list commands.\n".into(), INK));
            }
            self.terminal_contents(ctx, raw);
        });
    }

    fn terminal_contents(&mut self, ctx: &egui::Context, raw: *mut c_void) {
        ctx.set_visuals(egui::Visuals::dark());
        // The shipping terminal does not forward Space to the player (playMenuItem loses its key).
        egui::Area::new(egui::Id::new("terminal-root")).fixed_pos(egui::Pos2::ZERO).show(ctx, |ui| {
            ui.set_min_size(ctx.content_rect().size());
            egui::Frame::new().fill(BACKGROUND).inner_margin(egui::Margin { left: 14, right: 14, top: 12, bottom: 12 }).show(ui, |ui| {
                let height = (ui.available_height() - 24.0).max(40.0);
                egui::ScrollArea::vertical().stick_to_bottom(true).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (text, color) in &self.term.lines {
                        ui.label(egui::RichText::new(text.trim_end_matches('\n')).monospace().size(crate::fs(13.0)).color(*color));
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new(&self.term.prompt).monospace().size(crate::fs(13.0)).color(PROMPT_INK));
                    let edit = egui::TextEdit::singleline(&mut self.term.input)
                        .font(egui::FontId::monospace(13.0))
                        .text_color(INK)
                        .frame(egui::Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .lock_focus(true);
                    let resp = ui.add(edit);
                    resp.request_focus();
                });
            });
        });
        let (enter, tab, up, down) = ctx.input(|i| {
            (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Tab), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown))
        });
        if enter {
            self.terminal_submit(ctx, raw);
        }
        if tab {
            if let Ok(line) = CString::new(self.term.input.clone()) {
                let candidate = unsafe { take(ap_terminal_complete(raw, line.as_ptr())) };
                if !candidate.is_empty() {
                    self.term.input = candidate;
                }
            }
        }
        if up && self.term.history_index > 0 {
            if self.term.history_index == self.term.history.len() {
                self.term.draft = self.term.input.clone();
            }
            self.term.history_index -= 1;
            self.term.input = self.term.history[self.term.history_index].clone();
        }
        if down && self.term.history_index < self.term.history.len() {
            self.term.history_index += 1;
            self.term.input = if self.term.history_index == self.term.history.len() { self.term.draft.clone() } else { self.term.history[self.term.history_index].clone() };
        }
    }

    /// submitTerminalLine.
    pub fn terminal_submit(&mut self, ctx: &egui::Context, raw: *mut c_void) {
        let line = std::mem::take(&mut self.term.input);
        if line.is_empty() {
            return;
        }
        if self.term.history.last() != Some(&line) {
            self.term.history.push(line.clone());
            if self.term.history.len() > 200 {
                self.term.history.remove(0);
            }
        }
        self.term.history_index = self.term.history.len();
        self.term.draft.clear();
        self.term.lines.push((format!("{} ", self.term.prompt), PROMPT_INK));
        self.term.lines.push((format!("{line}\n"), INK));
        let mut action = 0i32;
        let output = match CString::new(line) {
            Ok(c) => unsafe { take(ap_terminal_command(raw, c.as_ptr(), &mut action)) },
            Err(_) => String::new(),
        };
        if action == 2 {
            self.term.lines.clear();
        }
        if !output.is_empty() {
            let text = if output.ends_with('\n') { output } else { format!("{output}\n") };
            self.term.lines.push((text, INK));
        }
        let total: usize = self.term.lines.iter().map(|l| l.0.len()).sum();
        if total > TRANSCRIPT_MAX {
            while self.term.lines.len() > 1 && self.term.lines.iter().map(|l| l.0.len()).sum::<usize>() > TRANSCRIPT_MAX - 16_000 {
                self.term.lines.remove(0);
            }
        }
        self.term.prompt = unsafe { take(ap_terminal_prompt(raw)) };
        // Theme changes: 9 = theme back, 10+index = a named theme.
        if action == 9 || (10..10 + 7).contains(&action) {
            let choice = if action == 9 { self.store.previous_theme.unwrap_or(Skin::Vlc) } else { Skin::from_index(action - 10).unwrap_or(Skin::Vlc) };
            let choice = if action == 9 && choice == Skin::Linux { Skin::Vlc } else { choice };
            self.pick_theme(choice);
        }
        if action == 1 {
            self.full(ctx);
        }
        if action == 3 && self.index() >= 0 {
            self.show_playing_item(self.index() as usize);
        }
        if action == 4 {
            self.volume = unsafe { ap_terminal_volume(raw) };
        }
    }
}
