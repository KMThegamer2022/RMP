//! Windows glue with the same names as `appkit.rs`: the video child window, caption colours, always
//! on top, wallpaper mode, About box, clipboard, drawn symbols, screenshots.
//!
//! libvlc draws into a child window of the egui window. A child window always paints over its
//! parent, so egui cannot draw over the video on Windows: the video is hidden while a menu or dialog
//! is open (see `layout::draw`) and shortened while the fullscreen transport shows.

use crate::theme::Skin;
use eframe::egui::{self, Rect};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, GetMonitorInfoW, GetStockObject,
    MonitorFromPoint, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLACK_BRUSH, CAPTUREBLT, DIB_RGB_COLORS, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY, SRCCOPY,
};
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetDoubleClickTime, GetFocus, SetFocus, VK_LBUTTON};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, DragQueryFileW, SetWindowSubclass, ShellExecuteW};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hwnd(v: isize) -> HWND {
    v as HWND
}

/// A top-level window handle.
#[derive(Clone, Copy)]
pub struct Win(pub isize);

static DOUBLE_CLICKED: AtomicBool = AtomicBool::new(false);

static VIDEO_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

static DOWNS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static INSIDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static MESSAGES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Test hook: left-button presses seen, how many were over the video, and WM_LBUTTONDBLCLK messages;
/// plus the video window rectangle and the cursor.
pub fn input_stats() -> String {
    let video = VIDEO_WINDOW.load(Ordering::Relaxed);
    let mut r = unsafe { std::mem::zeroed::<RECT>() };
    if video != 0 {
        unsafe { GetWindowRect(hwnd(video), &mut r) };
    }
    let p = cursor();
    format!(
        "downs={} inside={} dblmsg={} rect={},{},{},{} visible={} cursor={},{} main={:x} focus={} foreground_ours={} keys_seen={} keys_injected={} subclass={}",
        DOWNS.load(Ordering::Relaxed),
        INSIDE.load(Ordering::Relaxed),
        MESSAGES.load(Ordering::Relaxed),
        r.left,
        r.top,
        r.right,
        r.bottom,
        video != 0 && unsafe { IsWindowVisible(hwnd(video)) } != 0,
        p.x,
        p.y,
        if video != 0 { unsafe { GetParent(hwnd(video)) as isize } } else { 0 },
        {
            let f = unsafe { GetFocus() } as isize;
            if f == 0 { "none".to_string() } else if f != video && unsafe { GetParent(hwnd(video)) } as isize != f { let mut name = [0u16; 64]; let n = unsafe { GetClassNameW(hwnd(f), name.as_mut_ptr(), 64) }; format!("other-{}", String::from_utf16_lossy(&name[..n.max(0) as usize])) } else if f == video { "video".to_string() } else if unsafe { GetParent(hwnd(video)) } as isize == f { "main".to_string() } else { format!("other-{f:x}") }
        },
        app_active(),
        KEYS_SEEN.load(Ordering::Relaxed),
        KEYS_INJECTED.load(Ordering::Relaxed),
        HOOK_STATUS.load(Ordering::Relaxed)
    )
}

struct ClickState {
    was_down: bool,
    last_press: Option<std::time::Instant>,
    last_pos: (i32, i32),
    last_fire: Option<std::time::Instant>,
}
static CLICKS: std::sync::Mutex<ClickState> = std::sync::Mutex::new(ClickState { was_down: false, last_press: None, last_pos: (0, 0), last_fire: None });

/// True once per double-click on the video. The player's own WM_LBUTTONDBLCLK is one source; the other
/// watches the left button and the pointer position, because libvlc's output window sits on top of ours
/// and does not reliably pass mouse messages up.
pub fn take_video_double_click() -> bool {
    let message = DOUBLE_CLICKED.swap(false, Ordering::Relaxed);
    let mut st = CLICKS.lock().unwrap();
    let now = std::time::Instant::now();
    let mut polled = false;
    let down = unsafe { GetAsyncKeyState(VK_LBUTTON as i32) } as u16 & 0x8000 != 0;
    if down && !st.was_down {
        DOWNS.fetch_add(1, Ordering::Relaxed);
        let p = cursor();
        let video = VIDEO_WINDOW.load(Ordering::Relaxed);
        let inside = video != 0 && unsafe {
            let mut r = std::mem::zeroed::<RECT>();
            IsWindowVisible(hwnd(video)) != 0
                && GetWindowRect(hwnd(video), &mut r) != 0
                && p.x >= r.left
                && p.x < r.right
                && p.y >= r.top
                && p.y < r.bottom
        };
        if inside {
            INSIDE.fetch_add(1, Ordering::Relaxed);
            // Whatever window under the pointer took the click, the main window gets the keyboard.
            unsafe {
                let parent = GetParent(hwnd(video));
                if !parent.is_null() {
                    SetFocus(parent);
                }
            }
            let window = std::time::Duration::from_millis(unsafe { GetDoubleClickTime() } as u64);
            let near = (p.x - st.last_pos.0).abs() <= 8 && (p.y - st.last_pos.1).abs() <= 8;
            if st.last_press.is_some_and(|t| now.duration_since(t) <= window) && near {
                polled = true;
                st.last_press = None;
            } else {
                st.last_press = Some(now);
                st.last_pos = (p.x, p.y);
            }
        } else {
            st.last_press = None;
        }
    }
    st.was_down = down;
    let fire = (message || polled) && st.last_fire.is_none_or(|t| now.duration_since(t) > std::time::Duration::from_millis(600));
    if fire {
        st.last_fire = Some(now);
    }
    fire
}

unsafe extern "system" fn video_proc(w: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let parent = GetParent(w);
    match msg {
        // The video window must never keep the keyboard: pass keys up to the main window and give it
        // the focus back after any click.
        WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP | WM_CHAR => {
            if !parent.is_null() {
                return SendMessageW(parent, msg, wp, lp);
            }
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            if !parent.is_null() {
                SetFocus(parent);
            }
        }
        _ => {}
    }
    if msg == WM_LBUTTONDBLCLK {
        MESSAGES.fetch_add(1, Ordering::Relaxed);
        DOUBLE_CLICKED.store(true, Ordering::Relaxed);
        return 0;
    }
    DefWindowProcW(w, msg, wp, lp)
}

/// The child window libvlc draws into.
pub struct Surfaces {
    parent: isize,
    video: isize,
    last: Cell<Option<(i32, i32, i32, i32)>>,
}

impl Surfaces {
    pub fn attach(frame: &eframe::Frame) -> Option<Self> {
        let RawWindowHandle::Win32(h) = frame.window_handle().ok()?.as_raw() else { return None };
        let parent = h.hwnd.get();
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let class = wide("RMPVideo");
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_DBLCLKS,
                lpfnWndProc: Some(video_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: inst,
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: GetStockObject(BLACK_BRUSH) as _,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
            let video = CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                0,
                0,
                1,
                1,
                hwnd(parent),
                std::ptr::null_mut(),
                inst,
                std::ptr::null(),
            );
            if video.is_null() {
                return None;
            }
            VIDEO_WINDOW.store(video as isize, Ordering::Relaxed);
            install_key_hook(hwnd(parent));
            Some(Surfaces { parent, video: video as isize, last: Cell::new(None) })
        }
    }

    pub fn video_handle(&self) -> *mut c_void {
        self.video as *mut c_void
    }

    fn scale(&self) -> f32 {
        let dpi = unsafe { GetDpiForWindow(hwnd(self.parent)) };
        if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
    }

    pub fn place_video(&self, rect: Option<Rect>) {
        unsafe {
            match rect {
                Some(r) if r.width() >= 1.0 && r.height() >= 1.0 => {
                    let s = self.scale();
                    let v = ((r.min.x * s).round() as i32, (r.min.y * s).round() as i32, (r.width() * s).round() as i32, (r.height() * s).round() as i32);
                    if self.last.get() != Some(v) {
                        SetWindowPos(hwnd(self.video), HWND_TOP, v.0, v.1, v.2, v.3, SWP_NOACTIVATE | SWP_SHOWWINDOW);
                        self.last.set(Some(v));
                    }
                }
                _ => {
                    if self.last.get().is_some() {
                        ShowWindow(hwnd(self.video), SW_HIDE);
                        self.last.set(None);
                    }
                }
            }
        }
    }

    /// Windows 7 glass is a macOS-only effect in this build.
    pub fn place_glass(&self, _rect: Option<Rect>) {}

    pub fn window(&self) -> Option<Win> {
        Some(Win(self.parent))
    }
}

/// Windows has no "reduce transparency" glass here, so the Windows 7 glass stays off.
pub fn reduce_transparency() -> bool {
    true
}

fn colorref(c: egui::Color32) -> u32 {
    let [r, g, b, _] = c.to_array();
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

/// applyTheme's window chrome: dark or light caption, and the skin's panel colour on Windows 11.
pub fn apply_window_chrome(window: &Win, skin: Skin, _translucent: bool) {
    let th = skin.theme();
    unsafe {
        let dark: u32 = th.dark as u32;
        DwmSetWindowAttribute(hwnd(window.0), 20, &dark as *const u32 as *const c_void, 4);
        let caption = colorref(skin.window_background());
        DwmSetWindowAttribute(hwnd(window.0), 35, &caption as *const u32 as *const c_void, 4);
        let text = colorref(th.text);
        DwmSetWindowAttribute(hwnd(window.0), 36, &text as *const u32 as *const c_void, 4);
    }
}

/// Always on Top (View menu).
pub fn set_floating(window: &Win, floating: bool) {
    unsafe {
        SetWindowPos(
            hwnd(window.0),
            if floating { HWND_TOPMOST } else { HWND_NOTOPMOST },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// What leave_wallpaper restores.
pub struct WallpaperRestore {
    style: isize,
    rect: RECT,
}

unsafe extern "system" fn find_workerw(top: HWND, out: LPARAM) -> i32 {
    let view = wide("SHELLDLL_DefView");
    if !FindWindowExW(top, std::ptr::null_mut(), view.as_ptr(), std::ptr::null()).is_null() {
        let class = wide("WorkerW");
        let worker = FindWindowExW(std::ptr::null_mut(), top, class.as_ptr(), std::ptr::null());
        *(out as *mut HWND) = worker;
    }
    1
}

/// Set as Wallpaper: the window becomes a child of the desktop's WorkerW layer behind the icons,
/// covering the screen (the shipping player's setVideoWallpaper).
pub fn enter_wallpaper(window: &Win) -> WallpaperRestore {
    unsafe {
        let w = hwnd(window.0);
        let mut rect = std::mem::zeroed::<RECT>();
        GetWindowRect(w, &mut rect);
        let restore = WallpaperRestore { style: GetWindowLongPtrW(w, GWL_STYLE), rect };
        let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
        let mut result = 0usize;
        SendMessageTimeoutW(progman, 0x052C, 0, 0, SMTO_NORMAL, 1000, &mut result);
        let mut worker: HWND = std::ptr::null_mut();
        EnumWindows(Some(find_workerw), &mut worker as *mut HWND as LPARAM);
        if !worker.is_null() {
            SetWindowLongPtrW(w, GWL_STYLE, ((restore.style as u32 & !(WS_CAPTION | WS_THICKFRAME)) | WS_CHILD) as isize);
            SetParent(w, worker);
            let (cx, cy) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
            SetWindowPos(w, HWND_BOTTOM, 0, 0, cx, cy, SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        }
        restore
    }
}

pub fn leave_wallpaper(window: &Win, restore: &WallpaperRestore, always_top: bool) {
    unsafe {
        let w = hwnd(window.0);
        SetParent(w, std::ptr::null_mut());
        SetWindowLongPtrW(w, GWL_STYLE, restore.style);
        let r = restore.rect;
        SetWindowPos(w, std::ptr::null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        set_floating(window, always_top);
    }
}

/// Fullscreen needs no extra presentation options on Windows.
pub fn set_presentation(_fullscreen: bool) {}

fn cursor() -> POINT {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    p
}

/// True when the mouse is within a few pixels of the top of its monitor.
pub fn pointer_near_screen_top() -> bool {
    let p = cursor();
    unsafe {
        let monitor = MonitorFromPoint(p, MONITOR_DEFAULTTOPRIMARY);
        let mut info = std::mem::zeroed::<MONITORINFO>();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return false;
        }
        p.y <= info.rcMonitor.top + 5
    }
}

/// Mouse location in screen coordinates.
pub fn mouse_location() -> (f64, f64) {
    let p = cursor();
    (p.x as f64, p.y as f64)
}

/// The window icon is set when the window is created (`window_icon`).
pub fn set_app_icon(_png: &[u8]) {}

/// The About box: name, version and the player's icon.
pub fn show_about_panel(version: &str, _icon_png: &[u8]) {
    let text = wide(&format!("RMP\nVersion {version}"));
    let title = wide("About RMP");
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION) };
}

/// Open Location from Clipboard: files copied in Explorer first, then plain text.
pub fn pasteboard_location() -> Option<String> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let mut result = None;
        if IsClipboardFormatAvailable(15) != 0 {
            // CF_HDROP
            let drop = GetClipboardData(15);
            if !drop.is_null() {
                let n = DragQueryFileW(drop as _, 0, std::ptr::null_mut(), 0);
                if n > 0 {
                    let mut buf = vec![0u16; n as usize + 1];
                    DragQueryFileW(drop as _, 0, buf.as_mut_ptr(), buf.len() as u32);
                    result = Some(String::from_utf16_lossy(&buf[..n as usize]));
                }
            }
        }
        if result.is_none() && IsClipboardFormatAvailable(13) != 0 {
            // CF_UNICODETEXT
            let handle = GetClipboardData(13);
            if !handle.is_null() {
                let ptr = GlobalLock(handle as _) as *const u16;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)).trim().to_string();
                    GlobalUnlock(handle as _);
                    if !text.is_empty() {
                        result = Some(text);
                    }
                }
            }
        }
        CloseClipboard();
        result
    }
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Opens a link in the default browser.
pub fn open_url(url: &str) {
    let (verb, target) = (wide("open"), wide(url));
    unsafe { ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
}

/// True while one of this process's windows is in the foreground.
pub fn app_active() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, &mut pid);
        pid == GetCurrentProcessId()
    }
}

/// No installed bundle on Windows; the installer makes the shortcuts.
pub fn bundle_path() -> Option<std::path::PathBuf> {
    None
}

pub fn set_desktop_shortcut(_on: bool) -> bool {
    false
}

/// Saves a screenshot of the window (or the whole primary screen) as a PNG.
pub fn screenshot(window: Option<&Win>, path: &str) {
    unsafe {
        let (mut x, mut y, mut w, mut h) = (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        if let Some(win) = window {
            let mut r = std::mem::zeroed::<RECT>();
            if GetWindowRect(hwnd(win.0), &mut r) != 0 {
                (x, y, w, h) = (r.left, r.top, r.right - r.left, r.bottom - r.top);
            }
        }
        if w <= 0 || h <= 0 {
            return;
        }
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(dc, bmp as _);
        BitBlt(dc, 0, 0, w, h, screen, x, y, SRCCOPY | CAPTUREBLT);
        let mut info = std::mem::zeroed::<BITMAPINFO>();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut px = vec![0u8; (w * h * 4) as usize];
        GetDIBits(dc, bmp, 0, h as u32, px.as_mut_ptr() as *mut c_void, &mut info, DIB_RGB_COLORS);
        SelectObject(dc, old);
        DeleteObject(bmp as _);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
        for p in px.chunks_exact_mut(4) {
            p.swap(0, 2);
            p[3] = 255;
        }
        let _ = image::save_buffer(path, &px, w as u32, h as u32, image::ColorType::Rgba8);
    }
}

// ---- Drawn symbols ---------------------------------------------------------------------------
// The macOS build draws SF Symbols. Windows has no equivalent, so the five icons the library uses
// are drawn from simple shapes.

fn seg(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = (((px - a.0) * dx + (py - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((px - a.0 - t * dx).powi(2) + (py - a.1 - t * dy).powi(2)).sqrt()
}

fn rrect_edge(px: f32, py: f32, c: (f32, f32), half: (f32, f32), r: f32) -> f32 {
    let q = ((px - c.0).abs() - half.0 + r, (py - c.1).abs() - half.1 + r);
    (q.0.max(0.0).powi(2) + q.1.max(0.0).powi(2)).sqrt() + q.0.max(q.1).min(0.0) - r
}

fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let s = |p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1);
    let (d1, d2, d3) = (s(p, a, b), s(p, b, c), s(p, c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

fn note(x: f32, y: f32, ox: f32, oy: f32, k: f32) -> bool {
    let (x, y) = ((x - ox) / k, (y - oy) / k);
    // head (ellipse), stem and flag
    let head = ((x - 0.34) / 0.2).powi(2) + ((y - 0.78) / 0.14).powi(2) <= 1.0;
    let stem = (0.5..=0.57).contains(&x) && (0.12..=0.78).contains(&y);
    let flag = seg(x, y, (0.54, 0.14), (0.84, 0.38)) < 0.06;
    head || stem || flag
}

/// A symbol rendered to RGBA, white on transparent (the caller tints it).
pub fn symbol_image(name: &str, point_size: f64) -> Option<egui::ColorImage> {
    let n = (point_size * 3.0).round() as usize;
    let mut img = egui::ColorImage::filled([n, n], egui::Color32::TRANSPARENT);
    let line = 0.045f32;
    for yi in 0..n {
        for xi in 0..n {
            let (x, y) = ((xi as f32 + 0.5) / n as f32, (yi as f32 + 0.5) / n as f32);
            let on = match name {
                "play.rectangle" => {
                    rrect_edge(x, y, (0.5, 0.5), (0.45, 0.33), 0.08).abs() < line
                        || in_triangle((x, y), (0.42, 0.38), (0.42, 0.62), (0.62, 0.5))
                }
                "list.bullet" => {
                    let dots = [0.25f32, 0.5, 0.75].iter().any(|&cy| (x - 0.14).powi(2) + (y - cy).powi(2) < 0.0036);
                    let bars = [0.25f32, 0.5, 0.75].iter().any(|&cy| (0.28..=0.9).contains(&x) && (y - cy).abs() < 0.04);
                    dots || bars
                }
                "music.note" => note(x, y, 0.1, 0.05, 0.85),
                "music.note.tv" => {
                    rrect_edge(x, y, (0.5, 0.45), (0.45, 0.33), 0.06).abs() < line
                        || seg(x, y, (0.3, 0.88), (0.7, 0.88)) < line
                        || note(x, y, 0.33, 0.2, 0.36)
                }
                "music.note.house" => {
                    let roof = seg(x, y, (0.08, 0.45), (0.5, 0.1)) < line || seg(x, y, (0.5, 0.1), (0.92, 0.45)) < line;
                    let walls = seg(x, y, (0.18, 0.4), (0.18, 0.9)) < line
                        || seg(x, y, (0.82, 0.4), (0.82, 0.9)) < line
                        || seg(x, y, (0.18, 0.9), (0.82, 0.9)) < line;
                    roof || walls || note(x, y, 0.33, 0.38, 0.4)
                }
                _ => false,
            };
            if on {
                img.pixels[yi * n + xi] = egui::Color32::WHITE;
            }
        }
    }
    Some(img)
}

/// A DWORD under HKCU\\Software\\RMP, where the shipping player and its installer keep settings.
pub fn registry_dword(name: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let (key, value) = (wide("Software\\RMP"), wide(name));
    let mut data = 0u32;
    let mut size = 4u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut data as *mut u32 as *mut c_void, &mut size)
    };
    (status == 0).then_some(data)
}

// ---- Keyboard fallback ------------------------------------------------------------------------
// Windows can leave the keyboard focus on a window winit does not read (libvlc's output window, or
// winit's own helper window after a fullscreen switch), and then no key reaches egui. The main
// window is subclassed so every key message is seen; `inject_keys` hands them to egui, dropping the
// copy winit delivers when it was focused after all.

static KEYS: std::sync::Mutex<Vec<(u32, bool)>> = std::sync::Mutex::new(Vec::new());
static KEYS_SEEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static KEYS_INJECTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static HOOK_STATUS: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(-1);

unsafe extern "system" fn key_subclass(w: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
    let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
    if down || up {
        KEYS_SEEN.fetch_add(1, Ordering::Relaxed);
        // Auto-repeat (bit 30 set on a key-down) is not a new press.
        let repeat = down && (lp as u32 >> 30) & 1 == 1;
        if !repeat {
            if let Ok(mut q) = KEYS.lock() {
                if q.len() < 64 {
                    q.push((wp as u32, down));
                }
            }
        }
    }
    DefSubclassProc(w, msg, wp, lp)
}

fn install_key_hook(parent: HWND) {
    unsafe {
        let ok = SetWindowSubclass(parent, Some(key_subclass), 1, 0);
        HOOK_STATUS.store(ok as isize, Ordering::Relaxed);
    }
}

fn egui_key(vk: u32) -> Option<egui::Key> {
    use egui::Key::*;
    Some(match vk {
        0x1B => Escape,
        0x20 => Space,
        0x0D => Enter,
        0x09 => Tab,
        0x08 => Backspace,
        0x25 => ArrowLeft,
        0x26 => ArrowUp,
        0x27 => ArrowRight,
        0x28 => ArrowDown,
        0x70 => F1,
        0x7A => F11,
        0xDB => OpenBracket,
        0xDD => CloseBracket,
        0x41..=0x5A => [A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z][(vk - 0x41) as usize],
        _ => return None,
    })
}

struct Recent {
    winit: Vec<(egui::Key, bool, std::time::Instant)>,
    ours: Vec<(egui::Key, bool, std::time::Instant)>,
}
static RECENT: std::sync::Mutex<Recent> = std::sync::Mutex::new(Recent { winit: Vec::new(), ours: Vec::new() });

/// Called from `raw_input_hook`. Adds key events winit did not deliver, and removes winit's copy of a
/// key this module already delivered, so each press reaches egui once.
pub fn inject_keys(raw: &mut egui::RawInput) {
    let window = std::time::Duration::from_millis(200);
    let now = std::time::Instant::now();
    let Ok(mut recent) = RECENT.lock() else { return };
    recent.winit.retain(|e| now.duration_since(e.2) < window);
    recent.ours.retain(|e| now.duration_since(e.2) < window);
    // winit's own events: drop ones we already injected, remember the rest.
    let mut kept = Vec::with_capacity(raw.events.len());
    for event in raw.events.drain(..) {
        if let egui::Event::Key { key, pressed, repeat: false, .. } = &event {
            if recent.ours.iter().any(|o| o.0 == *key && o.1 == *pressed) {
                continue;
            }
            recent.winit.push((*key, *pressed, now));
        }
        kept.push(event);
    }
    raw.events = kept;
    let keys: Vec<(u32, bool)> = KEYS.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default();
    let held = |vk: i32| unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000 != 0;
    let (ctrl, shift, alt) = (held(0x11), held(0x10), held(0x12));
    for (vk, pressed) in keys {
        let Some(key) = egui_key(vk) else { continue };
        if recent.winit.iter().any(|o| o.0 == key && o.1 == pressed) {
            continue;
        }
        recent.ours.push((key, pressed, now));
        KEYS_INJECTED.fetch_add(1, Ordering::Relaxed);
        raw.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers { alt, ctrl, shift, mac_cmd: false, command: ctrl },
        });
    }
}

/// Windows passes files on the command line; there is no open-document event.
pub fn register_open_handler(_ctx: &egui::Context) {}
pub fn take_opened_files() -> Vec<String> {
    Vec::new()
}
pub fn register_open_handler_at_launch() {}
