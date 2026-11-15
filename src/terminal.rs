//! RMP's command line interprets player commands and literal filesystem paths.
use super::*;
use std::ffi::CString;
use std::path::{PathBuf, MAIN_SEPARATOR};

const COMMANDS: &[&str] = &[
    "help",
    "theme",
    "pwd",
    "ls",
    "cd",
    "play",
    "pause",
    "resume",
    "stop",
    "seek",
    "volume",
    "fullscreen",
    "clear",
    "queue",
    "next",
    "prev",
];
const HELP: &str = "pwd              Show current folder\nls [folder]      List files\ncd [folder]      Change folder (default: home)\nplay FILE        Play a local file\npause / resume   Pause or resume playback\nstop             Stop playback\nseek M:SS        Jump to a time\nvolume [0-100]   Show or set volume\nqueue            List queued tracks\nnext / prev      Play an adjacent queued track\nfullscreen       Toggle fullscreen\ntheme [name]     List themes or switch (theme back returns)\nclear            Clear the terminal\nhelp             Show these commands";

pub struct Terminal {
    cwd: PathBuf,
    home: PathBuf,
}

impl Terminal {
    pub fn new() -> Self {
        let home = std::env::home_dir()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR));
        Self {
            cwd: home.clone(),
            home,
        }
    }

    fn resolve(&self, literal: &str) -> Result<PathBuf, String> {
        if literal.contains("://") || literal.starts_with("//") || literal.starts_with(r"\\") {
            return Err("Use a local path.".into());
        }
        if literal == "~" {
            return Ok(self.home.clone());
        }
        if let Some(rest) = literal
            .strip_prefix("~/")
            .or_else(|| literal.strip_prefix(&format!("~{MAIN_SEPARATOR}")))
        {
            return Ok(self.home.join(rest));
        }
        Ok(self.cwd.join(literal))
    }

    fn list(&self, arg: &str) -> Result<String, String> {
        let path = self.resolve(arg)?;
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&path).map_err(|e| format!("{}: {e}", path.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let mut name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() {
                name.push(MAIN_SEPARATOR);
            }
            names.push(name);
        }
        names.sort();
        Ok(names.join("\n"))
    }

    fn cd(&mut self, arg: &str) -> Result<String, String> {
        let path = if arg.is_empty() {
            self.home.clone()
        } else {
            self.resolve(arg)?
        };
        let dir = local_directory(&path)?;
        if !dir.is_dir() {
            return Err(format!("{} is not a folder.", path.display()));
        }
        self.cwd = dir;
        Ok(String::new())
    }

    fn display_path(&self) -> String {
        self.cwd.display().to_string()
    }

    fn prompt(&self) -> String {
        let location = match self.cwd.strip_prefix(&self.home) {
            Ok(relative) if relative.as_os_str().is_empty() => "~".into(),
            Ok(relative) => format!(
                "~/{}",
                relative.to_string_lossy().replace(MAIN_SEPARATOR, "/")
            ),
            Err(_) => self.display_path().replace(MAIN_SEPARATOR, "/"),
        };
        format!("rmp:{location}$")
    }

    fn complete(&self, input: &str) -> String {
        let trimmed = input.trim_start();
        let Some(index) = trimmed.find(char::is_whitespace) else {
            let names: Vec<_> = COMMANDS
                .iter()
                .filter(|name| name.starts_with(trimmed))
                .copied()
                .collect();
            return match names.as_slice() {
                [] => input.to_owned(),
                [name] => format!("{name} "),
                _ => common_prefix(&names),
            };
        };
        let command = &trimmed[..index];
        if !matches!(command, "play" | "cd" | "ls") {
            return input.to_owned();
        }
        let raw = trimmed[index..].trim_start();
        let quoted = raw.starts_with('"');
        let value = if quoted {
            raw.trim_start_matches('"').trim_end_matches('"')
        } else {
            raw
        };
        let split = value.rfind(std::path::is_separator).map_or(0, |i| i + 1);
        let (prefix, part) = value.split_at(split);
        let Ok(folder) = self.resolve(prefix) else {
            return input.to_owned();
        };
        let Ok(entries) = std::fs::read_dir(folder) else {
            return input.to_owned();
        };
        let mut candidates = Vec::new();
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !starts_with_name(&name, part) {
                continue;
            }
            let directory = entry.path().is_dir();
            if directory || command == "play" {
                candidates.push(if directory {
                    format!("{name}{MAIN_SEPARATOR}")
                } else {
                    name
                });
            }
        }
        if candidates.is_empty() {
            return input.to_owned();
        }
        candidates.sort();
        let completion = common_prefix(&candidates);
        if candidates.len() > 1 && completion.chars().count() <= part.chars().count() {
            return input.to_owned();
        }
        let close_quote = quoted && candidates.len() == 1 && !completion.ends_with(MAIN_SEPARATOR);
        format!(
            "{command} {}{prefix}{completion}{}",
            if quoted { "\"" } else { "" },
            if close_quote { "\"" } else { "" }
        )
    }
}

fn local_directory(path: &std::path::Path) -> Result<PathBuf, String> {
    let dir = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    // GetFullPathName and LibVLC expect ordinary drive paths. Rust's
    // canonicalize returns a verbatim path even for a short local name.
    #[cfg(windows)]
    let dir = {
        let raw = dir
            .to_str()
            .ok_or_else(|| "Unsupported folder name.".to_string())?;
        let drive = raw.strip_prefix(r"\\?\").unwrap_or(raw);
        if drive.as_bytes().get(1) != Some(&b':') {
            return Err("Use a local drive folder.".into());
        }
        PathBuf::from(drive)
    };
    Ok(dir)
}

fn starts_with_name(name: &str, prefix: &str) -> bool {
    if cfg!(windows) {
        name.to_lowercase().starts_with(&prefix.to_lowercase())
    } else {
        name.starts_with(prefix)
    }
}

fn common_prefix<T: AsRef<str>>(names: &[T]) -> String {
    let Some(first) = names.first() else {
        return String::new();
    };
    let mut result = first.as_ref();
    for name in &names[1..] {
        while !name.as_ref().starts_with(result) {
            result = &result[..result.char_indices().next_back().unwrap().0];
        }
    }
    result.to_string()
}

fn timestamp(arg: &str) -> Option<i64> {
    let (mins, secs) = arg.split_once(':')?;
    if mins.is_empty()
        || !mins.bytes().all(|b| b.is_ascii_digit())
        || secs.len() != 2
        || !secs.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let seconds = secs.parse::<i64>().ok()?;
    if seconds >= 60 {
        return None;
    }
    mins.parse::<i64>()
        .ok()?
        .checked_mul(60)?
        .checked_add(seconds)?
        .checked_mul(1000)
}

unsafe fn run(p: &mut Player, line: &str) -> (i32, String) {
    let line = line.trim();
    if line.is_empty() {
        return (0, String::new());
    }
    let (cmd, raw) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let arg = raw.trim_start();
    let arg = arg
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(arg);
    let mut action = 0;
    let text = match cmd {
        "help" if arg.is_empty() => HELP.into(),
        "theme" => {
            let name = arg.to_ascii_lowercase();
            match name.as_str() {
                "" => "Themes: VLC, RMP Default, Windows 10/11, Windows 7, Windows XP, Mac, Linux.\nUse theme NAME to switch or theme back to return.".into(),
                "back" | "previous" => { action = 9; "Returning to previous theme.".into() }
                _ => {
                    // Action 10 + the stable menu preference ID (themes.h).
                    let id = match name.as_str() {
                        "windows 10/11" | "windows 10" | "windows 11" | "win10" | "win11" => 0,
                        "windows 7" | "win7" => 1,
                        "windows xp" | "xp" => 2,
                        "mac" | "macos" => 3,
                        "rmp default" | "rmp" | "default" => 4,
                        "vlc" => 5,
                        "linux" => 6,
                        _ => return (0, "Unknown theme. Type theme to list your choices.".into()),
                    };
                    action = 10 + id;
                    format!("Theme: {arg}")
                }
            }
        }
        "pwd" if arg.is_empty() => p.terminal.display_path(),
        "ls" => p.terminal.list(arg).unwrap_or_else(|e| e),
        "cd" => p.terminal.cd(arg).unwrap_or_else(|e| e),
        "clear" if arg.is_empty() => {
            action = 2;
            String::new()
        }
        "fullscreen" if arg.is_empty() => {
            action = 1;
            String::new()
        }
        "play" if arg.is_empty() => "Usage: play FILE".into(),
        "play" => {
            let path = match p.terminal.resolve(arg) {
                Ok(v) => v,
                Err(e) => return (0, e),
            };
            // Reject devices, directories, and failed lookups before calling LibVLC.
            if !path.is_file() {
                return (0, format!("No local file at {}.", path.display()));
            }
            let Some(path) = path.to_str().and_then(|s| CString::new(s).ok()) else {
                return (0, "Unsupported file name.".into());
            };
            ap_add(p, path.as_ptr(), false);
            if ap_play(p, p.queue.len() - 1) {
                action = 3;
                format!("Playing {arg}")
            } else {
                "Could not start playback.".into()
            }
        }
        "pause" | "resume" if arg.is_empty() => {
            let state = ap_state(p);
            if !matches!(state, 1..=4) {
                return (0, "No active track.".into());
            }
            libvlc_media_player_set_pause(p.player, i32::from(cmd == "pause"));
            if cmd == "pause" {
                "Paused."
            } else {
                "Resumed."
            }
            .into()
        }
        "stop" if arg.is_empty() => {
            ap_stop(p);
            "Stopped.".into()
        }
        "seek" => {
            let Some(t) = timestamp(arg) else {
                return (0, "Usage: seek M:SS".into());
            };
            if !ap_seekable(p) {
                return (0, "This track cannot seek.".into());
            }
            ap_seek(p, t);
            String::new()
        }
        "volume" if arg.is_empty() => format!("Volume {}%", p.volume),
        "volume" => match arg.parse::<i32>() {
            Ok(value @ 0..=100) => {
                ap_volume(p, value);
                action = 4;
                format!("Volume {value}%")
            }
            _ => "Usage: volume 0-100".into(),
        },
        "queue" if arg.is_empty() => {
            if p.queue.is_empty() {
                "Queue is empty.".into()
            } else {
                p.queue
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        format!(
                            "{} {}  {}",
                            if p.current == Some(i) { ">" } else { " " },
                            i + 1,
                            name.to_string_lossy()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "next" | "prev" if arg.is_empty() => {
            let index = match (cmd, p.current) {
                ("next", Some(i)) => i.checked_add(1),
                ("prev", Some(i)) => i.checked_sub(1),
                (_, None) => Some(0),
                _ => None,
            };
            if index.is_some_and(|i| ap_play(p, i)) {
                action = 3;
                String::new()
            } else {
                "No queued track in that direction.".into()
            }
        }
        _ if COMMANDS.contains(&cmd) => format!("Usage: {cmd}"),
        _ => format!("Unknown command: {cmd}. Type help."),
    };
    (action, text)
}

fn owned(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}
#[no_mangle]
pub unsafe extern "C" fn ap_terminal_command(
    p: &mut Player,
    line: *const c_char,
    action: *mut i32,
) -> *mut c_char {
    let (code, text) = if line.is_null() {
        (0, String::new())
    } else {
        match CStr::from_ptr(line).to_str() {
            Ok(line) => run(p, line),
            Err(_) => (0, "Input must be UTF-8.".into()),
        }
    };
    if !action.is_null() {
        *action = code;
    }
    owned(text)
}
#[no_mangle]
pub unsafe extern "C" fn ap_terminal_complete(p: &Player, line: *const c_char) -> *mut c_char {
    if line.is_null() {
        return owned(String::new());
    }
    owned(
        CStr::from_ptr(line)
            .to_str()
            .map(|s| p.terminal.complete(s))
            .unwrap_or_default(),
    )
}
#[no_mangle]
pub extern "C" fn ap_terminal_prompt(p: &Player) -> *mut c_char {
    owned(p.terminal.prompt())
}
#[no_mangle]
pub extern "C" fn ap_terminal_volume(p: &Player) -> i32 {
    p.volume
}
#[no_mangle]
pub unsafe extern "C" fn ap_terminal_free(text: *mut c_char) {
    if !text.is_null() {
        drop(CString::from_raw(text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture {
        folder: PathBuf,
        term: Terminal,
    }
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let folder = std::env::temp_dir().join(format!(
                "rmp-term-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&folder).unwrap();
            let folder = local_directory(&folder).unwrap();
            Self {
                term: Terminal {
                    cwd: folder.clone(),
                    home: folder.clone(),
                },
                folder,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn browse_and_complete_literal_unicode_paths() {
        let mut f = Fixture::new();
        let sub = f.folder.join("音 楽");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("café hits.mid"), b"test").unwrap();
        std::fs::write(sub.join("café live.mid"), b"test").unwrap();
        assert_eq!(f.term.complete("vol"), "volume ");
        assert_eq!(
            f.term.complete("cd 音"),
            format!("cd 音 楽{MAIN_SEPARATOR}")
        );
        assert_eq!(f.term.prompt(), "rmp:~$");
        f.term.cd("音 楽").unwrap();
        assert_eq!(f.term.prompt(), "rmp:~/音 楽$");
        assert_eq!(f.term.list("").unwrap(), "café hits.mid\ncafé live.mid");
        assert_eq!(f.term.complete("play caf"), "play café ");
        assert_eq!(f.term.complete("play \"café hi"), "play \"café hits.mid\"");
        assert_eq!(f.term.complete("cd café"), "cd café");
        assert!(f.term.cd("café hits.mid").is_err());
        assert_eq!(f.term.cwd, sub);
        f.term.cd("~").unwrap();
        assert_eq!(f.term.cwd, f.folder);
    }

    #[test]
    fn rejected_commands_cannot_execute_shell_or_play_urls() {
        unsafe {
            let mut f = Fixture::new();
            let raw = ap_create(ptr::null_mut());
            let p = &mut *raw;
            std::mem::swap(&mut p.terminal, &mut f.term);
            for bogus in [
                "touch marker",
                "play https://example.org/file",
                "play vlc://quit",
                "play missing; touch marker",
                "play .",
            ] {
                let (action, answer) = run(p, bogus);
                assert_eq!(action, 0);
                assert!(!answer.is_empty());
            }
            assert!(p.queue.is_empty());
            assert!(!f.folder.join("marker").exists());
            assert!(p.player.is_null());
            assert_eq!(run(p, "volume 37"), (4, "Volume 37%".into()));
            assert_eq!(run(p, "volume 101").0, 0);
            assert_eq!(p.volume, 37);
            assert_eq!(run(p, "theme").0, 0);
            assert_eq!(run(p, "theme back").0, 9);
            assert_eq!(run(p, "theme Windows 7").0, 11);
            assert_eq!(run(p, "theme VLC").0, 15);
            assert_eq!(run(p, "theme nope").0, 0);
            assert_eq!(run(p, "fullscreen").0, 1);
            assert_eq!(run(p, "clear").0, 2);
            assert_eq!(run(p, "pause").0, 0);
            assert_eq!(run(p, "resume").0, 0);
            ap_destroy(raw);
        }
    }

    #[test]
    fn seek_time_is_checked_before_playback() {
        assert_eq!(timestamp("100:59"), Some(6_059_000));
        for invalid in ["2:60", "x:12", "-1:01", "1:", "2:5", "9999999999999999:00"] {
            assert_eq!(timestamp(invalid), None);
        }
    }
}
