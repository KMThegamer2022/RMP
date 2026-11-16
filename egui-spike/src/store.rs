//! Persistent preferences, matching the NSUserDefaults keys the shipping app keeps
//! (PlayerTheme, PreviousTheme, AdvancedControls, ShowStatusBar, UndockedPlaylist, StartupVolume,
//! RecentMedia, DesktopShortcut). Stored as key=value lines under Application Support.

use crate::theme::Skin;
use std::path::PathBuf;

const RECENT_MAX: usize = 20;

fn dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library/Application Support/RMP egui spike"))
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("RMP egui"))
    }
}

#[derive(Clone, Debug)]
pub struct Store {
    pub theme: Skin,
    pub previous_theme: Option<Skin>,
    pub advanced: bool,
    pub status_bar: bool,
    pub undocked_playlist: bool,
    pub startup_volume: Option<i32>,
    pub desktop_shortcut: bool,
    pub recent: Vec<String>,
    /// RMP_SPIKE_THEME overrides the theme for one run and nothing is saved while it is set.
    theme_override: bool,
}

/// Windows: the theme the installer asked for. Elsewhere there is none.
fn first_run_theme() -> Option<Skin> {
    #[cfg(windows)]
    {
        return crate::appkit::installer_theme().and_then(Skin::from_index);
    }
    #[cfg(not(windows))]
    None
}

impl Store {
    pub fn load() -> Self {
        let mut s = Store {
            theme: Skin::Vlc, // ui.m registers VLC_THEME as the default
            previous_theme: None,
            advanced: false,
            status_bar: false,
            undocked_playlist: false,
            startup_volume: None,
            desktop_shortcut: true, // desktopShortcutWanted() defaults to YES
            recent: Vec::new(),
            theme_override: false,
        };
        if let Some(d) = dir() {
            if let Ok(text) = std::fs::read_to_string(d.join("prefs.txt")) {
                for line in text.lines() {
                    let Some((k, v)) = line.split_once('=') else { continue };
                    match k {
                        "theme" => s.theme = Skin::from_slug(v).unwrap_or(Skin::Vlc),
                        "previous_theme" => s.previous_theme = Skin::from_slug(v),
                        "advanced" => s.advanced = v == "1",
                        "status_bar" => s.status_bar = v == "1",
                        "undocked_playlist" => s.undocked_playlist = v == "1",
                        "startup_volume" => s.startup_volume = v.parse().ok(),
                        "desktop_shortcut" => s.desktop_shortcut = v == "1",
                        _ => {}
                    }
                }
            } else if let Some(skin) = first_run_theme() {
                s.theme = skin;
            } else if let Ok(t) = std::fs::read_to_string(d.join("theme.txt")) {
                // Earlier spike builds kept only the theme.
                s.theme = Skin::from_slug(&t).unwrap_or(Skin::Vlc);
            }
            if let Ok(text) = std::fs::read_to_string(d.join("recent.txt")) {
                s.recent = text.lines().filter(|l| !l.is_empty()).map(String::from).collect();
            }
        }
        if let Some(skin) = std::env::var("RMP_SPIKE_THEME").ok().and_then(|v| Skin::from_slug(&v)) {
            s.theme = skin;
            s.theme_override = true;
        }
        s
    }

    pub fn save(&self) {
        if self.theme_override {
            return;
        }
        let Some(d) = dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let mut text = format!(
            "theme={}\nadvanced={}\nstatus_bar={}\nundocked_playlist={}\ndesktop_shortcut={}\n",
            self.theme.slug(),
            self.advanced as u8,
            self.status_bar as u8,
            self.undocked_playlist as u8,
            self.desktop_shortcut as u8
        );
        if let Some(p) = self.previous_theme {
            text.push_str(&format!("previous_theme={}\n", p.slug()));
        }
        if let Some(v) = self.startup_volume {
            text.push_str(&format!("startup_volume={v}\n"));
        }
        let _ = std::fs::write(d.join("prefs.txt"), text);
    }

    /// rememberRecent: newest first, no duplicates, at most 20.
    pub fn remember_recent(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.recent.retain(|r| r != path);
        self.recent.insert(0, path.to_string());
        self.recent.truncate(RECENT_MAX);
        self.save_recent();
    }

    pub fn clear_recent(&mut self) {
        self.recent.clear();
        self.save_recent();
    }

    fn save_recent(&self) {
        if self.theme_override {
            // Screenshot runs still keep recent media; only the theme override is temporary.
        }
        let Some(d) = dir() else { return };
        let _ = std::fs::create_dir_all(&d);
        let _ = std::fs::write(d.join("recent.txt"), self.recent.join("\n"));
    }
}
