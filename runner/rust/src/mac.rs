//! macOS バックエンド。
//!
//! - ウィンドウ・イベント: AppKit を objc_msgSend で直接呼ぶ（GUIエンジンは使わない）。
//! - 画素: 自前のバッファを CGBitmapContext に渡す。図形はこちらで画素を書き、
//!   文字は OS 標準フォントを使って同じバッファに描く（NSString の描画）。
//! - 表示: 更新のたびに CGImage を作り、NSImageView に載せる。
//! - GPU の API（Metal / OpenGL / CoreAnimation の独自描画）は使わない。
//!   ウィンドウの合成は macOS のウィンドウサーバーが行う。

use core::ffi::{c_char, c_void};

use crate::app::{App, H, W};
use crate::canvas::{Canvas, Rgb, TextStyle};
use crate::sys::sleep_us;

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

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    static NSFontAttributeName: Id;
    static NSForegroundColorAttributeName: Id;
}

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
    fn CGContextScaleCTM(ctx: *mut c_void, sx: f64, sy: f64);
    fn CGContextSaveGState(ctx: *mut c_void);
    fn CGContextRestoreGState(ctx: *mut c_void);
    fn CGContextClipToRect(ctx: *mut c_void, rect: Rect);
}

/// kCGImageAlphaNoneSkipLast: メモリ上は R,G,B,X。canvas の画素配置と一致する。
const KCG_IMAGE_ALPHA_NONE_SKIP_LAST: u32 = 5;
/// NSUTF8StringEncoding
const NS_UTF8_ENCODING: usize = 4;
/// NSEventTypeLeftMouseDown / NSEventTypeScrollWheel
const EV_LEFT_MOUSE_DOWN: u64 = 1;
const EV_SCROLL_WHEEL: u64 = 22;

/// 0 終端の文字列リテラルを C 文字列ポインタにする。
fn cs(s: &'static [u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

unsafe fn cls(name: &'static [u8]) -> Id {
    objc_getClass(cs(name))
}

unsafe fn sel(name: &'static [u8]) -> Sel {
    sel_registerName(cs(name))
}

/// objc_msgSend を、呼び出し側が指定した関数型として取り出す。
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

unsafe fn m_bytes(o: Id, s: Sel, p: *const u8, len: usize, enc: usize) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *const u8, usize, usize) -> Id = msg();
    f(o, s, p, len, enc)
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

unsafe fn m_size_ret1(o: Id, s: Sel, a: Id) -> Size {
    let f: unsafe extern "C" fn(Id, Sel, Id) -> Size = msg();
    f(o, s, a)
}

unsafe fn m_font(o: Id, s: Sel, size: f64) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, f64) -> Id = msg();
    f(o, s, size)
}

unsafe fn m_font_named(o: Id, s: Sel, name: Id, size: f64) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, Id, f64) -> Id = msg();
    f(o, s, name, size)
}

unsafe fn m_font_trait(o: Id, s: Sel, font: Id, traits: usize) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, Id, usize) -> Id = msg();
    f(o, s, font, traits)
}

unsafe fn m_color(o: Id, s: Sel, r: f64, g: f64, b: f64, a: f64) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, f64, f64, f64, f64) -> Id = msg();
    f(o, s, r, g, b, a)
}

unsafe fn m_set(o: Id, s: Sel, obj: Id, key: Id) {
    let f: unsafe extern "C" fn(Id, Sel, Id, Id) = msg();
    f(o, s, obj, key)
}

unsafe fn m_gc(o: Id, s: Sel, ctx: *mut c_void, flipped: i8) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, *mut c_void, i8) -> Id = msg();
    f(o, s, ctx, flipped)
}

unsafe fn m_draw(o: Id, s: Sel, p: Point, attrs: Id) {
    let f: unsafe extern "C" fn(Id, Sel, Point, Id) = msg();
    f(o, s, p, attrs)
}

unsafe fn m_event(o: Id, s: Sel, mask: u64, until: Id, mode: Id, dequeue: i8) -> Id {
    let f: unsafe extern "C" fn(Id, Sel, u64, Id, Id, i8) -> Id = msg();
    f(o, s, mask, until, mode, dequeue)
}

// ---- 文字描画（OS 標準フォント） ----

/// 描画中のコンテキストと論理の高さ。draw_text / measure が参照する。
static mut TEXT_CTX: *mut c_void = core::ptr::null_mut();
static mut LOGICAL_H: f64 = 0.0;

fn style_font(st: TextStyle) -> (f64, bool) {
    match st {
        TextStyle::Title => (20.0, true),
        TextStyle::Heading => (15.0, true),
        TextStyle::Body => (13.0, false),
        TextStyle::Small => (11.0, false),
        TextStyle::Button => (13.0, true),
    }
}

unsafe fn ns_string(s: &[u8]) -> Id {
    m_bytes(
        cls(b"NSString\0"),
        sel(b"stringWithBytes:length:encoding:\0"),
        s.as_ptr(),
        s.len(),
        NS_UTF8_ENCODING,
    )
}

unsafe fn text_attrs(st: TextStyle, c: Rgb) -> Id {
    let (size, bold) = style_font(st);
    let font_cls = cls(b"NSFont\0");
    let harmony_name = ns_string(b"HarmonyOS Sans SC");
    let harmony_font = m_font_named(font_cls, sel(b"fontWithName:size:\0"), harmony_name, size);
    let font = if !harmony_font.is_null() {
        if bold {
            let manager = m_id(cls(b"NSFontManager\0"), sel(b"sharedFontManager\0"));
            let emphasized = m_font_trait(manager, sel(b"convertFont:toHaveTrait:\0"), harmony_font, 2);
            if emphasized.is_null() { harmony_font } else { emphasized }
        } else {
            harmony_font
        }
    } else if bold {
        m_font(font_cls, sel(b"boldSystemFontOfSize:\0"), size)
    } else {
        m_font(font_cls, sel(b"systemFontOfSize:\0"), size)
    };
    let color = m_color(
        cls(b"NSColor\0"),
        sel(b"colorWithSRGBRed:green:blue:alpha:\0"),
        c[0] as f64 / 255.0,
        c[1] as f64 / 255.0,
        c[2] as f64 / 255.0,
        1.0,
    );
    let dict = m_id(cls(b"NSMutableDictionary\0"), sel(b"dictionary\0"));
    m_set(dict, sel(b"setObject:forKey:\0"), font, NSFontAttributeName);
    m_set(dict, sel(b"setObject:forKey:\0"), color, NSForegroundColorAttributeName);
    dict
}

/// no_std には f64::ceil が無いので自前で切り上げる（非負の値のみ）。
fn ceil_px(v: f64) -> i32 {
    let t = v as i32;
    if v > t as f64 {
        t + 1
    } else {
        t
    }
}

/// 文字列の描画幅（論理ポイント）。
pub fn measure(s: &[u8], st: TextStyle) -> i32 {
    if s.is_empty() {
        return 0;
    }
    unsafe {
        let str_ = ns_string(s);
        let attrs = text_attrs(st, [0, 0, 0]);
        let size = m_size_ret1(str_, sel(b"sizeWithAttributes:\0"), attrs);
        ceil_px(size.w)
    }
}

/// 文字列を (x, y) を左上として描き、描画幅を返す。
/// clip は論理座標の (x0, y0, x1, y1)。
pub fn draw_text(x: i32, y: i32, s: &[u8], st: TextStyle, c: Rgb, clip: (i32, i32, i32, i32)) -> i32 {
    if s.is_empty() {
        return 0;
    }
    unsafe {
        let ctx = TEXT_CTX;
        if ctx.is_null() {
            return 0;
        }
        let lh = LOGICAL_H;
        let str_ = ns_string(s);
        let attrs = text_attrs(st, c);
        let size = m_size_ret1(str_, sel(b"sizeWithAttributes:\0"), attrs);

        // CoreGraphics の座標は左下が原点。NSString は左下の点を基準に描く。
        let (x0, y0, x1, y1) = clip;
        CGContextSaveGState(ctx);
        CGContextClipToRect(
            ctx,
            Rect {
                x: x0 as f64,
                y: lh - (y1 as f64),
                w: (x1 - x0) as f64,
                h: (y1 - y0) as f64,
            },
        );
        m_draw(
            str_,
            sel(b"drawAtPoint:withAttributes:\0"),
            Point { x: x as f64, y: lh - (y as f64 + size.h) },
            attrs,
        );
        CGContextRestoreGState(ctx);
        ceil_px(size.w)
    }
}

/// 画素バッファの内容を CGImage にして NSImageView に表示する。
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

/// ウィンドウを開き、閉じられるまでイベントを処理する。
/// buf は論理サイズ W×H の画素を、最大 2 倍（Retina）まで格納できる領域であること。
pub fn run(app: &mut App, buf: *mut u8) -> i32 {
    unsafe {
        let nsapp = m_id(cls(b"NSApplication\0"), sel(b"sharedApplication\0"));
        m_isize1(nsapp, sel(b"setActivationPolicy:\0"), 0);
        m_void(nsapp, sel(b"finishLaunching\0"));

        let frame = Rect { x: 0.0, y: 0.0, w: W as f64, h: H as f64 };
        let win = m_init_window(
            m_id(cls(b"NSWindow\0"), sel(b"alloc\0")),
            sel(b"initWithContentRect:styleMask:backing:defer:\0"),
            frame,
            1 | 2 | 4, // タイトル・閉じる・最小化（サイズは固定）
            2,         // NSBackingStoreBuffered
            0,
        );
        // 閉じた後に解放されると以降の呼び出しが壊れるため、解放を止める
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
        m_id1(win, sel(b"setContentView:\0"), view);
        m_void(win, sel(b"center\0"));
        m_id1(win, sel(b"makeKeyAndOrderFront:\0"), core::ptr::null_mut());
        m_bool1(nsapp, sel(b"activateIgnoringOtherApps:\0"), 1);

        // Retina など高解像度の画面では 2 倍の画素で描く
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
            return 1;
        }
        // 論理座標で描けるよう、CTM を s 倍する
        CGContextScaleCTM(ctx, s as f64, s as f64);
        TEXT_CTX = ctx;
        LOGICAL_H = H as f64;
        let gc = m_gc(
            cls(b"NSGraphicsContext\0"),
            sel(b"graphicsContextWithCGContext:flipped:\0"),
            ctx,
            0,
        );
        m_id1(cls(b"NSGraphicsContext\0"), sel(b"setCurrentContext:\0"), gc);

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
        let pool_alloc = sel(b"alloc\0");
        let pool_init = sel(b"init\0");
        let pool_drain = sel(b"drain\0");

        loop {
            // フレームごとにオートリリースプールを作って解放する（一時オブジェクトの蓄積を防ぐ）
            let pool = m_id(m_id(cls(b"NSAutoreleasePool\0"), pool_alloc), pool_init);

            let ev = m_event(nsapp, next_event, u64::MAX, distant_past, mode, 1);
            let mut dirty = app.tick(crate::sys::monotonic_ms());
            if !ev.is_null() {
                let t = m_u64(ev, ev_type);
                if t == EV_LEFT_MOUSE_DOWN {
                    let p = m_point_ret(ev, ev_loc);
                    // ウィンドウは左下が原点なので、論理座標（左上原点）に直す
                    dirty |= app.click(p.x, H as f64 - p.y);
                } else if t == EV_SCROLL_WHEEL {
                    dirty |= app.scroll(m_f64(ev, ev_dy));
                }
                m_id1(nsapp, send_event, ev);
            }
            if dirty {
                app.render(&mut canvas);
                present(view, ctx);
            }
            m_void(nsapp, update_windows);
            m_void(pool, pool_drain);

            if m_bool(win, is_visible) == 0 {
                break;
            }
            if ev.is_null() {
                sleep_us(4_000);
            }
        }
        TEXT_CTX = core::ptr::null_mut();
        0
    }
}
