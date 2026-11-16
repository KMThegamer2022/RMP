//! The Windows Media Player-style layout for the XP and Windows 7 skins (mac-library.h): the header
//! with breadcrumb, search and Open files…, the navigation pane, and the library / playlist table.
//! Also the docked and floating "RMP Playlist" (mac-menus.h makeViewControls).

use crate::engine::display_name;
use crate::layout::{Geometry, NAV_W};
use crate::menus::{Action, Picked};
use crate::theme::{rgb, Skin, Theme};
use crate::widgets::gradient_up;
use crate::{appkit, App, MediaPage};
use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

/// MediaPane.drawRect: the gradient for the pane's style and a 1 pt border.
#[derive(Clone, Copy)]
enum Pane {
    XpHeader,
    XpNav,
    XpQueue,
    SevenHeader,
    SevenNav,
}

fn paint_pane(ui: &egui::Ui, rect: Rect, pane: Pane) {
    let p = ui.painter().with_clip_rect(rect);
    let colors: Vec<Color32> = match pane {
        Pane::XpHeader => vec![rgb(0x455c9b), rgb(0x536eb1), rgb(0x0b53c7), rgb(0x2c87ff)],
        Pane::XpNav => vec![rgb(0x8293bf), rgb(0xe4e9f7), rgb(0xc4d0eb)],
        Pane::XpQueue => vec![rgb(0x39436b), rgb(0x5f70a8)],
        Pane::SevenHeader => vec![rgb(0xc8d8e8), rgb(0xf7faff), rgb(0xe0ebf7)],
        Pane::SevenNav => vec![rgb(0xdce8f2), rgb(0xf5f9fe)],
    };
    gradient_up(&p, rect, &colors);
    if matches!(pane, Pane::XpHeader) {
        // The glossy strip along the bottom edge (NSMakeRect(1, 1, w - 2, 7), origin at the bottom).
        let strip = Rect::from_min_max(Pos2::new(rect.left() + 1.0, rect.bottom() - 8.0), Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0));
        gradient_up(&p, strip, &[rgb(0x687eae), rgb(0xdfebf9)]);
    }
    ui.painter().rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.0, rgb(0x91a6be)), egui::StrokeKind::Inside);
}

fn crumb(skin: Skin, page: MediaPage) -> &'static str {
    let i = page.index();
    if skin == Skin::Xp {
        ["RMP  ›  Now Playing", "Media Library", "Current Playlist"][i]
    } else {
        ["Now Playing", "Library  ›  Media  ›  All media", "Library  ›  Playlists  ›  Current playlist"][i]
    }
}

pub fn draw(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context, g: &Geometry, th: &Theme) {
    let skin = app.skin;
    let xp = skin == Skin::Xp;
    let playing_page = app.media_page == MediaPage::NowPlaying;
    let sidebar = xp && playing_page;
    let ink = if xp { Color32::WHITE } else { rgb(0x183b60) };

    // Header: breadcrumb, search (not on Now Playing), Open files….
    paint_pane(ui, g.header, if xp { Pane::XpHeader } else { Pane::SevenHeader });
    let mut open_files = false;
    let inner = g.header.shrink2(Vec2::new(12.0, 0.0));
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)));
    row.spacing_mut().item_spacing.x = 10.0;
    crate::widgets::bold(&mut row, crumb(skin, app.media_page), 14.0, ink);
    row.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let r = ui.add(egui::Button::new(RichText::new("Open files…").size(crate::fs(13.0)).color(ink)).frame(false));
        if r.clicked() {
            open_files = true;
        }
        if !playing_page {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.search)
                    .hint_text("Search")
                    .desired_width(158.0)
                    .margin(egui::Margin::symmetric(8, 3)),
            );
            if resp.changed() {
                app.search_cursor = 0;
            }
        }
    });
    if open_files {
        app.open_files(true);
    }

    // Navigation pane.
    paint_pane(ui, g.nav, if xp { Pane::XpNav } else { Pane::SevenNav });
    let mut nav = ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(Pos2::new(g.nav.left() + 10.0, g.nav.top() + 15.0), g.nav.max)));
    nav.spacing_mut().item_spacing.y = 7.0;
    let mut go: Option<MediaPage> = None;
    let mut playlist_action: Option<Action> = None;
    for (title, page, symbol) in [
        ("Now Playing", MediaPage::NowPlaying, "play.rectangle"),
        ("Library", MediaPage::Library, "music.note.house"),
        ("Current Playlist", MediaPage::Playlist, "list.bullet"),
    ] {
        let chosen = app.media_page == page;
        let (rect, resp) = nav.allocate_exact_size(Vec2::new(NAV_W - 20.0, 28.0), Sense::click());
        if chosen {
            nav.painter().rect_filled(rect, 3.0, rgb(if xp { 0xb1c3e6 } else { 0xc4def4 }));
        } else if resp.hovered() {
            nav.painter().rect_filled(rect, 3.0, Color32::from_black_alpha(10));
        }
        if let Some(tex) = app.tex.symbol(ctx, symbol) {
            let icon = Rect::from_center_size(Pos2::new(rect.left() + 14.0, rect.center().y), Vec2::splat(15.0));
            egui::Image::new(&tex).tint(rgb(0x264a6e)).fit_to_exact_size(Vec2::splat(15.0)).paint_at(&nav, icon);
        }
        let font = FontId::new(crate::fs(13.0), FontFamily::Proportional);
        let color = if chosen { rgb(0x2e3550) } else { rgb(0x6b728a) };
        let galley = nav.painter().layout_no_wrap(title.into(), font, color);
        let pos = Pos2::new(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0);
        nav.painter().galley(pos, galley.clone(), color);
        if chosen {
            // Semibold: the same text drawn once more, offset by a fraction of a point.
            nav.painter().galley(pos + Vec2::new(0.5, 0.0), galley, color);
        }
        if resp.clicked() {
            go = Some(page);
        }
    }
    let (sep, _) = nav.allocate_exact_size(Vec2::new(NAV_W - 20.0, 1.0), Sense::hover());
    nav.painter().rect_filled(sep, 0.0, Color32::from_black_alpha(40));
    for (title, action) in [("Open playlist…", Action::OpenPlaylist), ("Save playlist…", Action::SavePlaylist)] {
        let (rect, resp) = nav.allocate_exact_size(Vec2::new(NAV_W - 20.0, 28.0), Sense::click());
        nav.painter().text(Pos2::new(rect.left() + 4.0, rect.center().y), egui::Align2::LEFT_CENTER, title, FontId::new(crate::fs(13.0), FontFamily::Proportional), rgb(0x6b728a));
        if resp.clicked() {
            playlist_action = Some(action);
        }
    }
    if let Some(page) = go {
        // navigateMedia:
        app.pending_video_switch = false;
        app.search_cursor = 0;
        app.media_page = page;
        app.search.clear();
        app.selected_row = None;
    }
    if let Some(a) = playlist_action {
        app.run_picked(Picked::Action(a), ctx);
    }

    // Browser pane: the library or playlist table, or the XP queue column beside the video.
    if let Some(rect) = g.browser {
        paint_pane(ui, rect, if sidebar { Pane::XpQueue } else { Pane::SevenNav });
        browser(app, ui, ctx, rect, sidebar, th);
    }
}

/// Shortens `text` in the middle with an ellipsis until it fits `max_w`.
fn middle_truncate(ui: &egui::Ui, text: &str, max_w: f32) -> String {
    let font = FontId::new(crate::fs(12.0), FontFamily::Proportional);
    let width = |s: &str| ui.painter().layout_no_wrap(s.to_string(), font.clone(), Color32::BLACK).size().x;
    if width(text) <= max_w {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut keep = chars.len();
    while keep > 2 {
        keep -= 1;
        let head = keep.div_ceil(2);
        let tail = keep - head;
        let s: String = chars[..head].iter().collect::<String>() + "…" + &chars[chars.len() - tail..].iter().collect::<String>();
        if width(&s) <= max_w {
            return s;
        }
    }
    "…".to_string()
}

/// Title shown for a path (the last component, percent-decoded).
fn title_of(path: &str) -> String {
    appkit::percent_decode(&display_name(path))
}

fn contains_ci(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}

struct Row {
    index: usize,
    path: String,
}

fn browser(app: &mut App, ui: &mut egui::Ui, ctx: &egui::Context, rect: Rect, sidebar: bool, _th: &Theme) {
    let library = app.media_page == MediaPage::Library;
    let query = if sidebar { String::new() } else { app.search.trim().to_string() };
    let count = app.engine.as_ref().map_or(0, |e| if library { e.library_count() } else { e.count() });
    let mut rows: Vec<Row> = Vec::new();
    for i in 0..count {
        let Some(e) = app.engine.as_ref() else { break };
        let path = if library { e.library_item(i) } else { e.item(i) };
        let Some(path) = path else { continue };
        let keep = query.is_empty() || contains_ci(&path, &query) || app.meta.get(&path).is_some_and(|m| contains_ci(&m.search_text(), &query));
        if keep {
            rows.push(Row { index: i, path });
        }
    }
    let head_color = if sidebar { Color32::WHITE } else { rgb(0x234a71) };
    let mut y = rect.top() + 7.0;
    {
        let label = if library { "All media" } else { "Current playlist" };
        let galley = ui.painter().layout_no_wrap(label.into(), FontId::new(crate::fs(12.0), FontFamily::Proportional), head_color);
        ui.painter().galley(Pos2::new(rect.left() + 8.0, y), galley.clone(), head_color);
        ui.painter().galley(Pos2::new(rect.left() + 8.6, y), galley, head_color);
    }
    y += 16.0;
    if sidebar {
        // queueArt: the current item's cover, or a placeholder symbol, 100 × 98 under the heading.
        let art = Rect::from_center_size(Pos2::new(rect.center().x, y + 3.0 + 49.0), Vec2::new(100.0, 98.0));
        let current = app.engine.as_ref().and_then(|e| {
            let c = e.count();
            (c > 0).then(|| e.item(if e.index() < 0 { 0 } else { e.index() as usize })).flatten()
        });
        let cover = current.as_ref().and_then(|p| app.meta.get(p)).and_then(|m| m.cover).zip(current.clone()).and_then(|(bytes, p)| app.tex.cover(ctx, &p, &bytes));
        match cover {
            Some(tex) => {
                egui::Image::new(&tex).fit_to_exact_size(art.size()).maintain_aspect_ratio(true).paint_at(ui, art);
            }
            None => {
                if let Some(tex) = app.tex.symbol_large(ctx, "music.note.tv") {
                    egui::Image::new(&tex).tint(rgb(0xbac7e8)).fit_to_exact_size(art.size()).maintain_aspect_ratio(true).paint_at(ui, art);
                }
            }
        }
        y = art.bottom() + 4.0;
    } else {
        y += 3.0;
    }
    let table = Rect::from_min_max(Pos2::new(rect.left() + 1.0, y), Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0));
    let bg = if sidebar { rgb(0x38436a) } else { rgb(0xeff5fb) };
    ui.painter().rect_filled(table, 0.0, bg);
    let text_color = if sidebar { Color32::WHITE } else { rgb(0x192f48) };
    let row_h = if sidebar { 25.0 } else { 54.0 };
    let width = table.width();
    // Column widths: Cover 52, Title 205, Album 125, Artist 125, Length 58 (uniform autoresizing).
    let cols: Vec<(&str, f32)> = if sidebar {
        vec![("Title", width)]
    } else {
        let flex = (width - 52.0 - 58.0).max(120.0);
        let unit = flex / (205.0 + 125.0 + 125.0);
        vec![("", 52.0), ("Title", 205.0 * unit), ("Album", 125.0 * unit), ("Artist", 125.0 * unit), ("Length", 58.0)]
    };
    // Header row.
    let header_h = 22.0;
    let header = Rect::from_min_size(table.min, Vec2::new(width, header_h));
    ui.painter().rect_filled(header, 0.0, Color32::WHITE);
    let mut x = header.left();
    for (name, w) in &cols {
        ui.painter().text(Pos2::new(x + 6.0, header.center().y), egui::Align2::LEFT_CENTER, *name, FontId::new(crate::fs(11.0), FontFamily::Proportional), rgb(0x262626));
        x += w;
    }
    ui.painter().line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(1.0, rgb(0xd0d0d0)));
    let body = Rect::from_min_max(Pos2::new(table.left(), header.bottom()), table.max);
    let mut table_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
    let mut play: Option<usize> = None;
    let mut select: Option<usize> = None;
    let selected = app.selected_row;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(&mut table_ui, row_h, rows.len(), |ui, range| {
        for ri in range {
            let row = &rows[ri];
            let (r, resp) = ui.allocate_exact_size(Vec2::new(width, row_h), Sense::click());
            let is_sel = selected == Some(ri);
            if is_sel {
                ui.painter().rect_filled(r, 0.0, rgb(0x2f6fe0));
            }
            let meta = app.meta.get(&row.path).unwrap_or_default();
            let fg = if is_sel { Color32::WHITE } else { text_color };
            let mut x = r.left();
            for (name, w) in &cols {
                let cell = Rect::from_min_size(Pos2::new(x, r.top()), Vec2::new(*w, row_h));
                match *name {
                    "" => {
                        let slot = Rect::from_center_size(cell.center(), Vec2::splat((row_h - 8.0).min(46.0)));
                        let tex = meta.cover.as_ref().and_then(|b| app.tex.cover(ctx, &row.path, b));
                        match tex {
                            Some(t) => {
                                egui::Image::new(&t).fit_to_exact_size(slot.size()).maintain_aspect_ratio(true).paint_at(ui, slot);
                            }
                            None => {
                                if let Some(t) = app.tex.symbol(ctx, "music.note") {
                                    let small = Rect::from_center_size(cell.center(), Vec2::splat(18.0));
                                    egui::Image::new(&t).tint(rgb(0x6892b6)).fit_to_exact_size(small.size()).paint_at(ui, small);
                                }
                            }
                        }
                    }
                    other => {
                        let text = match other {
                            "Title" => meta.title.clone().unwrap_or_else(|| title_of(&row.path)),
                            "Album" => meta.album.clone().unwrap_or_default(),
                            "Artist" => meta.artist.clone().unwrap_or_default(),
                            _ => meta.length.clone().unwrap_or_default(),
                        };
                        // NSLineBreakByTruncatingMiddle (label() in ui.m).
                        let text = middle_truncate(ui, &text, cell.width() - 12.0);
                        let clip = ui.painter().with_clip_rect(Rect::from_min_max(Pos2::new(cell.left() + 4.0, cell.top()), Pos2::new(cell.right() - 4.0, cell.bottom())));
                        clip.text(Pos2::new(cell.left() + 6.0, cell.top() + if row_h > 30.0 { 8.0 } else { row_h / 2.0 }), if row_h > 30.0 { egui::Align2::LEFT_TOP } else { egui::Align2::LEFT_CENTER }, text, FontId::new(crate::fs(12.0), FontFamily::Proportional), fg);
                    }
                }
                if !sidebar && *name != "Length" {
                    ui.painter().line_segment([cell.right_top(), cell.right_bottom()], Stroke::new(1.0, rgb(0xdbe5f0)));
                }
                x += w;
            }
            resp.clone().on_hover_text(&row.path);
            if resp.clicked() {
                select = Some(ri);
            }
            if resp.double_clicked() {
                play = Some(row.index);
            }
        }
    });
    if let Some(s) = select {
        app.selected_row = Some(s);
    }
    if let Some(i) = play {
        // playTableRow:
        if library {
            let ok = app.engine.as_mut().is_some_and(|e| e.library_play(i));
            if ok && app.index() >= 0 {
                app.show_playing_item(app.index() as usize);
            }
        } else {
            app.launch(i);
        }
    }
}

/// The "Current playlist" table of the docked panel and of the floating "RMP Playlist" window.
pub fn playlist_table(app: &mut App, ui: &mut egui::Ui, _ctx: &egui::Context, rect: Rect, th: &Theme, _window: bool) {
    let count = app.count();
    let names: Vec<String> = (0..count).map(|i| app.engine.as_ref().map(|e| e.display(i)).unwrap_or_default()).collect();
    let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 22.0));
    ui.painter().rect_filled(header, 0.0, th.face);
    ui.painter().text(Pos2::new(header.left() + 6.0, header.center().y), egui::Align2::LEFT_CENTER, "Current playlist", FontId::new(crate::fs(11.0), FontFamily::Proportional), th.text);
    ui.painter().line_segment([header.left_bottom(), header.right_bottom()], Stroke::new(1.0, th.edge));
    let body = Rect::from_min_max(Pos2::new(rect.left(), header.bottom()), rect.max);
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(body));
    let selected = app.queue_selected;
    let mut play = None;
    let mut select = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(&mut inner, 20.0, names.len(), |ui, range| {
        for i in range {
            let (r, resp) = ui.allocate_exact_size(Vec2::new(rect.width(), 20.0), Sense::click());
            if selected == Some(i) {
                ui.painter().rect_filled(r, 0.0, th.accent.gamma_multiply(0.5));
            }
            ui.painter().text(Pos2::new(r.left() + 6.0, r.center().y), egui::Align2::LEFT_CENTER, &names[i], FontId::new(crate::fs(12.0), FontFamily::Proportional), th.text);
            if resp.clicked() {
                select = Some(i);
            }
            if resp.double_clicked() {
                play = Some(i);
            }
        }
    });
    if let Some(i) = select {
        app.queue_selected = Some(i);
    }
    if let Some(i) = play {
        app.launch(i);
    }
}

/// queuePanel: a 330 × 340 titled "RMP Playlist"; closing it clears playlistShown.
pub fn playlist_window(app: &mut App, ctx: &egui::Context, th: &Theme) {
    let mut open = true;
    egui::Window::new("RMP Playlist").open(&mut open).default_size([330.0, 340.0]).show(ctx, |ui| {
        let rect = ui.available_rect_before_wrap();
        let rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 300.0));
        playlist_table(app, ui, ctx, rect, th, true);
        ui.allocate_rect(rect, Sense::hover());
    });
    if !open {
        app.playlist_shown = false;
    }
}
