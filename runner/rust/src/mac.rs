//! macOS backend.
//!
//! - Window and input events: AppKit called directly through objc_msgSend.
//! - Pixels: the shared software Canvas renders into a buffer.
//! - Each update builds a CGImage and shows it in an NSImageView.

use core::ffi::{c_char, c_void};

use crate::app::{App, H, W};
use crate::canvas::Canvas;
use crate::sys::{eprint, monotonic_ms, sleep_us};

static mut ACTIVE_APP: *mut App = core::ptr::null_mut();
static mut INPUT_DIRTY: bool = false;
static mut INPUT_VIEW_CLASS: Id = core::ptr::null_mut();

type Id = *mut c_void;
type Sel = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    w: f64,
    h: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    fn objc_allocateClassPair(superclass: Id, name: *const c_char, extra_bytes: usize) -> Id;
    fn objc_registerClassPair(cls: Id);
    fn class_addMethod(cls: Id, name: Sel, imp: *const c_void, types: *const c_char) -> i8;
}

// AppKit must be linked, otherwise NSApplication / NSWindow / NSImageView
// are not registered and objc_getClass returns nil.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "Foundation", kind = "framework")]
extern "C" {
    static NSDefaultRunLoopMode: Id;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        color_space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGBitmapContextCreateImage(ctx: *mut c_void) -> *mut c_void;
    fn CGImageRelease(img: *mut c_void);
}

/// kCGImageAlphaNoneSkipLast: memory layout R,G,B,X, matching the canvas.
const KCG_IMAGE_ALPHA_NONE_SKIP_LAST: u32 = 5;
/// NSEventTypeLeftMouseDown / NSEventTypeScrollWheel
const EV_LEFT_MOUSE_DOWN: u64 = 1;
const EV_LEFT_MOUSE_UP: u64 = 2;
const EV_MOUSE_MOVED: u64 = 5;
const EV_LEFT_MOUSE_DRAGGED: u64 = 6;
const EV_SCROLL_WHEEL: u64 = 22;
/// How long to wait for the window to become visible (ms).
const SHOW_TIMEOUT_MS: u64 = 5000;

fn cs(s: &'static [u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

unsafe fn cls(name: &'static [u8]) -> Id {
    objc_getClass(cs(name))
}

unsafe fn sel(name: &'static [u8]) -> Sel {
    sel_registerName(cs(name))
}

/// Reinterpret objc_msgSend as the function type the caller needs.
unsafe fn msg<F: Copy>() -> F {
    let p: unsafe extern "C" fn() = objc_msgSend;
    core::mem::transmute_copy(&p)
}

unsafe fn m_id(o: Id, s: Sel) -> Id {
    let f: unsafe extern "C" fn(Id, Sel) -> Id = msg();
    f(o, s)
}

unsafe fn m_void(o: Id, s: Sel) {
    let f: unsafe extern "C" fn(Id, Sel) = msg();
    f(o, s)
}

unsafe fn m_u64(o: Id, s: Sel) -> u64 {
    let f: unsafe extern "C" fn(Id, Sel) -> u64 = msg();
    f(o, s)
}

unsafe fn m_f64(o: Id, s: Sel) -> f64 {
    let f: unsafe extern "C" fn(Id, Sel) -> f64 = msg();
    f(o, s)
}

unsafe fn m_bool(o: Id, s: Sel) -> i8 {
    let f: unsafe extern "C" fn(Id, Sel) -> i8 = msg();
    f(o, s)
}

unsafe fn m_id1(o: Id, s: Sel, a: Id) {
    let f: unsafe extern "C" fn(Id, Sel, Id) = msg();
    f(o, s, a)
}

unsafe fn m_void_bool1(o: Id, s: Sel, a: i8) {
    let f: unsafe extern "C" fn(Id, Sel, i8) = msg();
    f(o, s, a)
}

unsafe fn m_bool_isize1(o: Id, s: Sel, a: isize) -> i8 {
    let f: unsafe extern "C" fn(Id, Sel, isize) -> i8 = msg();
    f(o, s, a)
}

unsafe fn m_cstr(o: Id, s: Sel, p: *const c_char) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id = msg();
    f(o, s, p)
}

unsafe fn m_cstr_ret(o: Id, s: Sel) -> *const c_char {
    let f: unsafe extern "C" fn(Id, Sel) -> *const c_char = msg();
    f(o, s)
}

extern "C" fn mb5_accepts_first_responder(_this: Id, _cmd: Sel) -> i8 { 1 }

extern "C" fn mb5_key_down(_this: Id, _cmd: Sel, event: Id) {
    unsafe {
        if ACTIVE_APP.is_null() || event.is_null() { return; }
        let key = m_u64(event, sel(b"keyCode\0"));
        if key == 51 {
            if (*ACTIVE_APP).text_input(&[], true) { INPUT_DIRTY = true; }
            return;
        }
        let chars = m_id(event, sel(b"characters\0"));
        if chars.is_null() { return; }
        let text = m_cstr_ret(chars, sel(b"UTF8String\0"));
        if text.is_null() { return; }
        let mut bytes = [0u8; 64];
        let mut len = 0usize;
        while len < bytes.len() && *text.add(len) != 0 {
            let byte = *text.add(len) as u8;
            if byte < 0x20 { return; }
            bytes[len] = byte;
            len += 1;
        }
        if len > 0 && (*ACTIVE_APP).text_input(&bytes[..len], false) { INPUT_DIRTY = true; }
    }
}

unsafe fn canvas_view_class() -> Id {
    if !INPUT_VIEW_CLASS.is_null() { return INPUT_VIEW_CLASS; }
    let superclass = cls(b"NSImageView\0");
    if superclass.is_null() { return core::ptr::null_mut(); }
    let name = b"MB5CanvasInputView\0";
    let class = objc_allocateClassPair(superclass, name.as_ptr() as *const c_char, 0);
    if class.is_null() { return superclass; }
    let accepts: extern "C" fn(Id, Sel) -> i8 = mb5_accepts_first_responder;
    let key_down: extern "C" fn(Id, Sel, Id) = mb5_key_down;
    let accepts_imp = core::mem::transmute::<extern "C" fn(Id, Sel) -> i8, *const c_void>(accepts);
    let key_down_imp = core::mem::transmute::<extern "C" fn(Id, Sel, Id), *const c_void>(key_down);
    let ok_a = class_addMethod(class, sel(b"acceptsFirstResponder\0"), accepts_imp, b"c@:\0".as_ptr() as *const c_char);
    let ok_k = class_addMethod(class, sel(b"keyDown:\0"), key_down_imp, b"v@:@\0".as_ptr() as *const c_char);
    if ok_a == 0 || ok_k == 0 { return core::ptr::null_mut(); }
    objc_registerClassPair(class);
    INPUT_VIEW_CLASS = class;
    class
}

unsafe fn m_init_rect(o: Id, s: Sel, r: Rect) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, Rect) -> Id = msg();
    f(o, s, r)
}

unsafe fn m_init_window(o: Id, s: Sel, r: Rect, style: usize, backing: usize, defer: i8) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, Rect, usize, usize, i8) -> Id = msg();
    f(o, s, r, style, backing, defer)
}

unsafe fn m_init_image(o: Id, s: Sel, cg: *mut c_void, size: Size) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *mut c_void, Size) -> Id = msg();
    f(o, s, cg, size)
}

unsafe fn m_point_ret(o: Id, s: Sel) -> Point {
    let f: unsafe extern "C" fn(Id, Sel) -> Point = msg();
    f(o, s)
}

unsafe fn m_event(o: Id, s: Sel, mask: u64, until: Id, mode: Id, dequeue: i8) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, u64, Id, Id, i8) -> Id = msg();
    f(o, s, mask, until, mode, dequeue)
}

/// Show the software framebuffer in the NSImageView.
unsafe fn present(view: Id, ctx: *mut c_void) {
    let img = CGBitmapContextCreateImage(ctx);
    if img.is_null() {
        return;
    }
    let ns_image = m_init_image(
        m_id(cls(b"NSImage\0"), sel(b"alloc\0")),
        sel(b"initWithCGImage:size:\0"),
        img,
        Size { w: W as f64, h: H as f64 },
    );
    m_id1(view, sel(b"setImage:\0"), ns_image);
    m_void(ns_image, sel(b"release\0"));
    CGImageRelease(img);
}

/// Open the window and run until it is closed.
/// buf must hold up to 2x the logical W x H pixels (for Retina).
pub fn run(app: &mut App, buf: *mut u8, speed_percent: Option<u32>) -> i32 {
    unsafe {
        ACTIVE_APP = app as *mut App;
        INPUT_DIRTY = false;
        let nsapp_class = cls(b"NSApplication\0");
        if nsapp_class.is_null() {
            eprint(b"mb5-runner: NSApplication class not found (AppKit not loaded)\n");
            return 1;
        }
        let nsapp = m_id(nsapp_class, sel(b"sharedApplication\0"));
        if nsapp.is_null() {
            eprint(b"mb5-runner: sharedApplication returned nil\n");
            return 1;
        }
        let _ = m_bool_isize1(nsapp, sel(b"setActivationPolicy:\0"), 0);
        m_void(nsapp, sel(b"finishLaunching\0"));

        let window_class = cls(b"NSWindow\0");
        if window_class.is_null() {
            eprint(b"mb5-runner: NSWindow class not found (AppKit not loaded)\n");
            return 1;
        }
        let frame = Rect { x: 0.0, y: 0.0, w: W as f64, h: H as f64 };
        let win = m_init_window(
            m_id(window_class, sel(b"alloc\0")),
            sel(b"initWithContentRect:styleMask:backing:defer:\0"),
            frame,
            1 | 2 | 4, // title, close, minimize (fixed size)
            2,         // NSBackingStoreBuffered
            0,
        );
        if win.is_null() {
            eprint(b"mb5-runner: NSWindow init returned nil\n");
            return 1;
        }
        // Do not release the window when closed; later calls would break.
        m_void_bool1(win, sel(b"setReleasedWhenClosed:\0"), 0);
        // Request mouseMoved events so software-rendered controls can show hover states.
        m_void_bool1(win, sel(b"setAcceptsMouseMovedEvents:\0"), 1);

        let title = app.title();
        let mut tbuf = [0u8; 128];
        let n = if title.len() < tbuf.len() - 1 { title.len() } else { tbuf.len() - 1 };
        tbuf[..n].copy_from_slice(&title[..n]);
        m_id1(
            win,
            sel(b"setTitle:\0"),
            m_cstr(cls(b"NSString\0"), sel(b"stringWithUTF8String:\0"), tbuf.as_ptr() as *const c_char),
        );

        let view_class = canvas_view_class();
        if view_class.is_null() { eprint(b"mb5-runner: input view class creation failed\n"); return 1; }
        let view = m_init_rect(m_id(view_class, sel(b"alloc\0")), sel(b"initWithFrame:\0"), frame);
        if view.is_null() {
            eprint(b"mb5-runner: NSImageView init returned nil\n");
            return 1;
        }
        m_id1(win, sel(b"setContentView:\0"), view);
        let make_first: unsafe extern "C" fn(Id, Sel, Id) -> i8 = msg();
        let _ = make_first(win, sel(b"makeFirstResponder:\0"), view);
        m_void(win, sel(b"center\0"));

        // Activate the app first, then bring the window to the front.
        m_void_bool1(nsapp, sel(b"activateIgnoringOtherApps:\0"), 1);
        m_id1(win, sel(b"makeKeyAndOrderFront:\0"), core::ptr::null_mut());
        m_void(win, sel(b"orderFrontRegardless\0"));

        // Use a 2x buffer on Retina displays.
        let backing = m_f64(win, sel(b"backingScaleFactor\0"));
        let s: usize = if backing >= 2.0 { 2 } else { 1 };
        let dev_w = W as usize * s;
        let dev_h = H as usize * s;
        let cs_rgb = CGColorSpaceCreateDeviceRGB();
        let ctx = CGBitmapContextCreate(
            buf as *mut c_void,
            dev_w,
            dev_h,
            8,
            dev_w * 4,
            cs_rgb,
            KCG_IMAGE_ALPHA_NONE_SKIP_LAST,
        );
        if ctx.is_null() {
            eprint(b"mb5-runner: CGBitmapContextCreate returned nil\n");
            return 1;
        }

        let mut canvas = Canvas::new(buf, s, W, H);
        app.render(&mut canvas);
        present(view, ctx);

        let distant_past = m_id(cls(b"NSDate\0"), sel(b"distantPast\0"));
        let mode = NSDefaultRunLoopMode;
        let next_event = sel(b"nextEventMatchingMask:untilDate:inMode:dequeue:\0");
        let send_event = sel(b"sendEvent:\0");
        let update_windows = sel(b"updateWindows\0");
        let is_visible = sel(b"isVisible\0");
        let ev_type = sel(b"type\0");
        let ev_loc = sel(b"locationInWindow\0");
        let ev_dy = sel(b"scrollingDeltaY\0");
        let ev_precise_scroll = sel(b"hasPreciseScrollingDeltas\0");
        let pool_alloc = sel(b"alloc\0");
        let pool_init = sel(b"init\0");
        let pool_drain = sel(b"drain\0");

        let start_ms = monotonic_ms();
        let mut shown = false;
        let mut pending_dirty = true;
        let mut last_render_ms = 0u64;
        // 100% = 60FPS / 64 script commands per tick. Lower percentages
        // proportionally reduce the frame and script budgets and stretch waits.
        let (frame_interval_ms, script_budget, time_scale_permille) = match speed_percent {
            Some(percent) => {
                let normalized = percent.min(100).max(1);
                let fps = ((60u64 * normalized as u64) / 100).max(1);
                let budget = ((64u64 * normalized as u64) / 100).max(1) as usize;
                let time_scale = (100_000u64 / normalized as u64).min(u32::MAX as u64) as u32;
                ((1000u64 + fps - 1) / fps, budget, time_scale)
            }
            None => (0, 64, 1000),
        };
        if let Some(percent) = speed_percent {
            eprint(b"mb5-runner: software speed limit enabled: ");
            let mut digits = [0u8; 3]; let mut value = percent; let mut len = 0usize;
            while value > 0 && len < digits.len() { digits[len] = b'0' + (value % 10) as u8; value /= 10; len += 1; }
            if len == 0 { crate::sys::write_all(crate::sys::STDERR, b"0"); }
            while len > 0 { len -= 1; crate::sys::write_all(crate::sys::STDERR, &digits[len..len + 1]); }
            eprint(b"%\n");
        }

        loop {
            let pool = m_id(m_id(cls(b"NSAutoreleasePool\0"), pool_alloc), pool_init);

            let ev = m_event(nsapp, next_event, u64::MAX, distant_past, mode, 1);
            let now_ms = monotonic_ms();
            let mut dirty = app.tick_with_profile(now_ms, script_budget, time_scale_permille);
            if !ev.is_null() {
                let t = m_u64(ev, ev_type);
                if t == EV_LEFT_MOUSE_DOWN {
                    let p = m_point_ret(ev, ev_loc);
                    // Window origin is bottom-left; convert to top-left logical coords.
                    dirty |= app.pointer_press(p.x, H as f64 - p.y);
                } else if t == EV_LEFT_MOUSE_UP {
                    let p = m_point_ret(ev, ev_loc);
                    let x = p.x;
                    let y = H as f64 - p.y;
                    dirty |= app.pointer_release(x, y);
                    dirty |= app.click(x, y);
                } else if t == EV_MOUSE_MOVED || t == EV_LEFT_MOUSE_DRAGGED {
                    let p = m_point_ret(ev, ev_loc);
                    dirty |= app.pointer_move(p.x, H as f64 - p.y);
                } else if t == EV_SCROLL_WHEEL {
                    let raw_delta = m_f64(ev, ev_dy);
                    let logical_delta = if m_bool(ev, ev_precise_scroll) != 0 {
                        raw_delta
                    } else {
                        raw_delta * 20.0
                    };
                    let p = m_point_ret(ev, ev_loc);
                    dirty |= app.scroll_at(p.x, H as f64 - p.y, logical_delta);
                }
                m_id1(nsapp, send_event, ev);
                if INPUT_DIRTY { pending_dirty = true; INPUT_DIRTY = false; }
            }
            pending_dirty |= dirty;
            let render_due = frame_interval_ms == 0 || now_ms.saturating_sub(last_render_ms) >= frame_interval_ms;
            if pending_dirty && render_due {
                app.render(&mut canvas);
                present(view, ctx);
                last_render_ms = now_ms;
                pending_dirty = false;
            }
            m_void(nsapp, update_windows);
            m_void(pool, pool_drain);

            let visible = m_bool(win, is_visible) != 0;
            if visible {
                shown = true;
            } else if shown {
                // Was shown and is now closed: normal exit.
                break;
            } else if monotonic_ms().saturating_sub(start_ms) > SHOW_TIMEOUT_MS {
                eprint(b"mb5-runner: window did not become visible\n");
                return 1;
            }

            if ev.is_null() {
                sleep_us(4_000);
            } else if frame_interval_ms > 0 {
                // Avoid a busy event-poll loop while a speed profile is active.
                sleep_us(1_000);
            }
        }
        ACTIVE_APP = core::ptr::null_mut();
        0
    }
}