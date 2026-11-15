//! egui spike: drives the existing Rust engine (repo-root src/lib.rs, the `ap_*` API) from egui.
//! macOS only. Layouts and colours follow the shipping app (ui.m, themes.h). The shipping app is not changed.

use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSAppearance, NSAppearanceCustomization, NSColor, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_void, CString};
use std::path::PathBuf;

// Engine: repo-root src/lib.rs compiled as this crate's library (see Cargo.toml [lib]).
use egui_spike as engine;

mod menus;
use menus::Native;

/// Text prompts raised by menu items (Open Network Stream…, Jump to Specific Time…).
pub enum Prompt {
    Stream(String),
    Time(String),
}

const RECENT_MAX: usize = 20;

fn recent_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/RMP egui spike/recent.txt"))
}

fn load_recent() -> Vec<String> {
    recent_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        .unwrap_or_default()
}

const INSET: f32 = 16.0;
const ROW_GAP: f32 = 8.0;
const TIMELINE_H: f32 = 21.0;
const BUTTON_H: f32 = 30.0;
const BOTTOM_INSET: f32 = 12.0;
const CLOCK_W: f32 = 84.0;
const VOLUME_W: f32 = 85.0;
const CONE_SIZE: f32 = 120.0;
const CONE_PNG: &[u8] = include_bytes!("../../assets/vlc-cone.png");

fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// The shipping app's skins, in the View → Theme menu order (ui.m), with their themes.h indices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Skin {
    Vlc,
    RmpDefault,
    Modern,
    Seven,
    Xp,
    Mac,
    Linux,
}

const SKINS: [Skin; 7] = [Skin::Vlc, Skin::RmpDefault, Skin::Modern, Skin::Seven, Skin::Xp, Skin::Mac, Skin::Linux];

impl Skin {
    fn label(self) -> &'static str {
        match self {
            Skin::Vlc => "VLC",
            Skin::RmpDefault => "RMP Default",
            Skin::Modern => "Windows 10/11",
            Skin::Seven => "Windows 7",
            Skin::Xp => "Windows XP",
            Skin::Mac => "Mac",
            Skin::Linux => "Linux",
        }
    }
    /// Stable id for the saved preference file and the RMP_SPIKE_THEME override.
    fn slug(self) -> &'static str {
        match self {
            Skin::Vlc => "vlc",
            Skin::RmpDefault => "rmp-default",
            Skin::Modern => "win10",
            Skin::Seven => "win7",
            Skin::Xp => "xp",
            Skin::Mac => "mac",
            Skin::Linux => "linux",
        }
    }
    /// themes.h index (the shipping app's PlayerTheme value).
    fn index(self) -> i32 {
        match self {
            Skin::Modern => 0,
            Skin::Seven => 1,
            Skin::Xp => 2,
            Skin::Mac => 3,
            Skin::RmpDefault => 4,
            Skin::Vlc => 5,
            Skin::Linux => 6,
        }
    }
    fn from_slug(s: &str) -> Option<Skin> {
        SKINS.into_iter().find(|k| k.slug() == s.trim())
    }
    fn theme(self) -> Theme {
        // Values from themes.h. Rows: panel, edge, face, highlight, text, accent, radius.
        let (panel, edge, face, highlight, text, accent, radius, dark) = match self {
            Skin::Modern => (0x101010, 0x555555, 0x101010, 0x303030, 0xffffff, 0x399bdb, 0.0, false),
            Skin::Seven => (0xd2e3f1, 0x8198ae, 0xb5ccdf, 0xffffff, 0x123150, 0x2673b5, 6.0, false),
            Skin::Xp => (0xece9d8, 0x30537f, 0xe4dfcc, 0xfffef7, 0x171710, 0x245edb, 3.0, false),
            Skin::Mac => (0x303034, 0x696970, 0x525257, 0x525257, 0xffffff, 0x286bd1, 10.0, true),
            Skin::RmpDefault => (0x171a21, 0x414756, 0x292e39, 0x353b49, 0xf4f5f7, 0xff7848, 8.0, true),
            Skin::Vlc => (0xf2f2f2, 0xb4b4b4, 0xf8f8f8, 0xffffff, 0x535353, 0x57bb39, 2.0, false),
            Skin::Linux => (0x101611, 0x37483b, 0x142019, 0x213527, 0xd3e8d7, 0x83dc9b, 0.0, true),
        };
        Theme {
            panel: rgb(panel),
            edge: rgb(edge),
            face: rgb(face),
            highlight: rgb(highlight),
            text: rgb(text),
            accent: rgb(accent),
            radius,
            dark,
        }
    }
}

#[derive(Clone, Copy)]
struct Theme {
    panel: Color32,
    edge: Color32,
    face: Color32,
    highlight: Color32,
    text: Color32,
    accent: Color32,
    radius: f32,
    dark: bool,
}

/// Preference file: ~/Library/Application Support/RMP egui spike/theme.txt holds the skin slug.
fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/RMP egui spike/theme.txt"))
}

fn load_skin() -> Skin {
    if let Some(s) = std::env::var("RMP_SPIKE_THEME").ok().and_then(|s| Skin::from_slug(&s)) {
        return s; // Override for screenshot runs; not saved.
    }
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| Skin::from_slug(&s))
        .unwrap_or(Skin::Vlc) // ui.m registers VLC_THEME as the default.
}

fn save_skin(skin: Skin) {
    if std::env::var_os("RMP_SPIKE_THEME").is_some() {
        return;
    }
    if let Some(p) = config_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, skin.slug());
    }
}

struct App {
    player: *mut engine::Player,
    video_view: Option<Retained<NSView>>,
    /// winit's content view (egui draws into it); the video view sits behind it in the frame view.
    content_view: Option<Retained<NSView>>,
    cone: Option<egui::TextureHandle>,
    file: Option<PathBuf>,
    status: String,
    volume: i32,
    scrub: Option<i64>,
    last_state: i32,
    skin: Skin,
    applied_skin: Option<Skin>,
    title_cmd: Option<String>,
    recent: Vec<String>,
    prompt: Option<Prompt>,
    note: Option<(String, String)>,
}

#[derive(Clone, Copy, PartialEq)]
enum Icon {
    Play,
    Pause,
    Prev,
    Next,
    Stop,
    Fullscreen,
}

#[derive(Clone, Copy)]
enum Item {
    Play,
    Prev,
    Stop,
    Next,
    Fullscreen,
    Queue,
    Repeat,
    Shuffle,
    Spacer,
    VolumeLabel,
    Volume,
}

/// Order of the bottom row per skin, from applyTheme in ui.m.
fn bottom_items(skin: Skin) -> Vec<Item> {
    use Item::*;
    match skin {
        Skin::Seven => vec![Shuffle, Repeat, Stop, Prev, Play, Next, Volume, Fullscreen],
        Skin::Xp => vec![Play, Stop, Prev, Next, Volume, Repeat, Shuffle, Fullscreen],
        _ => vec![Play, Prev, Stop, Next, Fullscreen, Queue, Repeat, Shuffle, Spacer, VolumeLabel, Volume],
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let skin = load_skin();
        Self {
            player: std::ptr::null_mut(),
            video_view: None,
            content_view: None,
            cone: None,
            file: None,
            status: "Ready".into(),
            volume: 80,
            scrub: None,
            last_state: 0,
            skin,
            applied_skin: None,
            title_cmd: None,
            recent: load_recent(),
            prompt: None,
            note: None,
        }
    }

    fn player(&mut self) -> Option<&mut engine::Player> {
        // SAFETY: from ap_create; lives until Drop; UI thread only.
        unsafe { self.player.as_mut() }
    }

    fn state(&mut self) -> i32 {
        match self.player() {
            Some(p) => unsafe { engine::ap_state(p) },
            None => 0,
        }
    }

    fn open_file(&mut self, path: PathBuf) {
        let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) else {
            self.status = "Path contains a NUL byte".into();
            return;
        };
        let Some(p) = self.player() else { return };
        unsafe {
            engine::ap_stop(p);
            let index = engine::ap_count(p);
            engine::ap_add(p, c.as_ptr(), false);
            let ok = engine::ap_play(p, index);
            self.status = if ok { "Opening…".into() } else { "Could not open media".into() };
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        self.title_cmd = Some(format!("RMP — {name}"));
        self.file = Some(path);
    }

    fn toggle(&mut self) {
        let state = self.state();
        let Some(p) = self.player() else { return };
        if (1..=4).contains(&state) {
            unsafe { engine::ap_toggle(p) };
        } else if unsafe { engine::ap_count(p) } > 0 {
            let i = usize::try_from(unsafe { engine::ap_index(p) }).unwrap_or(0);
            unsafe { engine::ap_play(p, i) };
        }
    }

    fn step(&mut self, delta: isize) {
        let Some(p) = self.player() else { return };
        let count = unsafe { engine::ap_count(p) } as isize;
        let next = unsafe { engine::ap_index(p) } + delta;
        if next >= 0 && next < count {
            unsafe { engine::ap_play(p, next as usize) };
        }
    }

    fn set_volume(&mut self, v: i32) {
        self.volume = v.clamp(0, 100);
        let volume = self.volume;
        if let Some(p) = self.player() {
            unsafe { engine::ap_volume(p, volume) };
        }
    }

    fn set_skin(&mut self, skin: Skin) {
        self.skin = skin;
        save_skin(skin);
    }

    fn open_dialog(&mut self, multi: bool) {
        let filter = ["mp4", "mkv", "mov", "mp3", "flac", "wav", "m4a", "avi", "webm", "ogg"];
        let dialog = rfd::FileDialog::new().add_filter("Media", &filter);
        let paths = if multi { dialog.pick_files().unwrap_or_default() } else { dialog.pick_file().into_iter().collect() };
        for (i, path) in paths.into_iter().enumerate() {
            if i == 0 {
                self.open_path(path);
            } else {
                self.add_to_queue(&path);
            }
        }
    }

    /// Opens a file and records it in Open Recent Media.
    pub fn open_path(&mut self, path: PathBuf) {
        self.open_file(path.clone());
        self.remember_recent(&path);
    }

    fn add_to_queue(&mut self, path: &std::path::Path) {
        let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) else { return };
        if let Some(p) = self.player() {
            unsafe { engine::ap_add(p, c.as_ptr(), false) };
        }
        self.remember_recent(path);
    }

    fn remember_recent(&mut self, path: &std::path::Path) {
        let text = path.to_string_lossy().into_owned();
        self.recent.retain(|r| r != &text);
        self.recent.insert(0, text);
        self.recent.truncate(RECENT_MAX);
        if let Some(file) = recent_path() {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(file, self.recent.join("\n"));
        }
    }

    pub fn clear_recent(&mut self) {
        self.recent.clear();
        if let Some(file) = recent_path() {
            let _ = std::fs::remove_file(file);
        }
    }

    /// Plays a network URL (Open Network Stream…). The engine resolves it like a local path.
    fn open_location(&mut self, url: &str) {
        let Ok(c) = CString::new(url) else { return };
        let Some(p) = self.player() else { return };
        unsafe {
            engine::ap_stop(p);
            let index = engine::ap_count(p);
            engine::ap_add(p, c.as_ptr(), true);
            let ok = engine::ap_play(p, index);
            self.status = if ok { "Opening…".into() } else { "Could not open media".into() };
        }
        self.title_cmd = Some(format!("RMP — {url}"));
        self.remember_recent(std::path::Path::new(url));
    }

    /// Creates the child NSView that libvlc draws into, inside the screen rect above the egui view.
    fn attach_video(&mut self, frame: &eframe::Frame) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(parent) = content_view(frame) else { return };
        let child = unsafe {
            NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)))
        };
        child.setWantsLayer(true);
        // Behind the egui view, in the window's frame view. egui draws its controls and menus on top,
        // and the egui view is clear over the screen rect, so the video shows through and stays visible
        // while a menu is open.
        let Some(frame_view) = (unsafe { parent.superview() }) else { return };
        frame_view.addSubview_positioned_relativeTo(&child, objc2_app_kit::NSWindowOrderingMode::Below, Some(parent));
        // SAFETY: winit's content view stays alive while the window does; we keep a retained handle.
        self.content_view = Some(unsafe { Retained::retain(parent as *const NSView as *mut NSView) }.expect("content view"));
        let raw = Retained::as_ptr(&child) as *mut c_void;
        self.player = unsafe { engine::ap_create(raw) };
        if let Some(p) = self.player() {
            engine::ap_prepare(p);
            unsafe { engine::ap_volume(p, 80) };
        }
        self.video_view = Some(child);
    }

    /// Moves the video view to `rect` (egui points, top-left origin) inside the content view.
    fn place_video(&self, rect: Rect) {
        let (Some(view), Some(content)) = (&self.video_view, &self.content_view) else { return };
        let Some(frame_view) = (unsafe { view.superview() }) else { return };
        // The rect is in egui's coordinates, relative to the content view.
        let y = if content.isFlipped() {
            rect.min.y
        } else {
            content.bounds().size.height as f32 - rect.max.y
        };
        let in_content = NSRect::new(
            NSPoint::new(rect.min.x as f64, y as f64),
            NSSize::new(rect.width() as f64, rect.height() as f64),
        );
        let in_frame = content.convertRect_toView(in_content, Some(&frame_view));
        view.setFrame(in_frame);
    }

    /// Title bar and window appearance for the current skin (ThemeSurface / applyTheme in ui.m).
    fn apply_chrome(&mut self, frame: &eframe::Frame, ctx: &egui::Context) {
        if self.applied_skin == Some(self.skin) {
            return;
        }
        let Some(view) = content_view(frame) else { return };
        let Some(window) = view.window() else { return };
        let skin = self.skin;
        let th = skin.theme();
        window.setTitlebarAppearsTransparent(true);
        let appearance = unsafe {
            if th.dark {
                NSAppearance::appearanceNamed(objc2_app_kit::NSAppearanceNameDarkAqua)
            } else {
                NSAppearance::appearanceNamed(objc2_app_kit::NSAppearanceNameAqua)
            }
        };
        window.setAppearance(appearance.as_deref());
        let bg = if skin == Skin::Xp {
            rgb(0xece9d8)
        } else if skin == Skin::Modern {
            rgb(0xf5f5f5)
        } else {
            th.panel
        };
        let [r, g, b, _] = bg.to_array();
        let ns = NSColor::colorWithSRGBRed_green_blue_alpha(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0);
        window.setBackgroundColor(Some(&ns));
        ctx.set_visuals(if th.dark { egui::Visuals::dark() } else { egui::Visuals::light() });
        self.applied_skin = Some(skin);
    }
}

/// winit's content NSView for this window.
fn content_view(frame: &eframe::Frame) -> Option<&NSView> {
    let h = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(a) = h.as_raw() else { return None };
    // SAFETY: winit keeps the content view alive for the window's lifetime.
    Some(unsafe { &*(a.ns_view.as_ptr() as *const NSView) })
}

/// Vertical gradient with colour stops at fractions (0..1) of the rect height.
fn vgradient(painter: &Painter, r: Rect, stops: &[(f32, Color32)]) {
    let mut m = egui::Mesh::default();
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        let y0 = r.top() + r.height() * t0;
        let y1 = r.top() + r.height() * t1;
        let base = m.vertices.len() as u32;
        m.colored_vertex(Pos2::new(r.left(), y0), c0);
        m.colored_vertex(Pos2::new(r.right(), y0), c0);
        m.colored_vertex(Pos2::new(r.right(), y1), c1);
        m.colored_vertex(Pos2::new(r.left(), y1), c1);
        m.add_triangle(base, base + 1, base + 2);
        m.add_triangle(base, base + 2, base + 3);
    }
    painter.add(Shape::mesh(m));
}

fn draw_icon(painter: &Painter, c: Pos2, icon: Icon, ink: Color32) {
    let tri = |pts: Vec<Pos2>| Shape::convex_polygon(pts, ink, Stroke::NONE);
    match icon {
        Icon::Play => {
            painter.add(tri(vec![Pos2::new(c.x - 4.0, c.y - 6.0), Pos2::new(c.x + 6.0, c.y), Pos2::new(c.x - 4.0, c.y + 6.0)]));
        }
        Icon::Pause => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x - 3.0, c.y), Vec2::new(3.0, 12.0)), 0.0, ink);
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x + 3.0, c.y), Vec2::new(3.0, 12.0)), 0.0, ink);
        }
        Icon::Prev => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x - 6.0, c.y), Vec2::new(2.0, 10.0)), 0.0, ink);
            painter.add(tri(vec![Pos2::new(c.x + 6.0, c.y - 5.0), Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x + 6.0, c.y + 5.0)]));
        }
        Icon::Next => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x + 6.0, c.y), Vec2::new(2.0, 10.0)), 0.0, ink);
            painter.add(tri(vec![Pos2::new(c.x - 6.0, c.y - 5.0), Pos2::new(c.x + 4.0, c.y), Pos2::new(c.x - 6.0, c.y + 5.0)]));
        }
        Icon::Stop => {
            painter.rect_filled(Rect::from_center_size(c, Vec2::splat(9.0)), 0.0, ink);
        }
        Icon::Fullscreen => {
            let r = Rect::from_center_size(c, Vec2::splat(11.0));
            let st = Stroke::new(1.0, ink);
            let l = 3.0;
            for (corner, dx, dy) in [
                (r.left_top(), 1.0, 1.0),
                (r.right_top(), -1.0, 1.0),
                (r.left_bottom(), 1.0, -1.0),
                (r.right_bottom(), -1.0, -1.0),
            ] {
                painter.line_segment([corner, corner + Vec2::new(dx * l, 0.0)], st);
                painter.line_segment([corner, corner + Vec2::new(0.0, dy * l)], st);
            }
        }
    }
}

struct BtnSpec<'a> {
    caption: &'a str,
    size: f32,
    icon: Option<Icon>,
    width: f32,
    /// `Open Files…` in the top row; XP draws it without the silver treatment.
    open: bool,
}

/// Transport button drawn like TransportButton in ui.m, per skin.
fn tbutton(ui: &mut egui::Ui, skin: Skin, th: &Theme, spec: BtnSpec) -> egui::Response {
    // The Mac skin captions every button with its title, so buttons size to their text (ui.m intrinsicContentSize).
    let width = if skin == Skin::Mac && spec.icon.is_none() {
        text_width(ui, spec.caption, spec.size) + 16.0
    } else {
        spec.width
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, BUTTON_H), Sense::click());
    let painter = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    let is_play = spec.icon == Some(Icon::Play) || spec.icon == Some(Icon::Pause);
    let mut ink = th.text;
    let r = rect.shrink2(Vec2::new(1.5, 2.0));
    let silver = skin == Skin::Xp && !spec.open;
    let orb = skin == Skin::Seven && is_play;
    let round = silver || orb;
    let shape = if round {
        let d = r.width().min(r.height());
        Rect::from_center_size(r.center(), Vec2::splat(d))
    } else {
        r
    };
    let radius = if round { shape.height() / 2.0 } else { th.radius };
    let face = if skin == Skin::RmpDefault && is_play { th.accent } else { th.face };
    let edge = if skin == Skin::RmpDefault && is_play { rgb(0xdd6238) } else { th.edge };
    let clip = painter.with_clip_rect(shape);
    if orb {
        painter.circle_filled(shape.center(), radius, if pressed { rgb(0x1953ae) } else { rgb(0x2585df) });
        ink = Color32::WHITE;
    } else if silver {
        painter.circle_filled(shape.center(), radius, rgb(if pressed { 0x9daec7 } else { 0xe1eaf7 }));
    } else if skin == Skin::Seven {
        let top = if pressed { edge } else { face };
        vgradient(&clip, shape, &[(0.0, top), (0.5, face), (1.0, th.highlight)]);
    } else {
        painter.rect_filled(shape, radius, if pressed { edge } else { face });
    }
    if skin == Skin::RmpDefault && is_play {
        ink = rgb(0x16191f);
    }
    if round {
        painter.circle_stroke(shape.center(), radius, Stroke::new(1.0, edge));
    } else if skin != Skin::Modern {
        painter.rect_stroke(shape, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    }
    if skin == Skin::Xp && !silver && !pressed {
        painter.rect_stroke(shape.shrink(1.0), 2.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);
    }
    match spec.icon {
        Some(icon) if skin != Skin::Mac => draw_icon(painter, shape.center(), icon, ink),
        _ => {
            let font = FontId::new(spec.size, FontFamily::Proportional);
            painter.text(shape.center(), egui::Align2::CENTER_CENTER, spec.caption, font, ink);
        }
    }
    resp.on_hover_text(spec.caption)
}

fn text_width(ui: &egui::Ui, caption: &str, size: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(caption.into(), FontId::new(size, FontFamily::Proportional), Color32::BLACK)
        .size()
        .x
}

/// Seek or volume bar in the NSSlider style: a thin rail filled in the accent colour, with a round knob.
/// Returns (fraction while dragging or clicking, whether it was released).
fn bar(ui: &mut egui::Ui, frac: f64, enabled: bool, width: f32, th: &Theme) -> (Option<f64>, bool) {
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TIMELINE_H), sense);
    let track = Rect::from_min_max(
        Pos2::new(rect.left() + 8.0, rect.center().y - 2.0),
        Pos2::new(rect.right() - 8.0, rect.center().y + 2.0),
    );
    let painter = ui.painter();
    let rail = if th.dark { rgb(0x555555) } else { rgb(0xd0d0d0) };
    painter.rect_filled(track, 2.0, rail);
    let x = track.left() + track.width() * frac as f32;
    painter.rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 2.0, th.accent);
    painter.circle(Pos2::new(x, rect.center().y), 8.0, if th.dark { rgb(0xdddddd) } else { Color32::WHITE }, Stroke::new(1.0, rgb(0x9a9a9a)));
    if !enabled {
        return (None, false);
    }
    let pointer = if resp.dragged() || resp.clicked() || resp.drag_stopped() {
        resp.interact_pointer_pos().map(|p| ((p.x - track.left()) / track.width()).clamp(0.0, 1.0) as f64)
    } else {
        None
    };
    (pointer, resp.drag_stopped() || resp.clicked())
}

/// VLC skin volume: a green wedge rising to the right (VolumeSlider.wedge in ui.m).
fn wedge(ui: &mut egui::Ui, value: &mut i32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(VOLUME_W, TIMELINE_H), Sense::click_and_drag());
    let mut changed = false;
    let r = rect.shrink2(Vec2::new(4.0, 2.0));
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let v = ((pos.x - r.left()) / r.width() * 100.0).round().clamp(0.0, 100.0) as i32;
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    let tri = vec![Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.top()), Pos2::new(r.right(), r.bottom())];
    let painter = ui.painter();
    painter.add(Shape::convex_polygon(tri.clone(), rgb(0xe2e2e2), Stroke::new(1.0, rgb(0x777777))));
    let fill_w = r.width() * *value as f32 / 100.0;
    let clip = painter.with_clip_rect(Rect::from_min_size(r.min, Vec2::new(fill_w, r.height())));
    clip.add(Shape::convex_polygon(tri, rgb(0x4bc23b), Stroke::NONE));
    changed
}

impl eframe::App for App {
    /// Fully clear so the video layered beneath shows through the screen rect unchanged.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.player.is_null() {
            self.attach_video(frame);
            if let Some(path) = std::env::args_os().nth(1) {
                self.open_path(PathBuf::from(path));
            }
        }
        if self.cone.is_none() {
            if let Ok(img) = image::load_from_memory(CONE_PNG) {
                let img = img.to_rgba8();
                let size = [img.width() as usize, img.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                self.cone = Some(ctx.load_texture("vlc-cone", color, egui::TextureOptions::LINEAR));
            }
        }
        if let Some(title) = self.title_cmd.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        self.apply_chrome(frame, &ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle();
        }
        // Keyboard shortcuts from installPlayerMenus (mac-menus.h).
        let (cmd, shift, ctrl, key_o, key_u, key_t, key_e, key_l, key_q, key_f, key_lb, key_rb) = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.modifiers.ctrl,
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::U),
                i.key_pressed(egui::Key::T),
                i.key_pressed(egui::Key::E),
                i.key_pressed(egui::Key::L),
                i.key_pressed(egui::Key::Q),
                i.key_pressed(egui::Key::F),
                i.key_pressed(egui::Key::OpenBracket),
                i.key_pressed(egui::Key::CloseBracket),
            )
        });
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.play_bundled_clip(&ctx);
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if !typing {
            if cmd && key_o && !shift {
                self.open_dialog(false);
            } else if cmd && key_o && shift {
                self.open_dialog(true);
            } else if cmd && key_u {
                self.note_or_prompt(Native::OpenStream);
            } else if cmd && key_t {
                self.note_or_prompt(Native::TimeJump);
            } else if cmd && key_e {
                let r = menus::engine_action(self, "effects", "");
                if !r.is_empty() {
                    self.note = Some(("Effects and Filters".into(), r));
                }
            } else if cmd && key_l {
                let r = menus::engine_action(self, "playlist", "");
                if !r.is_empty() {
                    self.note = Some(("Playlist".into(), r));
                }
            } else if cmd && ctrl && key_f {
                self.run_native(Native::Fullscreen, &ctx);
            } else if cmd && key_q {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if key_lb {
                self.step(-1);
            } else if key_rb {
                self.step(1);
            }
        }

        let skin = self.skin;
        let th = skin.theme();
        let state = self.state();
        if state != self.last_state {
            self.last_state = state;
            self.status = ["Ready", "Opening…", "Buffering…", "Playing", "Paused", "Stopped", "Finished", "Playback error"]
                .get(state.clamp(0, 7) as usize)
                .unwrap_or(&"Ready")
                .to_string();
        }

        // Menu bar (installPlayerMenus). Actions are collected and run after the bar is drawn.
        let mut picked: Vec<Native> = Vec::new();
        let mut notes: Vec<(String, String)> = Vec::new();
        egui::Panel::top("menubar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                for (title, entries) in menus::menu_bar() {
                    ui.menu_button(title, |ui| menus::render(ui, self, &entries, &mut picked, &mut notes));
                }
            });
        });
        for (title, text) in notes {
            self.note = Some((title, text));
        }
        for action in picked {
            if action == Native::ClearRecent {
                self.clear_recent();
            } else {
                self.run_native(action, &ctx);
            }
        }
        self.draw_dialogs(&ctx);

        let full = ui.available_rect_before_wrap();
        let (w, h) = (full.width(), full.height());
        // Everything below is laid out relative to `full`, which starts under the menu bar.
        let origin = full.min.to_vec2();
        let ink = match skin {
            Skin::Xp => Color32::WHITE,
            Skin::Modern => rgb(0x181818),
            _ => th.text,
        };

        let has_top = skin != Skin::Vlc;
        let bottom_top = h - BUTTON_H - BOTTOM_INSET;
        let timeline_top = bottom_top - 6.0 - TIMELINE_H;

        // Top row (ui.m `top`): name, status, Open Files…. Hidden in the VLC skin.
        let mut screen_top = 0.0;
        if has_top {
            let top_rect = Rect::from_min_max(Pos2::new(INSET, 10.0), Pos2::new(w - INSET, 10.0 + BUTTON_H)).translate(origin);
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(top_rect));
            row.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.add(egui::Label::new(egui::RichText::new("RMP").strong().size(18.0).color(ink)));
                ui.add(egui::Label::new(egui::RichText::new(&self.status).size(12.0).color(ink)));
                let open_w = text_width(ui, "Open Files…", 13.0) + 14.0;
                let spacer = (top_rect.width() - open_w - 170.0).max(0.0);
                ui.add_space(spacer);
                if tbutton(ui, skin, &th, BtnSpec { caption: "Open Files…", size: 13.0, icon: None, width: open_w, open: true }).clicked() {
                    self.open_dialog(true);
                }
            });
            screen_top = top_rect.bottom() + 10.0 - origin.y;
        }

        // Screen: black, with the cone in the VLC skin while idle (ui.m `cone`).
        let screen = Rect::from_min_max(Pos2::new(0.0, screen_top), Pos2::new(w, timeline_top - 6.0)).translate(origin);
        // Panel backgrounds cover everything except the video screen, which stays clear so the video
        // layered beneath egui shows through.
        let above = Rect::from_min_max(full.min, Pos2::new(full.right(), screen.top()));
        let below = Rect::from_min_max(Pos2::new(full.left(), screen.bottom()), full.max);
        for region in [above, below] {
            if region.height() <= 0.0 {
                continue;
            }
            let p = ui.painter().with_clip_rect(region);
            p.rect_filled(region, 0.0, th.panel);
            if skin == Skin::Seven {
                vgradient(&p, full, &[(0.0, th.panel), (0.33, rgb(0xf2faff)), (0.66, rgb(0xa2c6e3)), (1.0, th.panel)]);
            }
            if skin == Skin::Xp {
                vgradient(
                    &p,
                    Rect::from_min_size(full.min, Vec2::new(w, 84.0)),
                    &[(0.0, rgb(0x7588b0)), (0.33, rgb(0xe1e7f1)), (0.66, rgb(0xfbfcff)), (1.0, rgb(0x869ac4))],
                );
                vgradient(
                    &p,
                    Rect::from_min_size(full.min, Vec2::new(w, 58.0)),
                    &[(0.0, rgb(0x1244b5)), (0.33, rgb(0x1260e9)), (0.66, rgb(0x0058e5)), (1.0, rgb(0x4796ff))],
                );
            }
        }
        let idle = match self.player() {
            Some(p) => (unsafe { engine::ap_index(p) }) < 0,
            None => true,
        };
        // Black unless libvlc has a video output; that output draws into the layered view beneath egui.
        let has_video = match self.player() {
            Some(p) => unsafe { engine::ap_has_video(p) },
            None => false,
        };
        if !has_video {
            ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
        }
        match skin {
            Skin::Seven => {
                ui.painter().rect_stroke(screen, 0.0, Stroke::new(2.0, rgb(0x829db5)), egui::StrokeKind::Inside);
            }
            Skin::Xp => {
                ui.painter().rect_stroke(screen, 0.0, Stroke::new(4.0, rgb(0x1256ce)), egui::StrokeKind::Inside);
            }
            _ => {}
        }
        if idle && skin == Skin::Vlc {
            if let Some(cone) = &self.cone {
                egui::Image::new(cone).paint_at(ui, Rect::from_center_size(screen.center(), Vec2::splat(CONE_SIZE)));
            }
        }
        if let Some(view) = &self.video_view {
            view.setHidden(idle);
        }
        self.place_video(screen);

        // Timeline row (ui.m `timeline`): seek bar + clock.
        let timeline_rect = Rect::from_min_max(Pos2::new(INSET, timeline_top), Pos2::new(w - INSET, timeline_top + TIMELINE_H)).translate(origin);
        let (t, d, seekable) = match self.player() {
            Some(p) => unsafe { (engine::ap_time(p), engine::ap_length(p), engine::ap_seekable(p)) },
            None => (0, 0, false),
        };
        let mut timeline = ui.new_child(egui::UiBuilder::new().max_rect(timeline_rect));
        timeline.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;
            let bar_w = (timeline_rect.width() - CLOCK_W - ROW_GAP).max(40.0);
            let mut frac = if d > 0 { (self.scrub.unwrap_or(t) as f64 / d as f64).clamp(0.0, 1.0) } else { 0.0 };
            let (pointer, released) = bar(ui, frac, seekable && d > 0, bar_w, &th);
            if let Some(f) = pointer {
                frac = f;
                self.scrub = Some((frac * d as f64) as i64);
            }
            if released {
                let target = (frac * d as f64) as i64;
                self.scrub = None;
                if let Some(p) = self.player() {
                    unsafe { engine::ap_seek(p, target) };
                }
            }
            let shown = self.scrub.unwrap_or(t);
            let clock = format!("{} / {}", stamp(shown), if d > 0 { stamp(d) } else { "—".into() });
            ui.add_sized(
                Vec2::new(CLOCK_W, TIMELINE_H),
                egui::Label::new(egui::RichText::new(clock).font(FontId::new(12.0, FontFamily::Monospace)).color(th.text)),
            );
        });

        // Bottom transport row (ui.m `bottom`), ordered per skin.
        let bottom_rect = Rect::from_min_max(Pos2::new(INSET, bottom_top), Pos2::new(w - INSET, bottom_top + BUTTON_H)).translate(origin);
        let mut bottom = ui.new_child(egui::UiBuilder::new().max_rect(bottom_rect));
        let mut vol = self.volume;
        let mut volume_changed = false;
        let mut actions: Vec<Item> = Vec::new();
        bottom.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = if skin == Skin::Seven { 2.0 } else { ROW_GAP };
            let items = bottom_items(skin);
            for item in items {
                match item {
                    Item::Play => {
                        let icon = if state == 3 || state == 2 || state == 1 { Icon::Pause } else { Icon::Play };
                        let caption = if icon == Icon::Pause { "Pause" } else { "Play" };
                        let resp = tbutton(ui, skin, &th, BtnSpec { caption, size: 17.0, icon: Some(icon), width: 52.0, open: false });
                        if resp.clicked() {
                            actions.push(Item::Play);
                        }
                    }
                    Item::Prev => {
                        let resp = tbutton(ui, skin, &th, BtnSpec { caption: "Previous", size: 13.0, icon: Some(Icon::Prev), width: 68.0, open: false });
                        if resp.clicked() {
                            actions.push(Item::Prev);
                        }
                    }
                    Item::Stop => {
                        let resp = tbutton(ui, skin, &th, BtnSpec { caption: "Stop", size: 13.0, icon: Some(Icon::Stop), width: 29.0, open: false });
                        if resp.clicked() {
                            actions.push(Item::Stop);
                        }
                    }
                    Item::Next => {
                        let resp = tbutton(ui, skin, &th, BtnSpec { caption: "Next", size: 13.0, icon: Some(Icon::Next), width: 44.0, open: false });
                        if resp.clicked() {
                            actions.push(Item::Next);
                        }
                    }
                    Item::Fullscreen => {
                        let resp = tbutton(ui, skin, &th, BtnSpec { caption: "Fullscreen", size: 13.0, icon: Some(Icon::Fullscreen), width: 26.0, open: false });
                        if resp.clicked() {
                            actions.push(Item::Fullscreen);
                        }
                    }
                    Item::Queue | Item::Repeat | Item::Shuffle => {
                        let (caption, glyph) = match item {
                            Item::Queue => ("Queue", None),
                            Item::Repeat => ("↻", Some("↻")),
                            _ => ("⤨", Some("⤨")),
                        };
                        let _ = glyph;
                        let width = text_width(ui, caption, 13.0) + 14.0;
                        let _ = tbutton(ui, skin, &th, BtnSpec { caption, size: 13.0, icon: None, width, open: false });
                    }
                    Item::Spacer => {
                        let used = ui.min_rect().width();
                        let right = bottom_rect.width() - (VOLUME_W + 60.0);
                        if right > used {
                            ui.add_space(right - used);
                        }
                    }
                    Item::VolumeLabel => {
                        ui.add(egui::Label::new(egui::RichText::new("Volume").size(12.0).color(th.text)));
                    }
                    Item::Volume => {
                        if skin == Skin::Vlc {
                            volume_changed = wedge(ui, &mut vol);
                        } else {
                            let frac = vol as f64 / 100.0;
                            let (pointer, _) = bar(ui, frac, true, VOLUME_W, &th);
                            if let Some(f) = pointer {
                                vol = (f * 100.0).round() as i32;
                                volume_changed = true;
                            }
                        }
                    }
                }
            }
        });
        if volume_changed {
            self.set_volume(vol);
        }
        for item in actions {
            match item {
                Item::Play => self.toggle(),
                Item::Prev => self.step(-1),
                Item::Next => self.step(1),
                Item::Stop => {
                    if let Some(p) = self.player() {
                        unsafe { engine::ap_stop(p) };
                    }
                }
                Item::Fullscreen => {
                    let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
                }
                _ => {}
            }
        }
    }
}

impl App {
    fn note_or_prompt(&mut self, action: Native) {
        match action {
            Native::OpenStream => self.prompt = Some(Prompt::Stream(String::new())),
            Native::TimeJump => self.prompt = Some(Prompt::Time(String::new())),
            _ => {}
        }
    }

    /// Note and prompt windows raised by menu items (menuNote / Open Network Stream… / Jump to Specific Time…).
    fn draw_dialogs(&mut self, ctx: &egui::Context) {
        let mut close_note = false;
        if let Some((title, detail)) = &self.note {
            egui::Window::new(title.clone()).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label(detail.clone());
                if ui.button("OK").clicked() {
                    close_note = true;
                }
            });
        }
        if close_note {
            self.note = None;
        }
        let mut submit: Option<Prompt> = None;
        let mut cancel = false;
        if let Some(prompt) = self.prompt.as_mut() {
            let (title, hint, text) = match prompt {
                Prompt::Stream(t) => ("Open Network Stream", "Enter a URL including its protocol, such as https://…", t),
                Prompt::Time(t) => ("Jump to Specific Time", "Enter a time as m:ss or h:mm:ss.", t),
            };
            egui::Window::new(title).collapsible(false).resizable(false).show(ctx, |ui| {
                ui.label(hint);
                let edit = ui.text_edit_singleline(text);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        submit = Some(match prompt_kind(&title) {
                            true => Prompt::Stream(text.clone()),
                            false => Prompt::Time(text.clone()),
                        });
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        }
        if cancel {
            self.prompt = None;
        }
        if let Some(prompt) = submit {
            self.prompt = None;
            match prompt {
                Prompt::Stream(url) => {
                    if url.trim().is_empty() {
                        return;
                    }
                    self.open_location(url.trim());
                }
                Prompt::Time(text) => match parse_time(&text) {
                    Some(ms) => {
                        if let Some(p) = self.player() {
                            unsafe { engine::ap_seek(p, ms) };
                        }
                    }
                    None => self.note = Some(("Jump to Specific Time".into(), "Enter a valid playback time.".into())),
                },
            }
        }
    }
}

impl App {
    /// Fullscreen playback of the clip bundled with the app (resources/clip.mp4).
    fn play_bundled_clip(&mut self, ctx: &egui::Context) {
        let Some(path) = bundled_resource("clip.mp4") else { return };
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        self.open_file(path);
    }
}

/// Finds a file in the app's resources: beside the executable, in an app bundle's Resources folder,
/// or in this crate's resources folder during development.
fn bundled_resource(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(PathBuf::from)) {
        candidates.push(dir.join("resources").join(name));
        candidates.push(dir.join("../Resources").join(name));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join(name));
    candidates.into_iter().find(|p| p.is_file())
}

fn prompt_kind(title: &str) -> bool {
    title == "Open Network Stream"
}

/// "m:ss" or "h:mm:ss" (or plain seconds) to milliseconds.
fn parse_time(text: &str) -> Option<i64> {
    let mut total = 0.0f64;
    for part in text.trim().split(':') {
        let v: f64 = part.trim().parse().ok()?;
        if v < 0.0 {
            return None;
        }
        total = total * 60.0 + v;
    }
    Some((total * 1000.0) as i64)
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut load = |name: &str, path: &str, fonts: &mut egui::FontDefinitions| {
        std::fs::read(path).ok().map(|bytes| {
            fonts.font_data.insert(name.into(), egui::FontData::from_owned(bytes).into());
        })
    };
    if load("sf", "/System/Library/Fonts/SFNS.ttf", &mut fonts).is_some() {
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "sf".into());
    }
    if load("sf-mono", "/System/Library/Fonts/SFNSMono.ttf", &mut fonts).is_some() {
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "sf-mono".into());
    }
    if load("symbols", "/System/Library/Fonts/Apple Symbols.ttf", &mut fonts).is_some() {
        fonts.families.entry(FontFamily::Proportional).or_default().push("symbols".into());
    }
    ctx.set_fonts(fonts);
}

fn stamp(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if !self.player.is_null() {
            unsafe { engine::ap_destroy(self.player) };
            self.player = std::ptr::null_mut();
        }
    }
}

fn main() -> eframe::Result {
    // The shipping app bundles plugins/ next to its executable. The spike runs from target/,
    // so point libvlc at the plugins inside VLC.app unless the caller already set it.
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
        ..Default::default()
    };
    eframe::run_native("RMP", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
