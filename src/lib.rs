// Media engine and queue. The AppKit adapter calls this API on the main thread.
use std::ffi::{c_char, c_void, CStr};
use std::ptr;
mod media_library;
mod terminal;
mod vlc_menu;
mod broadcast;
use media_library::{Item, Library};
use std::path::Path;
type Handle = *mut c_void;
#[repr(C)]
struct Track {
    id: i32,
    name: *const c_char,
    next: *mut Track,
}
extern "C" {
    fn libvlc_new(n: i32, args: *const *const c_char) -> Handle;
    fn libvlc_release(i: Handle);
    fn libvlc_media_player_new(i: Handle) -> Handle;
    fn libvlc_media_player_release(p: Handle);
    fn libvlc_media_new_path(i: Handle, s: *const c_char) -> Handle;
    fn libvlc_media_new_location(i: Handle, s: *const c_char) -> Handle;
    fn libvlc_media_release(m: Handle);
    fn libvlc_media_player_set_media(p: Handle, m: Handle);
    #[cfg(windows)]
    fn libvlc_video_set_mouse_input(p: Handle, enabled: u32);
    #[cfg(windows)]
    fn libvlc_video_set_key_input(p: Handle, enabled: u32);
    #[cfg(windows)]
    fn libvlc_media_player_set_hwnd(p: Handle, v: Handle);
    #[cfg(target_os = "macos")]
    fn libvlc_media_player_set_nsobject(p: Handle, v: Handle);
    fn libvlc_media_player_play(p: Handle) -> i32;
    fn libvlc_media_player_stop(p: Handle);
    fn libvlc_media_player_set_pause(p: Handle, v: i32);
    fn libvlc_media_player_has_vout(p: Handle) -> u32;
    fn libvlc_media_player_get_state(p: Handle) -> i32;
    fn libvlc_media_player_get_time(p: Handle) -> i64;
    fn libvlc_media_player_get_length(p: Handle) -> i64;
    fn libvlc_media_player_is_seekable(p: Handle) -> i32;
    fn libvlc_media_player_set_time(p: Handle, v: i64);
    fn libvlc_audio_set_volume(p: Handle, v: i32) -> i32;
    fn libvlc_audio_get_track_description(p: Handle) -> *mut Track;
    fn libvlc_video_get_spu_description(p: Handle) -> *mut Track;
    fn libvlc_track_description_list_release(t: *mut Track);
    fn libvlc_audio_set_track(p: Handle, v: i32) -> i32;
    fn libvlc_video_set_spu(p: Handle, v: i32) -> i32;
    fn libvlc_audio_get_track(p: Handle) -> i32;
    fn libvlc_video_get_spu(p: Handle) -> i32;
}
pub struct Player {
    warming: Option<std::thread::JoinHandle<Engine>>,
    view: Handle,
    engine: Handle,
    player: Handle,
    queue: Vec<Item>,
    library: Library,
    terminal: terminal::Terminal,
    menus: vlc_menu::Menus,
    current: Option<usize>,
    volume: i32,
}
#[no_mangle]
pub unsafe extern "C" fn ap_create(view: Handle) -> *mut Player {
    Box::into_raw(Box::new(Player {
        warming: None,
        view,
        engine: ptr::null_mut(),
        player: ptr::null_mut(),
        queue: Vec::new(),
        library: Library::new(),
        terminal: terminal::Terminal::new(),
        menus: vlc_menu::Menus::default(),
        current: None,
        volume: 80,
    }))
}
// LibVLC creates both objects without a video view. The worker transfers both
// references together; only the UI thread attaches an HWND or NSView.
struct Engine {
    instance: Handle,
    player: Handle,
}
unsafe impl Send for Engine {}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            if !self.player.is_null() {
                libvlc_media_player_release(self.player);
            }
            if !self.instance.is_null() {
                libvlc_release(self.instance);
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn ap_prepare(p: &mut Player) {
    if p.player.is_null() && p.warming.is_none() {
        p.warming = std::thread::Builder::new()
            .name("rmp-playback-init".into())
            .spawn(|| unsafe { create_engine() })
            .ok();
    }
}
unsafe fn create_engine() -> Engine {
    let mut args = vec![
        c"--no-video-title-show".as_ptr(),
        c"--ignore-config".as_ptr(),
    ];
    let plugins = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("plugins/plugins.dat")));
    if plugins.is_some_and(|cache| cache.is_file()) {
        args.push(c"--no-plugins-scan".as_ptr());
    }
    // FluidSynth resolves its preset bank separately from the plugin cache.
    #[cfg(windows)]
    let soundfont = std::env::current_exe().ok().and_then(|exe| {
        let bank = exe.parent()?.join("TimGM6mb.sf2");
        std::ffi::CString::new(format!("--soundfont={}", bank.to_str()?)).ok()
    });
    #[cfg(windows)]
    if let Some(bank) = &soundfont {
        args.push(bank.as_ptr());
    }
    let instance = libvlc_new(args.len() as i32, args.as_ptr());
    let player = if instance.is_null() {
        ptr::null_mut()
    } else {
        libvlc_media_player_new(instance)
    };
    Engine { instance, player }
}
unsafe fn ensure_engine(p: &mut Player) -> bool {
    if !p.player.is_null() {
        return true;
    }
    let engine = p.warming.take().and_then(|task| task.join().ok())
        .unwrap_or_else(|| create_engine());
    let player = engine.player;
    if player.is_null() {
        return false;
    }
    #[cfg(windows)]
    {
        libvlc_video_set_mouse_input(player, 0);
        libvlc_video_set_key_input(player, 0);
        libvlc_media_player_set_hwnd(player, p.view);
    }
    #[cfg(target_os = "macos")]
    libvlc_media_player_set_nsobject(player, p.view);
    p.engine = engine.instance;
    p.player = player;
    vlc_menu::attach(p);
    std::mem::forget(engine); // Ownership moves to Player until ap_destroy.
    true
}
#[no_mangle]
pub unsafe extern "C" fn ap_destroy(raw: *mut Player) {
    let mut p = Box::from_raw(raw);
    vlc_menu::close(&mut p);
    if p.player.is_null() {
        return;
    }
    libvlc_media_player_stop(p.player);
    libvlc_media_player_release(p.player);
    libvlc_release(p.engine);
}
#[no_mangle]
pub unsafe extern "C" fn ap_add(p: &mut Player, s: *const c_char, network: bool) {
    let name = CStr::from_ptr(s);
    let item = if network {
        (name.to_owned(), true)
    } else {
        name.to_str().ok().and_then(|path| media_library::local_item(path).ok())
            .unwrap_or_else(|| (name.to_owned(), false))
    };
    p.library.remember(std::slice::from_ref(&item));
    p.queue.push(item);
}
#[no_mangle]
pub extern "C" fn ap_library_count(p: &Player) -> usize {
    p.library.items.len()
}
#[no_mangle]
pub extern "C" fn ap_library_item(p: &Player, index: usize) -> *const c_char {
    p.library.items.get(index).map_or(ptr::null(), |item| item.0.as_ptr())
}
#[no_mangle]
pub unsafe extern "C" fn ap_library_play(p: &mut Player, index: usize) -> bool {
    let Some(item) = p.library.items.get(index) else { return false; };
    let index = p.queue.iter().position(|queued| queued == item).unwrap_or_else(|| {
        p.queue.push(item.clone());
        p.queue.len()-1
    });
    ap_play(p, index)
}
#[no_mangle]
pub unsafe extern "C" fn ap_playlist_save(p: &Player, path: *const c_char) -> bool {
    !path.is_null() && CStr::from_ptr(path).to_str().is_ok_and(|path|
        media_library::write_playlist(Path::new(path), &p.queue).is_ok())
}
#[no_mangle]
pub unsafe extern "C" fn ap_playlist_load(p: &mut Player, path: *const c_char) -> bool {
    if path.is_null() { return false; }
    let Ok(path) = CStr::from_ptr(path).to_str() else { return false; };
    let Ok(items) = media_library::read_playlist(Path::new(path)) else { return false; };
    ap_stop(p);
    p.current = None;
    p.library.remember(&items);
    p.queue = items;
    true
}
#[no_mangle]
pub extern "C" fn ap_count(p: &Player) -> usize {
    p.queue.len()
}
#[no_mangle]
pub extern "C" fn ap_index(p: &Player) -> isize {
    p.current.map_or(-1, |i| i as isize)
}
#[no_mangle]
pub extern "C" fn ap_item(p: &Player, i: usize) -> *const c_char {
    p.queue.get(i).map_or(ptr::null(), |item| item.0.as_ptr())
}
#[no_mangle]
pub unsafe extern "C" fn ap_play(p: &mut Player, i: usize) -> bool {
    if i >= p.queue.len() || !ensure_engine(p) {
        return false;
    }
    let Some((s, network)) = p.queue.get(i) else {
        return false;
    };
    let m = if *network {
        libvlc_media_new_location(p.engine, s.as_ptr())
    } else {
        libvlc_media_new_path(p.engine, s.as_ptr())
    };
    if m.is_null() {
        return false;
    }
    libvlc_media_player_stop(p.player);
    vlc_menu::configure_media(p,m,i);
    libvlc_media_player_set_media(p.player, m);
    libvlc_media_release(m);
    p.current = Some(i);
    let ok = libvlc_media_player_play(p.player) == 0;
    libvlc_audio_set_volume(p.player, p.volume);
    ok
}
#[no_mangle]
pub unsafe extern "C" fn ap_has_video(p: &Player) -> bool {
    !p.player.is_null() && libvlc_media_player_has_vout(p.player) != 0
}
#[no_mangle]
pub unsafe extern "C" fn ap_state(p: &Player) -> i32 {
    if p.player.is_null() {
        return 0;
    }
    libvlc_media_player_get_state(p.player)
}
#[no_mangle]
pub unsafe extern "C" fn ap_toggle(p: &mut Player) {
    match ap_state(p) {
        1..=3 => libvlc_media_player_set_pause(p.player, 1),
        4 => libvlc_media_player_set_pause(p.player, 0),
        _ => {
            ap_play(p, p.current.unwrap_or(0));
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn ap_time(p: &Player) -> i64 {
    if p.player.is_null() {
        return 0;
    }
    libvlc_media_player_get_time(p.player)
}
#[no_mangle]
pub unsafe extern "C" fn ap_length(p: &Player) -> i64 {
    if p.player.is_null() {
        return 0;
    }
    libvlc_media_player_get_length(p.player)
}
#[no_mangle]
pub unsafe extern "C" fn ap_seekable(p: &Player) -> bool {
    if p.player.is_null() {
        return false;
    }
    libvlc_media_player_is_seekable(p.player) != 0
}
#[no_mangle]
pub unsafe extern "C" fn ap_seek(p: &Player, t: i64) {
    if ap_seekable(p) {
        libvlc_media_player_set_time(p.player, t.clamp(0, ap_length(p).max(0)))
    }
}
#[no_mangle]
pub unsafe extern "C" fn ap_volume(p: &mut Player, v: i32) {
    p.volume = v.clamp(0, 100);
    if p.player.is_null() {
        return;
    }
    libvlc_audio_set_volume(p.player, p.volume);
}
#[no_mangle]
pub unsafe extern "C" fn ap_tracks(
    p: &Player,
    sub: bool,
    ctx: Handle,
    add: extern "C" fn(Handle, i32, *const c_char, bool),
) {
    if p.player.is_null() {
        return;
    }
    let head = if sub {
        libvlc_video_get_spu_description(p.player)
    } else {
        libvlc_audio_get_track_description(p.player)
    };
    let active = if sub {
        libvlc_video_get_spu(p.player)
    } else {
        libvlc_audio_get_track(p.player)
    };
    let mut t = head;
    while !t.is_null() {
        let track = &*t;
        if !track.name.is_null() {
            add(ctx, track.id, track.name, track.id == active)
        }
        t = track.next
    }
    if !head.is_null() {
        libvlc_track_description_list_release(head)
    }
}
#[no_mangle]
pub unsafe extern "C" fn ap_select(p: &Player, sub: bool, id: i32) -> bool {
    if p.player.is_null() {
        return false;
    }
    (if sub {
        libvlc_video_set_spu(p.player, id)
    } else {
        libvlc_audio_set_track(p.player, id)
    }) == 0
}

#[no_mangle]
pub unsafe extern "C" fn ap_stop(p: &Player) {
    if !p.player.is_null() {
        libvlc_media_player_stop(p.player);
    }
}
