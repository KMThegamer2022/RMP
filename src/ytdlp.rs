//! Resolves video page links (YouTube) to a direct stream with the bundled yt-dlp helper.
use std::ffi::{CStr, CString};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(60);

fn helper() -> Option<PathBuf> {
    let name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    let path = std::env::current_exe().ok()?.parent()?.join(name);
    path.is_file().then_some(path)
}

fn host_of(url: &str) -> String {
    let rest = url.split_once("://").map_or("", |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    host.split(':').next().unwrap_or("").to_ascii_lowercase()
}

pub fn is_video_page(url: &str) -> bool {
    let host = host_of(url);
    host == "youtu.be" || host == "youtube.com" || host.ends_with(".youtube.com")
}

/// Returns a playable stream URL for a video page, or None so the original link is used.
pub fn direct_url(page: &CStr) -> Option<CString> {
    let url = page.to_str().ok()?;
    if !is_video_page(url) {
        return None;
    }
    let mut command = Command::new(helper()?);
    command
        .args(["--no-playlist", "--no-warnings", "-g", "-f", "b"])
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().ok()?;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().ok()? {
            if !status.success() {
                return None;
            }
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    let line = out.lines().map(str::trim).find(|line| line.starts_with("http"))?;
    CString::new(line).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_youtube_hosts() {
        assert!(is_video_page("https://www.youtube.com/watch?v=abc"));
        assert!(is_video_page("https://youtu.be/abc"));
        assert!(is_video_page("https://m.youtube.com/watch?v=abc"));
        assert!(!is_video_page("https://example.com/youtube.com/x.mp4"));
        assert!(!is_video_page("http://127.0.0.1:8765/open-video.mp4"));
    }
}
