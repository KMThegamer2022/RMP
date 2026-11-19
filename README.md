<div align="center">

<img src="assets/RMP.png" width="128" alt="RMP icon">

# RMP · Rust Media Player

**A fast, standalone media player for Apple Silicon Macs and Windows x64.**
Built on a Rust playback core and libVLC, with native interfaces on each platform.

[![Latest release](https://img.shields.io/github/v/release/KMThegamer2022/RMP?style=for-the-badge&color=ff3c00)](https://github.com/KMThegamer2022/RMP/releases/latest)
[![Windows build](https://img.shields.io/github/actions/workflow/status/KMThegamer2022/RMP/windows.yml?style=for-the-badge&label=Windows%20build)](https://github.com/KMThegamer2022/RMP/actions/workflows/windows.yml)

[**Download RMP**](https://github.com/KMThegamer2022/RMP/releases/latest) · [All releases](https://github.com/KMThegamer2022/RMP/releases)

</div>

---

## ✨ Highlights

| | |
|---|---|
| 🎞️ **Queue & playlists** | Open one file or many. Save and reopen your queue in M3U format. |
| 🎨 **Six themes** | VLC, RMP Default, Windows 10/11, Windows 7, Windows XP, Mac, and Linux. |
| 🖥️ **Native interfaces** | AppKit on Mac, Win32 on Windows, with Aero glass on Windows 7. |
| 🐧 **Linux terminal theme** | A simulated command window (`ls`, `cd`, `play`, `theme`) beside the video. |
| ⬇️ **Built-in updates** | Checks GitHub releases, verifies the SHA-256 digest, installs, and reopens. |
| 🎹 **MIDI playback** | Standard MIDI files play offline on Windows, using the bundled General MIDI SoundFont. |

## 🚀 Getting started

| Platform | Download | Install |
|---|---|---|
| 🍎 **Mac** (Apple Silicon) | `RMP-Setup-…-macOS-arm64.dmg` | Open the disk image and run **RMP Setup**. It installs to `~/Applications`. |
| 🪟 **Windows** (x64, Windows 7 SP1 and newer) | `RMP-Setup-x64.exe` | Runs per user, with a Start menu shortcut and a Programs and Features entry. |
| 📦 **Windows portable** | `RMP-Windows-x64.zip` | Extract the whole ZIP to one folder and run `RMP.exe`. Does not register with Windows. |

> New installations start with the **VLC** theme. You can change it any time under **View → Theme**.

> **Windows 7 users:** RMP runs on Windows 7 SP1, but it has not been tested on Windows 7 yet. YouTube links need Windows 8.1 or newer, because the bundled yt-dlp does not start on Windows 7.

## ⌨️ Controls

| Action | Mac | Windows |
|---|---|---|
| Open file | `⌘O` | `Ctrl+O` |
| Open multiple files | `⇧⌘O` | `Ctrl+Shift+O` |
| Open network stream | `⌘U` | `Ctrl+N` |
| Play / pause | `Space` | `Space` |
| Previous / next item | `⌘[` / `⌘]` | Previous / Next buttons |
| Toggle full screen | `⌃⌘F` | `F11` or double-click the video |

In fullscreen, the floating controls hide after 2.5 seconds and return when the pointer moves. Press Escape or the fullscreen control to leave.

Use **Media → Open File** for one item, or **Open Multiple Files** to add several. **View → Playlist** shows the playback order. **Media → Open Network Stream** accepts a direct media URL. **Audio** and **Subtitle** list the tracks provided by the current media.

## 🎨 Themes

| Theme | Look | Notes |
|---|---|---|
| **VLC** | Compact, light transport | Default for new installs. Keeps the VLC cone while paused and stopped. |
| **RMP Default** | RMP's own default look | — |
| **Windows 10/11** | Modern Windows look | — |
| **Windows 7** | Aero library | Searchable library with a centered playback bar. Opens video in Now Playing. |
| **Windows XP** | Now Playing layout | Queue beside the video, with a navigation strip. |
| **Mac** | AppKit | Behind-window visual effect on the playback bar. |
| **Linux** | Terminal + playback window | See below. |

Changing themes keeps the current media and playback position, and RMP remembers your choice on this computer.

Open **View → Theme** from the menu bar on either platform. The Windows XP and Windows 7 themes draw on screenshots of [XP Explorer](https://gekk.info/articles/explorer.html), [Windows Media Player 12 on Windows 7](https://harisnadeem.com/2009/01/29/windows-7-screenshots-eye-candy/windows-media-player-12/), and [Films & TV on Windows 10](https://learn.microsoft.com/en-us/answers/questions/2813678/video-playback-stretched-over-windows-media-player).

### 🐧 Linux terminal theme

Select **View → Theme → Linux** to open the RMP command window beside the playback window. Switching themes keeps the current track playing.

```text
rmp:~$ cd ~/Music
rmp:~$ ls
rmp:~$ play "Live Set.flac"
rmp:~$ theme back
```

- Type `help` for the built-in commands.
- `theme back` restores your previous theme. `theme vlc` switches directly.
- Tab completes filenames, and ↑ recalls earlier commands.
- It interprets input inside RMP only. There's no system shell or program launcher.

## 🔄 Updating

RMP checks GitHub releases once per launch. A newer stable release with a matching download opens a release-notes prompt.

1. Choose **Update** (**Yes** on Windows) to download the release.
2. RMP verifies the asset's SHA-256 digest against the GitHub release.
3. It shows a progress bar and a short countdown, then installs and reopens.
4. Choose **Not now** (**No** on Windows) to be asked again next launch.

On macOS, an update launched from a read-only location installs into your `~/Applications` folder. If you're upgrading from 2.2 or earlier, install the current release manually once.

## 🛠️ Build from source

**Mac:** install a recent stable Rust toolchain, the Xcode Command Line Tools, and an arm64 VLC 3.0.x app at `/Applications/VLC.app`. Then run:

```bash
./build.sh                  # creates build/RMP.app
VLC_APP=/path/to/VLC.app ./build.sh   # use another VLC bundle
```

VLC is needed only at build time. The resulting app carries its own libraries and plugins.

**Project layout**
- `src/lib.rs`: the queue, playback state and libVLC integration (Rust).
- `ui.m`: the native AppKit interface (Objective-C).
- `windows/`: the Win32 interface and installer.
- `.github/workflows/windows.yml`: the Windows build, tests and packaging.

This Mac build is locally signed and hasn't been notarized by Apple.

## ✅ Verification

- **Mac:** tested locally with H.264 video and AAC audio in MKV, embedded SRT subtitles, FLAC audio, and fast-start MP4 over HTTP.
- **Windows:** GitHub Actions builds the x64 executable, bundles the pinned VideoLAN runtime, and runs playback, installation, installed playback and uninstallation tests on Windows Server 2022.
- **Windows 7 SP1:** visual and runtime testing is still pending. The bundled yt-dlp needs Windows 8.1 or newer, so on Windows 7 YouTube links are handed straight to LibVLC, which may not resolve them.

Format and stream compatibility depend on the codecs, encryption and server behaviour involved.

## 🪟 Windows details

- The interface uses Segoe UI and themed Win32 controls.
- On a Windows 7 Aero desktop, the frame extends behind the bottom playback controls. A solid frame is used when composition is unavailable or High Contrast is on.
- The Windows 7 theme uses `DwmExtendFrameIntoClientArea` with only a bottom margin, so Windows and the compositor decide the final look.
- RMP's Windows installer registers it under **Default apps**. Choose **Rust Media Player** in **Settings → Apps → Default apps**, or pick it from **Open with** for a media file.
- Standard MIDI files (`.mid`, `.midi`) play through the normal file, queue and playback controls. Windows includes the TimGM6mb General MIDI SoundFont for offline synthesis.

## 🍎 Mac details

- Choose a media file in Finder, then **Get Info → Open with → RMP → Change All** to make RMP the default.
- On macOS, the playback bar uses a behind-window visual effect while transparency is enabled.
- Window management buttons and system dialogs follow the host OS.
- Standard MIDI files use the system AudioToolbox instruments.

---

<div align="center">

Made by [KMThegamer2022](https://github.com/KMThegamer2022) · [Report an issue](https://github.com/KMThegamer2022/RMP/issues)

</div>
