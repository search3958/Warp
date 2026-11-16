//! Mosaic Board 5 ランナー（no_std / GUIエンジンなし / GPUなし）
//!
//! 使い方:
//!   mb5-runner [components_dir] [--ppm] [--speed-percent N]
//!
//!   components_dir : components/ のパス（省略時は ./components）
//!   --ppm          : ウィンドウを開かず、描画結果を PPM (P6) として標準出力に書く
//!   --speed-percent N : ソフトウェア速度上限（1〜100%、100% = 通常速度）
//!
//! 図形・角丸・テキストは canvas.rs と font.rs がOS非依存で描画する。
//! macOSでは画素バッファをAppKitのウィンドウに表示する。
//! --ppmはウィンドウバックエンドなしで同じ描画結果を書き出す。

#![no_std]
#![no_main]

mod app;
mod canvas;
mod font;
mod project;
mod script;
mod sys;

#[cfg(target_os = "macos")]
mod mac;

use core::ffi::c_char;
use core::panic::PanicInfo;
use core::ptr::addr_of_mut;

use app::{App, H, W};
use canvas::Canvas;
use project::Project;
use sys::CPath;

/// ファイル読み込み用バッファ（bss 領域）。
static mut SRC: [u8; 256 * 1024] = [0; 256 * 1024];
/// 画素バッファ（論理サイズの 2 倍まで、bss 領域）。
const MAX_DEV_W: usize = W as usize * 2;
const MAX_DEV_H: usize = H as usize * 2;
static mut PIXELS: [u8; MAX_DEV_W * MAX_DEV_H * 4] = [0; MAX_DEV_W * MAX_DEV_H * 4];
/// Large fixed-capacity project state lives in static storage, not the 8 MiB
/// macOS main-thread stack. Project is exclusively borrowed by App after parsing.
static mut PROJECT: Project = Project::new();

#[panic_handler]
fn on_panic(_info: &PanicInfo) -> ! {
    sys::eprint(b"mb5-runner: internal error\n");
    sys::die(101)
}

fn usage() {
    sys::eprint(b"usage: mb5-runner [components_dir] [--ppm] [--speed-percent 1..100]\n");
}

/// `dir/rel` を読み込む。
fn load(dir: &[u8], rel: &[u8], buf: &mut [u8]) -> Option<usize> {
    let mut d = dir;
    while d.len() > 1 && d[d.len() - 1] == b'/' {
        d = &d[..d.len() - 1];
    }
    let mut path = CPath::new();
    if !(path.push(d) && path.push(b"/") && path.push(rel)) {
        return None;
    }
    sys::read_file(path.as_cstr(), buf)
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) -> i32 {
    let mut dir: &[u8] = &b"components"[..];
    let mut ppm = false;
    let mut help = false;
    let mut speed_percent: Option<u32> = None;

    let mut i = 1usize;
    while i < argc as usize {
        let a = unsafe { sys::cstr_arg(argv, i) };
        if a == b"--ppm" {
            ppm = true;
        } else if a == b"-h" || a == b"--help" {
            help = true;
        } else if a == b"--speed-percent" {
            i += 1;
            if i >= argc as usize { usage(); return 2; }
            let value = unsafe { sys::cstr_arg(argv, i) };
            let mut parsed = 0u32;
            if value.is_empty() || value.iter().any(|b| !b.is_ascii_digit()) { usage(); return 2; }
            for &digit in value { parsed = match parsed.checked_mul(10).and_then(|v| v.checked_add((digit - b'0') as u32)) { Some(v) => v, None => { usage(); return 2; } }; }
            if parsed == 0 || parsed > 100 { usage(); return 2; }
            speed_percent = Some(parsed);
        } else {
            dir = a;
        }
        i += 1;
    }
    if help {
        usage();
        return 0;
    }

    let buf = unsafe { &mut *addr_of_mut!(SRC) };
    // Project contains large fixed-capacity component/script tables. Keeping it
    // in static storage avoids copying several MiB through the macOS main stack.
    let proj_ptr = addr_of_mut!(PROJECT);
    let proj: &'static mut Project = unsafe { &mut *proj_ptr };

    if let Some(n) = load(dir, b"config.ini", buf) {
        project::parse_config(proj, &buf[..n]);
    }

    let n = match load(dir, b"app/main.mb5", buf) {
        Some(n) => n,
        None => {
            sys::eprint("app/main.mb5 が読めません（components_dir を確認してください）\n".as_bytes());
            return 1;
        }
    };
    project::parse_mb5(proj, &buf[..n]);

    if let Some(n) = load(dir, b"models/main.yaml", buf) {
        project::resolve_labels(proj, &buf[..n]);
    }

    // A separate scripts/main.mb5 is preferred. The inline @mainScript
    // section remains supported as a backwards-compatible fallback.
    if let Some(n) = load(dir, b"scripts/main.mb5", buf) {
        proj.script_source.set_external(&buf[..n]);
    }

    for diagnostic in proj.diagnostics.iter().take(proj.ndiagnostics) {
        sys::eprint(b"MB5 warning: ");
        sys::eprint(diagnostic.get());
        sys::eprint(b"\n");
    }

    if !font::init() {
        sys::eprint(b"mb5-runner: bundled TrueType font initialization failed; using bitmap fallback\n");
    }
    let mut app = App::new(proj);
    let pixels = addr_of_mut!(PIXELS) as *mut u8;

    if ppm {
        // 出力は論理サイズそのまま（1 倍）
        let mut canvas = unsafe { Canvas::new(pixels, 1, W, H) };
        app.render(&mut canvas);
        return if canvas.write_ppm() { 0 } else { 1 };
    }

    open_window(&mut app, pixels, speed_percent)
}

#[cfg(target_os = "macos")]
fn open_window(app: &mut App, pixels: *mut u8, speed_percent: Option<u32>) -> i32 {
    mac::run(app, pixels, speed_percent)
}

#[cfg(not(target_os = "macos"))]
fn open_window(_app: &mut App, _pixels: *mut u8, _speed_percent: Option<u32>) -> i32 {
    sys::eprint("ウィンドウ表示は macOS のみ対応です。--ppm で画像を出力できます\n".as_bytes());
    2
}
