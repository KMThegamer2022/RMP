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

## Round 3: all themes, View → Theme, saved choice

Screenshots: `screenshots/side-by-side-<slug>.png` (shipping on the left, spike on the right), with the
single captures `shipping-<slug>.png` and `egui-spike-<slug>.png`. Slugs: `vlc`, `rmp-default`, `win10`,
`win7`, `xp`, `mac`, `linux`. All captured at 900×570 content on the same fixture.

- Themes come from `themes.h` (colours, radius, dark/light appearance). Shipping order and `PlayerTheme`
  indices are kept: VLC=5, RMP Default=4, Windows 10/11=0, Windows 7=1, Windows XP=2, Mac=3, Linux=6.
- View → Theme is an in-window egui menu. The choice is saved to
  `~/Library/Application Support/RMP egui spike/theme.txt` and restored at launch.
  `RMP_SPIKE_THEME=<slug>` overrides it for one run without saving (used for the screenshots).
- The shipping app was driven with `-PlayerTheme <index>` as a command-line default, so its prefs were
  not written.
- Title bar: transparent, with the appearance (light or dark) and background colour from the theme.
  Title shows `RMP — <file>`.
- Seek, volume and clock work in each theme. Verified by CoreGraphics clicks in VLC (earlier round) and
  XP (seek to about 0:18; volume knob moved). Other themes use the same `bar()` code path, but were not
  clicked individually.

### Differences by theme
- Window translucency: shipping Windows 7 is translucent (aero), so the desktop shows through the top.
  The spike is opaque.
- Windows XP: silver and blue gradients are approximations. Buttons are circles with a flat silver
  fill, not gradients. The shipping title bar is beige; the spike's matches that colour.
- Windows 7: orb play button is a flat circle. The shipping bottom row is centred; the spike is left-aligned.
- Windows 10/11 and Mac: flat fills match the colours; spacing and fonts are close but not exact.
- Mac: button widths follow the text. Shipping uses system titles with a different weight.
- RMP Default and Linux: colours match. Linux in the shipping app opens a separate terminal window and
  hides the controls; the spike keeps the controls visible, so there is no terminal.
- The paused RMP mark overlay and the idle "welcome" label are not drawn.
- Menus: the spike has a small in-window menu bar (File, View). The shipping app uses the native macOS
  menu bar, which is outside the window, so the spike's content starts about 22 pt lower.
- Fonts are SF (via `SFNS.ttf`), not the system font API; glyph shapes differ slightly.

### Still missing (in this order)
1. **Menus**: the full native menu bar (File, Edit, View, Playback, Window, Help, plus the items each one holds in `ui.m`
   and `mac-menus.h`). Only File → Open and View → Theme exist.
2. **Library**: `mac-library.h` / `src/media_library.rs` (browse, remember, play from library).
3. **Queue**: the queue popup, playlist open and save (`ap_playlist_save/load`), shuffle and repeat behaviour.
   Repeat and shuffle buttons do nothing yet.
4. **Terminal**: `mac-terminal.h`, `src/terminal.rs`, and the Linux skin's terminal window.
5. **Preferences**: none yet, beyond the theme choice.
6. **Keyboard shortcuts**: only Space (play/pause) and Cmd+O (open) exist. The shipping app's key equivalents
   (`mac-menus.h`) are not mapped.

### Not verified
- Each theme's seek, volume and clock by clicks (only VLC and XP were clicked).
- Stop, Previous, Next, Fullscreen, Queue, repeat and shuffle in every theme.
- Audio output.

## Round 4: menus

Menus follow `installPlayerMenus` in `mac-menus.h`: Media, Playback, Audio, Video, Subtitle, Tools, View, Help.
Source of truth: `src/menus.rs` (spike). Engine-backed entries call the engine's exported
`ap_menu_action` and `ap_menu_options`, as the shipping app does. Native-only items (Open File/Multiple,
Open Playlist/Save, Open Network Stream…, Jump to Specific Time…, Play/Pause, Stop, Previous/Next, Fullscreen,
Quit, About) are in the spike.

### Works
- All eight menus with the shipping items and order, including separators.
- Engine submenus (Audio Track, Audio Device, Stereo Mode, Visualizations, Video Track, Title, Chapter,
  Programme, Zoom, Aspect Ratio, Crop, Deinterlace, Subtitle Track…) list rows from the engine.
  Verified: Audio Track shows "None available" for the fixture, same as the shipping app.
- View → Theme and its saved choice (from round 3).
- Help menu matches the shipping items: Help, Check for Updates…, About RMP.
- Engine commands (Mute, Increase/Decrease Volume, etc.) run through `ap_menu_action`; result text is
  shown in a note window like `menuNote`.
- Keyboard shortcuts for the native items: Space (play/pause), Cmd+O, Cmd+Shift+O, Cmd+U, Cmd+T, Cmd+E,
  Cmd+L, Ctrl+Cmd+F, Cmd+Q, `[` and `]`.
- Open Recent Media: list is kept in `recent.txt` under Application Support (max 20).

### Verified by running
- Menu bar renders and Audio, Help and Audio Track menus open (screenshots in `screenshots/menus/`).
- Space pauses and resumes (before/after frames differ in the play icon and time).
- Side-by-side Audio Track submenu with the shipping app's native menu.

### Not verified
- Each menu action (Jump, Stop, Next, Open dialogs, Save Playlist, Open Stream) by clicking.
- Keyboard shortcuts other than Space.
- Engine Options rows with real tracks (the fixture has none).

### Differences from the shipping app
- The shipping app uses the native macOS menu bar at the top of the screen. The spike draws its menus
  inside the window. This is a decision for the user (see report).
- While a menu or dialog is open, the spike hides the video view (the video is a native view above egui),
  so the screen goes black until the menu closes.
- Dialog windows are egui windows, not native sheets.

### Still missing
- Media library, queue, terminal, preferences, keyboard shortcut customisation.

## Round 5: Linux theme, stopped logo, fullscreen

Screenshots: `screenshots/round5/`.

### Linux theme (ui.m `arrangeChrome`, mac-terminal.h)
- Layout is bare: the screen fills the window, with no top row, timeline or transport. The menu bar stays.
- Black screen unless a video frame is playing (`idleBlack`). The RMP logo shows when stopped.
- RMP Terminal opens as its own window with the Linux skin. It runs the engine's `ap_terminal_*` calls:
  command input, output, prompt, history (Up/Down), Tab completion, and actions (clear, fullscreen, theme,
  volume). The welcome text and prompt match `mac-terminal.h`.
- Verified by running: black screen with video playing; `help` lists the shipping commands; `stop`
  prints "Stopped." and shows the logo.
- Not verified: `theme`, `volume`, `queue`, `seek`, and history and Tab completion by typing.
- Difference: the windowed Linux layout has no seek or volume controls, matching the shipping bare layout.

### Stopped / paused mark (syncPausedMark)
- VLC skin shows the cone (96 pt) when paused or stopped with a file loaded; idle shows the 120 pt cone.
- Other skins show the RMP logo (96 pt) when paused, stopped, or idle.
- Verified by running: VLC stopped (cone), Windows 10/11 stopped (logo), Linux stopped (logo).

### Fullscreen (mac-fullscreen.h)
- Fullscreen hides the menu bar, the top row, the timeline and the transport. The screen fills the window.
- A floating transport (seek, Previous, Play/Pause, Next, volume, Exit Full Screen) shows for 2.5 s after the
  mouse moves. The menu bar shows again when the pointer is at the top edge.
- Verified by running: transport visible after mouse movement; hidden after 2.5 s without movement.
- Verified by running (round 12): Ctrl+Cmd+F enters and leaves fullscreen; double-clicking the screen enters it.
- Not verified: the Exit Full Screen button after the rewrite.
- Linux fullscreen shows no floating transport, as in the shipping code (`refreshFloating` returns for Linux).

### Known issues
- AppKit logs "adding an unknown subview" when the video view is attached to the frame view. The
  layering still works.

### Rewrite verification (round 12)
The spike was rewritten to cover the shipping app's behaviour (store, menus, commands, dialogs, library,
queue, update check, About). Checked by running it, with the state dump from the script hook:
- Shortcuts: Space (play/pause), Cmd+T (jump-to-time dialog), Cmd+L (playlist toggle), Cmd+U (network
  stream dialog), Ctrl+Cmd+F (fullscreen both ways), Cmd+Q (quits).
- Queue button shows a popup with a check on the current item (`screenshots/round12/queue-popup.png`).
- Double-click on the screen enters fullscreen.
- Windows 7: opens on the Library page; library search filters; Current Playlist page lists the queue;
  playing from the library switches to Now Playing when video starts.
- About panel and "up to date" update message (`screenshots/round12/`); wallpaper mode state toggles.
- Not verified by running: `]` and `[` seek keys, Edit-menu paste into a text field, the capture-device
  dialog, native file dialogs, the wallpaper window visually, the update install (a non-bundle cannot replace itself).
- Differences that remain: menu bar is in-window (egui) rather than the native one; font rendering
  differs slightly; shipping app has no shortcut customisation, so none here.
