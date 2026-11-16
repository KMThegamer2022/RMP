//! RMP with an egui interface, driving the existing Rust engine (repo-root src/lib.rs) on macOS.
//! Behaviour follows the shipping AppKit app: ui.m, mac-menus.h, mac-library.h, mac-fullscreen.h,
//! mac-terminal.h, mac-theme.h and updates/mac.h. The shipping app is not changed.

mod appkit;
mod commands;
mod dialogs;
mod engine;
mod layout;
mod media;
mod menus;
mod metadata;
mod script;
mod store;
mod terminal;
mod theme;
mod updates;
mod widgets;

use eframe::egui;
use engine::Engine;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use store::Store;
use theme::Skin;

const CONE_PNG: &[u8] = include_bytes!("../../assets/vlc-cone.png");
const LOGO_PNG: &[u8] = include_bytes!("../../assets/RMP.png");
const LINK_TARGET: &str = "https://www.youtube.com/watch?v=yEh_4GqXqFc";
/// egui lays SF out about 12% narrower than AppKit does, so explicit font sizes are scaled to match
/// the shipping text widths (measured on "Previous", the clock and the 18 pt item name).
pub fn fs(size: f32) -> f32 {
    size * 1.12
}

/// How long the fullscreen transport stays up after the mouse last moved (refreshFloating).
const FLOAT_HIDE_SECS: f64 = 2.5;

/// Pages of the XP and Windows 7 media layout (mediaPage in mac-library.h).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MediaPage {
    NowPlaying,
    Library,
    Playlist,
}

impl MediaPage {
    pub fn index(self) -> usize {
        match self {
            MediaPage::NowPlaying => 0,
            MediaPage::Library => 1,
            MediaPage::Playlist => 2,
        }
    }
}

/// Textures the UI draws: the cone, the RMP logo, SF Symbols and cover art.
#[derive(Default)]
pub struct Textures {
    pub cone: Option<egui::TextureHandle>,
    pub logo: Option<egui::TextureHandle>,
    pub symbols: HashMap<String, Option<egui::TextureHandle>>,
    pub covers: HashMap<String, Option<egui::TextureHandle>>,
}

impl Textures {
    pub fn symbol(&mut self, ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.symbols.get(name) {
            return t.clone();
        }
        // Template symbols render black; make them white so Image::tint colours them.
        let t = appkit::symbol_image(name, 16.0).map(|mut img| {
            for px in img.pixels.iter_mut() {
                *px = egui::Color32::from_white_alpha(px.a());
            }
            ctx.load_texture(format!("sf-{name}"), img, egui::TextureOptions::LINEAR)
        });
        self.symbols.insert(name.to_string(), t.clone());
        t
    }
    /// A symbol rendered at 100 pt for the queue art.
    pub fn symbol_large(&mut self, ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
        let key = format!("{name}@large");
        if let Some(t) = self.symbols.get(&key) {
            return t.clone();
        }
        let t = appkit::symbol_image(name, 100.0).map(|mut img| {
            for px in img.pixels.iter_mut() {
                *px = egui::Color32::from_white_alpha(px.a());
            }
            ctx.load_texture(format!("sf-{key}"), img, egui::TextureOptions::LINEAR)
        });
        self.symbols.insert(key, t.clone());
        t
    }
    pub fn cover(&mut self, ctx: &egui::Context, path: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
        if let Some(t) = self.covers.get(path) {
            return t.clone();
        }
        let t = image::load_from_memory(bytes).ok().map(|img| {
            let img = img.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            ctx.load_texture(format!("cover-{path}"), egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR)
        });
        self.covers.insert(path.to_string(), t.clone());
        t
    }
}

fn png_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR))
}

pub struct App {
    pub engine: Option<Engine>,
    pub surfaces: Option<appkit::Surfaces>,
    pub store: Store,
    pub skin: Skin,
    applied_skin: Option<Skin>,
    pub tex: Textures,
    pub status: String,
    pub volume: i32,
    pub scrub: Option<i64>,
    /// refresh: the state seen on the previous tick (-1 right after a launch).
    previous: i32,
    pub quit_at_end: bool,
    pub recording: bool,
    pub wallpaper: Option<appkit::WallpaperRestore>,
    pub always_top: bool,
    pub minimal: bool,
    pub playlist_shown: bool,
    pub repeat_one: bool,
    pub shuffle: bool,
    pub pending_video_switch: bool,
    pub media_page: MediaPage,
    pub search: String,
    pub search_cursor: usize,
    pub selected_row: Option<usize>,
    pub queue_selected: Option<usize>,
    pub effect_values: HashMap<String, String>,
    pub dialogs: Vec<dialogs::Dialog>,
    pub in_full: bool,
    presentation_full: Option<bool>,
    last_pointer: (f64, f64),
    last_activity: Instant,
    pub float_visible: bool,
    pub title: String,
    title_sent: String,
    pub name: String,
    pub term: terminal::Terminal,
    pub meta: metadata::Loader,
    pub update_rx: Option<(Receiver<updates::Outcome>, bool)>,
    pub pending_edit: Option<menus::Action>,
    pub folder_rx: Option<Receiver<Vec<String>>>,
    pub capture_rx: Option<Receiver<(bool, String)>>,
    pub quit_requested: bool,
    pub force_button: std::cell::Cell<Option<usize>>,
    /// Paths the test script supplies in place of the next file picker(s).
    pub pick: Vec<std::path::PathBuf>,
    pub script: Option<script::Script>,
    app_active: bool,
    started: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let store = Store::load();
        let skin = store.theme;
        let mut tex = Textures::default();
        tex.cone = png_texture(&cc.egui_ctx, "vlc-cone", CONE_PNG);
        tex.logo = png_texture(&cc.egui_ctx, "rmp-logo", LOGO_PNG);
        App {
            engine: None,
            surfaces: None,
            store,
            skin,
            applied_skin: None,
            tex,
            status: "Ready".into(),
            volume: 80,
            scrub: None,
            previous: 0,
            quit_at_end: false,
            recording: false,
            wallpaper: None,
            always_top: false,
            minimal: false,
            playlist_shown: false,
            repeat_one: false,
            shuffle: false,
            pending_video_switch: false,
            // applyTheme: Windows 7 opens on the library, every other skin on Now Playing.
            media_page: if skin == Skin::Seven { MediaPage::Library } else { MediaPage::NowPlaying },
            search: String::new(),
            search_cursor: 0,
            selected_row: None,
            queue_selected: None,
            effect_values: HashMap::new(),
            dialogs: Vec::new(),
            in_full: false,
            presentation_full: None,
            last_pointer: (0.0, 0.0),
            last_activity: Instant::now(),
            float_visible: false,
            title: "RMP".into(),
            title_sent: String::new(),
            name: "RMP".into(),
            term: terminal::Terminal::new(),
            meta: metadata::Loader::new(),
            update_rx: None,
            pending_edit: None,
            folder_rx: None,
            capture_rx: None,
            quit_requested: false,
            force_button: std::cell::Cell::new(None),
            pick: Vec::new(),
            script: script::Script::load(),
            app_active: true,
            started: false,
        }
    }

    pub fn has_engine(&self) -> bool {
        self.engine.is_some()
    }
    pub fn state(&self) -> i32 {
        self.engine.as_ref().map_or(0, |e| e.state())
    }
    pub fn count(&self) -> usize {
        self.engine.as_ref().map_or(0, |e| e.count())
    }
    pub fn index(&self) -> isize {
        self.engine.as_ref().map_or(-1, |e| e.index())
    }

    /// The layout is "regular" when nothing hides the chrome (updateViewControls).
    pub fn regular(&self) -> bool {
        !self.in_full && !self.minimal && self.wallpaper.is_none() && self.skin != Skin::Linux
    }
    /// arrangeChrome: bare = inFull || terminal || wallpaper || minimalInterface.
    pub fn bare(&self) -> bool {
        self.in_full || self.skin == Skin::Linux || self.wallpaper.is_some() || self.minimal
    }
    /// layoutMediaChrome: active = hasMediaLayout && !inFull && !minimalInterface && !wallpaper.
    pub fn media_active(&self) -> bool {
        self.skin.has_media_layout() && !self.in_full && !self.minimal && self.wallpaper.is_none()
    }

    // ---- Playback (ui.m) ------------------------------------------------------------------------

    /// accept: queue the sources and play the first new one.
    pub fn accept(&mut self, sources: Vec<(String, bool)>) {
        let Some(e) = self.engine.as_mut() else { return };
        let start = e.count();
        let n = sources.len();
        for (s, network) in sources {
            e.add(&s, network);
        }
        if n > 0 {
            self.launch(start);
        }
    }

    /// launch: show the item, remember it, play it.
    pub fn launch(&mut self, i: usize) {
        if i >= self.count() {
            return;
        }
        self.show_playing_item(i);
        let item = self.engine.as_ref().and_then(|e| e.item(i)).unwrap_or_default();
        self.store.remember_recent(&item);
        let ok = self.engine.as_mut().is_some_and(|e| e.play(i));
        self.status = if ok { "Opening…".into() } else { "Could not open media".into() };
    }

    /// showPlayingItem: title, name, and the Windows 7 switch back to the video once it starts.
    pub fn show_playing_item(&mut self, i: usize) {
        if i >= self.count() {
            return;
        }
        self.previous = -1;
        self.name = self.engine.as_ref().map(|e| e.display(i)).unwrap_or_default();
        self.title = format!("RMP — {}", self.name);
        self.pending_video_switch = self.skin == Skin::Seven && self.media_page != MediaPage::NowPlaying;
    }

    /// toggle: with an empty queue, Play opens the file picker.
    pub fn toggle(&mut self) {
        if self.count() == 0 {
            self.open_files(true);
        } else if let Some(e) = self.engine.as_mut() {
            e.toggle();
        }
    }

    pub fn prev(&mut self) {
        let i = self.index();
        if i > 0 {
            self.launch(i as usize - 1);
        } else if self.count() > 0 {
            self.launch(0);
        }
    }

    pub fn next(&mut self) {
        let n = self.count();
        let i = self.index();
        if self.shuffle && n > 1 {
            let r = random_below(n - 1);
            self.launch(((i + 1) as usize + r) % n);
        } else {
            self.launch((i + 1) as usize);
        }
    }

    pub fn stop(&mut self) {
        if let Some(e) = self.engine.as_mut() {
            e.stop();
        }
    }

    pub fn set_volume(&mut self, v: i32) {
        self.volume = v.clamp(0, 100);
        let v = self.volume;
        if let Some(e) = self.engine.as_mut() {
            e.set_volume(v);
        }
    }

    pub fn seek(&mut self, ms: i64) {
        if let Some(e) = self.engine.as_mut() {
            e.seek(ms);
        }
    }

    /// full: leave wallpaper mode first, then toggle fullscreen.
    pub fn full(&mut self, ctx: &egui::Context) {
        if self.wallpaper.is_some() {
            self.toggle_wallpaper();
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!self.in_full));
    }

    /// pickTheme: remember the theme before Linux so `theme back` can return to it.
    pub fn pick_theme(&mut self, skin: Skin) {
        if skin == Skin::Linux && self.skin != Skin::Linux {
            self.store.previous_theme = Some(self.skin);
        }
        self.apply_theme(skin);
        self.store.theme = skin;
        self.store.save();
    }

    /// applyTheme's state changes; the window chrome follows in `apply_chrome`.
    fn apply_theme(&mut self, skin: Skin) {
        let old = self.skin;
        self.skin = skin;
        if old != skin {
            self.pending_video_switch = false;
            self.media_page = if skin == Skin::Seven { MediaPage::Library } else { MediaPage::NowPlaying };
            self.search.clear();
            self.selected_row = None;
        }
        if skin == Skin::Linux {
            self.term.hidden = false; // showTerminal
        }
    }

    pub fn note(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.dialogs.push(dialogs::Dialog::Note { title: title.into(), detail: detail.into() });
    }

    /// The tick from refresh: (every 0.2 s in the shipping app; every frame here).
    fn refresh(&mut self, ctx: &egui::Context) {
        let Some(e) = self.engine.as_ref() else { return };
        let state = e.state();
        let has_video = e.has_video();
        let index = e.index();
        let count = e.count();
        if self.pending_video_switch && has_video {
            self.pending_video_switch = false;
            self.media_page = MediaPage::NowPlaying;
        }
        if state >= 6 {
            self.pending_video_switch = false;
        }
        if state == 6 && self.previous != 6 && self.repeat_one {
            self.launch(index.max(0) as usize);
            return;
        }
        if state == 6 && self.previous != 6 && (self.shuffle || index + 1 < count as isize) {
            self.next();
            return;
        }
        if state == 6 && self.previous != 6 && self.quit_at_end {
            self.quit_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.previous = state;
        const STATES: [&str; 8] = ["Ready", "Opening…", "Buffering…", "Playing", "Paused", "Stopped", "Finished", "Playback error"];
        self.status = STATES[state.clamp(0, 7) as usize].to_string();
    }

    /// refreshFloating: the fullscreen transport shows when the mouse moves or rests on it, and hides
    /// 2.5 s after the last movement.
    fn refresh_floating(&mut self, pointer_on_panel: bool) {
        let point = appkit::mouse_location();
        let now = Instant::now();
        if point != self.last_pointer || (self.float_visible && pointer_on_panel) {
            self.last_activity = now;
            self.float_visible = true;
        } else if now.duration_since(self.last_activity).as_secs_f64() > FLOAT_HIDE_SECS {
            self.float_visible = false;
        }
        self.last_pointer = point;
    }

    /// Window chrome for the skin (applyTheme): appearance, background, glass, title text colour.
    fn apply_chrome(&mut self, ctx: &egui::Context) {
        if self.applied_skin == Some(self.skin) {
            return;
        }
        let Some(window) = self.surfaces.as_ref().and_then(|s| s.window()) else { return };
        let th = self.skin.theme();
        appkit::apply_window_chrome(&window, self.skin, self.translucent());
        ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if th.dark { egui::SystemTheme::Dark } else { egui::SystemTheme::Light }));
        ctx.set_visuals(if th.dark { egui::Visuals::dark() } else { egui::Visuals::light() });
        if self.always_top {
            appkit::set_floating(&window, true);
        }
        self.applied_skin = Some(self.skin);
    }

    /// Windows 7 glass unless the user reduced transparency.
    pub fn translucent(&self) -> bool {
        self.skin == Skin::Seven && !appkit::reduce_transparency()
    }

    pub fn toggle_wallpaper(&mut self) {
        let Some(window) = self.surfaces.as_ref().and_then(|s| s.window()) else { return };
        if let Some(restore) = self.wallpaper.take() {
            appkit::leave_wallpaper(&window, &restore, self.always_top);
        } else {
            if self.in_full {
                self.note("Set as Wallpaper", "Leave fullscreen to use the playback window as your desktop wallpaper.");
                return;
            }
            self.wallpaper = Some(appkit::enter_wallpaper(&window));
        }
    }

    fn start(&mut self, ctx: &egui::Context, frame: &eframe::Frame) {
        self.started = true;
        appkit::set_app_icon(LOGO_PNG);
        self.surfaces = appkit::Surfaces::attach(frame);
        let view = self.surfaces.as_ref().map_or(std::ptr::null_mut(), |s| s.video_handle());
        self.engine = Engine::create(view);
        if self.engine.is_none() {
            // ui.m: "RMP could not load its playback engine." then quit.
            self.note("RMP could not load its playback engine.", "");
            return;
        }
        // StartupVolume when the user set one in Preferences; the slider default is 80.
        let v = self.store.startup_volume.unwrap_or(80);
        self.set_volume(v);
        let args: Vec<String> = std::env::args().skip(1).collect();
        if !args.is_empty() {
            let sources = args
                .into_iter()
                .map(|a| {
                    let file = std::path::Path::new(&a).exists();
                    let path = if file { std::fs::canonicalize(&a).map(|p| p.to_string_lossy().into_owned()).unwrap_or(a) } else { a };
                    (path, !file)
                })
                .collect();
            self.accept(sources);
        }
        if std::env::var_os("RMP_SPIKE_NO_UPDATE_CHECK").is_none() {
            self.update_rx = Some((updates::check(), false)); // checkMacUpdates(NO)
        }
        let _ = ctx;
    }

    fn poll_background(&mut self) {
        if let Some((rx, explicit)) = &self.update_rx {
            if let Ok(outcome) = rx.try_recv() {
                let explicit = *explicit;
                self.update_rx = None;
                match outcome {
                    updates::Outcome::Available { version, notes, .. } => {
                        self.dialogs.push(dialogs::Dialog::UpdateOffer { version, notes });
                    }
                    updates::Outcome::UpToDate { current } if explicit => {
                        self.note("RMP is up to date", format!("You are running RMP {current}."));
                    }
                    updates::Outcome::Failed { reason } if explicit => {
                        self.note("Could not check for updates", reason);
                    }
                    _ => {}
                }
            }
        }
        if let Some(rx) = &self.folder_rx {
            if let Ok(files) = rx.try_recv() {
                self.folder_rx = None;
                if files.is_empty() {
                    self.note("Open Folder", "No supported media files were found in this folder.");
                } else {
                    self.accept(files.into_iter().map(|f| (f, false)).collect());
                }
            }
        }
        if let Some(rx) = &self.capture_rx {
            if let Ok((granted, source)) = rx.try_recv() {
                self.capture_rx = None;
                if granted {
                    self.open_location(&source);
                } else {
                    self.note("Capture Device", "Allow camera or microphone access for RMP in System Settings to use this device.");
                }
            }
        }
    }

    /// Keyboard equivalents from the menus. Modifiers come from each key event: the frame's
    /// modifier state can lag the key on macOS, which dropped Command shortcuts.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let presses: Vec<(egui::Key, bool, bool, bool)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, modifiers.shift, modifiers.command, modifiers.ctrl)),
                    _ => None,
                })
                .collect()
        });
        let hit = |key: egui::Key, shift: bool, cmd: bool, ctrl: bool| presses.iter().any(|p| p.0 == key && p.1 == shift && p.2 == cmd && p.3 == ctrl);
        if hit(egui::Key::F1, false, false, false) {
            appkit::open_url(LINK_TARGET);
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if hit(egui::Key::Q, false, true, false) {
            self.quit_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if typing || !self.dialogs.is_empty() {
            return;
        }
        use menus::{Action, Picked};
        let mut run: Option<Picked> = None;
        if hit(egui::Key::Space, false, false, false) {
            run = Some(Picked::Action(Action::PlayPause));
        } else if hit(egui::Key::O, false, true, false) {
            run = Some(Picked::Action(Action::OpenSingle));
        } else if hit(egui::Key::O, true, true, false) {
            run = Some(Picked::Action(Action::OpenMultiple));
        } else if hit(egui::Key::U, false, true, false) {
            run = Some(Picked::Action(Action::OpenStream));
        } else if hit(egui::Key::T, false, true, false) {
            if self.menu_enabled("time") {
                run = Some(Picked::Command { key: "time", value: String::new(), title: "Jump to Specific Time…".into() });
            }
        } else if hit(egui::Key::E, false, true, false) {
            run = Some(Picked::Command { key: "effects", value: String::new(), title: "Effects and Filters…".into() });
        } else if hit(egui::Key::L, false, true, false) {
            run = Some(Picked::Command { key: "playlist", value: String::new(), title: "Playlist".into() });
        } else if hit(egui::Key::F, false, true, true) {
            run = Some(Picked::Action(Action::Fullscreen));
        } else if hit(egui::Key::OpenBracket, false, false, false) {
            run = Some(Picked::Action(Action::Previous));
        } else if hit(egui::Key::CloseBracket, false, false, false) {
            run = Some(Picked::Action(Action::Next));
        } else if self.wallpaper.is_some() && hit(egui::Key::Escape, false, false, false) {
            self.toggle_wallpaper();
        }
        if let Some(p) = run {
            self.run_picked(p, ctx);
        }
    }
}

/// arc4random_uniform stand-in for shuffle.
fn random_below(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut x = (nanos as u64) ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    (x % n as u64) as usize
}

impl eframe::App for App {
    /// Fully clear, so the video layered beneath shows through the screen rectangle unchanged.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    /// Edit → Cut/Copy/Paste/Select All become the matching egui events for the focused text field.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(action) = self.pending_edit.take() {
            use menus::Action;
            match action {
                Action::Cut => raw.events.push(egui::Event::Cut),
                Action::Copy => raw.events.push(egui::Event::Copy),
                Action::Paste => {
                    if let Some(text) = appkit::pasteboard_location() {
                        raw.events.push(egui::Event::Paste(text));
                    }
                }
                Action::SelectAll => raw.events.push(egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }),
                _ => {}
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.start(&ctx, frame);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
        if self.title != self.title_sent {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title.clone()));
            self.title_sent = self.title.clone();
        }
        // Fullscreen state, and the presentation that hides the macOS menu bar while it lasts.
        let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if full != self.in_full {
            self.in_full = full;
            if full {
                self.last_activity = Instant::now();
                self.float_visible = self.skin != Skin::Linux;
            }
        }
        if self.presentation_full != Some(full) {
            self.presentation_full = Some(full);
            appkit::set_presentation(full);
        }
        // Double-click on the screen toggles fullscreen (the NSEvent monitor in applicationDidFinishLaunching).
        if ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) && !ctx.egui_wants_pointer_input() {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                let g = layout::geometry(self, ctx.content_rect());
                if g.screen_visible && g.screen.contains(pos) && self.has_engine() {
                    self.full(&ctx);
                }
            }
        }
        // Wallpaper mode ends when the app is activated again (applicationShouldHandleReopen).
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            let active = objc2_app_kit::NSApplication::sharedApplication(mtm).isActive();
            if active && !self.app_active && self.wallpaper.is_some() {
                self.toggle_wallpaper();
            }
            self.app_active = active;
        }
        self.apply_chrome(&ctx);
        self.poll_background();
        self.refresh(&ctx);
        self.shortcuts(&ctx);
        self.run_script(&ctx);
        layout::draw(self, ui, &ctx);
        dialogs::draw(self, &ctx);
        if self.skin == Skin::Linux && !self.term.hidden {
            self.terminal_window(&ctx);
        }
        if self.playlist_shown && !self.store.undocked_playlist == false && self.regular() {
            // undocked playlist panel: drawn by layout::playlist_panel
        }
        if self.in_full {
            let on_panel = ctx.memory(|m| m.area_rect(egui::Id::new("floating-transport"))).is_some_and(|r| {
                ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| r.contains(p))
            });
            self.refresh_floating(on_panel);
        }
    }

    fn on_exit(&mut self) {
        appkit::set_presentation(false);
        self.engine = None; // ap_destroy (applicationWillTerminate)
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut load = |name: &str, path: &str, fonts: &mut egui::FontDefinitions| {
        std::fs::read(path).ok().map(|bytes| {
            fonts.font_data.insert(name.into(), egui::FontData::from_owned(bytes).into());
        })
    };
    if load("sf", "/System/Library/Fonts/SFNS.ttf", &mut fonts).is_some() {
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "sf".into());
    }
    if load("sf-mono", "/System/Library/Fonts/SFNSMono.ttf", &mut fonts).is_some() {
        fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, "sf-mono".into());
    }
    if load("symbols", "/System/Library/Fonts/Apple Symbols.ttf", &mut fonts).is_some() {
        fonts.families.entry(egui::FontFamily::Proportional).or_default().push("symbols".into());
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result {
    // ui.m main(): VLC_PLUGIN_PATH points at the bundle's plugins. The spike runs from target/, so it
    // uses the plugins inside VLC.app unless the caller set the variable.
    let bundled = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("plugins"))).filter(|d| d.is_dir());
    if let (None, Some(dir)) = (std::env::var_os("VLC_PLUGIN_PATH"), bundled) {
        std::env::set_var("VLC_PLUGIN_PATH", dir);
    }
    if std::env::var_os("VLC_PLUGIN_PATH").is_none() {
        let app = std::env::var("VLC_APP").unwrap_or_else(|_| "/Applications/VLC.app".into());
        std::env::set_var("VLC_PLUGIN_PATH", format!("{app}/Contents/MacOS/plugins"));
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 570.0])
            .with_min_inner_size([720.0, 380.0])
            .with_title("RMP")
            .with_transparent(true),
        centered: true,
        ..Default::default()
    };
    eframe::run_native("RMP", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
