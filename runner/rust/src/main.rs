//! Mosaic Board 5 ランナー（no_std / GUIエンジンなし / GPUなし）
//!
//! 使い方:
//!   mb5-runner [components_dir] [--ppm]
//!
//!   components_dir : components/ のパス（省略時は ./components）
//!   --ppm          : ウィンドウを開かず、描画結果を PPM (P6) として標準出力に書く
//!
//! 画素は canvas.rs のバッファに書き込む。macOS ではそのバッファを
//! CoreGraphics のビットマップとして AppKit のウィンドウに表示する。

#![no_std]
#![no_main]

mod app;
mod canvas;
mod font;
mod project;
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

#[panic_handler]
fn on_panic(_info: &PanicInfo) -> ! {
    sys::eprint(b"mb5-runner: internal error\n");
    sys::die(101)
}

fn usage() {
    sys::eprint(b"usage: mb5-runner [components_dir] [--ppm]\n");
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

    let mut i = 1usize;
    while i < argc as usize {
        let a = unsafe { sys::cstr_arg(argv, i) };
        if a == b"--ppm" {
            ppm = true;
        } else if a == b"-h" || a == b"--help" {
            help = true;
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
    let mut proj = Project::new();

    if let Some(n) = load(dir, b"config.ini", buf) {
        project::parse_config(&mut proj, &buf[..n]);
    }

    let n = match load(dir, b"app/main.mb5", buf) {
        Some(n) => n,
        None => {
            sys::eprint("app/main.mb5 が読めません（components_dir を確認してください）\n".as_bytes());
            return 1;
        }
    };
    project::parse_mb5(&mut proj, &buf[..n]);

    if let Some(n) = load(dir, b"models/main.yaml", buf) {
        project::resolve_labels(&mut proj, &buf[..n]);
    }

    let mut app = App::new(proj);
    let pixels = unsafe { addr_of_mut!(PIXELS) as *mut u8 };

    if ppm {
        // 出力は論理サイズそのまま（1 倍）
        let mut canvas = unsafe { Canvas::new(pixels, 1, W, H) };
        app.render(&mut canvas);
        return if canvas.write_ppm() { 0 } else { 1 };
    }

    open_window(&mut app, pixels)
}

#[cfg(target_os = "macos")]
fn open_window(app: &mut App, pixels: *mut u8) -> i32 {
    mac::run(app, pixels)
}

#[cfg(not(target_os = "macos"))]
fn open_window(_app: &mut App, _pixels: *mut u8) -> i32 {
    sys::eprint("ウィンドウ表示は macOS のみ対応です。--ppm で画像を出力できます\n".as_bytes());
    2
}
