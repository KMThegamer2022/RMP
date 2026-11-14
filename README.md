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
