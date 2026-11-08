//! macOS backend.
//!
//! - Window and input events: AppKit called directly through objc_msgSend.
//! - Pixels: the shared software Canvas renders into a buffer.
//! - Each update builds a CGImage and shows it in an NSImageView.

use core::ffi::{c_char, c_void};

use crate::app::{App, H, W};
use crate::canvas::Canvas;
use crate::sys::{eprint, monotonic_ms, sleep_us};

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

unsafe fn m_bool1(o: Id, s: Sel, a: i8) {
    let f: unsafe extern "C" fn(Id, Sel, i8) = msg();
    f(o, s, a)
}

unsafe fn m_isize1(o: Id, s: Sel, a: isize) {
    let f: unsafe extern "C" fn(Id, Sel, isize) = msg();
    f(o, s, a)
}

unsafe fn m_cstr(o: Id, s: Sel, p: *const c_char) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id = msg();
    f(o, s, p)
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
pub fn run(app: &mut App, buf: *mut u8) -> i32 {
    unsafe {
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
        m_isize1(nsapp, sel(b"setActivationPolicy:\0"), 0);
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
        m_bool1(win, sel(b"setReleasedWhenClosed:\0"), 0);

        let title = app.title();
        let mut tbuf = [0u8; 128];
        let n = if title.len() < tbuf.len() - 1 { title.len() } else { tbuf.len() - 1 };
        tbuf[..n].copy_from_slice(&title[..n]);
        m_id1(
            win,
            sel(b"setTitle:\0"),
            m_cstr(cls(b"NSString\0"), sel(b"stringWithUTF8String:\0"), tbuf.as_ptr() as *const c_char),
        );

        let view = m_init_rect(m_id(cls(b"NSImageView\0"), sel(b"alloc\0")), sel(b"initWithFrame:\0"), frame);
        if view.is_null() {
            eprint(b"mb5-runner: NSImageView init returned nil\n");
            return 1;
        }
        m_id1(win, sel(b"setContentView:\0"), view);
        m_void(win, sel(b"center\0"));

        // Activate the app first, then bring the window to the front.
        m_bool1(nsapp, sel(b"activateIgnoringOtherApps:\0"), 1);
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

        loop {
            let pool = m_id(m_id(cls(b"NSAutoreleasePool\0"), pool_alloc), pool_init);

            let ev = m_event(nsapp, next_event, u64::MAX, distant_past, mode, 1);
            let mut dirty = app.tick(monotonic_ms());
            if !ev.is_null() {
                let t = m_u64(ev, ev_type);
                if t == EV_LEFT_MOUSE_DOWN {
                    let p = m_point_ret(ev, ev_loc);
                    // Window origin is bottom-left; convert to top-left logical coords.
                    dirty |= app.click(p.x, H as f64 - p.y);
                } else if t == EV_SCROLL_WHEEL {
                    let raw_delta = m_f64(ev, ev_dy);
                    let logical_delta = if m_bool(ev, ev_precise_scroll) != 0 {
                        raw_delta
                    } else {
                        raw_delta * 20.0
                    };
                    dirty |= app.scroll(logical_delta);
                }
                m_id1(nsapp, send_event, ev);
            }
            if dirty {
                app.render(&mut canvas);
                present(view, ctx);
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
            }
        }
        0
    }
}