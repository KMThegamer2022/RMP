//! The menu bar from ui.m `menus` and mac-menus.h `installPlayerMenus`, drawn in the window.
//! Items are collected while the bar is drawn and run afterwards (see `Picked`).

use crate::theme::{Skin, SKINS};
use crate::App;
use eframe::egui;

/// Actions the shipping app binds to selectors rather than to runMenuCommand keys.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    About,
    CheckUpdates,
    Quit,
    Cut,
    Copy,
    Paste,
    SelectAll,
    OpenSingle,
    OpenMultiple,
    OpenStream,
    OpenPlaylist,
    SavePlaylist,
    PlayPause,
    Stop,
    Previous,
    Next,
    Fullscreen,
}

/// What a click picked, run after the menu bar is drawn.
#[derive(Clone, Debug)]
pub enum Picked {
    Command { key: &'static str, value: String, title: String },
    Action(Action),
    Option { key: &'static str, id: String, title: String },
    Theme(Skin),
}

pub enum Entry {
    Sep,
    /// runMenuCommand key/value.
    Cmd { title: &'static str, key: &'static str, value: &'static str, shortcut: &'static str },
    Act { title: &'static str, action: Action, shortcut: &'static str },
    /// Submenu filled from ap_menu_options(key) when opened.
    Options { title: &'static str, key: &'static str },
    Sub { title: &'static str, items: Vec<Entry> },
    Recent,
    Theme,
}

fn cmd(title: &'static str, key: &'static str) -> Entry {
    Entry::Cmd { title, key, value: "", shortcut: "" }
}
fn cmdv(title: &'static str, key: &'static str, value: &'static str) -> Entry {
    Entry::Cmd { title, key, value, shortcut: "" }
}
fn cmds(title: &'static str, key: &'static str, shortcut: &'static str) -> Entry {
    Entry::Cmd { title, key, value: "", shortcut }
}
fn act(title: &'static str, action: Action, shortcut: &'static str) -> Entry {
    Entry::Act { title, action, shortcut }
}
fn opts(title: &'static str, key: &'static str) -> Entry {
    Entry::Options { title, key }
}

pub fn menu_bar() -> Vec<(&'static str, Vec<Entry>)> {
    use Action::*;
    use Entry::Sep;
    vec![
        // ui.m `menus`: the app menu and Edit.
        ("RMP", vec![act("About RMP", About, ""), act("Check for Updates…", CheckUpdates, ""), Sep, act("Quit RMP", Quit, "⌘Q")]),
        ("Edit", vec![act("Cut", Cut, "⌘X"), act("Copy", Copy, "⌘C"), act("Paste", Paste, "⌘V"), act("Select All", SelectAll, "⌘A")]),
        (
            "Media",
            vec![
                act("Open File…", OpenSingle, "⌘O"),
                act("Open Multiple Files…", OpenMultiple, "⇧⌘O"),
                cmd("Open Folder…", "folder"),
                cmd("Open Disc…", "disc"),
                act("Open Network Stream…", OpenStream, "⌘U"),
                cmd("Open Capture Device…", "capture"),
                cmd("Open Location from Clipboard", "clipboard"),
                Entry::Sub { title: "Open Recent Media", items: vec![Entry::Recent] },
                Sep,
                act("Open Playlist…", OpenPlaylist, ""),
                act("Save Playlist to File…", SavePlaylist, ""),
                cmd("Convert / Save…", "convert"),
                cmd("Stream…", "stream-out"),
                cmd("Stop Media Output", "output-stop"),
                Sep,
                cmd("Quit at the End of Playlist", "quit-end"),
                act("Quit RMP", Quit, ""),
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
                cmdv("Jump Forward", "jump", "10"),
                cmdv("Jump Backward", "jump", "-10"),
                cmds("Jump to Specific Time…", "time", "⌘T"),
                Sep,
                act("Play / Pause", PlayPause, "Space"),
                act("Stop", Stop, ""),
                act("Previous", Previous, "["),
                act("Next", Next, "]"),
                cmd("Record", "record"),
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
                cmdv("Increase Volume", "volume-step", "5"),
                cmdv("Decrease Volume", "volume-step", "-5"),
                cmd("Mute", "mute"),
            ],
        ),
        (
            "Video",
            vec![
                opts("Video Track", "video-track"),
                Sep,
                act("Fullscreen", Fullscreen, "⌃⌘F"),
                cmd("Always Fit Window", "fit"),
                cmd("Set as Wallpaper", "wallpaper"),
                opts("Zoom", "zoom"),
                opts("Aspect Ratio", "aspect"),
                opts("Crop", "crop"),
                opts("Deinterlace", "deinterlace"),
                opts("Deinterlace Mode", "deinterlace-mode"),
                Sep,
                cmd("Take Snapshot…", "snapshot"),
            ],
        ),
        ("Subtitle", vec![cmd("Add Subtitle File…", "subtitle-file"), opts("Sub Track", "subtitle-track")]),
        (
            "Tools",
            vec![
                cmds("Effects and Filters…", "effects", "⌘E"),
                cmd("Track Synchronization…", "sync"),
                cmd("Media Information", "media-info"),
                cmd("Codec Information", "codec-info"),
                cmd("VLM Configuration", "vlm"),
                cmd("Program Guide", "guide"),
                cmd("Messages", "messages"),
                cmd("Plugins and Extensions", "plugins"),
                Sep,
                cmd("Customize Interface…", "customize"),
                cmd("Preferences…", "preferences"),
                cmd("Desktop Shortcut", "desktop-shortcut"),
            ],
        ),
        (
            "View",
            vec![
                cmds("Playlist", "playlist", "⌘L"),
                cmd("Docked Playlist", "docked"),
                cmd("Always on Top", "top"),
                Sep,
                cmd("Minimal Interface", "minimal"),
                act("Fullscreen Interface", Fullscreen, ""),
                cmd("Advanced Controls", "advanced"),
                cmd("Status Bar", "status"),
                Entry::Sub { title: "Add Interface", items: vec![cmd("RMP Terminal", "terminal")] },
                cmd("VLsub…", "vlsub"),
                Entry::Sub { title: "Theme", items: vec![Entry::Theme] },
            ],
        ),
        ("Help", vec![cmd("Help", "help"), act("Check for Updates…", CheckUpdates, ""), Sep, act("About RMP", About, "")]),
    ]
}

/// A menu item with an optional checkmark column and shortcut, like an NSMenuItem.
fn item(ui: &mut egui::Ui, title: &str, shortcut: &str, checked: Option<bool>, enabled: bool) -> bool {
    let label = match checked {
        Some(true) => format!("✓  {title}"),
        Some(false) => format!("     {title}"),
        None => title.to_string(),
    };
    let mut b = egui::Button::new(label);
    if !shortcut.is_empty() {
        b = b.shortcut_text(shortcut);
    }
    ui.add_enabled(enabled, b).clicked()
}

pub fn render(ui: &mut egui::Ui, app: &mut App, entries: &[Entry], picked: &mut Vec<Picked>) {
    for entry in entries {
        match entry {
            Entry::Sep => {
                ui.separator();
            }
            Entry::Cmd { title, key, value, shortcut } => {
                if item(ui, title, shortcut, app.menu_checked(key), app.menu_enabled(key)) {
                    picked.push(Picked::Command { key, value: value.to_string(), title: title.to_string() });
                    ui.close();
                }
            }
            Entry::Act { title, action, shortcut } => {
                if item(ui, title, shortcut, None, true) {
                    picked.push(Picked::Action(*action));
                    ui.close();
                }
            }
            Entry::Options { title, key } => {
                ui.menu_button(*title, |ui| {
                    let rows = app.engine_options(key);
                    if rows.is_empty() {
                        ui.add_enabled(false, egui::Button::new("None available"));
                    }
                    for (id, label) in rows {
                        if ui.button(&label).clicked() {
                            picked.push(Picked::Option { key, id, title: label });
                            ui.close();
                        }
                    }
                });
            }
            Entry::Sub { title, items } => {
                ui.menu_button(*title, |ui| render(ui, app, items, picked));
            }
            Entry::Recent => {
                let recent = app.store.recent.clone();
                if recent.is_empty() {
                    ui.add_enabled(false, egui::Button::new("No recent media"));
                } else {
                    for path in recent {
                        let name = std::path::Path::new(&path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| path.clone());
                        if ui.button(name).on_hover_text(&path).clicked() {
                            picked.push(Picked::Command { key: "recent", value: path.clone(), title: "Open Recent Media".into() });
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent Media").clicked() {
                        picked.push(Picked::Command { key: "clear-recent", value: String::new(), title: "Clear Recent Media".into() });
                        ui.close();
                    }
                }
            }
            Entry::Theme => {
                for k in SKINS {
                    if item(ui, k.label(), "", Some(app.skin == k), true) {
                        picked.push(Picked::Theme(k));
                        ui.close();
                    }
                }
            }
        }
    }
}
