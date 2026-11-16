//! Alerts and panels, laid out like the shipping NSAlerts (menuNote, askValue, configureView,
//! effectsPanel, openCapture, the VLsub and update prompts). The first button is the default and
//! sits rightmost; Return presses it and Escape presses the last button.

use crate::App;
use eframe::egui::{self, Color32, RichText, Vec2};

pub enum AskPurpose {
    NetworkStream,
    StreamOut,
    Time,
}

pub enum Dialog {
    Note { title: String, detail: String },
    Ask { title: String, detail: String, value: String, placeholder: String, purpose: AskPurpose },
    Capture { devices: Vec<(String, String)>, selected: usize },
    Effects { sync: bool, eq_rows: Vec<(String, String)>, eq_selected: Option<usize>, fields: Vec<(String, String, String)> },
    Configure { preferences: bool, advanced: bool, status: bool, docked: bool, volume: i32 },
    Vlsub,
    UpdateOffer { version: String, notes: String },
}

/// One NSAlert: icon, bold title, optional text, accessory view, buttons. Returns the pressed button.
fn alert(
    ctx: &egui::Context,
    app: &App,
    id: &str,
    title: &str,
    detail: &str,
    buttons: &[&str],
    body: impl FnOnce(&mut egui::Ui),
) -> Option<usize> {
    let mut pressed = None;
    let modal = egui::Modal::new(egui::Id::new(("alert", id))).backdrop_color(Color32::TRANSPARENT).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.horizontal_top(|ui| {
            if let Some(logo) = &app.tex.logo {
                ui.add(egui::Image::new(logo).fit_to_exact_size(Vec2::splat(64.0)));
            }
            ui.vertical(|ui| {
                ui.set_width(290.0);
                ui.label(RichText::new(title).strong().size(crate::fs(13.0)));
                if !detail.is_empty() {
                    ui.label(RichText::new(detail).size(crate::fs(11.0)));
                }
                ui.add_space(8.0);
                body(ui);
            });
        });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (i, label) in buttons.iter().enumerate() {
                let mut b = egui::Button::new(*label).min_size(Vec2::new(72.0, 22.0));
                if i == 0 {
                    b = b.fill(Color32::from_rgb(0x0a, 0x84, 0xff));
                    let text = RichText::new(*label).color(Color32::WHITE);
                    b = egui::Button::new(text).fill(Color32::from_rgb(0x0a, 0x84, 0xff)).min_size(Vec2::new(72.0, 22.0));
                }
                if ui.add(b).clicked() {
                    pressed = Some(i);
                }
            }
        });
    });
    let _ = modal;
    if pressed.is_none() {
        pressed = app.force_button.take();
    }
    if pressed.is_none() {
        let (enter, esc) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if enter {
            pressed = Some(0);
        } else if esc {
            pressed = Some(buttons.len() - 1);
        }
    }
    pressed
}

/// A long message goes in a scrolling text view, like menuNote's NSScrollView.
fn long_text(detail: &str) -> bool {
    detail.len() > 360 || detail.lines().count() > 7
}

pub fn draw(app: &mut App, ctx: &egui::Context) {
    if app.dialogs.is_empty() {
        return;
    }
    let mut dialog = app.dialogs.remove(0);
    let keep = step(app, ctx, &mut dialog);
    if keep {
        app.dialogs.insert(0, dialog);
    }
}

/// Draws one dialog; returns true while it should stay open.
fn step(app: &mut App, ctx: &egui::Context, dialog: &mut Dialog) -> bool {
    match dialog {
        Dialog::Note { title, detail } => {
            let scroll = long_text(detail);
            let text = detail.clone();
            let info = if scroll { String::new() } else { detail.clone() };
            let r = alert(ctx, app, "note", title, &info, &["OK"], |ui| {
                if scroll {
                    egui::ScrollArea::vertical().max_height(255.0).show(ui, |ui| {
                        ui.add(egui::Label::new(RichText::new(&text).size(crate::fs(12.0))).wrap());
                    });
                }
            });
            r.is_none()
        }
        Dialog::Ask { title, detail, value, placeholder, purpose } => {
            let r = alert(ctx, app, "ask", title, detail, &["OK", "Cancel"], |ui| {
                let edit = egui::TextEdit::singleline(value).hint_text(placeholder.as_str()).desired_width(f32::INFINITY);
                let resp = ui.add(edit);
                if !resp.has_focus() {
                    resp.request_focus();
                }
            });
            match r {
                Some(0) => {
                    let (v, t) = (value.clone(), title.clone());
                    match purpose {
                        AskPurpose::NetworkStream => app.open_stream(&v),
                        AskPurpose::StreamOut => app.stream_out(&v, &t),
                        AskPurpose::Time => app.jump_to_time(&v, &t),
                    }
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
        Dialog::Capture { devices, selected } => {
            let names: Vec<String> = devices.iter().map(|d| d.0.clone()).collect();
            let r = alert(ctx, app, "capture", "Open Capture Device", "Choose the source to play.", &["Play", "Cancel"], |ui| {
                egui::ComboBox::from_id_salt("capture-devices")
                    .width(300.0)
                    .selected_text(names.get(*selected).cloned().unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            ui.selectable_value(selected, i, n);
                        }
                    });
            });
            match r {
                Some(0) => {
                    let source = devices.get(*selected).map(|d| d.1.clone()).unwrap_or_default();
                    app.capture_chosen(source);
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
        Dialog::Effects { sync, eq_rows, eq_selected, fields } => {
            let title = if *sync { "Track Synchronization" } else { "Effects and Filters" };
            let r = alert(ctx, app, "effects", title, "", &["Apply", "Cancel"], |ui| {
                if !eq_rows.is_empty() {
                    ui.label(RichText::new("Equalizer preset").size(crate::fs(12.0)));
                    let current = eq_selected.and_then(|i| eq_rows.get(i)).map(|r| r.1.clone()).unwrap_or_default();
                    egui::ComboBox::from_id_salt("eq-preset").width(180.0).selected_text(current).show_ui(ui, |ui| {
                        for (i, (_, name)) in eq_rows.iter().enumerate() {
                            ui.selectable_value(eq_selected, Some(i), name);
                        }
                    });
                }
                for (_, label, value) in fields.iter_mut() {
                    ui.label(RichText::new(label.as_str()).size(crate::fs(12.0)));
                    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
                }
            });
            match r {
                Some(0) => {
                    let eq = eq_selected.and_then(|i| eq_rows.get(i)).map(|r| r.0.clone());
                    let f = fields.iter().map(|(k, _, v)| (k.clone(), v.clone())).collect();
                    app.apply_effects(*sync, eq, f);
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
        Dialog::Configure { preferences, advanced, status, docked, volume } => {
            let title = if *preferences { "RMP Preferences" } else { "Customize Interface" };
            let pref = *preferences;
            let r = alert(ctx, app, "configure", title, "", &["Apply", "Cancel"], |ui| {
                ui.checkbox(advanced, "Show advanced playback controls");
                ui.checkbox(status, "Show status bar");
                ui.checkbox(docked, "Dock playlist in player");
                if pref {
                    ui.label(RichText::new("Startup volume").size(crate::fs(12.0)));
                    ui.add(egui::Slider::new(volume, 0..=100).show_value(false));
                }
            });
            match r {
                Some(0) => {
                    let (a, s, d, v) = (*advanced, *status, *docked, *volume);
                    app.apply_configure(a, s, d, pref.then_some(v));
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
        Dialog::Vlsub => {
            let r = alert(
                ctx,
                app,
                "vlsub",
                "Search for Subtitles",
                "Open a browser search on OpenSubtitles using the current media title. After downloading a subtitle, choose Subtitle → Add Subtitle File.",
                &["Search in Browser", "Cancel"],
                |_| {},
            );
            match r {
                Some(0) => {
                    app.subtitle_search();
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
        Dialog::UpdateOffer { version, notes } => {
            let title = format!("Update to RMP {version}?");
            let n = notes.clone();
            let r = alert(
                ctx,
                app,
                "update",
                &title,
                "RMP will download the update and reopen. If this copy is read-only, the update will be installed in your personal Applications folder.",
                &["Update", "Not now"],
                |ui| {
                    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                        ui.add(egui::Label::new(RichText::new(&n).size(crate::fs(12.0))).wrap());
                    });
                },
            );
            match r {
                Some(0) => {
                    // The shipping app downloads and replaces RMP.app. This build is not a bundle, so
                    // there is nothing to replace; say so instead of pretending.
                    app.note(
                        "RMP could not update",
                        "This copy of RMP is the egui preview and is not an installed app bundle, so it cannot replace itself. Download the release from GitHub.",
                    );
                    false
                }
                Some(_) => false,
                None => true,
            }
        }
    }
}
