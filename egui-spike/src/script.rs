//! Test hook: RMP_SPIKE_SCRIPT=<file> runs a list of commands and RMP_SPIKE_LOG=<file> receives state
//! dumps. Used to verify behaviour by running the app; it does nothing unless the variable is set.
//!
//! Lines: `wait SECS`, `cmd KEY [VALUE]`, `act NAME`, `theme SLUG`, `open PATH`, `seek MS`, `volume N`,
//! `ask TEXT` (fill the open prompt and press its default button), `press N` (press button N of the
//! open alert), `dump LABEL`, `quit`.

use crate::menus::{Action, Picked};
use crate::theme::Skin;
use crate::App;
use eframe::egui;
use std::io::Write;
use std::time::{Duration, Instant};

pub struct Script {
    lines: Vec<String>,
    next: usize,
    resume: Instant,
    log: Option<std::path::PathBuf>,
}

impl Script {
    pub fn load() -> Option<Script> {
        let path = std::env::var_os("RMP_SPIKE_SCRIPT")?;
        let text = std::fs::read_to_string(path).ok()?;
        Some(Script {
            lines: text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect(),
            next: 0,
            resume: Instant::now() + Duration::from_secs(2),
            log: std::env::var_os("RMP_SPIKE_LOG").map(Into::into),
        })
    }
}

fn action_named(name: &str) -> Option<Action> {
    Some(match name {
        "about" => Action::About,
        "updates" => Action::CheckUpdates,
        "quit" => Action::Quit,
        "open" => Action::OpenSingle,
        "openmulti" => Action::OpenMultiple,
        "stream" => Action::OpenStream,
        "openplaylist" => Action::OpenPlaylist,
        "saveplaylist" => Action::SavePlaylist,
        "play" => Action::PlayPause,
        "stop" => Action::Stop,
        "prev" => Action::Previous,
        "next" => Action::Next,
        "fullscreen" => Action::Fullscreen,
        _ => return None,
    })
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

impl App {
    pub fn dump(&self, label: &str) -> String {
        let e = self.engine.as_ref();
        let titles: Vec<String> = self
            .dialogs
            .iter()
            .map(|d| match d {
                crate::dialogs::Dialog::Note { title, detail } => format!("note:{title}|{}", detail.replace('\n', " ")),
                crate::dialogs::Dialog::Ask { title, .. } => format!("ask:{title}"),
                crate::dialogs::Dialog::Capture { .. } => "capture".into(),
                crate::dialogs::Dialog::Effects { sync, .. } => format!("effects:{sync}"),
                crate::dialogs::Dialog::Configure { preferences, .. } => format!("configure:{preferences}"),
                crate::dialogs::Dialog::Vlsub => "vlsub".into(),
                crate::dialogs::Dialog::UpdateOffer { version, .. } => format!("update:{version}"),
                crate::dialogs::Dialog::Updating { version, .. } => format!("updating:{version}"),
            })
            .collect();
        format!(
            "{label} skin={} state={} time={} length={} volume={} count={} index={} status={:?} title={:?} page={} full={} minimal={} wallpaper={} top={} quit_end={} recording={} adv={} statusbar={} undocked={} playlist={} shuffle={} repeat={} recent={} lib={} dialogs=[{}] seekable={} video={} input=[{}]",
            self.skin.slug(),
            e.map_or(0, |e| e.state()),
            e.map_or(0, |e| e.time()),
            e.map_or(0, |e| e.length()),
            self.volume,
            e.map_or(0, |e| e.count()),
            e.map_or(-1, |e| e.index()),
            self.status,
            self.title,
            self.media_page.index(),
            self.in_full,
            self.minimal,
            self.wallpaper.is_some(),
            self.always_top,
            self.quit_at_end,
            self.recording,
            self.store.advanced,
            self.store.status_bar,
            self.store.undocked_playlist,
            self.playlist_shown,
            self.shuffle,
            self.repeat_one,
            self.store.recent.len(),
            e.map_or(0, |e| e.library_count()),
            titles.join(","),
            e.is_some_and(|e| e.seekable()),
            e.is_some_and(|e| e.has_video()),
            crate::appkit::input_stats(),
        )
    }

    pub fn run_script(&mut self, ctx: &egui::Context) {
        let Some(mut script) = self.script.take() else { return };
        while Instant::now() >= script.resume && script.next < script.lines.len() {
            let line = script.lines[script.next].clone();
            script.next += 1;
            let (op, rest) = line.split_once(' ').unwrap_or((line.as_str(), ""));
            match op {
                "wait" => script.resume = Instant::now() + Duration::from_secs_f64(rest.parse().unwrap_or(1.0)),
                "cmd" => {
                    let (key, value) = rest.split_once(' ').unwrap_or((rest, ""));
                    self.run_picked(Picked::Command { key: leak(key), value: value.to_string(), title: key.to_string() }, ctx);
                }
                "act" => {
                    if let Some(a) = action_named(rest) {
                        self.run_picked(Picked::Action(a), ctx);
                    }
                }
                "theme" => {
                    if let Some(s) = Skin::from_slug(rest) {
                        self.pick_theme(s);
                    }
                }
                "open" => self.accept(vec![(rest.to_string(), !std::path::Path::new(rest).exists())]),
                "libplay" => {
                    let i: usize = rest.parse().unwrap_or(0);
                    let ok = self.engine.as_mut().is_some_and(|e| e.library_play(i));
                    if ok && self.index() >= 0 {
                        self.show_playing_item(self.index() as usize);
                    }
                }
                "shuffle" => self.shuffle = !self.shuffle,
                "repeat" => self.repeat_one = !self.repeat_one,
                "search" => self.search = rest.to_string(),
                "page" => {
                    self.media_page = match rest {
                        "1" => crate::MediaPage::Library,
                        "2" => crate::MediaPage::Playlist,
                        _ => crate::MediaPage::NowPlaying,
                    }
                }
                "shot" => {
                    if let Some(w) = self.surfaces.as_ref().and_then(|s| s.window()) {
                        crate::appkit::screenshot(Some(&w), rest);
                    }
                }
                "shotall" => {
                    crate::appkit::screenshot(None, rest);
                }
                "term" => {
                    if let Some(raw) = self.engine.as_ref().map(|e| e.raw()) {
                        self.term.set_input(rest);
                        self.terminal_submit(ctx, raw);
                    }
                }
                "termdump" => {
                    if let Some(path) = &script.log {
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                            let _ = writeln!(f, "--- terminal transcript ({rest}) ---\n{}\n---", self.term.transcript());
                        }
                    }
                }
                "pick" => self.pick.push(rest.into()),
                "seek" => self.seek(rest.parse().unwrap_or(0)),
                "volume" => self.set_volume(rest.parse().unwrap_or(80)),
                "ask" => {
                    if let Some(crate::dialogs::Dialog::Ask { value, .. }) = self.dialogs.first_mut() {
                        *value = rest.to_string();
                    }
                    self.force_button.set(Some(0));
                }
                "press" => self.force_button.set(Some(rest.parse().unwrap_or(0))),
                "dump" => {
                    let text = self.dump(rest);
                    if let Some(path) = &script.log {
                        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                            let _ = writeln!(f, "{text}");
                        }
                    }
                }
                "quit" => {
                    self.quit_requested = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                _ => {}
            }
            // Give the UI a frame between commands so dialogs and state settle.
            if op != "wait" {
                script.resume = Instant::now() + Duration::from_millis(400);
            }
        }
        self.script = Some(script);
    }
}
