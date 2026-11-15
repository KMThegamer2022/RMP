//! egui spike: drives the existing Rust engine (repo-root src/lib.rs, the `ap_*` API) from egui.
//! macOS only. The layout follows the shipping app's default theme (VLC skin, ui.m / mac-theme.h).
//! The shipping AppKit app is not changed.

use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSAppearance, NSAppearanceCustomization, NSColor, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_void, CString};
use std::path::PathBuf;

// Engine: repo-root src/lib.rs compiled as this crate's library (see Cargo.toml [lib]).
use egui_spike as engine;

// Default theme from themes.h (index VLC_THEME = 5).
const PANEL: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf2);
const EDGE: Color32 = Color32::from_rgb(0xb4, 0xb4, 0xb4);
const FACE: Color32 = Color32::from_rgb(0xf8, 0xf8, 0xf8);
const TEXT: Color32 = Color32::from_rgb(0x53, 0x53, 0x53);
const ACCENT: Color32 = Color32::from_rgb(0x57, 0xbb, 0x39);
const WEDGE_BG: Color32 = Color32::from_rgb(0xe2, 0xe2, 0xe2);
const WEDGE_FILL: Color32 = Color32::from_rgb(0x4b, 0xc2, 0x3b);
const WEDGE_EDGE: Color32 = Color32::from_rgb(0x77, 0x77, 0x77);

// Layout constants taken from ui.m applicationDidFinishLaunching / arrangeChrome.
const INSET: f32 = 16.0; // side inset for timeline and bottom rows
const ROW_GAP: f32 = 8.0; // stack spacing for the VLC skin
const TIMELINE_H: f32 = 21.0; // NSSlider height
const BUTTON_H: f32 = 30.0; // TransportButton intrinsic height
const BOTTOM_INSET: f32 = 12.0;
const TIMELINE_TOP_GAP: f32 = 6.0;
const CLOCK_W: f32 = 84.0; // room for "0:00 / 0:00" at 12pt monospaced
const VOLUME_W: f32 = 85.0;
const CONE_SIZE: f32 = 120.0;

const CONE_PNG: &[u8] = include_bytes!("../../assets/vlc-cone.png");

struct App {
    player: *mut engine::Player,
    video_view: Option<Retained<NSView>>,
    cone: Option<egui::TextureHandle>,
    file: Option<PathBuf>,
    status: String,
    volume: i32,
    scrub: Option<i64>,
    last_state: i32,
    start: std::time::Instant,
    selftest: bool,
    selftest_stage: u32,
    title_cmd: Option<String>,
    chrome_done: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        Self {
            player: std::ptr::null_mut(),
            video_view: None,
            cone: None,
            file: None,
            status: "Ready".into(),
            volume: 80,
            scrub: None,
            last_state: 0,
            start: std::time::Instant::now(),
            selftest: std::env::var_os("RMP_SPIKE_SELFTEST").is_some(),
            selftest_stage: 0,
            title_cmd: None,
            chrome_done: false,
        }
    }

    fn player(&mut self) -> Option<&mut engine::Player> {
        // SAFETY: the pointer comes from ap_create and lives until Drop; UI thread only.
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
        if (1..=3).contains(&state) || state == 4 {
            unsafe { engine::ap_toggle(p) };
        } else if unsafe { engine::ap_count(p) } > 0 {
            let i = usize::try_from(unsafe { engine::ap_index(p) }).unwrap_or(0);
            unsafe { engine::ap_play(p, i) };
        }
    }

    fn set_volume(&mut self, v: i32) {
        self.volume = v.clamp(0, 100);
        let volume = self.volume;
        if let Some(p) = self.player() {
            unsafe { engine::ap_volume(p, volume) };
        }
    }

    /// Self-test (RMP_SPIKE_SELFTEST): seek, volume and pause through the engine, logged to stderr.
    fn run_selftest(&mut self) {
        if !self.selftest {
            return;
        }
        let secs = self.start.elapsed().as_secs_f32();
        let due = [3.0, 4.0, 6.0, 9.0, 10.0, 12.0];
        let stage = self.selftest_stage as usize;
        if stage >= due.len() || secs < due[stage] {
            return;
        }
        self.selftest_stage += 1;
        let Some(p) = self.player() else { return };
        match stage {
            1 => unsafe { engine::ap_volume(p, 20) },
            2 => unsafe { engine::ap_seek(p, engine::ap_length(p) / 2) },
            4 => unsafe { engine::ap_toggle(p) },
            _ => {}
        }
        let line = unsafe {
            format!(
                "stage {stage}: state={} time={}ms length={}ms seekable={}",
                engine::ap_state(p), engine::ap_time(p), engine::ap_length(p), engine::ap_seekable(p)
            )
        };
        eprintln!("SELFTEST {line}");
        if self.selftest_stage as usize == due.len() {
            std::process::exit(0);
        }
    }

    /// Creates the child NSView that libvlc draws into. It sits inside the black screen rect,
    /// above the egui view, so the egui controls below it stay visible.
    fn attach_video(&mut self, frame: &eframe::Frame) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let parent_ptr = frame.window_handle().ok().and_then(|h| match h.as_raw() {
            RawWindowHandle::AppKit(a) => Some(a.ns_view.as_ptr() as *const NSView),
            _ => None,
        });
        let Some(parent_ptr) = parent_ptr else { return };
        // SAFETY: winit's NSView outlives this frame callback; we only add a subview to it.
        let parent: &NSView = unsafe { &*parent_ptr };
        let child = unsafe {
            NSView::initWithFrame(NSView::alloc(mtm), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)))
        };
        child.setWantsLayer(true);
        parent.addSubview(&child);
        let raw = Retained::as_ptr(&child) as *mut c_void;
        self.player = unsafe { engine::ap_create(raw) };
        if let Some(p) = self.player() {
            engine::ap_prepare(p);
            unsafe { engine::ap_volume(p, 80) };
        }
        self.video_view = Some(child);
    }

    /// Moves the child view to `rect` (egui points, top-left origin) inside the parent view.
    fn place_video(&self, rect: Rect) {
        let Some(view) = &self.video_view else { return };
        let Some(parent) = (unsafe { view.superview() }) else { return };
        // winit's content view is flipped (top-left origin); convert only if it is not.
        let y = if parent.isFlipped() {
            rect.min.y
        } else {
            parent.bounds().size.height as f32 - rect.max.y
        };
        let frame = NSRect::new(
            NSPoint::new(rect.min.x as f64, y as f64),
            NSSize::new(rect.width() as f64, rect.height() as f64),
        );
        view.setFrame(frame);
    }
}

/// Matches the shipping window chrome: transparent titlebar, light appearance, panel-coloured background.
fn style_titlebar(frame: &eframe::Frame) -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let _ = mtm;
    let Ok(h) = frame.window_handle() else { return false };
    let RawWindowHandle::AppKit(a) = h.as_raw() else { return false };
    // SAFETY: the NSView is winit's content view, alive for the window's lifetime.
    let view: &NSView = unsafe { &*(a.ns_view.as_ptr() as *const NSView) };
    let Some(window) = view.window() else { return false };
    window.setTitlebarAppearsTransparent(true);
    let aqua = unsafe { NSAppearance::appearanceNamed(objc2_app_kit::NSAppearanceNameAqua) };
    window.setAppearance(aqua.as_deref());
    let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0xf2 as f64 / 255.0, 0xf2 as f64 / 255.0, 0xf2 as f64 / 255.0, 1.0);
    window.setBackgroundColor(Some(&bg));
    true
}

fn install_fonts(ctx: &egui::Context) {
    // SF is the shipping app's system font; Apple Symbols covers the transport glyphs.
    let mut fonts = egui::FontDefinitions::default();
    let load = |name: &str, path: &str, fonts: &mut egui::FontDefinitions| {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.into(), egui::FontData::from_owned(bytes).into());
            true
        } else {
            false
        }
    };
    if load("sf", "/System/Library/Fonts/SFNS.ttf", &mut fonts) {
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "sf".into());
    }
    if load("sf-mono", "/System/Library/Fonts/SFNSMono.ttf", &mut fonts) {
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "sf-mono".into());
    }
    if load("symbols", "/System/Library/Fonts/Apple Symbols.ttf", &mut fonts) {
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

#[derive(Clone, Copy)]
enum Icon {
    Play,
    Pause,
    Prev,
    Next,
    Stop,
    Fullscreen,
}

fn draw_icon(painter: &egui::Painter, c: Pos2, icon: Icon) {
    let ink = TEXT;
    let tri = |pts: Vec<Pos2>| egui::Shape::convex_polygon(pts, ink, Stroke::NONE);
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
            for (corner, dx, dy) in [(r.left_top(), 1.0, 1.0), (r.right_top(), -1.0, 1.0), (r.left_bottom(), 1.0, -1.0), (r.right_bottom(), -1.0, -1.0)] {
                painter.line_segment([corner, corner + Vec2::new(dx * l, 0.0)], st);
                painter.line_segment([corner, corner + Vec2::new(0.0, dy * l)], st);
            }
        }
    }
}

/// VLC-skin transport button: light face, 1pt grey edge, 2pt corner radius, centred caption.
fn transport_button(ui: &mut egui::Ui, caption: &str, size: f32, icon: Option<Icon>) -> egui::Response {
    let font = FontId::new(size, FontFamily::Proportional);
    let text_w = ui.painter().layout_no_wrap(caption.into(), font.clone(), TEXT).size().x;
    let w = match icon {
        Some(Icon::Play) | Some(Icon::Pause) => 52.0,
        Some(Icon::Prev) => 68.0,
        Some(Icon::Next) => 44.0,
        Some(Icon::Stop) => 29.0,
        Some(Icon::Fullscreen) => 26.0,
        None => text_w + 14.0,
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, BUTTON_H), Sense::click());
    let painter = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    let face = if pressed { EDGE } else { FACE };
    painter.rect(rect.shrink2(Vec2::new(1.5, 2.0)), 2.0, face, Stroke::new(1.0, EDGE), egui::StrokeKind::Inside);
    match icon {
        Some(kind) => draw_icon(painter, rect.center(), kind),
        None => {
            painter.text(rect.center(), egui::Align2::CENTER_CENTER, caption, font, TEXT);
        }
    }
    resp.on_hover_text(caption)
}

/// Seek bar in the style of the shipping NSSlider: a thin rail filled in the accent colour up to a round knob.
/// Returns (pointer fraction while dragging or clicking, whether the pointer was released).
fn seek_bar(ui: &mut egui::Ui, frac: f64, enabled: bool, width: f32) -> (Option<f64>, bool) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, TIMELINE_H), if enabled { Sense::click_and_drag() } else { Sense::hover() });
    let track = Rect::from_min_max(Pos2::new(rect.left() + 8.0, rect.center().y - 2.0), Pos2::new(rect.right() - 8.0, rect.center().y + 2.0));
    let painter = ui.painter();
    painter.rect_filled(track, 2.0, Color32::from_rgb(0xd0, 0xd0, 0xd0));
    let x = track.left() + track.width() * frac as f32;
    painter.rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 2.0, Color32::from_rgb(0x3f, 0x9a, 0x1f));
    painter.circle(Pos2::new(x, rect.center().y), 8.0, Color32::WHITE, Stroke::new(1.0, Color32::from_rgb(0x9a, 0x9a, 0x9a)));
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

/// Volume control: the VLC skin draws a green wedge rising to the right (VolumeSlider.wedge).
fn wedge_slider(ui: &mut egui::Ui, value: &mut i32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(VOLUME_W, TIMELINE_H), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let r = rect.shrink2(Vec2::new(4.0, 2.0));
            let v = ((pos.x - r.left()) / r.width() * 100.0).round().clamp(0.0, 100.0) as i32;
            if v != *value {
                *value = v;
                changed = true;
            }
        }
    }
    let r = rect.shrink2(Vec2::new(4.0, 2.0));
    let tri = vec![Pos2::new(r.left(), r.bottom()), Pos2::new(r.right(), r.top()), Pos2::new(r.right(), r.bottom())];
    let painter = ui.painter();
    painter.add(egui::Shape::convex_polygon(tri.clone(), WEDGE_BG, Stroke::new(1.0, WEDGE_EDGE)));
    let fill_w = r.width() * *value as f32 / 100.0;
    let clip = painter.with_clip_rect(Rect::from_min_size(r.min, Vec2::new(fill_w, r.height())));
    clip.add(egui::Shape::convex_polygon(tri, WEDGE_FILL, Stroke::NONE));
    changed
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.player.is_null() {
            self.attach_video(frame);
            if let Some(path) = std::env::args_os().nth(1) {
                self.open_file(PathBuf::from(path));
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
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        // Keyboard: Space toggles, Cmd+O opens (the shipping app has these in its menus).
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.toggle();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open_dialog();
        }

        let state = self.state();
        if state != self.last_state {
            self.last_state = state;
            self.status = ["Ready", "Opening…", "Buffering…", "Playing", "Paused", "Stopped", "Finished", "Playback error"]
                .get(state.clamp(0, 7) as usize)
                .unwrap_or(&"Ready")
                .to_string();
        }

        if let Some(title) = self.title_cmd.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        if !self.chrome_done {
            self.chrome_done = style_titlebar(frame);
        }

        let full = ui.max_rect();
        let (w, h) = (full.width(), full.height());
        let timeline_top = h - BOTTOM_INSET - BUTTON_H - TIMELINE_TOP_GAP - TIMELINE_H;
        let screen = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(w, timeline_top - TIMELINE_TOP_GAP));

        let idle = match self.player() {
            Some(p) => (unsafe { engine::ap_index(p) }) < 0,
            None => true,
        };
        // Black screen; the cone shows while idle (VLC skin), video shows once a file is loaded.
        ui.painter().rect_filled(full, 0.0, PANEL);
        ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
        if idle {
            if let Some(cone) = &self.cone {
                let c = screen.center();
                let r = Rect::from_center_size(c, Vec2::splat(CONE_SIZE));
                egui::Image::new(cone).paint_at(ui, r);
            }
        }
        if let Some(view) = &self.video_view {
            view.setHidden(idle);
        }
        self.place_video(screen);

        // Timeline row: seek slider + clock (ui.m `timeline`).
        let timeline_rect = Rect::from_min_max(Pos2::new(INSET, timeline_top), Pos2::new(w - INSET, timeline_top + TIMELINE_H));
        let (t, d, seekable) = match self.player() {
            Some(p) => unsafe { (engine::ap_time(p), engine::ap_length(p), engine::ap_seekable(p)) },
            None => (0, 0, false),
        };
        let mut timeline = ui.new_child(egui::UiBuilder::new().max_rect(timeline_rect));
        timeline.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;
            let slider_w = (timeline_rect.width() - CLOCK_W - ROW_GAP).max(40.0);
            let mut frac = if d > 0 { (self.scrub.unwrap_or(t) as f64 / d as f64).clamp(0.0, 1.0) } else { 0.0 };
            let (pointer, released) = seek_bar(ui, frac, seekable && d > 0, slider_w);
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
                egui::Label::new(egui::RichText::new(clock).font(FontId::new(12.0, FontFamily::Monospace)).color(TEXT)),
            );
        });

        // Bottom transport row (ui.m `bottom`): play, prev, stop, next, fullscreen, queue, repeat, shuffle … Volume.
        let bottom_rect = Rect::from_min_max(Pos2::new(INSET, h - BUTTON_H - BOTTOM_INSET), Pos2::new(w - INSET, h - BOTTOM_INSET));
        let mut bottom = ui.new_child(egui::UiBuilder::new().max_rect(bottom_rect));
        bottom.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;
            let play_icon = if (1..=3).contains(&state) { Icon::Pause } else { Icon::Play };
            if transport_button(ui, "Play", 13.0, Some(play_icon)).clicked() {
                self.toggle();
            }
            if transport_button(ui, "Previous", 13.0, Some(Icon::Prev)).clicked() {
                self.step(-1);
            }
            if transport_button(ui, "Stop", 13.0, Some(Icon::Stop)).clicked() {
                if let Some(p) = self.player() {
                    unsafe { engine::ap_stop(p) };
                }
            }
            if transport_button(ui, "Next", 13.0, Some(Icon::Next)).clicked() {
                self.step(1);
            }
            if transport_button(ui, "Fullscreen", 13.0, Some(Icon::Fullscreen)).clicked() {
                let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
            }
            let _ = transport_button(ui, "Queue", 13.0, None);
            let _ = transport_button(ui, "↻", 13.0, None);
            let _ = transport_button(ui, "⤨", 13.0, None);
            // Flexible spacer, then "Volume" label and the wedge slider at the right edge.
            let used = ui.min_rect().width();
            let right = bottom_rect.width() - (VOLUME_W + 52.0);
            if right > used {
                ui.add_space(right - used);
            }
            ui.add(egui::Label::new(egui::RichText::new("Volume").size(12.0).color(TEXT)));
            let mut vol = self.volume;
            if wedge_slider(ui, &mut vol) {
                self.set_volume(vol);
            }
        });

        self.run_selftest();
    }
}

impl App {
    fn step(&mut self, delta: isize) {
        let Some(p) = self.player() else { return };
        let count = unsafe { engine::ap_count(p) } as isize;
        let index = unsafe { engine::ap_index(p) };
        let next = index + delta;
        if next >= 0 && next < count {
            unsafe { engine::ap_play(p, next as usize) };
        }
    }

    fn open_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Media", &["mp4", "mkv", "mov", "mp3", "flac", "wav", "m4a", "avi", "webm", "ogg"])
            .pick_file()
        {
            self.open_file(path);
        }
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
            .with_title("RMP"),
        ..Default::default()
    };
    eframe::run_native("RMP", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
