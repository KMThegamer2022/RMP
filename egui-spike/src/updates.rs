//! Update check (checkMacUpdates in updates/mac.h): reads updates/latest.json from GitHub and offers a
//! newer macOS release. The spike can check; installing replaces an RMP.app bundle, which the spike is not.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;

pub const MANIFEST_URL: &str = "https://raw.githubusercontent.com/KMThegamer2022/RMP/main/updates/latest.json";
const DOWNLOAD_BASE: &str = "https://github.com/KMThegamer2022/RMP/releases/download";

/// Debug builds can read the manifest and the release files from somewhere else (a file:// path), so
/// the whole update path can be tried without publishing anything. Release builds ignore these.
fn override_var(name: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(name).ok().filter(|v| !v.is_empty()) } else { None }
}
fn manifest_url() -> String {
    override_var("RMP_SPIKE_UPDATE_MANIFEST").unwrap_or_else(|| MANIFEST_URL.to_string())
}
fn download_base() -> String {
    override_var("RMP_SPIKE_UPDATE_BASE").unwrap_or_else(|| DOWNLOAD_BASE.to_string())
}

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
        .args(["-fsSL", "--max-time", "15", "-H", "User-Agent: RMP-Updater", "-H", "Cache-Control: no-cache", &manifest_url()])
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
            url: format!("{}/{tag}/{name}", download_base()),
            sha256: sha,
        };
    }
    Outcome::UpToDate { current }
}

// ---- Downloading and installing (macOS) ------------------------------------------------------
// Mirrors updates/mac.h: download the release zip, check its SHA-256, show a 5 second countdown,
// unpack it next to the running app, verify the signature, bundle id and version, then let the
// bundle's own mac-install.sh swap the apps and reopen RMP.

pub enum Progress {
    Fraction(f64),
    Ready(PathBuf),
    Failed(String),
}

pub struct Download {
    pub rx: Receiver<Progress>,
    pub cancel: Arc<AtomicBool>,
}

fn content_length(url: &str) -> u64 {
    let out = std::process::Command::new(if cfg!(windows) { "curl.exe" } else { "/usr/bin/curl" }).args(["-sIL", "--max-time", "15", url]).output();
    let Ok(out) = out else { return 0 };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<u64>().ok()))
        .last()
        .unwrap_or(0)
}

fn sha256_of(path: &Path) -> Option<String> {
    let out = std::process::Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(path).output().ok()?;
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().map(str::to_string)
}

fn unique() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

#[allow(dead_code)]
pub fn download(url: String, sha256: String) -> Download {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        let file = std::env::temp_dir().join(format!("rmp-update-{}.zip", unique()));
        let expected = content_length(&url);
        let child = std::process::Command::new("/usr/bin/curl").args(["-fsSL", "--retry", "3", "-o"]).arg(&file).arg(&url).spawn();
        let Ok(mut child) = child else {
            let _ = tx.send(Progress::Failed("The download failed. Try again next time you open RMP.".into()));
            return;
        };
        let status = loop {
            if flag.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&file);
                return;
            }
            match child.try_wait() {
                Ok(Some(s)) => break Some(s),
                Ok(None) => {}
                Err(_) => break None,
            }
            if expected > 0 {
                let done = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
                let _ = tx.send(Progress::Fraction((done as f64 / expected as f64).min(1.0)));
            }
            std::thread::sleep(Duration::from_millis(150));
        };
        if !status.is_some_and(|s| s.success()) {
            let _ = std::fs::remove_file(&file);
            let _ = tx.send(Progress::Failed("The download failed. Try again next time you open RMP.".into()));
            return;
        }
        if sha256_of(&file).as_deref() != Some(sha256.as_str()) {
            let _ = std::fs::remove_file(&file);
            let _ = tx.send(Progress::Failed("The download did not match the release checksum.".into()));
            return;
        }
        let _ = tx.send(Progress::Fraction(1.0));
        let _ = tx.send(Progress::Ready(file));
    });
    Download { rx, cancel }
}

fn tool(path: &str, args: &[&std::ffi::OsStr]) -> Option<String> {
    let out = std::process::Command::new(path).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn plist_value(plist: &Path, key: &str) -> Option<String> {
    tool("/usr/libexec/PlistBuddy", &["-c".as_ref(), format!("Print :{key}").as_ref(), plist.as_os_str()])
}

fn make_stage(root: &Path) -> Option<PathBuf> {
    let stage = root.join(format!(".rmp-update-{}", unique()));
    std::fs::create_dir(&stage).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&stage, std::fs::Permissions::from_mode(0o700));
    }
    Some(stage)
}

/// installDownloadedUpdate: stage, verify, then hand over to mac-install.sh. The caller quits the app.
#[allow(dead_code)]
pub fn install(zip: &Path, version: &str) -> Result<(), String> {
    let fail = |m: &str| Err(m.to_string());
    let Some(bundle) = crate::appkit::bundle_path() else {
        return fail("This copy of RMP is not an installed app, so it cannot replace itself.");
    };
    let mut dest = bundle.clone();
    let mut stage = bundle.parent().and_then(make_stage);
    if stage.is_none() {
        // A read-only location: install into the personal Applications folder instead.
        let root = PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Applications");
        let _ = std::fs::create_dir_all(&root);
        dest = root.join("RMP.app");
        stage = make_stage(&root);
    }
    let Some(stage) = stage else {
        let _ = std::fs::remove_file(zip);
        return fail("Could not prepare an update next to RMP or in your personal Applications folder.");
    };
    let app = stage.join("RMP.app");
    let verified = tool("/usr/bin/ditto", &["-x".as_ref(), "-k".as_ref(), zip.as_os_str(), stage.as_os_str()]).is_some()
        && tool("/usr/bin/codesign", &["--verify".as_ref(), "--deep".as_ref(), "--strict".as_ref(), app.as_os_str()]).is_some()
        && plist_value(&app.join("Contents/Info.plist"), "CFBundleIdentifier").is_some_and(|id| {
            plist_value(&bundle.join("Contents/Info.plist"), "CFBundleIdentifier") == Some(id)
        })
        && plist_value(&app.join("Contents/Info.plist"), "CFBundleShortVersionString").as_deref() == Some(version);
    let _ = std::fs::remove_file(zip);
    if !verified {
        let _ = std::fs::remove_dir_all(&stage);
        return fail("The downloaded app failed verification.");
    }
    let helper = std::process::Command::new("/bin/bash")
        .arg(app.join("Contents/Resources/mac-install.sh"))
        .arg(std::process::id().to_string())
        .arg(&stage)
        .arg(&dest)
        .spawn();
    if helper.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
        return fail("The update helper could not start.");
    }
    Ok(())
}

/// Windows: the shipping update.ps1 does the checking, downloading, checksum, install and relaunch.
/// Returns false when the script is not next to the player.
#[cfg(windows)]
pub fn launch_windows_check(manual: bool) -> bool {
    use std::os::windows::process::CommandExt;
    let Ok(exe) = std::env::current_exe() else { return false };
    let Some(script) = exe.parent().map(|d| d.join("update.ps1")).filter(|s| s.is_file()) else { return false };
    let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let shell = Path::new(&system).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let mut cmd = std::process::Command::new(shell);
    cmd.args(["-NoProfile", "-NonInteractive", "-STA", "-ExecutionPolicy", "Bypass", "-File"]).arg(script);
    if manual {
        cmd.arg("-Manual");
    }
    cmd.arg("-PlayerId").arg(std::process::id().to_string()).arg("-CurrentVersion").arg(current_version()).arg("-PlayerPath").arg(exe);
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd.spawn().is_ok()
}
