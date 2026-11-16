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
    fn popen(command: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn pclose(stream: *mut c_void) -> c_int;
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


fn append_byte(out: &mut [u8], len: &mut usize, byte: u8) -> bool {
    if *len + 1 >= out.len() { return false; }
    out[*len] = byte;
    *len += 1;
    true
}

fn append_bytes(out: &mut [u8], len: &mut usize, bytes: &[u8]) -> bool {
    if (*len).saturating_add(bytes.len()) >= out.len() { return false; }
    out[*len..*len + bytes.len()].copy_from_slice(bytes);
    *len += bytes.len();
    true
}

/// Append one POSIX-shell single-quoted argument. Every dynamic value passed
/// to curl goes through this function; shell metacharacters remain data.
fn shell_quote(out: &mut [u8], len: &mut usize, value: &[u8]) -> bool {
    if !append_byte(out, len, b'\'') { return false; }
    for &byte in value {
        if byte == b'\'' {
            if !append_bytes(out, len, b"'\\''") { return false; }
        } else if byte == 0 || !append_byte(out, len, byte) {
            return false;
        }
    }
    append_byte(out, len, b'\'')
}

/// Fetch a UTF-8 text response through the platform curl executable.
/// The request is bounded by 5s connect / 15s total and a 16KiB response.
/// Returns (response length, HTTP status). HTTP error statuses are returned
/// normally so the script can inspect HTTP_STATUS and the response body.
pub fn http_fetch(method: &[u8], url: &[u8], body: &[u8], content_type: &[u8], headers: &[u8], out: &mut [u8]) -> Option<(usize, i32)> {
    if out.len() < 64 || method.is_empty() || url.is_empty() { return None; }
    if !url.starts_with(b"https://") && !url.starts_with(b"http://") { return None; }
    if method.iter().any(|b| !b.is_ascii_uppercase()) || url.iter().any(|&b| b == 0 || b == b'\n' || b == b'\r') { return None; }
    if body.len() > 4096 || content_type.len() > 255 || headers.len() > 2047 || body.iter().any(|&b| b == 0) { return None; }
    if content_type.iter().any(|&b| b == 0 || b == b'\r' || b == b'\n') { return None; }

    let mut command = [0u8; 8192];
    let mut len = 0usize;
    if !append_bytes(&mut command, &mut len, b"curl --silent --show-error --location --max-redirs 5 --proto '=http,https' --proto-redir '=http,https' --connect-timeout 5 --max-time 15 --max-filesize 16384 --request ")
        || !shell_quote(&mut command, &mut len, method)
        || !append_bytes(&mut command, &mut len, b" --header ")
    { return None; }
    let mut header = [0u8; 320];
    let mut hlen = 0usize;
    if !append_bytes(&mut header, &mut hlen, b"Content-Type: ") || !append_bytes(&mut header, &mut hlen, content_type) { return None; }
    if !shell_quote(&mut command, &mut len, &header[..hlen]) { return None; }
    // Optional headers are newline-separated. Names and values are validated
    // before each header is shell-quoted; no user text becomes shell syntax.
    for raw_header in headers.split(|&b| b == b'\n') {
        if raw_header.is_empty() { continue; }
        let colon = raw_header.iter().position(|&b| b == b':')?;
        let name = &raw_header[..colon];
        let value = &raw_header[colon + 1..];
        if name.is_empty() || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
            || value.iter().any(|b| *b < 0x20 && *b != b'\t' || *b == 0x7f) { return None; }
        let mut line = [0u8; 768];
        let mut line_len = 0usize;
        if !append_bytes(&mut line, &mut line_len, name) || !append_bytes(&mut line, &mut line_len, b":")
            || !append_bytes(&mut line, &mut line_len, value) || !append_bytes(&mut command, &mut len, b" --header ")
            || !shell_quote(&mut command, &mut len, &line[..line_len]) { return None; }
    }
    if !body.is_empty() {
        if !append_bytes(&mut command, &mut len, b" --data-binary ") || !shell_quote(&mut command, &mut len, body) { return None; }
    }
    if !append_bytes(&mut command, &mut len, b" --write-out ") || !shell_quote(&mut command, &mut len, b"\\n__MB5_HTTP_STATUS__%{http_code}")
        || !append_byte(&mut command, &mut len, b' ') || !shell_quote(&mut command, &mut len, url) || !append_bytes(&mut command, &mut len, b" 2>/dev/null")
    { return None; }
    if len + 1 >= command.len() { return None; }
    command[len] = 0;

    let stream = unsafe { popen(command.as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char) };
    if stream.is_null() { return None; }
    let mut used = 0usize;
    let mut overflow = false;
    loop {
        if used + 1 >= out.len() { overflow = true; break; }
        let n = unsafe { fread(out[used..].as_mut_ptr() as *mut c_void, 1, out.len() - used - 1, stream) };
        if n == 0 { break; }
        used += n;
    }
    let exit_status = unsafe { pclose(stream) };
    if overflow || exit_status != 0 { return None; }
    out[used] = 0;

    const MARKER: &[u8] = b"\n__MB5_HTTP_STATUS__";
    let marker_at = out[..used].windows(MARKER.len()).rposition(|w| w == MARKER)?;
    let status_bytes = &out[marker_at + MARKER.len()..used];
    if status_bytes.len() < 3 { return None; }
    let mut status = 0i32;
    for &digit in status_bytes.iter().take(3) {
        if !digit.is_ascii_digit() { return None; }
        status = status * 10 + (digit - b'0') as i32;
    }
    let body_len = marker_at;
    if out[..body_len].contains(&0) { return None; }
    out[body_len] = 0;
    Some((body_len, status))
}
