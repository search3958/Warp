//! libc の最小限のラッパー。std は使わない。
//! macOS では libSystem、Linux では libc をリンクする。

use core::ffi::{c_char, c_int, c_void};

#[link(name = "c")]
extern "C" {
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn getenv(name: *const c_char) -> *mut c_char;
    #[allow(dead_code)]
    fn usleep(us: u32) -> c_int;
    fn exit(code: c_int) -> !;
}

const O_RDONLY: c_int = 0;
pub const STDOUT: c_int = 1;
pub const STDERR: c_int = 2;

pub fn write_all(fd: c_int, mut data: &[u8]) -> bool {
    while !data.is_empty() {
        let n = unsafe { write(fd, data.as_ptr() as *const c_void, data.len()) };
        if n <= 0 {
            return false;
        }
        data = &data[n as usize..];
    }
    true
}

pub fn write_num(fd: c_int, mut n: usize) -> bool {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    write_all(fd, &buf[i..])
}

pub fn eprint(s: &[u8]) {
    let _ = write_all(STDERR, s);
}


/// Return an environment variable as UTF-8 bytes without allocating.
/// The C runtime owns the returned memory for the lifetime of the process.
pub fn env_bytes(name: &[u8]) -> Option<&'static [u8]> {
    let mut key = [0u8; 96];
    if name.is_empty() || name.len() >= key.len() || name.iter().any(|&b| b == 0) {
        return None;
    }
    key[..name.len()].copy_from_slice(name);
    let ptr = unsafe { getenv(key.as_ptr() as *const c_char) };
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        Some(core::slice::from_raw_parts(ptr as *const u8, len))
    }
}

pub fn die(code: i32) -> ! {
    unsafe { exit(code) }
}

#[allow(dead_code)]
pub fn sleep_us(us: u32) {
    unsafe {
        usleep(us);
    }
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[link(name = "c")]
extern "C" {
    fn clock_gettime(clock_id: c_int, tp: *mut Timespec) -> c_int;
}

/// Monotonic milliseconds for script scheduling.
pub fn monotonic_ms() -> u64 {
    #[cfg(target_os = "macos")]
    const CLOCK_ID: c_int = 6; // CLOCK_MONOTONIC
    #[cfg(target_os = "linux")]
    const CLOCK_ID: c_int = 1; // CLOCK_MONOTONIC
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    const CLOCK_ID: c_int = 1;

    let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
    let result = unsafe { clock_gettime(CLOCK_ID, &mut ts) };
    if result != 0 || ts.tv_sec < 0 || ts.tv_nsec < 0 { return 0; }
    (ts.tv_sec as u64).saturating_mul(1000).saturating_add((ts.tv_nsec as u64) / 1_000_000)
}

/// argv[i] を &[u8] として取り出す（NUL終端まで）。
///
/// # Safety
/// argv は main に渡されたポインタで、i < argc であること。
pub unsafe fn cstr_arg<'a>(argv: *const *const c_char, i: usize) -> &'a [u8] {
    let p = *argv.add(i) as *const u8;
    let mut n = 0;
    while *p.add(n) != 0 {
        n += 1;
    }
    core::slice::from_raw_parts(p, n)
}

/// NUL終端付きのパスを固定長バッファで組み立てる。
pub struct CPath {
    buf: [u8; 512],
    len: usize,
}

impl CPath {
    pub const fn new() -> Self {
        CPath { buf: [0; 512], len: 0 }
    }

    pub fn push(&mut self, s: &[u8]) -> bool {
        if self.len + s.len() + 1 > self.buf.len() {
            return false;
        }
        self.buf[self.len..self.len + s.len()].copy_from_slice(s);
        self.len += s.len();
        true
    }

    pub fn as_cstr(&mut self) -> *const c_char {
        self.buf[self.len] = 0;
        self.buf.as_ptr() as *const c_char
    }
}

/// ファイル全体を buf に読み込み、読んだバイト数を返す。
/// 存在しない・読めない・buf に収まらない場合は None。
pub fn read_file(path: *const c_char, buf: &mut [u8]) -> Option<usize> {
    let fd = unsafe { open(path, O_RDONLY) };
    if fd < 0 {
        return None;
    }
    let mut n = 0usize;
    while n < buf.len() {
        let r = unsafe { read(fd, buf.as_mut_ptr().add(n) as *mut c_void, buf.len() - n) };
        if r <= 0 {
            break;
        }
        n += r as usize;
    }
    unsafe {
        close(fd);
    }
    if n == buf.len() {
        return None;
    }
    Some(n)
}
