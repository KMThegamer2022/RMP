# egui spike (macOS) — notes

Branch `egui-spike`. Not a release. The shipping AppKit app (`ui.m`, `mac-*.h`, `updates/*`) and the
Windows app are unchanged.

## What the spike is

- `egui-spike/` is a standalone crate (its own `[workspace]`, own `Cargo.lock`). It keeps egui out of
  the shipping `rmp` package, so `build.sh` / `--locked` builds are unaffected.
- `Cargo.toml [lib] path = "../src/lib.rs"`: the existing engine is compiled unchanged as this crate's
  library, so its `ap_*` functions are called directly from Rust (`src/main.rs`).
- `build.rs` links `libvlc` from `/Applications/VLC.app/Contents/MacOS/lib` (override with `VLC_APP`) and
  sets an rpath to it.
- Run: `cargo run -- /absolute/path/to/file.mp4`, or open a file with the Open… button.
  `RMP_SPIKE_SELFTEST=1` runs a scripted check of the engine calls and logs to stderr.

## What worked (verified by running)

- `cargo build` succeeds in `egui-spike/` (no warnings) and in the repo root (`rmp`, unchanged).
- Window opens; the eframe/egui 0.37 app runs on macOS (Metal/glow via eframe default).
- `packaging/fixtures/open-video.mp4` (30 s) plays through `ap_add` / `ap_play`.
  Self-test log: state 3 (playing) at 1.9 s, advancing.
- Seek: `ap_seek` to 50% moved time from ~2.9 s to 15.0 s; playback continued to ~17.9 s after 2 s.
- Pause: `ap_toggle` moved state 3 → 4 and froze time at 18655 ms.

## What did not work / problems found

- **Plugin path.** `libvlc_new` returns null when run from `target/` unless `VLC_PLUGIN_PATH` is set.
  The shipping app avoids this by bundling `plugins/` beside the executable. The spike sets
  `VLC_PLUGIN_PATH` to VLC.app's plugins directory at startup. Any real egui build needs a bundled
  plugins directory (or this same env setup).
- libvlc logged `No active audio output` when playback started. Time advances, so the clock runs, but
  **audio was not heard or measured**. Treat audio output as unverified.
- Video: the NSView from eframe is passed as the engine view (`ap_create`). Whether video draws inside
  the egui window was **not** verified visually (the spike window was not captured on screen).
- Engine `ap_create(view)` with an `NSView` from eframe is untested beyond not crashing.

## Not verified

- Clicking the UI controls (Open…, Play/Pause, Stop, seek-bar drag, volume slider) with a real mouse.
  Only the engine calls behind them were exercised, through the self-test.
- Volume: `ap_volume(20)` was called, but the engine has no getter and the spike cannot read libvlc's
  volume without private engine state. Actual loudness is unverified.
- Seek-bar drag behaviour (`scrub` while dragging, seek on release) — coded, not exercised by hand.
- Windows, themes, and every feature below.

## Full parity still needs

- **Themes**: `themes.h` and `mac-theme.h` palettes and the Windows 7 theme styling.
- **Menus**: the menus in `mac-menus.h` / `src/vlc_menu.rs` (programmes, renderers, subtitles, tracks,
  output devices, snapshots, etc.). The spike has none of these.
- **Library**: `mac-library.h` and `src/media_library.rs` (library browse, library play).
- **Queue**: `ap_add`/`ap_playlist_save`/`ap_playlist_load`/`ap_library_play` UI; the spike adds one item.
- **Terminal**: `mac-terminal.h` and `src/terminal.rs`.
- **Preferences**: none exist in the spike.
- **Fullscreen, shortcuts, drag and drop, media keys, now-playing info**: `mac-fullscreen.h` and friends.
- **Video embedding**: confirm libvlc draws into the eframe NSView, or decide on a separate video surface.
- **Audio**: confirm the audio output works in this build and bundle the plugin set.
- **Windows**: the spike is macOS-only. The Win32 app and `libvlc_media_player_set_hwnd` path need an
  equivalent, and `windows/` needs its own parity list.
- **Updates, packaging, signing**: `updates/*`, `build.sh`, and the `.app` bundle layout.
- A parity checklist and switch-over decision come later, with the user.
