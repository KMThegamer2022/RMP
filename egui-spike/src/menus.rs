//! Menu bar, following installPlayerMenus in mac-menus.h. Engine-backed entries (`Cmd`, `Options`)
//! call the engine's ap_menu_action / ap_menu_options, the same functions the shipping app calls.

use crate::{App, SKINS};
use eframe::egui;
use std::ffi::{c_char, CStr, CString};

extern "C" {
    // Frees a string returned by ap_menu_action / ap_menu_options (the shipping app calls ap_terminal_free).
    fn ap_terminal_free(text: *mut c_char);
}

/// App-level actions the shipping app implements in ObjC rather than in the engine.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Native {
    OpenFile,
    OpenMultiple,
    OpenStream,
    OpenPlaylist,
    SavePlaylist,
    TimeJump,
    PlayPause,
    Stop,
    Previous,
    Next,
    Fullscreen,
    Quit,
    ClearRecent,
    About,
    /// Shipping items that this spike does not implement yet; shows the same note style as menuNote.
    Pending(&'static str),
}

pub enum Entry {
    Sep,
    /// Engine command, e.g. ap_menu_action("mute", "").
    Cmd { title: &'static str, key: &'static str, value: &'static str, shortcut: &'static str },
    /// Submenu whose rows come from ap_menu_options(key), filled when opened.
    Options { title: &'static str, key: &'static str },
    Native { title: &'static str, action: Native, shortcut: &'static str },
    Sub { title: &'static str, items: Vec<Entry> },
    /// Recent media list (Open Recent Media).
    Recent,
    /// Theme choices (View → Theme).
    Theme,
}

fn cmd(title: &'static str, key: &'static str, value: &'static str) -> Entry {
    Entry::Cmd { title, key, value, shortcut: "" }
}
fn cmd_sc(title: &'static str, key: &'static str, value: &'static str, shortcut: &'static str) -> Entry {
    Entry::Cmd { title, key, value, shortcut }
}
fn opts(title: &'static str, key: &'static str) -> Entry {
    Entry::Options { title, key }
}
fn nat(title: &'static str, action: Native, shortcut: &'static str) -> Entry {
    Entry::Native { title, action, shortcut }
}

/// Menu bar in the order of installPlayerMenus (mac-menus.h). The View menu also carries Theme, as in ui.m.
pub fn menu_bar() -> Vec<(&'static str, Vec<Entry>)> {
    use Entry::Sep;
    use Native::*;
    vec![
        (
            "Media",
            vec![
                nat("Open File…", OpenFile, "⌘O"),
                nat("Open Multiple Files…", OpenMultiple, "⇧⌘O"),
                cmd("Open Folder…", "folder", ""),
                cmd("Open Disc…", "disc", ""),
                nat("Open Network Stream…", OpenStream, "⌘U"),
                cmd("Open Capture Device…", "capture", ""),
                cmd("Open Location from Clipboard", "clipboard", ""),
                Entry::Sub { title: "Open Recent Media", items: vec![Entry::Recent] },
                Sep,
                nat("Open Playlist…", OpenPlaylist, ""),
                nat("Save Playlist to File…", SavePlaylist, ""),
                cmd("Convert / Save…", "convert", ""),
                cmd("Stream…", "stream-out", ""),
                cmd("Stop Media Output", "output-stop", ""),
                Sep,
                cmd("Quit at the End of Playlist", "quit-end", ""),
                nat("Quit RMP", Quit, "⌘Q"),
            ],
        ),
        (
            "Playback",
            vec![
                opts("Title", "title"),
                opts("Chapter", "chapter"),
                opts("Programme", "programme"),
                opts("Custom Bookmarks", "bookmark"),
                Sep,
                opts("Renderer", "renderer"),
                opts("Speed", "speed"),
                Sep,
                cmd_sc("Jump Forward", "jump", "10", ""),
                cmd_sc("Jump Backward", "jump", "-10", ""),
                nat("Jump to Specific Time…", TimeJump, "⌘T"),
                Sep,
                nat("Play / Pause", PlayPause, "Space"),
                nat("Stop", Stop, ""),
                nat("Previous", Previous, "["),
                nat("Next", Next, "]"),
                cmd("Record", "record", ""),
            ],
        ),
        (
            "Audio",
            vec![
                opts("Audio Track", "audio-track"),
                opts("Audio Device", "audio-device"),
                opts("Stereo Mode", "stereo"),
                opts("Visualizations", "visualization"),
                Sep,
                cmd_sc("Increase Volume", "volume-step", "5", ""),
                cmd_sc("Decrease Volume", "volume-step", "-5", ""),
                cmd("Mute", "mute", ""),
            ],
        ),
        (
            "Video",
            vec![
                opts("Video Track", "video-track"),
                Sep,
                nat("Fullscreen", Fullscreen, "⌃⌘F"),
                cmd("Always Fit Window", "fit", ""),
                cmd("Set as Wallpaper", "wallpaper", ""),
                opts("Zoom", "zoom"),
                opts("Aspect Ratio", "aspect"),
                opts("Crop", "crop"),
                opts("Deinterlace", "deinterlace"),
                opts("Deinterlace Mode", "deinterlace-mode"),
                Sep,
                cmd("Take Snapshot…", "snapshot", ""),
            ],
        ),
        (
            "Subtitle",
            vec![cmd("Add Subtitle File…", "subtitle-file", ""), opts("Sub Track", "subtitle-track")],
        ),
        (
            "Tools",
            vec![
                cmd_sc("Effects and Filters…", "effects", "", "⌘E"),
                cmd("Track Synchronization…", "sync", ""),
                cmd("Media Information", "media-info", ""),
                cmd("Codec Information", "codec-info", ""),
                cmd("VLM Configuration", "vlm", ""),
                cmd("Program Guide", "guide", ""),
                cmd("Messages", "messages", ""),
                cmd("Plugins and Extensions", "plugins", ""),
                Sep,
                cmd("Customize Interface…", "customize", ""),
                cmd("Preferences…", "preferences", ""),
                cmd("Desktop Shortcut", "desktop-shortcut", ""),
            ],
        ),
        (
            "View",
            vec![
                cmd_sc("Playlist", "playlist", "", "⌘L"),
                cmd("Docked Playlist", "docked", ""),
                cmd("Always on Top", "top", ""),
                Sep,
                cmd("Minimal Interface", "minimal", ""),
                nat("Fullscreen Interface", Fullscreen, ""),
                cmd("Advanced Controls", "advanced", ""),
                cmd("Status Bar", "status", ""),
                Entry::Sub { title: "Add Interface", items: vec![nat("RMP Terminal", Pending("RMP Terminal"), "")] },
                cmd("VLsub…", "vlsub", ""),
                Sep,
                Entry::Sub { title: "Theme", items: vec![Entry::Theme] },
            ],
        ),
        (
            "Help",
            vec![
                cmd("Help", "help", ""),
                nat("Check for Updates…", Pending("Check for Updates"), ""),
                Sep,
                nat("About RMP", About, ""),
            ],
        ),
    ]
}

/// Engine call: returns the note text ("" when the command succeeded silently).
pub fn engine_action(app: &mut App, key: &str, value: &str) -> String {
    let (Ok(k), Ok(v)) = (CString::new(key), CString::new(value)) else { return String::new() };
    if app.player.is_null() {
        return "No media engine.".into();
    }
    unsafe { take_string(engine_menu_action(app.player as *mut std::ffi::c_void, k.as_ptr(), v.as_ptr())) }
}

/// Rows for an Options submenu: (identifier, title).
pub fn engine_options(app: &mut App, key: &str) -> Vec<(String, String)> {
    let Ok(k) = CString::new(key) else { return Vec::new() };
    if app.player.is_null() {
        return Vec::new();
    }
    let text = unsafe { take_string(engine_menu_options(app.player as *mut std::ffi::c_void, k.as_ptr())) };
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(id, title)| (id.to_string(), title.to_string()))
        .collect()
}

unsafe fn take_string(raw: *mut c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
    ap_terminal_free(raw);
    s
}

// The engine's exported menu functions (src/vlc_menu.rs). They are `#[no_mangle]`, so they link into this binary.
extern "C" {
    #[link_name = "ap_menu_action"]
    fn engine_menu_action(p: *mut std::ffi::c_void, key: *const c_char, value: *const c_char) -> *mut c_char;
    #[link_name = "ap_menu_options"]
    fn engine_menu_options(p: *mut std::ffi::c_void, key: *const c_char) -> *mut c_char;
}

/// Renders one menu's entries. Engine submenus refresh while open, as populateMediaMenu does.
pub fn render(ui: &mut egui::Ui, app: &mut App, entries: &[Entry], actions: &mut Vec<Native>, notes: &mut Vec<(String, String)>) {
    for entry in entries {
        match entry {
            Entry::Sep => {
                ui.separator();
            }
            Entry::Cmd { title, key, value, shortcut } => {
                if button(ui, title, shortcut) {
                    let result = engine_action(app, key, value);
                    if !result.is_empty() {
                        notes.push((title.to_string(), result));
                    }
                    ui.close();
                }
            }
            Entry::Options { title, key } => {
                ui.menu_button(*title, |ui| {
                    let rows = engine_options(app, key);
                    if rows.is_empty() {
                        ui.add_enabled(false, egui::Button::new("None available"));
                    }
                    for (id, label) in rows {
                        if ui.button(label.clone()).clicked() {
                            let result = engine_action(app, key, &id);
                            if !result.is_empty() {
                                notes.push((label, result));
                            }
                            ui.close();
                        }
                    }
                });
            }
            Entry::Native { title, action, shortcut } => {
                if button(ui, title, shortcut) {
                    actions.push(*action);
                    ui.close();
                }
            }
            Entry::Sub { title, items } => {
                ui.menu_button(*title, |ui| render(ui, app, items, actions, notes));
            }
            Entry::Recent => {
                let recent = app.recent.clone();
                if recent.is_empty() {
                    ui.add_enabled(false, egui::Button::new("No recent media"));
                } else {
                    for path in recent {
                        let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                        if ui.button(name).on_hover_text(&path).clicked() {
                            app.open_path(std::path::PathBuf::from(path));
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent Media").clicked() {
                        actions.push(Native::ClearRecent);
                        ui.close();
                    }
                }
            }
            Entry::Theme => {
                for k in SKINS {
                    if ui.radio(app.skin == k, k.label()).clicked() {
                        app.set_skin(k);
                        ui.close();
                    }
                }
            }
        }
    }
}

fn button(ui: &mut egui::Ui, title: &str, shortcut: &str) -> bool {
    let mut b = egui::Button::new(title);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
    }
    ui.add(b).clicked()
}

impl App {
    /// Runs a menu action picked this frame.
    pub fn run_native(&mut self, action: Native, ctx: &egui::Context) {
        match action {
            Native::OpenFile => self.open_dialog(false),
            Native::OpenMultiple => self.open_dialog(true),
            Native::OpenStream => self.prompt = Some(crate::Prompt::Stream(String::new())),
            Native::OpenPlaylist => {
                if let Some(path) = rfd::FileDialog::new().add_filter("Playlist", &["m3u", "m3u8", "xspf"]).pick_file() {
                    if let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) {
                        if let Some(p) = self.player() {
                            if !unsafe { crate::engine::ap_playlist_load(p, c.as_ptr()) } {
                                self.note = Some(("Open Playlist".into(), "Could not read that playlist.".into()));
                            }
                        }
                    }
                }
            }
            Native::SavePlaylist => {
                if let Some(path) = rfd::FileDialog::new().set_file_name("playlist.m3u").save_file() {
                    if let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) {
                        if let Some(p) = self.player() {
                            if !unsafe { crate::engine::ap_playlist_save(p, c.as_ptr()) } {
                                self.note = Some(("Save Playlist to File".into(), "Could not write the playlist.".into()));
                            }
                        }
                    }
                }
            }
            Native::TimeJump => self.prompt = Some(crate::Prompt::Time(String::new())),
            Native::PlayPause => self.toggle(),
            Native::Stop => {
                if let Some(p) = self.player() {
                    unsafe { crate::engine::ap_stop(p) };
                }
            }
            Native::Previous => self.step(-1),
            Native::Next => self.step(1),
            Native::Fullscreen => {
                let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
            }
            Native::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Native::ClearRecent => self.clear_recent(),
            Native::About => {
                self.note = Some(("About RMP".into(), "RMP egui spike. Not the shipping app.".into()));
            }
            Native::Pending(name) => {
                self.note = Some((name.into(), "This item is not in the egui spike yet.".into()));
            }
        }
    }
}
