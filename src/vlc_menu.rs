use super::*;
use ffi::*;
use std::ffi::CString;
mod ffi;
mod log;
mod output;
mod renderer;

#[derive(Default)]
pub struct Menus {
    bookmarks: Vec<(CString, i64)>,
    renderers: renderer::Renderers,
    log: Box<log::Log>,
    vlm_ready: bool,
    output: bool,
    deinterlace_mode: String,
    visualization: String,
    programmes: std::collections::HashMap<CString, u16>,
    resume: Option<(i64, bool)>,
}
pub unsafe fn attach(p: &Player) {
    log::install(p);
}
pub unsafe fn close(p: &mut Player) {
    if p.engine.is_null() {
        return;
    }
    std::mem::take(&mut p.menus.renderers);
    if p.menus.vlm_ready {
        libvlc_vlm_release(p.engine);
    }
    log::libvlc_log_unset(p.engine);
}
pub unsafe fn configure_media(p: &mut Player, m: Handle, index: usize) {
    if let Some(id) = p.menus.programmes.get(&p.queue[index].0) {
        libvlc_media_add_option(m, c(&format!(":program={id}")).as_ptr());
    }
    if let Some((start, paused)) = p.menus.resume.take() {
        if start > 0 {
            libvlc_media_add_option(
                m,
                c(&format!(":start-time={:.3}", start as f64 / 1000.)).as_ptr(),
            );
        }
        if paused {
            libvlc_media_add_option(m, c":start-paused".as_ptr());
        }
    }
    if p.menus.visualization == "spectrum" {
        libvlc_media_add_option(m, c":audio-visual=visual".as_ptr());
        libvlc_media_add_option(m, c":effect-list=spectrum".as_ptr());
    }
}
unsafe fn string(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}
unsafe fn take_string(p: *mut c_char) -> String {
    let s = string(p);
    if !p.is_null() {
        libvlc_free(p.cast());
    }
    s
}
type Rows = Vec<(String, String)>;
fn choices(items: &[(&str, &str)]) -> Rows {
    items
        .iter()
        .map(|(id, label)| (id.to_string(), label.to_string()))
        .collect()
}
fn same(items: &[&str]) -> Rows {
    items
        .iter()
        .map(|v| (v.to_string(), v.to_string()))
        .collect()
}

unsafe fn tracks(p: &Player, key: &str) -> Rows {
    let list = match key {
        "audio-track" => libvlc_audio_get_track_description(p.player),
        "video-track" => libvlc_video_get_track_description(p.player),
        _ => libvlc_video_get_spu_description(p.player),
    };
    let mut rows = Vec::new();
    let mut t = list;
    while !t.is_null() {
        rows.push(((*t).id.to_string(), string((*t).name)));
        t = (*t).next;
    }
    if !list.is_null() {
        libvlc_track_description_list_release(list);
    }
    rows
}
unsafe fn options(p: &mut Player, key: &str) -> Rows {
    match key {
        "programme" => crate::broadcast::programme_options(p),
        "speed" => choices(&[
            ("0.25", "0.25×"),
            ("0.5", "0.5×"),
            ("0.75", "0.75×"),
            ("1", "Normal"),
            ("1.25", "1.25×"),
            ("1.5", "1.5×"),
            ("2", "2×"),
            ("4", "4×"),
        ]),
        "zoom" => choices(&[
            ("0", "Fit window"),
            ("0.5", "Half size"),
            ("1", "Original size"),
            ("1.5", "1.5×"),
            ("2", "Double size"),
        ]),
        "aspect" | "crop" => {
            let mut rows = choices(&[("default", "Default")]);
            rows.extend(same(&["16:9", "4:3", "1:1", "16:10", "2.35:1", "5:4"]));
            rows
        }
        "deinterlace" => choices(&[("off", "Off"), ("on", "On")]),
        "deinterlace-mode" => same(&[
            "blend", "bob", "discard", "linear", "mean", "x", "yadif", "yadif2x", "phosphor",
            "ivtc",
        ]),
        "stereo" => choices(&[
            ("1", "Stereo"),
            ("2", "Reverse stereo"),
            ("3", "Left"),
            ("4", "Right"),
            ("5", "Dolby Surround"),
        ]),
        "visualization" => choices(&[("off", "Off"), ("spectrum", "Spectrum")]),
        "equalizer" => {
            let mut rows = choices(&[("off", "Off")]);
            for i in 0..libvlc_audio_equalizer_get_preset_count() {
                rows.push((
                    i.to_string(),
                    string(libvlc_audio_equalizer_get_preset_name(i)),
                ));
            }
            rows
        }
        "renderer" => {
            if ensure_engine(p) {
                p.menus.renderers.options(p.engine)
            } else {
                vec![]
            }
        }
        "bookmark" => {
            let mut rows = choices(&[("add", "Add bookmark here")]);
            if let Some((name, _)) = p.current.and_then(|i| p.queue.get(i)) {
                for (i, (source, t)) in p.menus.bookmarks.iter().enumerate() {
                    if name == source {
                        rows.push((
                            i.to_string(),
                            format!("{:02}:{:02}", t / 60000, t / 1000 % 60),
                        ));
                    }
                }
            }
            rows
        }
        _ if p.player.is_null() => vec![],
        "audio-track" | "subtitle-track" | "video-track" => tracks(p, key),
        "audio-device" => {
            let head = libvlc_audio_output_device_enum(p.player);
            let mut d = head;
            let mut rows = Vec::new();
            while !d.is_null() {
                rows.push((string((*d).id), string((*d).name)));
                d = (*d).next;
            }
            if !head.is_null() {
                libvlc_audio_output_device_list_release(head);
            }
            rows
        }
        "title" => {
            let mut list = ptr::null_mut();
            let n = libvlc_media_player_get_full_title_descriptions(p.player, &mut list);
            let mut rows = Vec::new();
            for i in 0..n {
                let name = string((**list.add(i as usize)).name);
                rows.push((
                    i.to_string(),
                    if name.is_empty() {
                        format!("Title {}", i + 1)
                    } else {
                        name
                    },
                ));
            }
            if n > 0 {
                libvlc_title_descriptions_release(list, n as u32);
            }
            rows
        }
        "chapter" => {
            let mut list = ptr::null_mut();
            let n = libvlc_media_player_get_full_chapter_descriptions(
                p.player,
                libvlc_media_player_get_title(p.player),
                &mut list,
            );
            let mut rows = Vec::new();
            for i in 0..n {
                let name = string((**list.add(i as usize)).name);
                rows.push((
                    i.to_string(),
                    if name.is_empty() {
                        format!("Chapter {}", i + 1)
                    } else {
                        name
                    },
                ));
            }
            if n > 0 {
                libvlc_chapter_descriptions_release(list, n as u32);
            }
            rows
        }
        _ => vec![],
    }
}

unsafe fn info(p: &Player, key: &str) -> String {
    let media = libvlc_media_player_get_media(p.player);
    if media.is_null() {
        return "Open media to see its details.".into();
    }
    let mut lines = Vec::new();
    if key == "media-info" {
        lines.push(format!(
            "Location: {}",
            take_string(libvlc_media_get_mrl(media))
        ));
        for (i, name) in [
            (0, "Title"),
            (1, "Artist"),
            (2, "Genre"),
            (4, "Album"),
            (8, "Date"),
            (11, "Language"),
            (14, "Encoded by"),
        ] {
            let v = take_string(libvlc_media_get_meta(media, i));
            if !v.is_empty() {
                lines.push(format!("{name}: {v}"));
            }
        }
        let d = ap_length(p);
        if d >= 0 {
            lines.push(format!("Duration: {}:{:02}", d / 60000, d / 1000 % 60));
        }
    } else {
        let mut array = ptr::null_mut();
        let count = libvlc_media_tracks_get(media, &mut array);
        for i in 0..count {
            let t = &**array.add(i as usize);
            let label = match t.kind {
                0 => "Audio",
                1 => "Video",
                2 => "Subtitle",
                _ => "Other",
            };
            let codec = string(libvlc_media_get_codec_description(t.kind, t.codec));
            let code = t.codec.to_le_bytes();
            lines.push(format!(
                "{label} #{}: {} ({})",
                t.id,
                codec,
                String::from_utf8_lossy(&code).trim_matches('\0')
            ));
            if t.bitrate > 0 {
                lines.push(format!("  {} kb/s", t.bitrate / 1000));
            }
        }
        if count > 0 {
            libvlc_media_tracks_release(array, count);
        }
        if lines.is_empty() {
            lines.push("No stream information is available yet.".into());
        }
    }
    libvlc_media_release(media);
    lines.join("\n")
}

unsafe fn plugins(p: &Player) -> String {
    let mut rows = Vec::new();
    for (kind, head) in [
        ("Audio", libvlc_audio_filter_list_get(p.engine)),
        ("Video", libvlc_video_filter_list_get(p.engine)),
    ] {
        let mut m = head;
        while !m.is_null() {
            rows.push(format!(
                "{kind}: {} ({})",
                string((*m).long),
                string((*m).name)
            ));
            m = (*m).next;
        }
        if !head.is_null() {
            libvlc_module_description_list_release(head);
        }
    }
    rows.sort();
    if rows.is_empty() {
        "No filter modules reported.".into()
    } else {
        rows.join("\n")
    }
}
fn number(value: &str, min: f64, max: f64) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= min && *n <= max)
        .ok_or_else(|| format!("Enter a number between {min} and {max}."))
}
fn rc(status: i32) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err("This operation is unavailable for the current media or output.".into())
    }
}
fn c(value: &str) -> CString {
    CString::new(value).expect("value originated in a C string")
}
unsafe fn restart_current(p: &mut Player) -> Result<(), String> {
    let index = p.current.ok_or("Open a track first.")?;
    p.menus.resume = Some((ap_time(p).max(0), ap_state(p) == 4));
    let started = ap_play(p, index);
    p.menus.resume = None;
    if started {
        Ok(())
    } else {
        Err("Could not restart the current media.".into())
    }
}

unsafe fn action(p: &mut Player, key: &str, value: &str) -> Result<String, String> {
    match key {
        "guide" => return Ok(crate::broadcast::guide(p)),
        "subtitle-search" => return Ok(crate::broadcast::subtitle_search(p)),
        "programme" => {
            if !crate::broadcast::programme_options(p)
                .iter()
                .any(|(id, _)| id == value)
            {
                return Err("No matching broadcast programme in this media.".into());
            }
            let id = value
                .parse::<u16>()
                .map_err(|_| "Choose a programme number.")?;
            let index = p.current.ok_or("Open a transport stream first.")?;
            p.menus.programmes.insert(p.queue[index].0.clone(), id);
            restart_current(p)?;
            return Ok(String::new());
        }
        "volume" => {
            let n = number(value, 0., 100.)?;
            ap_volume(p, n.round() as i32);
            return Ok(String::new());
        }
        "messages" => {
            return Ok(p
                .menus
                .log
                .lock()
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| v.iter().cloned().collect::<Vec<_>>().join("\n"))
                .unwrap_or_else(|| "No playback messages yet.".into()))
        }
        "plugins" => {
            return if ensure_engine(p) {
                Ok(plugins(p))
            } else {
                Err("Playback engine unavailable.".into())
            }
        }
        "renderer" => {
            if !ensure_engine(p) || !p.menus.renderers.select(p.player, value) {
                return Err("This renderer is no longer available.".into());
            }
            return Ok(String::new());
        }
        _ => {}
    }
    if p.player.is_null() {
        return Err("Open media before using this command.".into());
    }
    let mp = p.player;
    match key {
        "time" | "jump" => {
            let t = (number(value, -1e9, 1e9)? * 1000.).round() as i64;
            if !ap_seekable(p) {
                return Err("This media cannot seek.".into());
            }
            ap_seek(
                p,
                if key == "jump" {
                    ap_time(p).saturating_add(t)
                } else {
                    t
                },
            );
        }
        "bookmark" => {
            if !ap_seekable(p) {
                return Err("Bookmarks require seekable media.".into());
            }
            let current = p
                .current
                .and_then(|i| p.queue.get(i))
                .ok_or("No current track.")?
                .0
                .clone();
            if value == "add" {
                let t = ap_time(p).max(0);
                p.menus.bookmarks.push((current, t));
            } else {
                let mark = value
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| p.menus.bookmarks.get(i))
                    .filter(|(item, _)| *item == current)
                    .ok_or("Unknown bookmark for this media.")?;
                ap_seek(p, mark.1);
            }
        }
        "speed" => rc(libvlc_media_player_set_rate(
            mp,
            number(value, 0.1, 8.)? as f32,
        ))?,
        "audio-track" | "subtitle-track" | "video-track" | "stereo" | "title" | "chapter" => {
            let id = value
                .parse::<i32>()
                .map_err(|_| "Choose a valid menu item.")?;
            if !options(p, key).iter().any(|(i, _)| i == value) {
                return Err("This item is no longer available.".into());
            }
            match key {
                "audio-track" => rc(libvlc_audio_set_track(mp, id))?,
                "subtitle-track" => rc(libvlc_video_set_spu(mp, id))?,
                "video-track" => rc(libvlc_video_set_track(mp, id))?,
                "stereo" => rc(libvlc_audio_set_channel(mp, id))?,
                "title" => libvlc_media_player_set_title(mp, id),
                _ => libvlc_media_player_set_chapter(mp, id),
            }
        }
        "audio-device" => {
            if !options(p, key).iter().any(|(id, _)| id == value) {
                return Err("This device is no longer available.".into());
            }
            libvlc_audio_output_device_set(mp, ptr::null(), c(value).as_ptr());
        }
        "mute" => libvlc_audio_toggle_mute(mp),
        "zoom" => libvlc_video_set_scale(mp, number(value, 0., 4.)? as f32),
        "aspect" | "crop" => {
            if !options(p, key).iter().any(|(id, _)| id == value) {
                return Err("Choose a valid aspect ratio.".into());
            }
            let val = c(value);
            let arg = if value == "default" {
                ptr::null()
            } else {
                val.as_ptr()
            };
            if key == "crop" {
                libvlc_video_set_crop_geometry(mp, arg);
            } else {
                libvlc_video_set_aspect_ratio(mp, arg);
            }
        }
        "deinterlace" | "deinterlace-mode" => {
            if !options(p, key).iter().any(|(id, _)| id == value) {
                return Err("Choose a valid deinterlace setting.".into());
            }
            let mode = if key == "deinterlace-mode" {
                p.menus.deinterlace_mode = value.into();
                value
            } else if value == "off" {
                ""
            } else if p.menus.deinterlace_mode.is_empty() {
                "yadif"
            } else {
                &p.menus.deinterlace_mode
            };
            libvlc_video_set_deinterlace(mp, c(mode).as_ptr());
        }
        "snapshot" => {
            if !ap_has_video(p) {
                return Err("Play a video to take a snapshot.".into());
            }
            if !Path::new(value).is_absolute() {
                return Err("Choose a snapshot destination.".into());
            }
            rc(libvlc_video_take_snapshot(mp, 0, c(value).as_ptr(), 0, 0))?;
        }
        "subtitle-file" => {
            if !Path::new(value).is_file() {
                return Err("Choose a readable subtitle file.".into());
            }
            if libvlc_video_set_subtitle_file(mp, c(value).as_ptr()) == 0 {
                return Err("Could not load the subtitle file.".into());
            }
        }
        "equalizer" => {
            let eq = if value == "off" {
                ptr::null_mut()
            } else {
                let preset = value
                    .parse::<u32>()
                    .ok()
                    .filter(|v| *v < libvlc_audio_equalizer_get_preset_count())
                    .ok_or("Choose an equalizer preset.")?;
                let eq = libvlc_audio_equalizer_new_from_preset(preset);
                if eq.is_null() {
                    return Err("Could not create an equalizer.".into());
                }
                eq
            };
            let status = libvlc_media_player_set_equalizer(mp, eq);
            if !eq.is_null() {
                libvlc_audio_equalizer_release(eq);
            }
            rc(status)?;
        }
        "brightness" | "contrast" => {
            let n = number(value, 0., 2.)?;
            libvlc_video_set_adjust_int(mp, 0, 1);
            libvlc_video_set_adjust_float(mp, if key == "contrast" { 1 } else { 2 }, n as f32);
        }
        "audio-delay" | "subtitle-delay" => {
            let micros = (number(value, -3600., 3600.)? * 1e6).round() as i64;
            rc(if key == "audio-delay" {
                libvlc_audio_set_delay(mp, micros)
            } else {
                libvlc_video_set_spu_delay(mp, micros)
            })?;
        }
        "media-info" | "codec-info" => return Ok(info(p, key)),
        "vlm" | "stream" | "record" | "convert" | "output-stop" => {
            return output::action(p, key, value)
        }
        "visualization" => {
            if !matches!(value, "off" | "spectrum") {
                return Err("Choose a visualization.".into());
            }
            p.menus.visualization = value.into();
            restart_current(p)?;
        }
        _ => return Err("Unrecognized menu command.".into()),
    }
    Ok(String::new())
}

unsafe fn input<'a>(s: *const c_char) -> Option<&'a str> {
    if s.is_null() {
        None
    } else {
        CStr::from_ptr(s).to_str().ok()
    }
}
fn owned(s: String) -> *mut c_char {
    CString::new(s.replace('\0', "�")).unwrap().into_raw()
}
#[no_mangle]
pub unsafe extern "C" fn ap_menu_action(
    p: &mut Player,
    key: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    let result = input(key)
        .ok_or_else(|| "Missing menu command.".to_owned())
        .and_then(|k| action(p, k, input(value).unwrap_or("")));
    owned(result.unwrap_or_else(|e| e))
}
#[no_mangle]
pub unsafe extern "C" fn ap_menu_options(p: &mut Player, key: *const c_char) -> *mut c_char {
    let rows = input(key).map(|k| options(p, k)).unwrap_or_default();
    owned(
        rows.into_iter()
            .filter(|(id, _)| !id.contains(['\t', '\r', '\n']))
            .map(|(id, label)| format!("{id}\t{}", label.replace(['\t', '\r', '\n'], " ")))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
