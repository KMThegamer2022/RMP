//! egui spike: drives the existing Rust engine (src/lib.rs, the `ap_*` API) from an egui window.
//! macOS only. Does not touch the shipping AppKit app.

// The engine (repo-root src/lib.rs) is this crate's library; see Cargo.toml [lib].
use egui_spike as engine;

use eframe::egui;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::{c_void, CString};
use std::path::PathBuf;

struct App {
    /// Frames since the file started; drives the optional self-test.
    selftest: Option<(u32, Vec<String>)>,
    start: std::time::Instant,
    player: *mut engine::Player,
    // Set on first frame, when the native view is available. Video renders into it.
    attached: bool,
    file: Option<PathBuf>,
    status: String,
    volume: i32,
    scrub: Option<i64>,
}

impl App {
    fn new() -> Self {
        Self {
            selftest: std::env::var_os("RMP_SPIKE_SELFTEST").map(|_| (0, Vec::new())),
            start: std::time::Instant::now(),
            player: std::ptr::null_mut(),
            attached: false,
            file: None,
            status: "Open a media file to begin.".into(),
            volume: 80,
            scrub: None,
        }
    }

    fn player(&self) -> Option<&mut engine::Player> {
        // SAFETY: the pointer comes from ap_create and lives until Drop; the spike is single-threaded (UI thread only).
        unsafe { self.player.as_mut() }
    }

    fn open_file(&mut self, path: PathBuf) {
        let Some(p) = self.player() else { return };
        let Ok(c) = CString::new(path.to_string_lossy().as_bytes()) else {
            self.status = "Path contains a NUL byte.".into();
            return;
        };
        unsafe {
            engine::ap_stop(p);
            let before = engine::ap_count(p);
            engine::ap_add(p, c.as_ptr(), false);
            let ok = engine::ap_play(p, before);
            let err = libvlc_errmsg();
            let err = if err.is_null() { String::new() } else { std::ffi::CStr::from_ptr(err).to_string_lossy().into_owned() };
            eprintln!("open_file: ap_play returned {ok}, count={}, libvlc error: {err:?}", engine::ap_count(p));
            self.status = if ok {
                format!("Playing {}", path.display())
            } else {
                format!("Could not play {}", path.display())
            };
        }
        self.file = Some(path);
    }

    fn toggle(&mut self) {
        let Some(p) = self.player() else { return };
        let state = unsafe { engine::ap_state(p) };
        if state == 0 || state == 5 || state == 6 || state == 7 {
            // Nothing loaded, stopped, ended, or errored: restart the current item.
            if let Some(i) = usize::try_from(engine::ap_index(p)).ok() {
                unsafe { engine::ap_play(p, i) };
            }
        } else {
            unsafe { engine::ap_toggle(p) };
        }
    }

    fn set_volume(&mut self, v: i32) {
        self.volume = v;
        if let Some(p) = self.player() {
            unsafe { engine::ap_volume(p, v) };
        }
    }
}

impl App {
    /// Scripted check of the engine calls (seek, volume, pause), logged to stderr.
    /// Runs only when RMP_SPIKE_SELFTEST is set. It is not a substitute for clicking the UI.
    fn run_selftest(&mut self) {
        let Some((stage, log)) = self.selftest.as_mut() else { return };
        let secs = self.start.elapsed().as_secs_f32();
        let next = match *stage {
            0 if secs >= 3.0 => "after 3s playback",
            1 if secs >= 4.0 => "volume set to 20",
            2 if secs >= 6.0 => "seek to 50% requested",
            3 if secs >= 9.0 => "2s after seek",
            4 if secs >= 10.0 => "pause toggled",
            5 if secs >= 12.0 => "after pause",
            _ => return,
        };
        let Some(p) = (unsafe { self.player.as_mut() }) else { return };
        match *stage {
            1 => unsafe { engine::ap_volume(p, 20) },
            2 => unsafe { engine::ap_seek(p, engine::ap_length(p) / 2) },
            4 => unsafe { engine::ap_toggle(p) },
            _ => {}
        }
        let snap = unsafe {
            format!(
                "{next}: state={} time={}ms length={}ms seekable={}",
                engine::ap_state(p), engine::ap_time(p), engine::ap_length(p), engine::ap_seekable(p)
            )
        };
        log.push(snap);
        *stage += 1;
        if *stage == 6 {
            for l in log.iter() { eprintln!("SELFTEST {l}"); }
            std::process::exit(0);
        }
    }
}

extern "C" {
    fn libvlc_errmsg() -> *const std::ffi::c_char;
}

fn fmt_time(ms: i64) -> String {
    let s = (ms.max(0) / 1000) as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        if !self.attached {
            self.attached = true;
            // Hand the native NSView to the engine so video draws inside the egui window.
            // ap_create takes a view handle; ensure_engine attaches it on first play.
            let view = frame
                .window_handle()
                .ok()
                .and_then(|h| match h.as_raw() {
                    RawWindowHandle::AppKit(a) => Some(a.ns_view.as_ptr() as *mut c_void),
                    _ => None,
                })
                .unwrap_or(std::ptr::null_mut());
            self.player = unsafe { engine::ap_create(view) };
            if let Some(p) = self.player() {
                engine::ap_prepare(p);
                unsafe { engine::ap_volume(p, self.volume) };
            }
            // Optional: open a file given on the command line (used for verification runs).
            if let Some(path) = std::env::args_os().nth(1) {
                self.open_file(PathBuf::from(path));
            }
        }

        self.run_selftest();

        // Keep the readout moving while something is loaded.
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Media", &["mp4", "mkv", "mov", "mp3", "flac", "wav", "m4a", "avi", "webm", "ogg"])
                        .pick_file()
                    {
                        self.open_file(path);
                    }
                }
                let playing = self.player().map(|p| unsafe { engine::ap_state(p) }) == Some(3);
                let label = if playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    self.toggle();
                }
                if ui.button("Stop").clicked() {
                    if let Some(p) = self.player() {
                        unsafe { engine::ap_stop(p) };
                    }
                }
            });
        });

        egui::Panel::bottom("transport").show(ui, |ui| {
            let (time, length, seekable) = match self.player() {
                Some(p) => unsafe { (engine::ap_time(p), engine::ap_length(p), engine::ap_seekable(p)) },
                None => (0, 0, false),
            };
            let max = length.max(1) as f64;
            let mut pos = self.scrub.unwrap_or(time).clamp(0, length.max(0)) as f64;
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!("{} / {}", fmt_time(self.scrub.unwrap_or(time)), fmt_time(length)));
                ui.spacing_mut().slider_width = (ui.available_width() - 200.0).max(120.0);
                let resp = ui.add_enabled(
                    seekable && length > 0,
                    egui::Slider::new(&mut pos, 0.0..=max).show_value(false),
                );
                if resp.dragged() {
                    self.scrub = Some(pos as i64);
                }
                if resp.drag_stopped() {
                    if let Some(t) = self.scrub.take() {
                        if let Some(p) = self.player() {
                            unsafe { engine::ap_seek(p, t) };
                        }
                    }
                }
                ui.label("Vol");
                let mut vol = self.volume;
                if ui.add(egui::Slider::new(&mut vol, 0..=100)).changed() {
                    self.set_volume(vol);
                }
            });
            ui.label(&self.status);
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Video plays in the native view below this panel when the file has video.");
            ui.label(match &self.file {
                Some(f) => f.display().to_string(),
                None => "No file open".into(),
            });
        });
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
        viewport: egui::ViewportBuilder::default().with_inner_size([820.0, 480.0]).with_title("RMP egui spike"),
        ..Default::default()
    };
    eframe::run_native(
        "RMP egui spike",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
