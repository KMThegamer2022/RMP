//! RMP Terminal (the Linux skin's terminal window), driven by the engine's ap_terminal_* functions,
//! as mac-terminal.h does in the shipping app.

use eframe::egui;
use std::ffi::{c_char, c_void, CStr, CString};

extern "C" {
    #[link_name = "ap_terminal_command"]
    fn engine_terminal_command(p: *mut c_void, line: *const c_char, action: *mut i32) -> *mut c_char;
    #[link_name = "ap_terminal_prompt"]
    fn engine_terminal_prompt(p: *mut c_void) -> *mut c_char;
    #[link_name = "ap_terminal_complete"]
    fn engine_terminal_complete(p: *mut c_void, line: *const c_char) -> *mut c_char;
    #[link_name = "ap_terminal_volume"]
    fn engine_terminal_volume(p: *mut c_void) -> i32;
    #[link_name = "ap_terminal_free"]
    fn free_engine_string(text: *mut c_char);
}

/// Terminal state: transcript, input line, history.
pub struct Terminal {
    pub hidden: bool,
    pub lines: Vec<(String, egui::Color32)>,
    pub input: String,
    pub history: Vec<String>,
    pub history_index: usize,
    pub draft: String,
    pub prompt: String,
    pub started: bool,
}

pub const INK: egui::Color32 = egui::Color32::from_rgb(0xdc, 0xe2, 0xdd);
pub const PROMPT_INK: egui::Color32 = egui::Color32::from_rgb(0x8b, 0xdc, 0x9e);
pub const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(0x11, 0x14, 0x16);

impl Terminal {
    pub fn new() -> Self {
        Self {
            hidden: false,
            lines: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            history_index: 0,
            draft: String::new(),
            prompt: String::new(),
            started: false,
        }
    }
}

unsafe fn take(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
    free_engine_string(raw);
    s
}

/// Runs one command. Returns (output, action code) as mac-terminal.h's submitTerminalLine reads them.
pub fn command(player: *mut c_void, line: &str) -> (String, i32) {
    let Ok(c) = CString::new(line) else { return (String::new(), 0) };
    let mut action = 0;
    let out = unsafe { take(engine_terminal_command(player, c.as_ptr(), &mut action)) };
    (out, action)
}

pub fn prompt(player: *mut c_void) -> String {
    unsafe { take(engine_terminal_prompt(player)) }
}

pub fn complete(player: *mut c_void, line: &str) -> String {
    let Ok(c) = CString::new(line) else { return String::new() };
    unsafe { take(engine_terminal_complete(player, c.as_ptr())) }
}

pub fn volume(player: *mut c_void) -> i32 {
    unsafe { engine_terminal_volume(player) }
}
