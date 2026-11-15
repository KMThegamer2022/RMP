use super::*;
use std::collections::VecDeque;
use std::sync::Mutex;
pub type Log = Mutex<VecDeque<String>>;
type Callback = unsafe extern "C" fn(Handle, i32, Handle, *const c_char, Handle);
extern "C" {
    fn libvlc_log_set(instance: Handle, callback: Callback, data: Handle);
    pub fn libvlc_log_unset(instance: Handle);
    #[cfg_attr(windows, link_name = "_vsnprintf")]
    fn vsnprintf(buf: *mut c_char, len: usize, fmt: *const c_char, args: Handle) -> i32;
}
unsafe extern "C" fn line(
    ctx: Handle,
    level: i32,
    _location: Handle,
    fmt: *const c_char,
    args: Handle,
) {
    if fmt.is_null() {
        return;
    }
    let mut buf = [0i8; 1024];
    vsnprintf(buf.as_mut_ptr(), buf.len() - 1, fmt, args);
    let message = string(buf.as_ptr());
    if let Ok(mut lines) = (*(ctx as *const Log)).lock() {
        if lines.len() >= 120 {
            lines.pop_front();
        }
        let tag = match level {
            0 => "Debug",
            2 => "Notice",
            3 => "Warning",
            4 => "Error",
            _ => "VLC",
        };
        lines.push_back(format!("{tag}  {message}"));
    }
}
pub unsafe fn install(p: &Player) {
    libvlc_log_set(
        p.engine,
        line,
        (&*p.menus.log as *const Log).cast_mut().cast(),
    );
}
