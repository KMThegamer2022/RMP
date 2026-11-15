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

## Round 2: default theme, main window only

Target: the shipping app's default theme (`VLC_THEME` in `themes.h`, the "VLC" skin that a fresh
launch uses). Window 900×570 content, min 720×380. Screenshots are in `screenshots/`:
`shipping-default-theme.png`, `egui-spike-default-theme.png`, and `side-by-side.png`. Both were taken
on the same fixture at the same window size.

### Matches
- Colours from `themes.h` (panel `f2f2f2`, edge `b4b4b4`, face `f8f8f8`, text `535353`, accent `57bb39`,
  wedge `e2e2e2` / `4bc23b` / `777777`), light appearance.
- Layout from `ui.m`/`arrangeChrome`: black screen filling the top, timeline row (16 pt inset, 6 pt
  above the transport row), transport row 12 pt from the bottom, 8 pt spacing, "Volume" label and
  85 pt wedge slider at the right edge. The top "RMP" row is hidden in this theme, as in the shipping app.
- Video is letterboxed by libvlc inside the black screen, as in the shipping app.
- Transport buttons: rounded 2 pt, 1 pt edge, icons drawn as shapes (play, pause, prev, next, stop,
  fullscreen); Queue, ↻ and ⤨ are text.
- Clock: monospaced 12 pt `m:ss / m:ss`, or `h:mm:ss` past an hour.
- Window title follows the open file (`RMP — open-video.mp4`).

### Architecture change that made video and controls coexist
The engine's video view is a child `NSView` created by the spike and added inside eframe's content
view. It is positioned over the black screen rect every frame. Before this, libvlc was given the whole
content view, so the video covered the controls.
Hiding that child when nothing is loaded shows the cone (egui draws the cone PNG from `assets/`).

### Verified by running
- Window opens at 900×570 and plays `open-video.mp4`.
- Play/Pause click (CoreGraphics mouse events): icon switches Play/Pause, and the time freezes when paused.
- Seek click on the timeline: knob and clock jump to the clicked point (≈0:14 of 0:30) and stay paused.
- Volume click on the wedge: fill changes to the clicked level.
- The screenshots above.

### Not verified
- Stop, Previous, Next, Fullscreen, Queue, repeat and shuffle clicks. They are coded, not exercised.
- Space and Cmd+O shortcuts.
- Drag gestures on the seek bar and the wedge.
- Audio output (the libvlc message "No active audio output" still appears in logs).
- Queue and repeat/shuffle do nothing yet (spike has one-item queues).

### What still differs from the shipping app
- Title bar text is faint in the spike. The window's appearance was set to Aqua, but the title colour
  still reads light. The shipping title is black.
- Seek bar knob is a little smaller; the fill and rail colours are close but not exact.
- Icon weights and sizes for Previous/Next/Stop and the fullscreen corners are hand-drawn, not the
  system glyphs. The shipping app uses the glyphs.
- Paused state: the shipping app keeps the frozen frame and overlays the RMP cone. The spike shows
  the frozen frame with no overlay, since egui cannot draw above the video view.
- Idle state (no file): the shipping app shows the cone on black. The spike has the same code path, but
  this was not captured side by side.
- Button widths and gaps are within a few points of the shipping app but not measured exactly.
- The window is a plain egui window, not a full-size content view. Traffic lights and the bottom
  edge match, but the title bar is still a system title bar.
- Top row (name, status, Open) is absent in this theme, as in the shipping app. The spike opens files
  with Cmd+O or the Open dialog from there; it has no menu bar.
- Status line text is not shown; the shipping default theme does not show it either.
