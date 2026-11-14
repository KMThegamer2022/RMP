// Run beside the packaged player to exercise its actual decoder and bank lookup.
include!("../src/lib.rs");

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static HEARD_NOTE: AtomicBool = AtomicBool::new(false);

extern "C" {
    fn libvlc_audio_set_format(p: Handle, format: *const c_char, rate: u32, channels: u32);
    fn libvlc_audio_set_callbacks(
        p: Handle,
        play: extern "C" fn(Handle, *const i16, u32, i64),
        pause: Option<extern "C" fn(Handle, i64)>,
        resume: Option<extern "C" fn(Handle, i64)>,
        flush: Option<extern "C" fn(Handle, i64)>,
        drain: Option<extern "C" fn(Handle)>,
        opaque: Handle,
    );
}

extern "C" fn samples(_: Handle, buffer: *const i16, frames: u32, _: i64) {
    let pcm = unsafe { std::slice::from_raw_parts(buffer, frames as usize) };
    if pcm.iter().any(|x| x.unsigned_abs() > 128) {
        HEARD_NOTE.store(true, Ordering::Relaxed);
    }
}

fn main() {
    let file = CString::new(std::env::args().nth(1).expect("MIDI fixture")).unwrap();
    unsafe {
        let p = ap_create(ptr::null_mut());
        assert!(ensure_engine(&mut *p));
        libvlc_audio_set_format((*p).player, c"S16N".as_ptr(), 44100, 1);
        libvlc_audio_set_callbacks((*p).player, samples, None, None, None, None, ptr::null_mut());
        ap_add(&mut *p, file.as_ptr(), false);
        assert!(ap_play(&mut *p, 0));
        let deadline = Instant::now() + Duration::from_secs(15);
        while !HEARD_NOTE.load(Ordering::Relaxed) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(25));
        }
        ap_destroy(p);
    }
    assert!(HEARD_NOTE.load(Ordering::Relaxed), "MIDI produced no audible samples");
}
