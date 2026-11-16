//! Safe wrappers over the engine's C API (repo-root src/lib.rs, compiled unchanged as this crate's
//! library), the same calls ui.m makes. All calls happen on the UI thread.

use egui_spike as ffi;
use std::ffi::{c_char, c_void, CStr, CString};

extern "C" {
    // Exported by src/vlc_menu.rs and src/terminal.rs with #[no_mangle].
    fn ap_menu_action(p: *mut c_void, key: *const c_char, value: *const c_char) -> *mut c_char;
    fn ap_menu_options(p: *mut c_void, key: *const c_char) -> *mut c_char;
    fn ap_terminal_free(text: *mut c_char);
}

pub struct Engine {
    p: *mut ffi::Player,
}

/// takeTerminalString: copy and free a string the engine returned.
unsafe fn take(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
    ap_terminal_free(raw);
    s
}

fn c(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

impl Engine {
    /// ap_create with the view libvlc draws into, then ap_prepare to warm libvlc up.
    pub fn create(view: *mut c_void) -> Option<Engine> {
        let p = unsafe { ffi::ap_create(view) };
        if p.is_null() {
            return None;
        }
        let mut e = Engine { p };
        ffi::ap_prepare(e.mut_ref());
        Some(e)
    }
    fn mut_ref(&mut self) -> &mut ffi::Player {
        unsafe { &mut *self.p }
    }
    fn r(&self) -> &ffi::Player {
        unsafe { &*self.p }
    }
    pub fn raw(&self) -> *mut c_void {
        self.p as *mut c_void
    }
    pub fn prepare(&mut self) {
        ffi::ap_prepare(self.mut_ref());
    }
    pub fn count(&self) -> usize {
        ffi::ap_count(self.r())
    }
    pub fn index(&self) -> isize {
        ffi::ap_index(self.r())
    }
    /// ap_item: the queued path or URL.
    pub fn item(&self, i: usize) -> Option<String> {
        let raw = ffi::ap_item(self.r(), i);
        (!raw.is_null()).then(|| unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned())
    }
    /// `display:` — the last path component, or the whole string.
    pub fn display(&self, i: usize) -> String {
        display_name(&self.item(i).unwrap_or_default())
    }
    pub fn library_count(&self) -> usize {
        ffi::ap_library_count(self.r())
    }
    pub fn library_item(&self, i: usize) -> Option<String> {
        let raw = ffi::ap_library_item(self.r(), i);
        (!raw.is_null()).then(|| unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned())
    }
    pub fn library_play(&mut self, i: usize) -> bool {
        unsafe { ffi::ap_library_play(self.mut_ref(), i) }
    }
    pub fn add(&mut self, source: &str, network: bool) {
        let s = c(source);
        unsafe { ffi::ap_add(self.mut_ref(), s.as_ptr(), network) };
    }
    pub fn play(&mut self, i: usize) -> bool {
        unsafe { ffi::ap_play(self.mut_ref(), i) }
    }
    pub fn toggle(&mut self) {
        unsafe { ffi::ap_toggle(self.mut_ref()) };
    }
    pub fn stop(&mut self) {
        unsafe { ffi::ap_stop(self.r()) };
    }
    pub fn state(&self) -> i32 {
        unsafe { ffi::ap_state(self.r()) }
    }
    pub fn time(&self) -> i64 {
        unsafe { ffi::ap_time(self.r()) }
    }
    pub fn length(&self) -> i64 {
        unsafe { ffi::ap_length(self.r()) }
    }
    pub fn seekable(&self) -> bool {
        unsafe { ffi::ap_seekable(self.r()) }
    }
    pub fn seek(&mut self, ms: i64) {
        unsafe { ffi::ap_seek(self.r(), ms) };
    }
    pub fn set_volume(&mut self, v: i32) {
        unsafe { ffi::ap_volume(self.mut_ref(), v) };
    }
    pub fn has_video(&self) -> bool {
        unsafe { ffi::ap_has_video(self.r()) }
    }
    pub fn playlist_save(&self, path: &str) -> bool {
        let s = c(path);
        unsafe { ffi::ap_playlist_save(self.r(), s.as_ptr()) }
    }
    pub fn playlist_load(&mut self, path: &str) -> bool {
        let s = c(path);
        unsafe { ffi::ap_playlist_load(self.mut_ref(), s.as_ptr()) }
    }
    /// engineAction: the note text, empty when the command succeeded silently.
    pub fn menu_action(&mut self, key: &str, value: &str) -> String {
        let (k, v) = (c(key), c(value));
        unsafe { take(ap_menu_action(self.raw(), k.as_ptr(), v.as_ptr())) }
    }
    /// ap_menu_options rows: (identifier, title).
    pub fn menu_options(&mut self, key: &str) -> Vec<(String, String)> {
        let k = c(key);
        let text = unsafe { take(ap_menu_options(self.raw(), k.as_ptr())) };
        text.lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { ffi::ap_destroy(self.p) };
    }
}

pub fn display_name(s: &str) -> String {
    let trimmed = s.trim_end_matches('/');
    match trimmed.rsplit('/').next() {
        Some(last) if !last.is_empty() => last.to_string(),
        _ => s.to_string(),
    }
}
