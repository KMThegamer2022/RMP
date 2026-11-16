//! The main window: menu bar, screen, timeline, transport, extra controls, status line, docked
//! playlist and the fullscreen floating transport. Geometry follows applicationDidFinishLaunching,
//! arrangeChrome, layoutMediaChrome and updateViewControls in the shipping app.

use crate::appkit;
use crate::media;
use crate::menus::{self, Picked};
use crate::metadata::stamp;
use crate::theme::{rgb, Skin, Theme};
use crate::widgets::{self, bar, gradient_up, text_width, tbutton, Role, BUTTON_H, SLIDER_H, VOLUME_W};
use crate::{App, MediaPage};
use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};

const INSET: f32 = 16.0;
const CLOCK_W: f32 = 84.0;
const CONE_SIZE: f32 = 120.0;
const MARK_SIZE: f32 = 96.0;
const DOCK_W: f32 = 222.0;
pub const MEDIA_HEADER_TOP: f32 = 3.0;
pub const MEDIA_HEADER_H: f32 = 42.0;
pub const NAV_W: f32 = 132.0;
pub const XP_QUEUE_W: f32 = 210.0;

/// All rectangles, in window (egui) coordinates.
pub struct Geometry {
    pub full: Rect,
    pub screen: Rect,
    pub timeline: Rect,
    pub bottom: Rect,
    pub top_row: Rect,
    pub extra_row: Rect,
    pub status_row: Rect,
    pub dock: Option<Rect>,
    pub nav: Rect,
    pub header: Rect,
    pub browser: Option<Rect>,
    pub bare: bool,
    pub media: bool,
    pub screen_visible: bool,
}

pub fn geometry(app: &App, full: Rect) -> Geometry {
    let skin = app.skin;
    let bare = app.bare();
    let media = app.media_active();
    let regular = app.regular();
    let adv = regular && app.store.advanced;
    let status = regular && app.store.status_bar;
    let extra_h = if adv { 30.0 } else { 0.0 };
    let status_h = if status { 18.0 } else { 0.0 };
    let b_bottom = full.bottom() - 12.0 - extra_h - status_h;
    let bottom = Rect::from_min_max(Pos2::new(full.left() + INSET, b_bottom - BUTTON_H), Pos2::new(full.right() - INSET, b_bottom));
    let t_bottom = bottom.top() - 6.0;
    let timeline = Rect::from_min_max(Pos2::new(full.left() + INSET, t_bottom - SLIDER_H), Pos2::new(full.right() - INSET, t_bottom));
    let top_row = Rect::from_min_max(Pos2::new(full.left() + INSET, full.top() + 10.0), Pos2::new(full.right() - INSET, full.top() + 10.0 + BUTTON_H));
    let extra_row = Rect::from_min_max(Pos2::new(full.left() + 12.0, full.bottom() - status_h - 30.0), Pos2::new(full.right() - 12.0, full.bottom() - status_h));
    let status_row = Rect::from_min_max(Pos2::new(full.left() + 12.0, full.bottom() - 20.0), Pos2::new(full.right() - 12.0, full.bottom() - 2.0));
    let playing_page = app.media_page == MediaPage::NowPlaying;
    let xp_sidebar = media && skin == Skin::Xp && playing_page;
    let edge = bare || skin == Skin::Vlc;
    let top = if bare || skin == Skin::Vlc && !media {
        full.top()
    } else if media {
        full.top() + MEDIA_HEADER_TOP + MEDIA_HEADER_H
    } else if edge {
        full.top()
    } else {
        top_row.bottom() + 10.0
    };
    let sbottom = if bare { full.bottom() } else { timeline.top() - 6.0 };
    let dock_visible = app.playlist_shown && !app.store.undocked_playlist && regular && !skin.has_media_layout();
    let left = if media { full.left() + 3.0 + NAV_W } else { full.left() };
    let right_inset = if xp_sidebar {
        213.0
    } else if dock_visible {
        225.0
    } else {
        0.0
    };
    let screen = Rect::from_min_max(Pos2::new(left, top), Pos2::new(full.right() - right_inset, sbottom));
    let dock = dock_visible.then(|| Rect::from_min_max(Pos2::new(full.right() - DOCK_W, screen.top()), Pos2::new(full.right(), screen.bottom())));
    let header = Rect::from_min_max(Pos2::new(full.left() + 3.0, full.top() + MEDIA_HEADER_TOP), Pos2::new(full.right() - 3.0, full.top() + MEDIA_HEADER_TOP + MEDIA_HEADER_H));
    let nav_bottom = timeline.top() - 5.0;
    let nav = Rect::from_min_max(Pos2::new(full.left() + 3.0, header.bottom()), Pos2::new(full.left() + 3.0 + NAV_W, nav_bottom));
    let browser = if !media {
        None
    } else if xp_sidebar {
        Some(Rect::from_min_max(Pos2::new(full.right() - 3.0 - XP_QUEUE_W, header.bottom()), Pos2::new(full.right() - 3.0, nav_bottom)))
    } else if !playing_page {
        Some(Rect::from_min_max(Pos2::new(nav.right(), header.bottom()), Pos2::new(full.right() - 3.0, nav_bottom)))
    } else {
        None
    };
    let screen_visible = !(media && !playing_page);
    Geometry { full, screen, timeline, bottom, top_row, extra_row, status_row, dock, nav, header, browser, bare, media, screen_visible }
}

/// ThemeSurface.drawRect: the window background for the skin, drawn in `region` only.
fn surface(ui: &egui::Ui, app: &App, th: &Theme, full: Rect, region: Rect, glass: bool) {
    if region.width() <= 0.0 || region.height() <= 0.0 {
        return;
    }
    let p = ui.painter().with_clip_rect(region);
    let skin = app.skin;
    if skin == Skin::Seven {
        if !glass {
            gradient_up(&p, full, &[th.panel, rgb(0xf2faff), rgb(0xa2c6e3), th.panel]);
        } else {
            gradient_up(&p, full, &[th.panel, rgb(0xf2faff), rgb(0xa2c6e3), th.panel]);
        }
        return;
    }
    p.rect_filled(region, 0.0, th.panel);
    if skin == Skin::Xp {
        // Silver band along the bottom (84 pt) and the blue header along the top (58 pt).
        gradient_up(&p, Rect::from_min_max(Pos2::new(full.left(), full.bottom() - 84.0), full.max), &[rgb(0x7588b0), rgb(0xe1e7f1), rgb(0xfbfcff), rgb(0x869ac4)]);
        gradient_up(&p, Rect::from_min_size(full.min, Vec2::new(full.width(), 58.0)), &[rgb(0x1244b5), rgb(0x1260e9), rgb(0x0058e5), rgb(0x4796ff)]);
    } else if skin == Skin::Modern {
        p.rect_filled(Rect::from_min_size(full.min, Vec2::new(full.width(), 58.0)), 0.0, rgb(0xf5f5f5));
    }
}

fn ink_for(skin: Skin, th: &Theme) -> Color32 {
    match skin {
        Skin::Xp => Color32::WHITE,
        Skin::Modern => rgb(0x181818),
        _ => th.text,
    }
}

pub fn draw(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context) {
    let skin = app.skin;
    let th = skin.theme();
    let state = app.state();
    let in_full = app.in_full;

    // ---- Menu bar ------------------------------------------------------------------------------
    // Fullscreen hides it; the pointer at the top edge reveals it. Linux has no menu bar (themes
    // change with the terminal's theme command). Wallpaper mode has no chrome at all.
    let at_top = in_full && appkit::pointer_near_screen_top();
    let show_bar = skin != Skin::Linux && app.wallpaper.is_none() && (!in_full || at_top);
    let mut picked: Vec<Picked> = Vec::new();
    if show_bar {
        egui::Panel::top("menubar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                for (title, entries) in menus::menu_bar() {
                    ui.menu_button(title, |ui| menus::render(ui, app, &entries, &mut picked));
                }
            });
        });
    }
    for p in picked {
        app.run_picked(p, ctx);
    }

    let full = ui.available_rect_before_wrap();
    let g = geometry(app, full);
    let media_video = g.screen_visible;
    let glass = app.translucent();

    // ---- Window surface ------------------------------------------------------------------------
    let mut regions: Vec<Rect> = Vec::new();
    if media_video {
        regions.push(Rect::from_min_max(full.min, Pos2::new(full.right(), g.screen.top())));
        regions.push(Rect::from_min_max(Pos2::new(full.left(), g.screen.bottom()), full.max));
        regions.push(Rect::from_min_max(Pos2::new(full.left(), g.screen.top()), Pos2::new(g.screen.left(), g.screen.bottom())));
        regions.push(Rect::from_min_max(Pos2::new(g.screen.right(), g.screen.top()), Pos2::new(full.right(), g.screen.bottom())));
    } else {
        regions.push(full);
    }
    let glass_rect = (skin == Skin::Seven && glass).then(|| Rect::from_min_max(Pos2::new(full.left(), g.screen.bottom()), full.max));
    for r in regions {
        let r = match glass_rect {
            // The Windows 7 glass replaces the surface below the screen.
            Some(gr) if r.top() >= gr.top() => continue,
            Some(gr) if r.bottom() > gr.top() => Rect::from_min_max(r.min, Pos2::new(r.right(), gr.top())),
            _ => r,
        };
        surface(ui, app, &th, full, r, glass);
    }
    if let Some(s) = app.surfaces.as_ref() {
        s.place_glass(glass_rect.filter(|r| r.height() > 0.0 && !g.bare));
    }

    let ink = ink_for(skin, &th);
    let has_video = app.engine.as_ref().is_some_and(|e| e.has_video());
    let idle = app.index() < 0;

    // ---- Screen --------------------------------------------------------------------------------
    // The video is a native view behind egui. It is placed over the screen rectangle; egui paints
    // black only when libvlc has no video output (and, in the Linux skin, while nothing plays).
    if let Some(s) = app.surfaces.as_ref() {
        let mut video_rect = media_video.then_some(g.screen).filter(|_| !idle || true);
        #[cfg(windows)]
        {
            // A child window paints over egui: show it only while libvlc has video, hide it under
            // menus and dialogs, and stop it above the fullscreen transport.
            let overlay = egui::Popup::is_any_open(ctx) || !app.dialogs.is_empty();
            let showing = has_video && state <= 4 && !overlay;
            video_rect = video_rect.filter(|_| showing);
            if in_full && app.float_visible {
                video_rect = video_rect.map(|r| Rect::from_min_max(r.min, Pos2::new(r.right(), r.bottom() - 124.0)));
            }
        }
        s.place_video(video_rect);
    }
    if media_video {
        let linux_black = skin == Skin::Linux && !(has_video && state <= 4);
        if !has_video || linux_black {
            ui.painter().rect_filled(g.screen, 0.0, Color32::BLACK);
        }
        match skin {
            Skin::Seven => {
                ui.painter().rect_stroke(g.screen, 0.0, Stroke::new(2.0, rgb(0x829db5)), egui::StrokeKind::Inside);
            }
            Skin::Xp => {
                ui.painter().rect_stroke(g.screen, 0.0, Stroke::new(4.0, rgb(0x1256ce)), egui::StrokeKind::Inside);
            }
            _ => {}
        }
        // cone: the VLC skin, until something is queued. pausedMark: paused or stopped (syncPausedMark).
        if skin == Skin::Vlc && idle {
            if let Some(cone) = &app.tex.cone {
                egui::Image::new(cone).paint_at(ui, Rect::from_center_size(g.screen.center(), Vec2::splat(CONE_SIZE)));
            }
        }
        let paused_or_stopped = state == 4 || state == 5;
        let mark = if skin == Skin::Vlc {
            (paused_or_stopped && !idle).then(|| app.tex.cone.clone()).flatten()
        } else if paused_or_stopped || idle {
            app.tex.logo.clone()
        } else {
            None
        };
        if let Some(mark) = mark {
            egui::Image::new(&mark).paint_at(ui, Rect::from_center_size(g.screen.center(), Vec2::splat(MARK_SIZE)));
        }
    }

    // ---- Media layout (XP / Windows 7) ---------------------------------------------------------
    if g.media {
        media::draw(app, ui, ctx, &g, &th);
    }

    // ---- Top row -------------------------------------------------------------------------------
    // Hidden in the VLC skin (edge) and in the media layout (its header replaces it).
    if !g.bare && !g.media && skin != Skin::Vlc {
        let mut open = false;
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(g.top_row).layout(egui::Layout::left_to_right(egui::Align::Center)));
        row.spacing_mut().item_spacing.x = 10.0;
        widgets::bold(&mut row, &app.name, 18.0, ink);
        row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            if tbutton(ui, skin, &th, "Open Files…", Role::Titled { open: true }).clicked() {
                open = true;
            }
            ui.add(egui::Label::new(egui::RichText::new(&app.status).size(crate::fs(12.0)).color(ink)));
        });
        if open {
            app.open_files(true);
        }
    }

    // ---- Docked playlist (View → Playlist, not in the media layout) -----------------------------
    if let Some(dock) = g.dock {
        ui.painter().rect_filled(dock, 0.0, th.panel);
        media::playlist_table(app, ui, ctx, dock, &th, false);
    }
    if app.playlist_shown && app.store.undocked_playlist && app.regular() && !skin.has_media_layout() {
        media::playlist_window(app, ctx, &th);
    }

    // ---- Timeline and transport ----------------------------------------------------------------
    if !g.bare {
        timeline(app, ui, &g, &th);
        transport(app, ui, ctx, &g, &th, state);
        if app.regular() && app.store.advanced {
            advanced_controls(app, ui, ctx, &g, &th);
        }
        if app.regular() && app.store.status_bar {
            let clock = clock_text(app);
            let mut row = ui.new_child(egui::UiBuilder::new().max_rect(g.status_row).layout(egui::Layout::left_to_right(egui::Align::Center)));
            row.label(egui::RichText::new(format!("{}   {}", app.status, clock)).size(crate::fs(11.0)).color(th.text));
        }
    }

    // ---- Fullscreen floating transport ---------------------------------------------------------
    if in_full && skin != Skin::Linux && app.float_visible {
        floating_transport(app, ctx, &th, state);
    }
}

fn clock_text(app: &App) -> String {
    let (t, d) = app.engine.as_ref().map_or((0, 0), |e| (e.time(), e.length()));
    format!("{} / {}", stamp(t), if d > 0 { stamp(d) } else { "—".into() })
}

fn timeline(app: &mut App, ui: &mut egui::Ui, g: &Geometry, th: &Theme) {
    let (t, d, seekable) = app.engine.as_ref().map_or((0, 0, false), |e| (e.time(), e.length(), e.seekable()));
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(g.timeline).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.spacing_mut().item_spacing.x = 10.0;
    let bar_w = (g.timeline.width() - CLOCK_W - 10.0).max(40.0);
    let shown = app.scrub.unwrap_or(t);
    let frac = if d > 0 { (shown as f64 / d as f64).clamp(0.0, 1.0) } else { 0.0 };
    // seek.continuous = NO: the engine seeks on release (scrub:), the knob follows the pointer meanwhile.
    let (pointer, released) = bar(&mut row, frac, seekable && d > 0, bar_w, th);
    if let Some(f) = pointer {
        app.scrub = Some((f * d as f64) as i64);
        if released {
            let target = (f * d as f64) as i64;
            app.scrub = None;
            app.seek(target);
        }
    }
    let clock = format!("{} / {}", stamp(app.scrub.unwrap_or(t)), if d > 0 { stamp(d) } else { "—".into() });
    row.add_sized(Vec2::new(CLOCK_W, SLIDER_H), egui::Label::new(egui::RichText::new(clock).font(FontId::new(crate::fs(12.0), FontFamily::Proportional)).color(th.text)));
}

/// The queue popup under the Queue button (queuePopup:): every item, the current one checked.
fn queue_popup(app: &mut App, resp: &egui::Response) {
    let names: Vec<String> = (0..app.count()).map(|i| app.engine.as_ref().map(|e| e.display(i)).unwrap_or_default()).collect();
    let current = app.index();
    let mut launch = None;
    egui::Popup::menu(resp).show(|ui| {
        if names.is_empty() {
            ui.add_enabled(false, egui::Button::new("Queue is empty"));
        }
        for (i, name) in names.iter().enumerate() {
            let label = if current == i as isize { format!("✓  {name}") } else { format!("     {name}") };
            if ui.button(label).clicked() {
                launch = Some(i);
                ui.close();
            }
        }
    });
    if let Some(i) = launch {
        app.launch(i);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum It {
    Play,
    Prev,
    Stop,
    Next,
    Full,
    Queue,
    Repeat,
    Shuffle,
    Spacer,
    VolLabel,
    Vol,
}

/// applyTheme's arrangement of the bottom row.
fn order(skin: Skin) -> Vec<It> {
    use It::*;
    match skin {
        Skin::Seven => vec![Shuffle, Repeat, Stop, Prev, Play, Next, Vol, Full],
        Skin::Xp => vec![Play, Stop, Prev, Next, Vol, Repeat, Shuffle, Full],
        _ => vec![Play, Prev, Stop, Next, Full, Queue, Repeat, Shuffle, Spacer, VolLabel, Vol],
    }
}

fn item_title(app: &App, it: It, playing: bool) -> String {
    match it {
        It::Play => if playing { "Pause" } else { "Play" }.into(),
        It::Prev => "Previous".into(),
        It::Stop => "■".into(),
        It::Next => "Next".into(),
        It::Full => "⛶".into(),
        It::Queue => "Queue".into(),
        It::Repeat => if app.repeat_one { "↻ 1" } else { "↻" }.into(),
        It::Shuffle => if app.shuffle { "⤨ ✓" } else { "⤨" }.into(),
        _ => String::new(),
    }
}

fn transport(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context, g: &Geometry, th: &Theme, state: i32) {
    let skin = app.skin;
    let playing = (1..=3).contains(&state);
    let items = order(skin);
    let spacing = skin.transport_spacing();
    // Windows 7 draws a capsule behind the row (transportCapsule).
    let widths: Vec<f32> = items
        .iter()
        .map(|it| match it {
            It::Spacer => 0.0,
            It::VolLabel => text_width(ui, "Volume", 12.0) + 4.0,
            It::Vol => VOLUME_W,
            other => widgets::button_width(ui, &item_title(app, *other, playing)),
        })
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + spacing * (items.len().saturating_sub(1)) as f32;
    let row_rect = if skin == Skin::Seven {
        let x = g.bottom.center().x - total / 2.0;
        Rect::from_min_size(Pos2::new(x, g.bottom.top()), Vec2::new(total, BUTTON_H))
    } else {
        g.bottom
    };
    if skin == Skin::Seven && g.media {
        let capsule = Rect::from_min_max(Pos2::new(row_rect.left() - 8.0, row_rect.top() + 2.0), Pos2::new(row_rect.right() + 8.0, row_rect.bottom() - 2.0));
        let p = ui.painter();
        gradient_up(&p.with_clip_rect(capsule), capsule, &[rgb(0xc8d8e8), rgb(0xf7faff), rgb(0xe0ebf7)]);
        p.rect_stroke(capsule, 15.0, Stroke::new(1.0, rgb(0x91a7b9)), egui::StrokeKind::Inside);
    }
    let mut run = Vec::new();
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(row_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.spacing_mut().item_spacing.x = spacing;
    let spacer_at = items.iter().position(|i| *i == It::Spacer);
    let mut vol_changed = None;
    for (idx, it) in items.iter().enumerate() {
        let title = item_title(app, *it, playing);
        match it {
            It::Play => {
                if tbutton(&mut row, skin, th, &title, Role::Play { playing }).clicked() {
                    run.push(Picked::Action(menus::Action::PlayPause));
                }
            }
            It::Prev => {
                if tbutton(&mut row, skin, th, &title, Role::Prev).clicked() {
                    run.push(Picked::Action(menus::Action::Previous));
                }
            }
            It::Next => {
                if tbutton(&mut row, skin, th, &title, Role::Next).clicked() {
                    run.push(Picked::Action(menus::Action::Next));
                }
            }
            It::Stop => {
                if tbutton(&mut row, skin, th, &title, Role::Titled { open: false }).clicked() {
                    run.push(Picked::Action(menus::Action::Stop));
                }
            }
            It::Full => {
                if tbutton(&mut row, skin, th, &title, Role::Titled { open: false }).clicked() {
                    run.push(Picked::Action(menus::Action::Fullscreen));
                }
            }
            It::Queue => {
                let r = tbutton(&mut row, skin, th, &title, Role::Titled { open: false });
                queue_popup(app, &r);
            }
            It::Repeat => {
                let r = tbutton(&mut row, skin, th, &title, Role::Titled { open: false });
                if r.clicked() {
                    app.repeat_one = !app.repeat_one;
                }
                r.on_hover_text("Repeat current item");
            }
            It::Shuffle => {
                let r = tbutton(&mut row, skin, th, &title, Role::Titled { open: false });
                if r.clicked() {
                    app.shuffle = !app.shuffle;
                }
                r.on_hover_text("Shuffle queue");
            }
            It::Spacer => {
                // The flexible NSView: everything after it sits at the right edge.
                let after: f32 = widths[idx + 1..].iter().sum::<f32>() + spacing * (items.len() - idx - 1) as f32;
                let used = row.min_rect().width();
                let gap = (row_rect.width() - used - after - spacing).max(0.0);
                row.add_space(gap);
            }
            It::VolLabel => {
                row.add(egui::Label::new(egui::RichText::new("Volume").size(crate::fs(12.0)).color(th.text)));
            }
            It::Vol => {
                vol_changed = widgets::volume(&mut row, skin, th, app.volume);
            }
        }
    }
    let _ = spacer_at;
    if let Some(v) = vol_changed {
        app.set_volume(v);
    }
    for p in run {
        app.run_picked(p, ctx);
    }
}

/// extraControls: Record, Snapshot, −10 s, +10 s (advancedButton:).
fn advanced_controls(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context, g: &Geometry, th: &Theme) {
    let skin = app.skin;
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(g.extra_row).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.spacing_mut().item_spacing.x = 10.0;
    for (title, key) in [("Record", "record"), ("Snapshot", "snapshot"), ("−10 s", "-10"), ("+10 s", "10")] {
        if tbutton(&mut row, skin, th, title, Role::Titled { open: false }).clicked() {
            if key.ends_with("10") {
                let r = app.engine.as_mut().map(|e| e.menu_action("jump", key)).unwrap_or_default();
                if !r.is_empty() {
                    app.note("Jump", r);
                }
            } else {
                let k: &'static str = if key == "record" { "record" } else { "snapshot" };
                app.run_picked(Picked::Command { key: k, value: String::new(), title: title.into() }, ctx);
            }
        }
    }
}

/// showFloatingTransport: seek bar over Previous, Play, Next, volume and Exit Full Screen, in a
/// 550 × 76 HUD panel 28 pt above the bottom edge.
fn floating_transport(app: &mut App, ctx: &egui::Context, th: &Theme, state: i32) {
    let skin = app.skin;
    let (t, d, seekable) = app.engine.as_ref().map_or((0, 0, false), |e| (e.time(), e.length(), e.seekable()));
    let playing = (1..=3).contains(&state);
    let mut run = Vec::new();
    let mut seek_to = None;
    let mut vol = None;
    let mut exit = false;
    egui::Area::new(egui::Id::new("floating-transport"))
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -28.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new().fill(Color32::from_black_alpha(200)).corner_radius(10.0).inner_margin(egui::Margin::symmetric(12, 6)).show(ui, |ui| {
                ui.set_width(526.0);
                let frac = if d > 0 { (t as f64 / d as f64).clamp(0.0, 1.0) } else { 0.0 };
                let (pointer, released) = bar(ui, frac, seekable && d > 0, 526.0, th);
                if let (Some(f), true) = (pointer, released) {
                    seek_to = Some((f * d as f64) as i64);
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if tbutton(ui, skin, th, "Previous", Role::Prev).clicked() {
                        run.push(menus::Action::Previous);
                    }
                    let title = if playing { "Pause" } else { "Play" };
                    if tbutton(ui, skin, th, title, Role::Play { playing }).clicked() {
                        run.push(menus::Action::PlayPause);
                    }
                    if tbutton(ui, skin, th, "Next", Role::Next).clicked() {
                        run.push(menus::Action::Next);
                    }
                    vol = widgets::volume(ui, skin, th, app.volume);
                    if tbutton(ui, skin, th, "Exit Full Screen", Role::Titled { open: false }).clicked() {
                        exit = true;
                    }
                });
            });
        });
    if let Some(ms) = seek_to {
        app.seek(ms);
    }
    if let Some(v) = vol {
        app.set_volume(v);
    }
    for a in run {
        app.run_action(a, ctx);
    }
    if exit {
        app.full(ctx);
    }
}
