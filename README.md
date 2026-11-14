# RMP · Rust Media Player

<img src="assets/RMP.png" width="120" alt="RMP icon">

Download the packaged app from [GitHub Releases](https://github.com/KMThegamer2022/RMP/releases).
A standalone native media player for Apple Silicon Macs. Open RMP.app, choose **Open Files…**, and select one or more media files. The Queue menu lists files in playback order. Choosing additional files appends them and starts the first new item. Use File → Open Network Stream for a direct media URL. Use Audio Track and Subtitles to select available embedded tracks during playback.

Space toggles playback. Command-O opens files, Command-U opens a stream, Command-[ and Command-] change queue items, and Control-Command-F toggles full screen.

## Build

On an Apple Silicon Mac, install a recent stable Rust toolchain, Xcode Command Line Tools and an arm64 VLC 3.0.x application at /Applications/VLC.app. Run `./build.sh` to create build/RMP.app. Set VLC_APP to use another VLC bundle, or pass an absolute output app path as the first argument. VLC is needed only when building. The resulting app carries its own libraries and plugins.

The Rust static library in src/lib.rs owns the queue, playback state and libVLC integration. ui.m is the native Objective-C AppKit interface. Hardware decoding is selected by libVLC when supported. This build is locally signed and has not been notarized by Apple.

## Verification

Tested locally with H.264 video and AAC audio in MKV, embedded SRT subtitles, FLAC audio, and fast-start MP4 over HTTP. Format and stream compatibility depend on codecs, encryption and server behavior.

## Windows x64

The Windows build targets Windows 7 SP1 x64 and newer. Run RMP-Setup-x64.exe for a per-user installation with a Start menu shortcut and an entry in Programs and Features. For portable use, extract the entire Windows release ZIP to one folder and run RMP.exe. Use Open for a file, File → Open network stream for a URL, or drag files onto the window. Queue, Audio track and Subtitles are in the menu bar. F11 toggles fullscreen. The executable uses the same Rust playback core and a native Win32 interface.

GitHub Actions builds the x64 executable, bundles the pinned VideoLAN runtime, and exercises WAV playback on a Windows runner. A successful Windows release tag publishes the ZIP. See [.github/workflows/windows.yml](.github/workflows/windows.yml) for the toolchain and packaging steps.

The Windows interface uses Segoe UI and themed Win32 controls. On a Windows 7 Aero desktop, the frame extends into the player above the video. A solid frame is used when composition is unavailable or High Contrast is active. GitHub Actions tests portable playback, installation, installed playback, and uninstallation on Windows Server 2022. Visual and runtime testing on Windows 7 SP1 is pending.

## Themes in RMP 2.0

Open **Theme ▾** in the player or **Theme** in the menu bar. Windows 10/11 uses a Films & TV-style black playback strip with white icon controls, Windows 7 uses a glass finish and a glossy blue play orb, Windows XP uses a cobalt Luna frame and beveled cream controls, and Mac uses a dark charcoal panel and rounded controls. RMP Default uses a charcoal surface, rounded controls, and orange playback accents. New installations start with RMP Default. The same five choices are available on both platforms. Changing themes keeps the current media and playback position. RMP remembers the selection on this computer.

The Windows 7 theme extends genuine DWM glass into the top strip and bottom playback bar on a Windows 7 desktop with composition enabled. On newer Windows desktops the same theme requests Desktop Acrylic or compositor blur. A blue gradient is available when the effect cannot be enabled. On macOS the playback bar uses a behind-window visual effect while transparency is enabled. Window management buttons and system dialogs follow the host OS.

The three Windows themes draw on screenshots of [XP Explorer](https://gekk.info/articles/explorer.html), [Windows Media Player 12 on Windows 7](https://harisnadeem.com/2009/01/29/windows-7-screenshots-eye-candy/windows-media-player-12/), and [Films & TV on Windows 10](https://learn.microsoft.com/en-us/answers/questions/2813678/video-playback-stretched-over-windows-media-player).

## Installing and updating

Both graphical installers let you select an initial theme. Windows installs for the current account. On macOS, open the setup disk image and run RMP Setup to install in ~/Applications. You can change the theme later from the player's Theme button or menu.

Once per launch, RMP checks published GitHub releases in the background. A newer stable version with a matching platform download opens a release-notes prompt. Choose Update (Yes on Windows) to download, verify the GitHub asset's SHA-256 digest, install and reopen. Choose Not now (No on Windows) to reconsider next launch. Updates require a writable installation folder and network access to GitHub. Install this updater-equipped release manually if upgrading from 2.2 or earlier.

## RMP 2.5

New installations start with **VLC**, a light, compact theme based on VLC's desktop transport. The other five themes remain available, and upgrades retain your selection. The Windows 7 theme uses the standard Windows caption, menus and buttons with the frame extended into the bottom transport area. Fullscreen expands video to the window edges. Its floating controls hide after 2.5 seconds of inactivity and return when the pointer moves. Use the fullscreen control or Escape to leave.

The Windows installer registers RMP under Default apps. Choose Rust Media Player in Windows Settings → Apps → Default apps, or choose it from Open with for a media file. On Mac, select a media file in Finder → Get Info → Open with → RMP → Change All. The portable Windows ZIP does not register the application with Windows.
