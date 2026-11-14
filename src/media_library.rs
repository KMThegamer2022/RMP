//! Playback history and UTF-8 M3U files shared by both native interfaces.

use std::collections::HashSet;
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;

pub type Item = (CString, bool);

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn absolute(path: &Path) -> io::Result<PathBuf> {
    // Resolves relative names without requiring a playlist's media to be online.
    std::path::absolute(path)
}

pub fn local_item(path: &str) -> io::Result<Item> {
    let path = absolute(Path::new(path))?;
    let name = path.to_str().ok_or_else(|| invalid("Path is not UTF-8"))?;
    Ok((CString::new(name)?, false))
}

fn is_location(name: &str) -> bool {
    name.split_once("://").is_some_and(|(scheme, _)| {
        let mut bytes = scheme.bytes();
        bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
            && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    })
}

pub fn read_playlist(path: &Path) -> io::Result<Vec<Item>> {
    let source = fs::read_to_string(path)?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
    let path = absolute(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Invalid playlist path"))?;
    let mut items = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if is_location(line) {
            items.push((CString::new(line)?, true));
        } else {
            let entry = parent.join(line);
            let name = entry.to_str().ok_or_else(|| invalid("Path is not UTF-8"))?;
            items.push((CString::new(name)?, false));
        }
    }
    if items.is_empty() && source.lines().next() != Some("#EXTM3U") {
        return Err(invalid("Playlist has no entries or M3U header"));
    }
    Ok(items)
}

pub fn write_playlist(path: &Path, items: &[Item]) -> io::Result<()> {
    let mut data = String::from("#EXTM3U\n");
    for (name, network) in items {
        let name = name
            .to_str()
            .map_err(|_| invalid("Media name is not UTF-8"))?;
        if name.contains(['\r', '\n']) {
            return Err(invalid("M3U cannot represent line breaks inside a name"));
        }
        if *network {
            data.push_str(name);
        } else {
            let full = absolute(Path::new(name))?;
            data.push_str(full.to_str().ok_or_else(|| invalid("Path is not UTF-8"))?);
        }
        data.push('\n');
    }
    atomic_write(path, data.as_bytes())
}

fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Invalid destination"))?;
    let (temp, mut file) = loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".rmp-{}-{}.tmp", std::process::id(), id));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temp) {
            Ok(file) => break (temp, file),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    };
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

fn library_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let root = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    #[cfg(not(windows))]
    let root = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    Some(root.join("RMP/library-v1"))
}

// Stable on-disk format: source kind byte, UTF-8 path/URL, then NUL.
// Unlike M3U, it also supports filenames containing line breaks.
fn read_library(path: &Path) -> io::Result<Vec<Item>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    if !bytes.is_empty() && bytes.last() != Some(&0) {
        return Err(invalid("Incomplete library record"));
    }
    bytes
        .split_inclusive(|b| *b == 0)
        .map(|record| {
            let (kind, name) = record[..record.len() - 1]
                .split_first()
                .ok_or_else(|| invalid("Empty record"))?;
            if !matches!(kind, b'L' | b'U') {
                return Err(invalid("Unknown media kind"));
            }
            Ok((CString::new(name)?, *kind == b'U'))
        })
        .collect()
}

fn merge_and_save(path: &Path, additions: &[Item]) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| invalid("Invalid library path"))?;
    fs::create_dir_all(dir)?;
    // The lock has a stable inode even while the data is replaced, and the OS
    // releases it when the writer closes or terminates.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("library.lock"))?;
    lock.lock()?;
    let mut items = read_library(path)?;
    let mut known: HashSet<_> = items.iter().cloned().collect();
    items.extend(
        additions
            .iter()
            .filter(|item| known.insert((*item).clone()))
            .cloned(),
    );
    let mut data = Vec::new();
    for (name, network) in items {
        data.push(if network { b'U' } else { b'L' });
        data.extend_from_slice(name.as_bytes_with_nul());
    }
    atomic_write(path, &data)
}

pub struct Library {
    pub items: Vec<Item>,
    known: HashSet<Item>,
    writer: Option<mpsc::Sender<Vec<Item>>>,
    thread: Option<thread::JoinHandle<()>>,
    path: Option<PathBuf>,
}

impl Library {
    pub fn new() -> Self {
        Self::at(library_path())
    }

    fn at(path: Option<PathBuf>) -> Self {
        let items = path
            .as_ref()
            .and_then(|path| read_library(path).ok())
            .unwrap_or_default();
        Self {
            known: items.iter().cloned().collect(),
            items,
            path,
            writer: None,
            thread: None,
        }
    }

    pub fn remember(&mut self, entries: &[Item]) {
        let mut additions = Vec::new();
        for item in entries {
            if self.known.insert(item.clone()) {
                self.items.push(item.clone());
                additions.push(item.clone());
            }
        }
        if additions.is_empty() {
            return;
        }
        if self.writer.is_none() {
            let Some(path) = self.path.clone() else {
                return;
            };
            let (sender, receiver) = mpsc::channel::<Vec<Item>>();
            let worker = thread::Builder::new()
                .name("rmp-library".into())
                .spawn(move || {
                    let mut pending = Vec::new();
                    for mut batch in receiver.iter() {
                        pending.append(&mut batch);
                        pending.extend(receiver.try_iter().flatten());
                        match merge_and_save(&path, &pending) {
                            Ok(()) => pending.clear(),
                            Err(err) => eprintln!("RMP: could not save library: {err}"),
                        }
                    }
                });
            if let Ok(worker) = worker {
                self.writer = Some(sender);
                self.thread = Some(worker);
            }
        }
        if let Some(writer) = &self.writer {
            let _ = writer.send(additions);
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(worker) = self.thread.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "rmp-playlist-test-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn relative_paths_unicode_duplicates_streams_and_overwrite() {
        let dir = Fixture::new();
        let playlist = dir.0.join("test.m3u8");
        fs::write(&playlist, "\u{feff}#EXTM3U\r\n#EXTINF:30,Track\r\ncafé song.mp3\r\nhttps://example.com/song?a=1#b\r\ncafé song.mp3\r\n").unwrap();
        let queue = read_playlist(&playlist).unwrap();
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue[0],
            local_item(dir.0.join("café song.mp3").to_str().unwrap()).unwrap()
        );
        assert_eq!(queue[0], queue[2]);
        assert!(queue[1].1);
        write_playlist(&playlist, &queue).unwrap();
        assert_eq!(read_playlist(&playlist).unwrap(), queue);
        write_playlist(&playlist, &[]).unwrap();
        assert!(read_playlist(&playlist).unwrap().is_empty());
    }

    #[test]
    fn failed_import_and_export_leave_existing_playlist_intact() {
        let dir = Fixture::new();
        let playlist = dir.0.join("list");
        let invalid = b"a\nwrong\0record\n";
        fs::write(&playlist, invalid).unwrap();
        assert!(read_playlist(&playlist).is_err());
        let embedded_line_break = (CString::new("split\nname.mp3").unwrap(), false);
        assert!(write_playlist(&playlist, &[embedded_line_break]).is_err());
        assert_eq!(fs::read(&playlist).unwrap(), invalid);
    }

    #[test]
    fn independent_players_merge_history_and_deduplicate() {
        let dir = Fixture::new();
        let path = dir.0.join("history");
        let mut first = Library::at(Some(path.clone()));
        let mut second = Library::at(Some(path.clone()));
        let a = local_item("one.mp3").unwrap();
        let b = local_item("two.mp3").unwrap();
        first.remember(&[a.clone(), a.clone()]);
        assert_eq!(first.items.len(), 1);
        second.remember(&[b.clone(), a.clone()]);
        drop(first);
        drop(second);
        let library = Library::at(Some(path));
        assert_eq!(library.items.len(), 2);
        assert!(library.known.contains(&a) && library.known.contains(&b));
    }
}
