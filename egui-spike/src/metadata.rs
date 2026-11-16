//! Library metadata, read with AVFoundation on two worker threads (metadataForPath in mac-library.h):
//! title, album, artist, artwork and duration from the asset's common metadata.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Debug)]
pub struct Meta {
    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub length: Option<String>,
    /// Encoded artwork bytes (JPEG or PNG), decoded on the UI thread into a texture.
    pub cover: Option<Arc<Vec<u8>>>,
}

impl Meta {
    /// searchNames: "title album artist".
    pub fn search_text(&self) -> String {
        format!(
            "{} {} {}",
            self.title.as_deref().unwrap_or(""),
            self.album.as_deref().unwrap_or(""),
            self.artist.as_deref().unwrap_or("")
        )
    }
}

#[derive(Default)]
struct Shared {
    done: HashMap<String, Meta>,
    pending: HashSet<String>,
}

pub struct Loader {
    shared: Arc<Mutex<Shared>>,
    tx: Sender<String>,
}

impl Loader {
    pub fn new() -> Self {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let (tx, rx) = channel::<String>();
        let rx = Arc::new(Mutex::new(rx));
        for _ in 0..2 {
            let rx = rx.clone();
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("rmp-metadata".into())
                .spawn(move || loop {
                    let path = match rx.lock().ok().and_then(|r| r.recv().ok()) {
                        Some(p) => p,
                        None => return,
                    };
                    #[cfg(target_os = "macos")]
                    let meta = objc2::rc::autoreleasepool(|_| read(&path));
                    #[cfg(windows)]
                    let meta = read(&path);
                    if let Ok(mut s) = shared.lock() {
                        s.pending.remove(&path);
                        s.done.insert(path, meta);
                    }
                })
                .ok();
        }
        Loader { shared, tx }
    }

    /// Returns cached metadata, or queues it and returns None (only absolute paths, as the shipping app).
    pub fn get(&self, path: &str) -> Option<Meta> {
        let mut s = self.shared.lock().ok()?;
        if let Some(m) = s.done.get(path) {
            return Some(m.clone());
        }
        if s.pending.contains(path) || !std::path::Path::new(path).is_absolute() || s.pending.len() >= 64 {
            return None;
        }
        s.pending.insert(path.to_string());
        let _ = self.tx.send(path.to_string());
        None
    }
}

pub fn stamp(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

#[cfg(target_os = "macos")]
fn read(path: &str) -> Meta {
    use objc2_av_foundation::{AVAsset, AVURLAsset};
    use objc2_foundation::{NSString, NSURL};
    let mut meta = Meta::default();
    if !std::path::Path::new(path).exists() {
        return meta;
    }
    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let asset = AVURLAsset::URLAssetWithURL_options(&url, None);
        #[allow(deprecated)]
        let items = asset.commonMetadata();
        for item in items.iter() {
            let Some(key) = item.commonKey() else { continue };
            let key = key.to_string();
            match key.as_str() {
                // AVMetadataCommonKeyTitle / AlbumName / Artist
                "title" | "albumName" | "artist" => {
                    if let Some(v) = item.stringValue() {
                        let v = v.to_string();
                        if !v.is_empty() {
                            match key.as_str() {
                                "title" => meta.title = Some(v),
                                "albumName" => meta.album = Some(v),
                                _ => meta.artist = Some(v),
                            }
                        }
                    }
                }
                "artwork" => {
                    if let Some(data) = item.dataValue() {
                        let bytes = data.to_vec();
                        if !bytes.is_empty() {
                            meta.cover = Some(Arc::new(bytes));
                        }
                    }
                }
                _ => {}
            }
        }
        #[allow(deprecated)]
        let t = AVAsset::duration(&asset);
        let valid = t.flags.0 & 1 != 0 && t.flags.0 & (2 | 4 | 8 | 16) == 0;
        if valid && t.timescale > 0 {
            meta.length = Some(stamp(((t.value as f64) * 1000.0 / t.timescale as f64).round() as i64));
        }
    }
    meta
}

/// Windows: tags, artwork and duration read with lofty.
#[cfg(windows)]
fn read(path: &str) -> Meta {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::tag::Accessor;
    let mut meta = Meta::default();
    let Ok(file) = lofty::read_from_path(path) else { return meta };
    if let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) {
        meta.title = tag.title().map(|v| v.to_string()).filter(|v| !v.is_empty());
        meta.album = tag.album().map(|v| v.to_string()).filter(|v| !v.is_empty());
        meta.artist = tag.artist().map(|v| v.to_string()).filter(|v| !v.is_empty());
        if let Some(pic) = tag.pictures().first() {
            meta.cover = Some(Arc::new(pic.data().to_vec()));
        }
    }
    let ms = file.properties().duration().as_millis() as i64;
    if ms > 0 {
        meta.length = Some(stamp(ms));
    }
    meta
}
