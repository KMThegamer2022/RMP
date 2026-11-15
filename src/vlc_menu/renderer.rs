use super::*;
use std::sync::Mutex;
#[repr(C)]
struct Description {
    name: *const c_char,
    longname: *const c_char,
}
#[repr(C)]
struct Event {
    kind: i32,
    object: Handle,
    item: Handle,
}
type Callback = unsafe extern "C" fn(*const Event, Handle);
extern "C" {
    fn libvlc_renderer_discoverer_list_get(e: Handle, list: *mut *mut *mut Description) -> usize;
    fn libvlc_renderer_discoverer_list_release(list: *mut *mut Description, n: usize);
    fn libvlc_renderer_discoverer_new(e: Handle, name: *const c_char) -> Handle;
    fn libvlc_renderer_discoverer_event_manager(d: Handle) -> Handle;
    fn libvlc_event_attach(m: Handle, kind: i32, f: Callback, ctx: Handle) -> i32;
    fn libvlc_event_detach(m: Handle, kind: i32, f: Callback, ctx: Handle);
    fn libvlc_renderer_discoverer_start(d: Handle) -> i32;
    fn libvlc_renderer_discoverer_stop(d: Handle);
    fn libvlc_renderer_discoverer_release(d: Handle);
    fn libvlc_renderer_item_hold(i: Handle) -> Handle;
    fn libvlc_renderer_item_release(i: Handle);
    fn libvlc_renderer_item_name(i: Handle) -> *const c_char;
    fn libvlc_media_player_set_renderer(p: Handle, i: Handle) -> i32;
}
const ADD: i32 = 0x502;
const REMOVE: i32 = 0x503;
#[derive(Default)]
struct Found {
    next: u64,
    items: Vec<(u64, usize)>,
}
unsafe extern "C" fn receive(e: *const Event, ctx: Handle) {
    let Ok(mut found) = (*(ctx as *const Mutex<Found>)).lock() else {
        return;
    };
    let item = (*e).item as usize;
    if (*e).kind == ADD && !found.items.iter().any(|(_, ptr)| *ptr == item) {
        let id = found.next;
        found.next += 1;
        libvlc_renderer_item_hold((*e).item);
        found.items.push((id, item));
    } else if (*e).kind == REMOVE {
        if let Some(i) = found.items.iter().position(|(_, ptr)| *ptr == item) {
            found.items.remove(i);
            libvlc_renderer_item_release((*e).item);
        }
    }
}
#[derive(Default)]
pub struct Renderers {
    found: Box<Mutex<Found>>,
    running: Vec<Handle>,
    started: bool,
}
impl Renderers {
    pub unsafe fn options(&mut self, e: Handle) -> Vec<(String, String)> {
        let ctx = (&*self.found as *const Mutex<Found>).cast_mut().cast();
        if !self.started {
            self.started = true;
            let mut list = ptr::null_mut();
            let count = libvlc_renderer_discoverer_list_get(e, &mut list);
            for i in 0..count {
                let d = libvlc_renderer_discoverer_new(e, (**list.add(i)).name);
                if d.is_null() {
                    continue;
                }
                let manager = libvlc_renderer_discoverer_event_manager(d);
                let added = libvlc_event_attach(manager, ADD, receive, ctx) == 0;
                let removed = added && libvlc_event_attach(manager, REMOVE, receive, ctx) == 0;
                if removed && libvlc_renderer_discoverer_start(d) == 0 {
                    self.running.push(d);
                    continue;
                }
                if added {
                    libvlc_event_detach(manager, ADD, receive, ctx);
                }
                if removed {
                    libvlc_event_detach(manager, REMOVE, receive, ctx);
                }
                libvlc_renderer_discoverer_release(d);
            }
            if !list.is_null() {
                libvlc_renderer_discoverer_list_release(list, count);
            }
        }
        let mut rows = vec![("local".into(), "Local playback".into())];
        if let Ok(found) = self.found.lock() {
            for (id, item) in &found.items {
                rows.push((
                    id.to_string(),
                    string(libvlc_renderer_item_name(*item as Handle)),
                ));
            }
        }
        rows
    }
    pub unsafe fn select(&self, p: Handle, id: &str) -> bool {
        if id == "local" {
            return libvlc_media_player_set_renderer(p, ptr::null_mut()) == 0;
        }
        let Some(item) = self.found.lock().ok().and_then(|f| {
            f.items
                .iter()
                .find(|(k, _)| Some(*k) == id.parse().ok())
                .map(|(_, ptr)| libvlc_renderer_item_hold(*ptr as Handle))
        }) else {
            return false;
        };
        let ok = libvlc_media_player_set_renderer(p, item) == 0;
        libvlc_renderer_item_release(item);
        ok
    }
}
impl Drop for Renderers {
    fn drop(&mut self) {
        unsafe {
            let ctx = (&*self.found as *const Mutex<Found>).cast_mut().cast();
            for d in self.running.drain(..) {
                libvlc_renderer_discoverer_stop(d);
                let m = libvlc_renderer_discoverer_event_manager(d);
                libvlc_event_detach(m, ADD, receive, ctx);
                libvlc_event_detach(m, REMOVE, receive, ctx);
                libvlc_renderer_discoverer_release(d);
            }
            if let Ok(mut f) = self.found.lock() {
                for (_, item) in f.items.drain(..) {
                    libvlc_renderer_item_release(item as Handle);
                }
            }
        }
    }
}
