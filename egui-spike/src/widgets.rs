//! Drawing helpers that copy the shipping AppKit controls (ui.m, mac-theme.h).

use crate::theme::{rgb, Skin, Theme};
use eframe::egui::{self, Color32, FontFamily, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2};

pub const BUTTON_H: f32 = 30.0;
pub const SLIDER_H: f32 = 21.0;
pub const VOLUME_W: f32 = 85.0;

/// Gradient drawn the way AppKit's NSGradient drawInRect:angle:90 does: the first colour at the
/// bottom, the last at the top, with the stops evenly spaced.
pub fn gradient_up(painter: &Painter, r: Rect, colors: &[Color32]) {
    if colors.len() < 2 {
        if let Some(c) = colors.first() {
            painter.rect_filled(r, 0.0, *c);
        }
        return;
    }
    let mut m = egui::Mesh::default();
    let n = (colors.len() - 1) as f32;
    for (i, w) in colors.windows(2).enumerate() {
        // stop i is at fraction i/n from the bottom
        let y0 = r.bottom() - r.height() * (i as f32 / n);
        let y1 = r.bottom() - r.height() * ((i + 1) as f32 / n);
        let base = m.vertices.len() as u32;
        m.colored_vertex(Pos2::new(r.left(), y0), w[0]);
        m.colored_vertex(Pos2::new(r.right(), y0), w[0]);
        m.colored_vertex(Pos2::new(r.right(), y1), w[1]);
        m.colored_vertex(Pos2::new(r.left(), y1), w[1]);
        m.add_triangle(base, base + 1, base + 2);
        m.add_triangle(base, base + 2, base + 3);
    }
    painter.add(Shape::mesh(m));
}

/// Circle filled with a bottom-to-top gradient (XP silver buttons, the Windows 7 orb).
pub fn gradient_circle(painter: &Painter, center: Pos2, radius: f32, colors: &[Color32]) {
    let segments = 48;
    let mut m = egui::Mesh::default();
    let color_at = |y: f32| -> Color32 {
        // y in [-radius, radius], -radius = top
        let t = ((radius - y) / (2.0 * radius)).clamp(0.0, 1.0); // 0 bottom .. 1 top
        let n = (colors.len() - 1) as f32;
        let pos = t * n;
        let i = (pos.floor() as usize).min(colors.len() - 2);
        let f = pos - i as f32;
        lerp(colors[i], colors[i + 1], f)
    };
    let c_idx = 0u32;
    m.colored_vertex(center, color_at(0.0));
    for k in 0..=segments {
        let a = k as f32 / segments as f32 * std::f32::consts::TAU;
        let p = Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin());
        m.colored_vertex(p, color_at(p.y - center.y));
    }
    for k in 1..=segments as u32 {
        m.add_triangle(c_idx, k, k + 1);
    }
    painter.add(Shape::mesh(m));
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Icon {
    Play,
    Pause,
    Prev,
    Next,
}

/// TransportButton glyphs for skins other than Mac: ▶ / Ⅱ at 17 pt, ❘◀ and ▶❘ at 13 pt.
fn draw_icon(painter: &Painter, c: Pos2, icon: Icon, ink: Color32) {
    let tri = |pts: Vec<Pos2>| Shape::convex_polygon(pts, ink, Stroke::NONE);
    match icon {
        Icon::Play => {
            painter.add(tri(vec![Pos2::new(c.x - 4.5, c.y - 6.5), Pos2::new(c.x + 6.5, c.y), Pos2::new(c.x - 4.5, c.y + 6.5)]));
        }
        Icon::Pause => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x - 3.0, c.y), Vec2::new(2.5, 12.0)), 0.0, ink);
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x + 3.0, c.y), Vec2::new(2.5, 12.0)), 0.0, ink);
        }
        Icon::Prev => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x - 5.5, c.y), Vec2::new(1.5, 9.0)), 0.0, ink);
            painter.add(tri(vec![Pos2::new(c.x + 5.0, c.y - 4.5), Pos2::new(c.x - 3.5, c.y), Pos2::new(c.x + 5.0, c.y + 4.5)]));
        }
        Icon::Next => {
            painter.rect_filled(Rect::from_center_size(Pos2::new(c.x + 5.5, c.y), Vec2::new(1.5, 9.0)), 0.0, ink);
            painter.add(tri(vec![Pos2::new(c.x - 5.0, c.y - 4.5), Pos2::new(c.x + 3.5, c.y), Pos2::new(c.x - 5.0, c.y + 4.5)]));
        }
    }
}

/// Bold text. egui cannot select a heavier weight of the system font, so the glyphs are drawn twice,
/// a fraction of a point apart (how a faux-bold looks at 12–18 pt).
pub fn bold(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.into(), FontId::new(size, FontFamily::Proportional), color);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + Vec2::new(0.7, 0.0), Sense::hover());
    ui.painter().galley(rect.min, galley.clone(), color);
    ui.painter().galley(rect.min + Vec2::new(0.7, 0.0), galley, color);
    resp
}

/// Width of a button for `title`: NSButton measures SF text wider than egui does (about 1.22×),
/// plus TransportButton's +14 and the cell padding. Measured on the shipping VLC skin.
pub fn button_width(ui: &egui::Ui, title: &str) -> f32 {
    text_width(ui, title, 13.0) * if cfg!(target_os = "macos") { 1.09 } else { 1.0 } + 16.0
}

pub fn text_width(ui: &egui::Ui, caption: &str, size: f32) -> f32 {
    ui.painter()
        .layout_no_wrap(caption.into(), FontId::new(size, FontFamily::Proportional), Color32::BLACK)
        .size()
        .x
}

/// What a transport button is: it decides the glyph, the XP silver look and the Seven orb.
#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Play { playing: bool },
    Prev,
    Next,
    /// Any other titled button; `open` is "Open Files…" (XP draws it without the silver treatment).
    Titled { open: bool },
}

/// TransportButton.drawRect from ui.m, per skin. Width is the title's width + 14 (intrinsicContentSize),
/// or the glyph caption's width for transport buttons outside the Mac skin.
pub fn tbutton(ui: &mut egui::Ui, skin: Skin, th: &Theme, title: &str, role: Role) -> egui::Response {
    let weight_font = FontId::new(crate::fs(13.0), FontFamily::Proportional);
    let transport = matches!(role, Role::Play { .. } | Role::Prev | Role::Next);
    // Width: the title's intrinsic width (text + 2) plus 14 (TransportButton.intrinsicContentSize).
    // Measured against the shipping VLC skin: Pause 53, Previous 69, ■ 29, Next 45 points.
    let width = button_width(ui, title);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, BUTTON_H), Sense::click());
    let painter = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    let mut r = rect.shrink2(Vec2::new(1.5, 2.0));
    let is_play = matches!(role, Role::Play { .. });
    let orb = skin == Skin::Seven && is_play;
    let silver = skin == Skin::Xp && !matches!(role, Role::Titled { open: true });
    if orb || silver {
        let d = r.width().min(r.height());
        r = Rect::from_center_size(r.center(), Vec2::splat(d));
    }
    let mut face = th.face;
    let mut edge = th.edge;
    let mut text = th.text;
    if skin == Skin::RmpDefault && is_play {
        face = th.accent;
        edge = rgb(0xdd6238);
        text = rgb(0x16191f);
    }
    let radius = if orb || silver { r.height() / 2.0 } else { th.radius };
    match skin {
        Skin::Xp | Skin::Seven => {
            if silver {
                gradient_circle(painter, r.center(), radius, &[rgb(0xf4f7fd), rgb(0x9daec7), rgb(0xe1eaf7), rgb(0xffffff)]);
            } else if orb {
                gradient_circle(painter, r.center(), radius, &[rgb(0x103369), rgb(0x2585df), rgb(0x1953ae), rgb(0xd8f7ff)]);
            } else {
                let clip = painter.with_clip_rect(r);
                let bottom = if pressed { edge } else { face };
                // Rounded rect fill with gradient: fill the rounded shape first, then the gradient clipped.
                painter.rect_filled(r, radius, face);
                gradient_up(&clip, r.shrink2(Vec2::new(0.0, radius.min(3.0) * 0.0)), &[bottom, face, th.highlight]);
            }
        }
        _ => {
            painter.rect_filled(r, radius, if pressed { edge } else { face });
        }
    }
    if orb || silver {
        painter.circle_stroke(r.center(), radius, Stroke::new(1.0, edge));
    } else if skin != Skin::Modern {
        painter.rect_stroke(r, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
    }
    if skin == Skin::Xp && !silver && !pressed {
        painter.rect_stroke(r.shrink(1.0), 2.0, Stroke::new(1.0, Color32::WHITE), egui::StrokeKind::Inside);
    }
    let ink = if orb { Color32::WHITE } else { text };
    if transport && skin != Skin::Mac {
        let icon = match role {
            Role::Play { playing } => {
                if playing {
                    Icon::Pause
                } else {
                    Icon::Play
                }
            }
            Role::Prev => Icon::Prev,
            _ => Icon::Next,
        };
        draw_icon(painter, r.center(), icon, ink);
    } else {
        if skin == Skin::Modern {
            // NSFontWeightMedium in the Windows 10/11 skin: drawn twice, a fraction of a point apart.
            painter.text(r.center() + Vec2::new(0.4, 0.0), egui::Align2::CENTER_CENTER, title, weight_font.clone(), ink);
        }
        painter.text(r.center(), egui::Align2::CENTER_CENTER, title, weight_font, ink);
    }
    resp.on_hover_text(title)
}

/// NSSlider in the shipping style: thin rail filled with the accent colour, round knob.
/// Returns (fraction while the pointer is pressed, whether it was released).
pub fn bar(ui: &mut egui::Ui, frac: f64, enabled: bool, width: f32, th: &Theme) -> (Option<f64>, bool) {
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, SLIDER_H), sense);
    let track = Rect::from_min_max(Pos2::new(rect.left() + 8.0, rect.center().y - 2.0), Pos2::new(rect.right() - 8.0, rect.center().y + 2.0));
    let painter = ui.painter();
    let rail = if th.dark { rgb(0x555555) } else { rgb(0xd0d0d0) };
    painter.rect_filled(track, 2.0, rail);
    let x = track.left() + track.width() * frac.clamp(0.0, 1.0) as f32;
    let fill = if enabled { th.accent } else { th.accent.gamma_multiply(0.4) };
    painter.rect_filled(Rect::from_min_max(track.min, Pos2::new(x, track.max.y)), 2.0, fill);
    let knob = if th.dark { rgb(0xdddddd) } else { Color32::WHITE };
    painter.circle(Pos2::new(x, rect.center().y), 8.0, knob, Stroke::new(1.0, rgb(0x9a9a9a)));
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

/// VolumeSlider.scrollWheel: precise deltas move by delta × 0.3, wheel clicks by 5.
pub fn wheel_step(ui: &egui::Ui, resp: &egui::Response) -> Option<f64> {
    if !resp.hovered() {
        return None;
    }
    let (delta, precise) = ui.input(|i| {
        let mut d = 0.0f32;
        let mut precise = true;
        for e in &i.events {
            if let egui::Event::MouseWheel { unit, delta, .. } = e {
                d += delta.y;
                if *unit != egui::MouseWheelUnit::Point {
                    precise = false;
                }
            }
        }
        (d, precise)
    });
    if delta == 0.0 {
        return None;
    }
    Some(if precise { delta as f64 * 0.3 } else { 5.0f64.copysign(delta as f64) })
}

/// Volume control: the wedge in the VLC skin (VolumeSlider.wedge), the accent slider elsewhere.
/// Returns the new volume when it changed.
pub fn volume(ui: &mut egui::Ui, skin: Skin, th: &Theme, value: i32) -> Option<i32> {
    let mut v = value;
    let resp;
    if skin == Skin::Vlc {
        let (rect, r) = ui.allocate_exact_size(Vec2::new(VOLUME_W, SLIDER_H), Sense::click_and_drag());
        let inner = rect.shrink2(Vec2::new(4.0, 2.0));
        if r.dragged() || r.clicked() {
            if let Some(pos) = r.interact_pointer_pos() {
                v = ((pos.x - inner.left()) / inner.width() * 100.0).round().clamp(0.0, 100.0) as i32;
            }
        }
        let tri = vec![Pos2::new(inner.left(), inner.bottom()), Pos2::new(inner.right(), inner.top()), Pos2::new(inner.right(), inner.bottom())];
        let painter = ui.painter();
        painter.add(Shape::convex_polygon(tri.clone(), rgb(0xe2e2e2), Stroke::NONE));
        let fill_w = inner.width() * v as f32 / 100.0;
        painter.with_clip_rect(Rect::from_min_size(inner.min, Vec2::new(fill_w, inner.height()))).add(Shape::convex_polygon(tri.clone(), rgb(0x4bc23b), Stroke::NONE));
        painter.add(Shape::closed_line(tri, Stroke::new(1.0, rgb(0x777777))));
        resp = r;
    } else {
        let start_x = ui.cursor().min.x;
        let (pointer, _) = bar(ui, v as f64 / 100.0, true, VOLUME_W, th);
        if let Some(f) = pointer {
            v = (f * 100.0).round() as i32;
        }
        let rect = Rect::from_min_size(Pos2::new(start_x, ui.min_rect().top()), Vec2::new(VOLUME_W, SLIDER_H));
        resp = ui.interact(rect, ui.id().with("volume-wheel"), Sense::hover());
    }
    if let Some(step) = wheel_step(ui, &resp) {
        v = (v as f64 + step).round().clamp(0.0, 100.0) as i32;
    }
    (v != value).then_some(v)
}
