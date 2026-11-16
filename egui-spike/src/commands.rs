//! Menu commands: runMenuCommand and validateMenuItem from mac-menus.h, and the selector-bound
//! actions from ui.m (open:, openSingle:, stream:, openPlaylist:, savePlaylist:, prev:, next:, full:).

use crate::dialogs::{AskPurpose, Dialog};
use crate::menus::{Action, Picked};
use crate::theme::Skin;
use crate::{appkit, metadata, updates, App, MediaPage};
use eframe::egui;
use std::path::{Path, PathBuf};

/// CFBundleDocumentTypes extensions from Info.plist (openFolder keeps only these).
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "3g2", "3gp", "aac", "ac3", "aif", "aifc", "aiff", "alac", "amr", "aob", "ape", "asf", "au", "avi", "axa", "axv", "bik", "caf", "crf",
    "divx", "dts", "dvdmedia", "eac3", "evo", "f4v", "flac", "flv", "gxf", "it", "m1v", "m2p", "m2ts", "m2v", "m4a", "m4b", "m4p", "m4v",
    "mid", "midi", "mk3d", "mka", "mks", "mkv", "mlp", "mod", "mov", "mp3", "mp4", "mpa", "mpc", "mpeg", "mpeg2", "mpeg4", "mpg", "mts",
    "mxf", "nsv", "nuv", "oga", "ogg", "ogm", "ogv", "oma", "opus", "rec", "rmi", "s3m", "spx", "tak", "tod", "ts", "tta", "vob", "voc",
    "vqf", "vro", "wav", "webm", "wma", "wmv", "wtv", "wv", "xa", "xesc", "xm",
];

const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub", "idx", "ttml", "smi"];
const README_URL: &str = "https://github.com/KMThegamer2022/RMP#readme";

impl App {
    pub fn run_picked(&mut self, picked: Picked, ctx: &egui::Context) {
        match picked {
            Picked::Command { key, value, title } => self.run_command(key, value, &title, ctx),
            Picked::Action(a) => self.run_action(a, ctx),
            Picked::Option { key, id, title } => {
                // applyOption
                let result = self.engine.as_mut().map(|e| e.menu_action(key, &id)).unwrap_or_default();
                if !result.is_empty() {
                    self.note(title, result);
                }
            }
            Picked::Theme(skin) => self.pick_theme(skin),
        }
    }

    /// ap_menu_options for an engine submenu (empty without an engine).
    pub fn engine_options(&mut self, key: &str) -> Vec<(String, String)> {
        self.engine.as_mut().map(|e| e.menu_options(key)).unwrap_or_default()
    }

    /// validateMenuItem: checkmarks.
    pub fn menu_checked(&self, key: &str) -> Option<bool> {
        Some(match key {
            "quit-end" => self.quit_at_end,
            "top" => self.always_top,
            "wallpaper" => self.wallpaper.is_some(),
            "minimal" => self.minimal,
            "advanced" => self.store.advanced,
            "status" => self.store.status_bar,
            "playlist" => self.playlist_shown || (self.skin.has_media_layout() && self.media_page == MediaPage::Playlist),
            "docked" => !self.store.undocked_playlist,
            "record" => self.recording,
            "desktop-shortcut" => self.store.desktop_shortcut,
            _ => return None,
        })
    }

    /// validateMenuItem: enabled state.
    pub fn menu_enabled(&self, key: &str) -> bool {
        let Some(e) = self.engine.as_ref() else { return !matches!(key, "snapshot" | "jump" | "time") };
        match key {
            "snapshot" => e.has_video(),
            "jump" | "time" => e.seekable(),
            "record" | "convert" | "stream-out" | "subtitle-file" | "vlsub" => e.index() >= 0,
            _ => true,
        }
    }

    pub fn run_action(&mut self, action: Action, ctx: &egui::Context) {
        match action {
            Action::About => appkit::show_about_panel(&updates::current_version(), crate::LOGO_PNG),
            Action::CheckUpdates => self.start_update_check(true),
            Action::Quit => {
                self.quit_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Cut | Action::Copy | Action::Paste | Action::SelectAll => self.pending_edit = Some(action),
            Action::OpenSingle => self.open_files(false),
            Action::OpenMultiple => self.open_files(true),
            Action::OpenStream => self.dialogs.push(Dialog::Ask {
                title: "Open Network Stream".into(),
                detail: String::new(),
                value: String::new(),
                placeholder: "https://".into(),
                purpose: AskPurpose::NetworkStream,
            }),
            Action::OpenPlaylist => self.open_playlist(),
            Action::SavePlaylist => self.save_playlist(),
            Action::PlayPause => self.toggle(),
            Action::Stop => self.stop(),
            Action::Previous => self.prev(),
            Action::Next => self.next(),
            Action::Fullscreen => self.full(ctx),
        }
    }

    /// open: and openSingle: — an NSOpenPanel with no type filter.
    pub fn open_files(&mut self, multiple: bool) {
        if let Some(e) = self.engine.as_mut() {
            e.prepare();
        }
        let files: Vec<PathBuf> = if !self.pick.is_empty() {
            if multiple { self.pick.drain(..).collect() } else { vec![self.pick.remove(0)] }
        } else {
            let dialog = rfd::FileDialog::new();
            if multiple { dialog.pick_files().unwrap_or_default() } else { dialog.pick_file().into_iter().collect() }
        };
        self.accept(files.into_iter().map(|p| (p.to_string_lossy().into_owned(), false)).collect());
    }

    /// openLocation: a path that exists, else a URL with a scheme.
    pub fn open_location(&mut self, location: &str) {
        if location.is_empty() {
            return;
        }
        let expanded = if let Some(rest) = location.strip_prefix('~') {
            format!("{}{}", std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default(), rest)
        } else {
            location.to_string()
        };
        if Path::new(&expanded).exists() {
            self.accept(vec![(expanded, false)]);
            return;
        }
        if has_scheme(location) {
            self.accept(vec![(location.to_string(), true)]);
            return;
        }
        self.note("Open Location", "Choose a media file or enter a URL including its protocol.");
    }

    /// stream: — the network URL must have a scheme and must not be a file URL.
    pub fn open_stream(&mut self, text: &str) {
        let url = text.trim();
        if has_scheme(url) && !url.starts_with("file:") {
            self.accept(vec![(url.to_string(), true)]);
        } else {
            self.note("Enter a stream URL including its protocol.", "");
        }
    }

    fn open_playlist(&mut self) {
        let picked = if self.pick.is_empty() { rfd::FileDialog::new().add_filter("Playlist", &["m3u8", "m3u"]).pick_file() } else { Some(self.pick.remove(0)) };
        let Some(path) = picked else { return };
        let ok = self.engine.as_mut().is_some_and(|e| e.playlist_load(&path.to_string_lossy()));
        if !ok {
            self.note("RMP could not read this playlist.", "");
            return;
        }
        self.search.clear();
        if self.skin.has_media_layout() {
            self.media_page = MediaPage::Playlist;
        }
        if self.count() > 0 {
            self.launch(0);
        } else {
            self.title = "RMP".into();
            self.name = "RMP".into();
        }
    }

    fn save_playlist(&mut self) {
        if self.count() == 0 {
            self.note("Add media to the queue before saving a playlist.", "");
            return;
        }
        let picked = if self.pick.is_empty() { rfd::FileDialog::new().set_file_name("Playlist.m3u8").add_filter("Playlist", &["m3u8", "m3u"]).save_file() } else { Some(self.pick.remove(0)) };
        let Some(path) = picked else { return };
        if !self.engine.as_ref().is_some_and(|e| e.playlist_save(&path.to_string_lossy())) {
            self.note("RMP could not write this playlist.", "");
        }
    }

    /// pickOutput: a save panel named after the current item, then the engine command with the path.
    /// RMP_SPIKE_OUTPUT_DIR skips the panel (used by automated checks only).
    fn pick_output(&mut self, key: &str, title: &str, ext: &str) {
        let name = if self.index() >= 0 { self.engine.as_ref().map(|e| e.display(self.index() as usize)).unwrap_or_default() } else { "RMP".into() };
        let stem = Path::new(&name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or(name);
        let default = format!("{stem}.{ext}");
        let target = match std::env::var_os("RMP_SPIKE_OUTPUT_DIR") {
            Some(dir) => Some(PathBuf::from(dir).join(&default)),
            None => rfd::FileDialog::new().set_title(title).set_file_name(&default).add_filter(ext, &[ext]).save_file(),
        };
        let Some(path) = target else { return };
        let result = self.engine.as_mut().map(|e| e.menu_action(key, &path.to_string_lossy())).unwrap_or_default();
        if !result.is_empty() {
            self.note(title, result);
        } else if key == "record" {
            self.recording = true;
        }
    }

    /// openFolder: every supported file below the folder, sorted like Finder, then accept.
    fn open_folder(&mut self) {
        let picked = if self.pick.is_empty() { rfd::FileDialog::new().pick_folder() } else { Some(self.pick.remove(0)) };
        let Some(root) = picked else { return };
        if let Some(e) = self.engine.as_mut() {
            e.prepare();
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.folder_rx = Some(rx);
        std::thread::spawn(move || {
            let mut files = Vec::new();
            walk(&root, &mut files);
            files.sort_by(|a, b| natural_cmp(a, b));
            let _ = tx.send(files);
        });
    }

    /// openCapture's device list: the screen, cameras and microphones.
    fn open_capture(&mut self) {
        let mut devices = vec![("Screen".to_string(), "screen://".to_string())];
        #[cfg(target_os = "macos")]
        unsafe {
            use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio, AVMediaTypeVideo};
            for (ty, scheme) in [(AVMediaTypeVideo, "avcapture://"), (AVMediaTypeAudio, "avaudiocapture://")] {
                let Some(ty) = ty else { continue };
                #[allow(deprecated)]
                let list = AVCaptureDevice::devicesWithMediaType(ty);
                for d in list.iter() {
                    devices.push((d.localizedName().to_string(), format!("{scheme}{}", d.uniqueID())));
                }
            }
        }
        #[cfg(windows)]
        {
            // DirectShow chooses its default camera and microphone.
            devices.push(("Default camera".to_string(), "dshow://".to_string()));
            devices.push(("Default microphone".to_string(), "dshow:// :dshow-vdev=none".to_string()));
        }
        self.dialogs.push(Dialog::Capture { devices, selected: 0 });
    }

    /// The Play button of the capture dialog: the screen opens directly; devices ask for access first.
    pub fn capture_chosen(&mut self, source: String) {
        if source.starts_with("screen:") || cfg!(windows) {
            self.open_location(&source);
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.capture_rx = Some(rx);
            let audio = source.starts_with("avaudio");
            unsafe {
                use objc2_av_foundation::{AVCaptureDevice, AVMediaTypeAudio, AVMediaTypeVideo};
                let ty = if audio { AVMediaTypeAudio } else { AVMediaTypeVideo };
                let Some(ty) = ty else { return };
                let tx = std::sync::Mutex::new(Some(tx));
                let src = source.clone();
                let block = block2::RcBlock::new(move |granted: objc2::runtime::Bool| {
                    if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
                        let _ = tx.send((granted.as_bool(), src.clone()));
                    }
                });
                AVCaptureDevice::requestAccessForMediaType_completionHandler(ty, &block);
            }
        }
    }

    /// effectsPanel Apply: equalizer preset, then each field, stopping at the first engine error.
    pub fn apply_effects(&mut self, sync: bool, eq: Option<String>, fields: Vec<(String, String)>) {
        let title = if sync { "Track Synchronization" } else { "Effects and Filters" };
        if let Some(id) = eq {
            let r = self.engine.as_mut().map(|e| e.menu_action("equalizer", &id)).unwrap_or_default();
            if !r.is_empty() {
                self.note(title, r);
                return;
            }
            self.effect_values.insert("equalizer".into(), id);
        }
        for (key, value) in fields {
            let r = self.engine.as_mut().map(|e| e.menu_action(&key, &value)).unwrap_or_default();
            if !r.is_empty() {
                self.note(title, r);
                return;
            }
            self.effect_values.insert(key, value);
        }
    }

    /// The Jump to Specific Time… answer: seconds, or h:m:s / m:s.
    pub fn jump_to_time(&mut self, text: &str, title: &str) {
        let mut value = text.trim().to_string();
        let parts: Vec<&str> = value.split(':').collect();
        if parts.len() > 1 {
            let mut t = 0.0f64;
            for c in parts {
                match c.trim().parse::<f64>() {
                    Ok(v) if v >= 0.0 && !c.trim().is_empty() => t = t * 60.0 + v,
                    _ => {
                        self.note(title, "Enter a valid playback time.");
                        return;
                    }
                }
            }
            value = format_number(t);
        }
        let r = self.engine.as_mut().map(|e| e.menu_action("time", &value)).unwrap_or_default();
        if !r.is_empty() {
            self.note(title, r);
        }
    }

    pub fn stream_out(&mut self, target: &str, title: &str) {
        if target.is_empty() {
            return;
        }
        let r = self.engine.as_mut().map(|e| e.menu_action("stream", target)).unwrap_or_default();
        if !r.is_empty() {
            self.note(title, r);
        }
    }

    /// The VLsub dialog's "Search in Browser".
    pub fn subtitle_search(&mut self) {
        let result = self.engine.as_mut().map(|e| e.menu_action("subtitle-search", "")).unwrap_or_default();
        if result.starts_with("https://") {
            appkit::open_url(&result);
        } else {
            self.note("Subtitles", result);
        }
    }

    /// Preferences / Customize Interface Apply.
    pub fn apply_configure(&mut self, advanced: bool, status: bool, docked: bool, startup_volume: Option<i32>) {
        self.store.advanced = advanced;
        self.store.status_bar = status;
        self.store.undocked_playlist = !docked;
        if let Some(v) = startup_volume {
            self.store.startup_volume = Some(v);
            self.set_volume(v);
        }
        self.store.save();
    }

    /// runMenuCommand.
    pub fn run_command(&mut self, key: &str, value: String, title: &str, ctx: &egui::Context) {
        match key {
            "folder" => self.open_folder(),
            "disc" => {
                let pick = if self.pick.is_empty() { rfd::FileDialog::new().set_title("Open DVD Disc or DVD Folder").set_directory("/Volumes").pick_folder() } else { Some(self.pick.remove(0)) };
                if let Some(dir) = pick {
                    let mut path = percent_encode_path(&dir.to_string_lossy());
                    if !path.ends_with('/') {
                        path.push('/');
                    }
                    self.accept(vec![(format!("dvdnav://{path}"), true)]);
                }
            }
            "capture" => self.open_capture(),
            "clipboard" => match appkit::pasteboard_location() {
                Some(s) if Path::new(&s).exists() => self.accept(vec![(s, false)]),
                Some(s) => self.open_location(&s),
                None => self.note(title, "Copy a file path or media URL first."),
            },
            "recent" => self.open_location(&value),
            "clear-recent" => self.store.clear_recent(),
            "quit-end" => self.quit_at_end = !self.quit_at_end,
            "desktop-shortcut" => {
                let on = !self.store.desktop_shortcut;
                self.store.desktop_shortcut = on;
                self.store.save();
                appkit::set_desktop_shortcut(on);
            }
            "record" if self.recording => {
                let r = self.engine.as_mut().map(|e| e.menu_action("record", "stop")).unwrap_or_default();
                if !r.is_empty() {
                    self.note("Record", r);
                } else {
                    self.recording = false;
                }
            }
            "record" => self.pick_output("record", title, "ts"),
            "convert" => self.pick_output("convert", title, "mp4"),
            "snapshot" => self.pick_output("snapshot", title, "png"),
            "stream-out" => self.dialogs.push(Dialog::Ask {
                title: title.into(),
                detail: "Stream the current media to a UDP receiver.".into(),
                value: "udp://127.0.0.1:1234".into(),
                placeholder: String::new(),
                purpose: AskPurpose::StreamOut,
            }),
            "time" => {
                let now = self.engine.as_ref().map_or(0, |e| e.time());
                self.dialogs.push(Dialog::Ask {
                    title: title.into(),
                    detail: "Enter elapsed seconds or a time such as 1:30.".into(),
                    value: metadata::stamp(now),
                    placeholder: String::new(),
                    purpose: AskPurpose::Time,
                });
            }
            "volume-step" => {
                let step: i32 = value.parse().unwrap_or(0);
                self.set_volume(self.volume + step);
            }
            "wallpaper" => self.toggle_wallpaper(),
            "subtitle-file" => {
                let pick = if self.pick.is_empty() { rfd::FileDialog::new().set_title("Add Subtitle File").add_filter("Subtitles", SUBTITLE_EXTENSIONS).pick_file() } else { Some(self.pick.remove(0)) };
                if let Some(p) = pick {
                    self.engine_note("subtitle-file", &p.to_string_lossy(), title);
                }
            }
            "effects" | "sync" => {
                let sync = key == "sync";
                let eq_rows = if sync { Vec::new() } else { self.engine_options("equalizer") };
                let eq_selected = self.effect_values.get("equalizer").and_then(|v| eq_rows.iter().position(|(id, _)| id == v)).or(if eq_rows.is_empty() { None } else { Some(0) });
                let keys: &[(&str, &str, &str)] = if sync {
                    &[("audio-delay", "Audio delay (seconds)", "0"), ("subtitle-delay", "Subtitle delay (seconds)", "0")]
                } else {
                    &[("brightness", "Brightness (0–2)", "1"), ("contrast", "Contrast (0–2)", "1")]
                };
                let fields = keys
                    .iter()
                    .map(|(k, t, d)| (k.to_string(), t.to_string(), self.effect_values.get(*k).cloned().unwrap_or_else(|| d.to_string())))
                    .collect();
                self.dialogs.push(Dialog::Effects { sync, eq_rows, eq_selected, fields });
            }
            "customize" | "preferences" => {
                let preferences = key == "preferences";
                self.dialogs.push(Dialog::Configure {
                    preferences,
                    advanced: self.store.advanced,
                    status: self.store.status_bar,
                    docked: !self.store.undocked_playlist,
                    volume: self.volume,
                });
            }
            "terminal" => self.pick_theme(Skin::Linux),
            "top" => {
                self.always_top = !self.always_top;
                if self.wallpaper.is_none() {
                    if let Some(w) = self.surfaces.as_ref().and_then(|s| s.window()) {
                        appkit::set_floating(&w, self.always_top);
                    }
                }
            }
            "minimal" => self.minimal = !self.minimal,
            "advanced" => {
                self.store.advanced = !self.store.advanced;
                self.store.save();
            }
            "status" => {
                self.store.status_bar = !self.store.status_bar;
                self.store.save();
            }
            "docked" => {
                self.store.undocked_playlist = !self.store.undocked_playlist;
                self.store.save();
            }
            "playlist" => {
                if !self.store.undocked_playlist && self.skin.has_media_layout() {
                    self.media_page = if self.media_page == MediaPage::Playlist { MediaPage::NowPlaying } else { MediaPage::Playlist };
                    self.search.clear();
                } else {
                    self.playlist_shown = !self.playlist_shown;
                }
            }
            "vlsub" => self.dialogs.push(Dialog::Vlsub),
            "help" => appkit::open_url(README_URL),
            "fit" => self.engine_note("zoom", "0", title),
            _ => {
                let r = self.engine.as_mut().map(|e| e.menu_action(key, &value)).unwrap_or_default();
                if key == "output-stop" && r.is_empty() {
                    self.recording = false;
                }
                if !r.is_empty() {
                    self.note(title, r);
                }
            }
        }
        let _ = ctx;
    }

    fn engine_note(&mut self, key: &str, value: &str, title: &str) {
        let r = self.engine.as_mut().map(|e| e.menu_action(key, value)).unwrap_or_default();
        if !r.is_empty() {
            self.note(title, r);
        }
    }
}

fn has_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(i) if i > 0 => s[..i].chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') && s.as_bytes()[0].is_ascii_alphabetic(),
        _ => false,
    }
}

fn format_number(t: f64) -> String {
    if t.fract() == 0.0 {
        format!("{}", t as i64)
    } else {
        format!("{t}")
    }
}

fn percent_encode_path(p: &str) -> String {
    let mut out = String::new();
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// NSDirectoryEnumerationSkipsHiddenFiles | SkipsPackageDescendants, keeping supported media.
fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if is_package(&path) {
                continue;
            }
            walk(&path, out);
        } else if ft.is_file() {
            let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if MEDIA_EXTENSIONS.contains(&ext.as_str()) {
                out.push(path.to_string_lossy().into_owned());
            }
        }
    }
}

/// A directory macOS presents as a single file.
fn is_package(path: &Path) -> bool {
    let Some(ext) = path.extension().map(|e| e.to_string_lossy().to_lowercase()) else { return false };
    matches!(
        ext.as_str(),
        "app" | "bundle" | "framework" | "plugin" | "kext" | "photoslibrary" | "musiclibrary" | "tvlibrary" | "imovielibrary" | "fcpbundle"
            | "rtfd" | "pages" | "numbers" | "key" | "xcodeproj" | "xcworkspace" | "playground" | "dvdmedia" | "pkg" | "mpkg" | "lproj"
    )
}

/// localizedStandardCompare: case-insensitive, with digit runs compared as numbers.
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut nx = String::new();
                while let Some(c) = ai.peek().copied().filter(|c| c.is_ascii_digit()) {
                    nx.push(c);
                    ai.next();
                }
                let mut ny = String::new();
                while let Some(c) = bi.peek().copied().filter(|c| c.is_ascii_digit()) {
                    ny.push(c);
                    bi.next();
                }
                let (vx, vy) = (nx.trim_start_matches('0'), ny.trim_start_matches('0'));
                let ord = vx.len().cmp(&vy.len()).then(vx.cmp(vy));
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}
