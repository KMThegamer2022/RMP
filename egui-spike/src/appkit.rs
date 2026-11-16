//! AppKit glue: the video view, window chrome, wallpaper mode, About panel, pasteboard and symbols.

use crate::theme::Skin;
use eframe::egui::{self, Rect};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSApplication, NSColor, NSImage, NSPasteboard, NSView, NSVisualEffectView,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSData, NSDictionary, NSPoint, NSRect, NSSize, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::c_void;

/// winit's content NSView for this window.
pub fn content_view(frame: &eframe::Frame) -> Option<&NSView> {
    let h = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(a) = h.as_raw() else { return None };
    // SAFETY: winit keeps the content view alive for the window's lifetime.
    Some(unsafe { &*(a.ns_view.as_ptr() as *const NSView) })
}

/// The view libvlc draws into, and the Windows 7 glass deck. Both sit behind egui's view in the
/// window's frame view, so egui's controls, menus and dialogs draw on top and the video stays visible.
pub struct Surfaces {
    content: Retained<NSView>,
    video: Retained<NSView>,
    glass: Retained<NSVisualEffectView>,
}

impl Surfaces {
    pub fn attach(frame: &eframe::Frame) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let parent = content_view(frame)?;
        let frame_view = unsafe { parent.superview() }?;
        let zero = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
        let video = NSView::initWithFrame(NSView::alloc(mtm), zero);
        video.setWantsLayer(true);
        let glass = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), zero);
        glass.setMaterial(objc2_app_kit::NSVisualEffectMaterial::Light);
        glass.setBlendingMode(objc2_app_kit::NSVisualEffectBlendingMode::BehindWindow);
        glass.setState(objc2_app_kit::NSVisualEffectState::Active);
        glass.setHidden(true);
        frame_view.addSubview_positioned_relativeTo(&glass, objc2_app_kit::NSWindowOrderingMode::Below, Some(parent));
        frame_view.addSubview_positioned_relativeTo(&video, objc2_app_kit::NSWindowOrderingMode::Below, Some(parent));
        // SAFETY: winit's content view lives as long as the window; keep a retained handle.
        let content = unsafe { Retained::retain(parent as *const NSView as *mut NSView) }?;
        Some(Surfaces { content, video, glass })
    }

    pub fn video_handle(&self) -> *mut c_void {
        Retained::as_ptr(&self.video) as *mut c_void
    }

    fn to_frame(&self, rect: Rect) -> NSRect {
        let y = if self.content.isFlipped() { rect.min.y } else { self.content.bounds().size.height as f32 - rect.max.y };
        let r = NSRect::new(NSPoint::new(rect.min.x as f64, y as f64), NSSize::new(rect.width() as f64, rect.height() as f64));
        match unsafe { self.video.superview() } {
            Some(fv) => self.content.convertRect_toView(r, Some(&fv)),
            None => r,
        }
    }

    pub fn place_video(&self, rect: Option<Rect>) {
        match rect {
            Some(r) => {
                self.video.setHidden(false);
                self.video.setFrame(self.to_frame(r));
            }
            None => self.video.setHidden(true),
        }
    }

    pub fn place_glass(&self, rect: Option<Rect>) {
        match rect {
            Some(r) => {
                self.glass.setHidden(false);
                self.glass.setFrame(self.to_frame(r));
            }
            None => self.glass.setHidden(true),
        }
    }

    pub fn window(&self) -> Option<Retained<NSWindow>> {
        self.content.window()
    }
}

fn ns_color(c: egui::Color32) -> Retained<NSColor> {
    let [r, g, b, a] = c.to_array();
    NSColor::colorWithSRGBRed_green_blue_alpha(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, a as f64 / 255.0)
}

/// True when the user asked macOS to reduce transparency (applyTheme turns the Windows 7 glass off).
pub fn reduce_transparency() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
}

/// applyTheme's window chrome: transparent titlebar, light/dark appearance, background colour,
/// and a clear, non-opaque window for the Windows 7 glass.
pub fn apply_window_chrome(window: &NSWindow, skin: Skin, translucent: bool) {
    window.setTitlebarAppearsTransparent(true);
    let appearance = unsafe {
        if skin.theme().dark {
            NSAppearance::appearanceNamed(objc2_app_kit::NSAppearanceNameDarkAqua)
        } else {
            NSAppearance::appearanceNamed(objc2_app_kit::NSAppearanceNameAqua)
        }
    };
    window.setAppearance(appearance.as_deref());
    window.setOpaque(!translucent);
    if translucent {
        window.setBackgroundColor(Some(&NSColor::clearColor()));
    } else {
        window.setBackgroundColor(Some(&ns_color(skin.window_background())));
    }
}

/// Always on Top (View menu): NSFloatingWindowLevel or NSNormalWindowLevel.
pub fn set_floating(window: &NSWindow, floating: bool) {
    window.setLevel(if floating { 3 } else { 0 });
}

extern "C" {
    fn CGWindowLevelForKey(key: i32) -> i32;
}
const K_CG_DESKTOP_ICON_WINDOW_LEVEL_KEY: i32 = 18;

/// What toggleWallpaper restores.
pub struct WallpaperRestore {
    style: NSWindowStyleMask,
    collection: NSWindowCollectionBehavior,
    frame: NSRect,
}

/// Set as Wallpaper: a borderless window behind the desktop icons, covering the screen, ignoring the mouse.
pub fn enter_wallpaper(window: &NSWindow) -> WallpaperRestore {
    let restore = WallpaperRestore { style: window.styleMask(), collection: window.collectionBehavior(), frame: window.frame() };
    window.setStyleMask(NSWindowStyleMask::Borderless);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::IgnoresCycle,
    );
    window.setLevel(unsafe { CGWindowLevelForKey(K_CG_DESKTOP_ICON_WINDOW_LEVEL_KEY) } as isize - 1);
    if let Some(screen) = window.screen() {
        window.setFrame_display(screen.frame(), true);
    }
    window.setIgnoresMouseEvents(true);
    window.orderFront(None);
    restore
}

pub fn leave_wallpaper(window: &NSWindow, restore: &WallpaperRestore, always_top: bool) {
    window.setIgnoresMouseEvents(false);
    window.setStyleMask(restore.style);
    window.setCollectionBehavior(restore.collection);
    set_floating(window, always_top);
    window.setFrame_display(restore.frame, true);
    window.makeKeyAndOrderFront(None);
}

/// Hides the macOS menu bar and Dock while fullscreen, so the top edge reveals the egui menu bar.
pub fn set_presentation(fullscreen: bool) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let options = if fullscreen {
        objc2_app_kit::NSApplicationPresentationOptions::HideMenuBar | objc2_app_kit::NSApplicationPresentationOptions::HideDock
    } else {
        objc2_app_kit::NSApplicationPresentationOptions::Default
    };
    app.setPresentationOptions(options);
}

/// True when the mouse is within a few points of the top of the main screen.
pub fn pointer_near_screen_top() -> bool {
    let Some(mtm) = MainThreadMarker::new() else { return false };
    let mouse = objc2_app_kit::NSEvent::mouseLocation();
    let Some(screen) = objc2_app_kit::NSScreen::mainScreen(mtm) else { return false };
    mouse.y > screen.frame().size.height - 6.0
}

/// The window's rectangle as `screencapture -R` wants it: x,y,w,h from the top-left of the main screen.
pub fn capture_rect(window: &NSWindow) -> Option<String> {
    let mtm = MainThreadMarker::new()?;
    let screen = objc2_app_kit::NSScreen::mainScreen(mtm)?;
    let f = window.frame();
    let top = screen.frame().size.height - (f.origin.y + f.size.height);
    Some(format!("{},{},{},{}", f.origin.x.round(), top.round(), f.size.width.round(), f.size.height.round()))
}

/// Mouse location in screen coordinates (refreshFloating compares it between ticks).
pub fn mouse_location() -> (f64, f64) {
    let p = objc2_app_kit::NSEvent::mouseLocation();
    (p.x, p.y)
}

fn image_from_png(png: &[u8]) -> Option<Retained<NSImage>> {
    use objc2::AnyThread;
    NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(png))
}

/// The Dock icon: the shipping bundle's RMP.icns. The spike is not a bundle, so set it at runtime.
pub fn set_app_icon(png: &[u8]) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    if let Some(image) = image_from_png(png) {
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
}

/// orderFrontStandardAboutPanel: with the bundle's name, version and icon.
pub fn show_about_panel(version: &str, icon_png: &[u8]) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApplication::sharedApplication(mtm);
    let name = NSString::from_str("RMP");
    let ver = NSString::from_str(version);
    let empty = NSString::from_str("");
    let icon = image_from_png(icon_png);
    unsafe {
        let mut keys: Vec<&NSString> = vec![
            objc2_app_kit::NSAboutPanelOptionApplicationName,
            objc2_app_kit::NSAboutPanelOptionApplicationVersion,
            objc2_app_kit::NSAboutPanelOptionVersion,
        ];
        let mut values: Vec<&AnyObject> = vec![&name, &ver, &empty];
        if let Some(icon) = icon.as_deref() {
            keys.push(objc2_app_kit::NSAboutPanelOptionApplicationIcon);
            values.push(icon);
        }
        let dict = NSDictionary::from_slices(&keys, &values);
        app.orderFrontStandardAboutPanelWithOptions(&dict);
        app.activate();
    }
}

/// Open Location from Clipboard: a file URL first, then plain text (runMenuCommand "clipboard").
pub fn pasteboard_location() -> Option<String> {
    let pb = NSPasteboard::generalPasteboard();
    unsafe {
        if let Some(url) = pb.stringForType(objc2_app_kit::NSPasteboardTypeFileURL) {
            let s = url.to_string();
            if let Some(path) = s.strip_prefix("file://") {
                return Some(percent_decode(path));
            }
        }
        let text = pb.stringForType(objc2_app_kit::NSPasteboardTypeString)?.to_string();
        let t = text.trim().to_string();
        (!t.is_empty()).then_some(t)
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

/// NSWorkspace openURL: through `open`, which uses the user's default browser.
pub fn open_url(url: &str) {
    let _ = std::process::Command::new("/usr/bin/open").arg(url).spawn();
}

/// An SF Symbol rendered to RGBA (navigation icons and cover placeholders in mac-library.h).
pub fn symbol_image(name: &str, point_size: f64) -> Option<egui::ColorImage> {
    let _mtm = MainThreadMarker::new()?;
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)?;
    // Draw at 3x the requested size so it stays sharp after scaling.
    let size = image.size();
    let scale = (point_size * 3.0) / size.height.max(1.0);
    image.setSize(NSSize::new(size.width * scale, size.height * scale));
    let tiff = image.TIFFRepresentation()?;
    let decoded = image::load_from_memory(&tiff.to_vec()).ok()?.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied([decoded.width() as usize, decoded.height() as usize], decoded.as_raw()))
}

/// The bundle path when running from inside an .app (the desktop shortcut links to it).
pub fn bundle_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")).map(|p| p.to_path_buf())
}

/// setDesktopShortcut: ~/Desktop/RMP.app is a symlink to the installed app.
/// Returns false when there is no bundle to link to (the spike runs from target/).
pub fn set_desktop_shortcut(on: bool) -> bool {
    let Some(home) = std::env::var_os("HOME") else { return false };
    let link = std::path::PathBuf::from(home).join("Desktop/RMP.app");
    let Some(app) = bundle_path() else { return false };
    if let Ok(meta) = std::fs::symlink_metadata(&link) {
        if meta.file_type().is_symlink() {
            let _ = std::fs::remove_file(&link);
        } else {
            return true; // A real file is there; leave it alone, as the shipping code does.
        }
    }
    if on {
        let _ = std::os::unix::fs::symlink(app, link);
    }
    true
}

/// True while this app is the active application.
pub fn app_active() -> bool {
    MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isActive())
}

/// Windows-only: keys that did not reach the main window. Nothing to do on macOS.
pub fn inject_keys(_raw: &mut eframe::egui::RawInput) {}

/// Test hook: what the mouse polling has seen (Windows only).
pub fn input_stats() -> String {
    String::new()
}

/// Windows-only: a double-click on the libvlc window. On macOS egui sees the click itself.
pub fn take_video_double_click() -> bool {
    false
}

/// Saves a PNG of the window, or of the whole screen when `window` is None (test hook).
pub fn screenshot(window: Option<&NSWindow>, path: &str) {
    let args: Vec<String> = match window.and_then(capture_rect) {
        Some(rect) => vec!["-x".into(), "-R".into(), rect, path.into()],
        None => vec!["-x".into(), path.into()],
    };
    let _ = std::process::Command::new("/usr/sbin/screencapture").args(args).status();
}

// ---- Open With / double-click in Finder --------------------------------------------------------
// Finder asks an app to open documents with an Apple Event ('aevt' 'odoc'). winit does not surface it,
// so a small handler object collects the file paths for the UI to open.

static OPENED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
static REPAINT: std::sync::OnceLock<egui::Context> = std::sync::OnceLock::new();

objc2::define_class!(
    #[unsafe(super(objc2_foundation::NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RMPOpenDocumentsHandler"]
    struct OpenHandler;

    impl OpenHandler {
        #[unsafe(method(handleOpen:withReply:))]
        fn handle_open(&self, event: &objc2_foundation::NSAppleEventDescriptor, _reply: &objc2_foundation::NSAppleEventDescriptor) {
            // keyDirectObject '----': a list of file URLs.
            if std::env::var_os("RMP_SPIKE_DEBUG").is_some() {
                eprintln!("odoc event received");
            }
            let Some(list) = event.paramDescriptorForKeyword(0x2D2D_2D2D) else { return };
            let mut paths = Vec::new();
            let debug = std::env::var_os("RMP_SPIKE_DEBUG").is_some();
            if debug {
                eprintln!("odoc list items: {}", list.numberOfItems());
            }
            for i in 1..=list.numberOfItems() {
                let Some(item) = list.descriptorAtIndex(i) else { continue };
                // The item is a file URL descriptor; fileURLValue resolves it (bookmark or file:// form).
                if let Some(path) = item.fileURLValue().and_then(|u| u.path()) {
                    paths.push(path.to_string());
                } else if let Some(url) = item.coerceToDescriptorType(0x6675_726C).and_then(|d| d.stringValue()) {
                    if let Some(path) = url.to_string().strip_prefix("file://") {
                        paths.push(percent_decode(path));
                    }
                }
            }
            if debug {
                eprintln!("odoc paths: {paths:?}");
            }
            if let Ok(mut q) = OPENED.lock() {
                q.extend(paths);
            }
            if let Some(ctx) = REPAINT.get() {
                ctx.request_repaint();
            }
        }
    }
);

/// Gives the open-document handler the context it wakes when a file arrives.
pub fn register_open_handler(ctx: &egui::Context) {
    let _ = REPAINT.set(ctx.clone());
}

/// A file that launched the app is delivered before the app finishes launching, and AppKit installs its
/// own handler first. Registering from the will-finish-launching notification (call this before the
/// event loop starts) puts this one in place in time.
pub fn register_open_handler_at_launch() {
    let Some(_mtm) = MainThreadMarker::new() else { return };
    let block = block2::RcBlock::new(|_note: std::ptr::NonNull<objc2_foundation::NSNotification>| install_open_handler());
    let center = objc2_foundation::NSNotificationCenter::defaultCenter();
    unsafe {
        let name = objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
        let token = center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block);
        std::mem::forget(token);
    }
}

fn install_open_handler() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let handler: Retained<OpenHandler> = unsafe { objc2::msg_send![mtm.alloc::<OpenHandler>(), init] };
    unsafe {
        objc2_foundation::NSAppleEventManager::sharedAppleEventManager().setEventHandler_andSelector_forEventClass_andEventID(
            &handler,
            objc2::sel!(handleOpen:withReply:),
            0x6165_7674, // kCoreEventClass 'aevt'
            0x6F64_6F63, // kAEOpenDocuments 'odoc'
        );
    }
    if std::env::var_os("RMP_SPIKE_DEBUG").is_some() {
        eprintln!("odoc handler registered");
    }
    std::mem::forget(handler); // the event manager keeps only a weak reference
}

/// Files Finder asked this app to open since the last call.
pub fn take_opened_files() -> Vec<String> {
    OPENED.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default()
}
