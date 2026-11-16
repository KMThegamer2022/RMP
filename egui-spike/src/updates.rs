//! Update check (checkMacUpdates in updates/mac.h): reads updates/latest.json from GitHub and offers a
//! newer macOS release. The spike can check; installing replaces an RMP.app bundle, which the spike is not.

use std::sync::mpsc::{channel, Receiver};

pub const MANIFEST_URL: &str = "https://raw.githubusercontent.com/KMThegamer2022/RMP/main/updates/latest.json";

#[derive(Debug, Clone)]
pub enum Outcome {
    UpToDate { current: String },
    Available { version: String, notes: String, url: String, sha256: String },
    Failed { reason: String },
}

/// The shipping version, from the engine crate's Cargo.toml (the bundle's CFBundleShortVersionString).
pub fn current_version() -> String {
    let manifest = include_str!("../../Cargo.toml");
    manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')).map(String::from))
        .unwrap_or_else(|| "0.0.0".into())
}

fn valid(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn newer(a: &str, b: &str) -> bool {
    let n = |v: &str| v.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    n(a) > n(b)
}

/// Starts a check on a background thread.
pub fn check() -> Receiver<Outcome> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(run());
    });
    rx
}

fn run() -> Outcome {
    let current = current_version();
    let curl = if cfg!(windows) { "curl.exe" } else { "/usr/bin/curl" };
    let mut command = std::process::Command::new(curl);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = command
        .args(["-fsSL", "--max-time", "15", "-H", "User-Agent: RMP-Updater", "-H", "Cache-Control: no-cache", MANIFEST_URL])
        .output();
    let body = match out {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => return Outcome::Failed { reason: String::from_utf8_lossy(&o.stderr).trim().to_string() },
        Err(e) => return Outcome::Failed { reason: e.to_string() },
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return Outcome::Failed { reason: "The update information was unexpected.".into() };
    };
    let s = |v: &serde_json::Value| v.as_str().unwrap_or("").to_string();
    let version = s(&json["version"]);
    let tag = s(&json["tag"]);
    let platform = if cfg!(windows) { "windows" } else { "macos" };
    let name = s(&json[platform]["name"]);
    let sha = s(&json[platform]["sha256"]);
    if valid(&version)
        && newer(&version, &current)
        && !tag.is_empty()
        && if cfg!(windows) { name == "RMP-Setup-x64.exe" } else { name.starts_with("RMP-") && name.ends_with("macOS-arm64.zip") }
        && !name.contains('/')
        && sha.len() == 64
    {
        return Outcome::Available {
            version,
            notes: s(&json["notes"]),
            url: format!("https://github.com/KMThegamer2022/RMP/releases/download/{tag}/{name}"),
            sha256: sha,
        };
    }
    Outcome::UpToDate { current }
}
