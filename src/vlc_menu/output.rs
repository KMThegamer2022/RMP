use super::*;

fn destination(path: &str) -> Result<String, String> {
    let file = Path::new(path);
    if !file.is_absolute() || file.file_name().is_none() || !file.parent().is_some_and(Path::is_dir)
    {
        return Err("Choose an output file in an existing folder.".into());
    }
    if file.is_dir() {
        return Err("Choose a file, not a folder.".into());
    }
    Ok(format!(
        "'{}'",
        path.replace('\\', "\\\\").replace('\'', "\\'")
    ))
}

fn endpoint(value: &str) -> Result<&str, String> {
    let addr = value.strip_prefix("udp://").unwrap_or(value);
    let valid = addr
        .parse::<std::net::SocketAddr>()
        .is_ok_and(|a| a.port() > 0)
        || addr.rsplit_once(':').is_some_and(|(host, port)| {
            !host.is_empty()
                && host.split('.').all(|part| {
                    !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                })
                && port.parse::<u16>().is_ok_and(|v| v > 0)
        });
    if valid {
        Ok(addr)
    } else {
        Err("Enter a UDP address such as udp://239.255.0.1:5004.".into())
    }
}

pub unsafe fn action(p: &mut Player, key: &str, value: &str) -> Result<String, String> {
    let state = &mut p.menus;
    if !state.vlm_ready {
        if libvlc_vlm_get_event_manager(p.engine).is_null() {
            return Err("The broadcast manager could not start.".into());
        }
        state.vlm_ready = true;
    }
    let name = c"rmp-output".as_ptr();
    if key == "vlm" {
        if !state.output {
            return Ok("No broadcast or conversion is configured.".into());
        }
        let info = take_string(libvlc_vlm_show_media(p.engine, name));
        return Ok(if info.is_empty() {
            "No active broadcast instance.".into()
        } else {
            info
        });
    }
    if key == "output-stop" || (key == "record" && value == "stop") {
        if !state.output {
            return Err("No output job is configured.".into());
        }
        libvlc_vlm_stop_media(p.engine, name);
        if libvlc_vlm_del_media(p.engine, name) != 0 {
            return Err("Could not remove the output job.".into());
        }
        state.output = false;
        return Ok(String::new());
    }
    if state.output {
        // A completed VLM broadcast stays configured. Allow a subsequent job
        // once the previous input is no longer running.
        if libvlc_vlm_get_media_instance_time(p.engine, name, 0) >= 0 {
            return Err("An output job is running. Stop it before starting another.".into());
        }
        libvlc_vlm_del_media(p.engine, name);
        state.output = false;
    }
    let item = p
        .current
        .and_then(|i| p.queue.get(i))
        .ok_or("Open media first.")?;
    if key != "stream" && !item.1 {
        let input = Path::new(item.0.to_str().map_err(|_| "Invalid source path.")?);
        if input == Path::new(value)
            || input
                .canonicalize()
                .ok()
                .zip(Path::new(value).canonicalize().ok())
                .is_some_and(|(a, b)| a == b)
        {
            return Err("Choose an output file different from the source.".into());
        }
    }
    let sout = match key {
        "stream" => format!("#std{{access=udp,mux=ts,dst={}}}", endpoint(value)?),
        "record" => format!("#std{{access=file,mux=ts,dst={}}}", destination(value)?),
        "convert" => format!(
            "#transcode{{vcodec=h264,acodec=mp4a,ab=160}}:std{{access=file,mux=mp4,dst={}}}",
            destination(value)?
        ),
        _ => return Err("Unknown output type.".into()),
    };
    let media = libvlc_media_player_get_media(p.player);
    if media.is_null() {
        return Err("Open media before starting an output job.".into());
    }
    let input = libvlc_media_get_mrl(media);
    libvlc_media_release(media);
    if input.is_null() {
        return Err("Could not locate the current media.".into());
    }
    let out = CString::new(sout).map_err(|_| "Invalid output.")?;
    let start = CString::new(format!(
        "start-time={:.3}",
        libvlc_media_player_get_time(p.player).max(0) as f64 / 1000.
    ))
    .unwrap();
    let opts = [start.as_ptr()];
    let rc = libvlc_vlm_add_broadcast(
        p.engine,
        name,
        input,
        out.as_ptr(),
        if key == "record" { 1 } else { 0 },
        opts.as_ptr(),
        1,
        0,
    );
    libvlc_free(input.cast());
    if rc != 0 {
        return Err("Could not configure the output. Check the destination.".into());
    }
    if libvlc_vlm_play_media(p.engine, name) != 0 {
        libvlc_vlm_del_media(p.engine, name);
        return Err("Could not start the output. See Messages for details.".into());
    }
    state.output = true;
    Ok(String::new())
}

extern "C" {
    fn libvlc_vlm_get_media_instance_time(e: Handle, name: *const c_char, instance: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_udp_endpoints_are_accepted() {
        assert!(endpoint("udp://127.0.0.1:5000").is_ok());
        for s in ["udp://:2", "udp://server:0", "udp://evil}:34", "a/path:4"] {
            assert!(endpoint(s).is_err());
        }
    }
}
