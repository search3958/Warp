//! Picol-shaped, fixed-memory host bridge for the MB5 script VM.
//! The VM implementation is bundled C; rendering and project state remain Rust.
use core::ffi::{c_char, c_void};
use crate::{project::Project, sys};

static mut ACTIVE_PROJECT: *mut Project = core::ptr::null_mut();

const MAX_SCHEDULED_CALLS: usize = 16;

#[derive(Clone, Copy)]
pub struct ScheduledCall {
    pub name: [u8; 64],
    pub name_len: usize,
    pub argument: [u8; 256],
    pub argument_len: usize,
}

#[derive(Clone, Copy)]
struct ScheduledSlot {
    active: bool,
    due_ms: u64,
    name: [u8; 64],
    name_len: usize,
    argument: [u8; 256],
    argument_len: usize,
}

const EMPTY_SCHEDULED_SLOT: ScheduledSlot = ScheduledSlot {
    active: false, due_ms: 0, name: [0; 64], name_len: 0,
    argument: [0; 256], argument_len: 0,
};

static mut SCHEDULED_CALLS: [ScheduledSlot; MAX_SCHEDULED_CALLS] = [EMPTY_SCHEDULED_SLOT; MAX_SCHEDULED_CALLS];
static mut NEXT_SCHEDULE_ID: i32 = 1;

/// Schedule a script procedure without blocking the UI event loop.
/// Called by the embedded VM's `after` command.
#[no_mangle]
pub extern "C" fn mb5_script_schedule(name: *const c_char, argument: *const c_char, delay_ms: u64) -> i32 {
    if name.is_null() || argument.is_null() || delay_ms > 3_600_000 { return 0; }
    let mut name_buf = [0u8; 64];
    let mut argument_buf = [0u8; 256];
    if !copy_c_string(&mut name_buf, name) || !copy_c_string(&mut argument_buf, argument) { return 0; }
    let name_len = nul_len(&name_buf);
    let argument_len = nul_len(&argument_buf);
    if name_len == 0 { return 0; }
    unsafe {
        let mut slot_index = None;
        for index in 0..MAX_SCHEDULED_CALLS {
            if !SCHEDULED_CALLS[index].active { slot_index = Some(index); break; }
        }
        let slot_index = match slot_index { Some(index) => index, None => return 0 };
        let id = NEXT_SCHEDULE_ID.max(1);
        NEXT_SCHEDULE_ID = if id == i32::MAX { 1 } else { id + 1 };
        SCHEDULED_CALLS[slot_index] = ScheduledSlot {
            active: true,
            due_ms: sys::monotonic_ms().saturating_add(delay_ms),
            name: name_buf,
            name_len,
            argument: argument_buf,
            argument_len,
        };
        id
    }
}

/// Remove and return the next due call. The runner is single-threaded, so the
/// fixed queue needs no allocator or synchronization primitive.
pub fn take_due_scheduled_call(now_ms: u64) -> Option<ScheduledCall> {
    unsafe {
        let mut selected: Option<usize> = None;
        for index in 0..MAX_SCHEDULED_CALLS {
            let slot = SCHEDULED_CALLS[index];
            if slot.active && slot.due_ms <= now_ms {
                if selected.map_or(true, |current| slot.due_ms < SCHEDULED_CALLS[current].due_ms) {
                    selected = Some(index);
                }
            }
        }
        let index = selected?;
        let slot = SCHEDULED_CALLS[index];
        SCHEDULED_CALLS[index] = EMPTY_SCHEDULED_SLOT;
        Some(ScheduledCall { name: slot.name, name_len: slot.name_len, argument: slot.argument, argument_len: slot.argument_len })
    }
}

fn clear_scheduled_calls() {
    unsafe {
        SCHEDULED_CALLS = [EMPTY_SCHEDULED_SLOT; MAX_SCHEDULED_CALLS];
        NEXT_SCHEDULE_ID = 1;
    }
}

fn copy_c_string(dst: &mut [u8], src: *const c_char) -> bool {
    if src.is_null() || dst.is_empty() { return false; }
    let mut i = 0usize;
    unsafe {
        while i + 1 < dst.len() && *src.add(i) != 0 { dst[i] = *src.add(i) as u8; i += 1; }
        dst[i] = 0;
        *src.add(i) == 0
    }
}

fn nul_len(src: &[u8]) -> usize { src.iter().position(|&b| b == 0).unwrap_or(src.len()) }

#[no_mangle]
pub extern "C" fn mb5_script_fetch(
    method: *const c_char, url: *const c_char, body: *const c_char,
    content_type: *const c_char, headers: *const c_char,
    out: *mut c_char, out_cap: usize, http_status: *mut i32,
) -> i32 {
    if out.is_null() || out_cap < 2 || http_status.is_null() { return -1; }
    let mut method_buf = [0u8; 16];
    let mut url_buf = [0u8; 2048];
    let mut body_buf = [0u8; 4096];
    let mut type_buf = [0u8; 256];
    let mut headers_buf = [0u8; 2048];
    if !copy_c_string(&mut method_buf, method) || !copy_c_string(&mut url_buf, url)
        || !copy_c_string(&mut body_buf, body) || !copy_c_string(&mut type_buf, content_type)
        || !copy_c_string(&mut headers_buf, headers) { return -1; }
    let method_len = nul_len(&method_buf);
    let url_len = nul_len(&url_buf);
    let body_len = nul_len(&body_buf);
    let type_len = nul_len(&type_buf);
    let headers_len = nul_len(&headers_buf);
    if ![b"GET".as_slice(), b"POST", b"PUT", b"PATCH", b"DELETE"].iter().any(|m| *m == &method_buf[..method_len]) || url_len == 0 { return -1; }
    let mut response = [0u8; 16384];
    match sys::http_fetch(&method_buf[..method_len], &url_buf[..url_len], &body_buf[..body_len], &type_buf[..type_len], &headers_buf[..headers_len], &mut response) {
        Some((len, status)) if len + 1 <= out_cap => unsafe {
            core::ptr::copy_nonoverlapping(response.as_ptr(), out as *mut u8, len);
            *out.add(len) = 0;
            *http_status = status;
            0
        },
        _ => -1,
    }
}

fn parse_color(value: &[u8]) -> Option<[u8; 3]> {
    let s = if value.first() == Some(&b'#') { &value[1..] } else { value };
    if s.len() != 6 { return None; }
    let mut out = [0u8; 3];
    for i in 0..3 {
        let hex = |c: u8| -> Option<u8> { match c { b'0'..=b'9' => Some(c-b'0'), b'a'..=b'f' => Some(c-b'a'+10), b'A'..=b'F' => Some(c-b'A'+10), _ => None } };
        out[i] = hex(s[i*2])?.saturating_mul(16).saturating_add(hex(s[i*2+1])?);
    }
    Some(out)
}

#[no_mangle]
pub extern "C" fn mb5_script_ui_set(id: *const c_char, property: *const c_char, value: *const c_char) -> i32 {
    let mut id_buf = [0u8; 64]; let mut prop_buf = [0u8; 32]; let mut value_buf = [0u8; 256];
    if !copy_c_string(&mut id_buf, id) || !copy_c_string(&mut prop_buf, property) || !copy_c_string(&mut value_buf, value) { return 0; }
    let id = &id_buf[..nul_len(&id_buf)]; let prop = &prop_buf[..nul_len(&prop_buf)]; let value = &value_buf[..nul_len(&value_buf)];
    if id.is_empty() { return 0; }
    unsafe {
        if ACTIVE_PROJECT.is_null() { return 0; }
        let project = &mut *ACTIVE_PROJECT;
        for comp in project.comps.iter_mut().take(project.ncomps) {
            if comp.id.get() != id { continue; }
            match prop {
                b"text" => {
                    if comp.kind.get() == b"text" { comp.label2.set(value); }
                    else { comp.label1.set(value); }
                },
                b"title" => comp.label1.set(value),
                b"message" | b"description" => comp.label2.set(value),
                b"value" => {
                    if comp.kind.get() == b"checkbox" || comp.kind.get() == b"switch" {
                        let checked = match parse_bool(value) { Some(v) => v, None => return 0 };
                        comp.checked = checked;
                    }
                    comp.value.set(value);
                },
                b"display" => {
                    if value == b"hide" || value == b"false" || value == b"none" { comp.hidden = true; }
                    else if value == b"show" || value == b"true" { comp.hidden = false; }
                    else { return 0; }
                },
                b"checked" => {
                    let checked = match parse_bool(value) { Some(v) => v, None => return 0 };
                    comp.checked = checked; comp.value.set(if checked { b"true" } else { b"false" });
                },
                b"opacity" => {
                    let percent = match parse_u32(value) { Some(v) if v <= 100 => v, _ => return 0 };
                    let n = (percent * 255 / 100) as u8;
                    if comp.transition_ms > 0 {
                        comp.opacity_from = comp.opacity;
                        comp.opacity_target = n;
                        comp.opacity_transition = true;
                        comp.transition_started_ms = sys::monotonic_ms();
                    } else { comp.opacity = n; comp.opacity_from = n; comp.opacity_target = n; }
                },
                b"bgColor" => if let Some(rgb) = parse_color(value) {
                    if comp.transition_ms > 0 {
                        comp.bg_from_color = comp.bg_color;
                        comp.bg_target_color = rgb;
                        comp.bg_transition = true;
                        comp.transition_started_ms = sys::monotonic_ms();
                    } else { comp.bg_color = rgb; comp.bg_from_color = rgb; comp.bg_target_color = rgb; }
                    comp.has_bg_color = true;
                } else { return 0; },
                b"fgColor" => if let Some(rgb) = parse_color(value) {
                    if comp.transition_ms > 0 {
                        comp.fg_from_color = comp.fg_color;
                        comp.fg_target_color = rgb;
                        comp.fg_transition = true;
                        comp.transition_started_ms = sys::monotonic_ms();
                    } else { comp.fg_color = rgb; comp.fg_from_color = rgb; comp.fg_target_color = rgb; }
                    comp.has_fg_color = true;
                } else { return 0; },
                b"gradientEnd" => if let Some(rgb) = parse_color(value) {
                    if comp.transition_ms > 0 {
                        comp.gradient_end_from = comp.gradient_end;
                        comp.gradient_end_target = rgb;
                        comp.gradient_end_transition = true;
                        comp.transition_started_ms = sys::monotonic_ms();
                    } else { comp.gradient_end = rgb; comp.gradient_end_from = rgb; comp.gradient_end_target = rgb; }
                    comp.has_gradient = true;
                } else { return 0; },
                _ => return 0,
            }
            return 1;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn mb5_script_ui_get(id: *const c_char, property: *const c_char, out: *mut c_char, out_cap: usize) -> i32 {
    if out.is_null() || out_cap == 0 { return 0; }
    let mut id_buf = [0u8; 64]; let mut prop_buf = [0u8; 32];
    if !copy_c_string(&mut id_buf, id) || !copy_c_string(&mut prop_buf, property) { return 0; }
    let id = &id_buf[..nul_len(&id_buf)]; let prop = &prop_buf[..nul_len(&prop_buf)];
    unsafe {
        if ACTIVE_PROJECT.is_null() { return 0; }
        let project = &*ACTIVE_PROJECT;
        for comp in project.comps.iter().take(project.ncomps) {
            if comp.id.get() != id { continue; }
            let value: &[u8] = match prop {
                b"text" => if comp.kind.get() == b"text" { comp.label2.get() } else { comp.label1.get() },
                b"title" => comp.label1.get(),
                b"message" | b"description" => comp.label2.get(),
                b"value" => comp.value.get(),
                b"display" => if comp.hidden { b"hide" } else { b"show" },
                b"checked" => if comp.checked { b"true" } else { b"false" },
                b"opacity" => {
                    let mut value = ((comp.opacity as u32 * 100 + 127) / 255) as u32;
                    let mut digits = [0u8; 3];
                    let mut count = 0usize;
                    loop {
                        digits[count] = b'0' + (value % 10) as u8;
                        count += 1;
                        value /= 10;
                        if value == 0 { break; }
                    }
                    if count + 1 > out_cap { return 0; }
                    for i in 0..count { *out.add(i) = digits[count - 1 - i] as c_char; }
                    *out.add(count) = 0;
                    return 1;
                },
                b"bgColor" => {
                    let mut tmp = [0u8; 8];
                    const HEX: &[u8; 16] = b"0123456789ABCDEF";
                    tmp[0] = b'#';
                    for i in 0..3 { tmp[1+i*2] = HEX[(comp.bg_color[i] >> 4) as usize]; tmp[2+i*2] = HEX[(comp.bg_color[i] & 15) as usize]; }
                    if out_cap < 8 { return 0; }
                    core::ptr::copy_nonoverlapping(tmp.as_ptr(), out as *mut u8, 7); *out.add(7) = 0; return 1;
                },
                b"fgColor" => {
                    let mut tmp = [0u8; 8]; const HEX: &[u8; 16] = b"0123456789ABCDEF"; tmp[0] = b'#';
                    for i in 0..3 { tmp[1+i*2] = HEX[(comp.fg_color[i] >> 4) as usize]; tmp[2+i*2] = HEX[(comp.fg_color[i] & 15) as usize]; }
                    if out_cap < 8 { return 0; }
                    core::ptr::copy_nonoverlapping(tmp.as_ptr(), out as *mut u8, 7); *out.add(7) = 0; return 1;
                },
                _ => return 0,
            };
            if value.len() + 1 > out_cap { return 0; }
            core::ptr::copy_nonoverlapping(value.as_ptr(), out as *mut u8, value.len());
            *out.add(value.len()) = 0;
            return 1;
        }
    }
    0
}

fn parse_bool(value: &[u8]) -> Option<bool> {
    match value {
        b"true" | b"1" | b"on" => Some(true),
        b"false" | b"0" | b"off" => Some(false),
        _ => None,
    }
}

fn parse_u32(value: &[u8]) -> Option<u32> {
    if value.is_empty() || value.iter().any(|byte| !byte.is_ascii_digit()) { return None; }
    let mut number = 0u32;
    for &byte in value {
        number = number.checked_mul(10)?.checked_add((byte - b'0') as u32)?;
    }
    Some(number)
}

#[no_mangle]
pub extern "C" fn mb5_script_log(message: *const c_char) {
    if message.is_null() { return; }
    let mut buf = [0u8; 512];
    if copy_c_string(&mut buf, message) { let n = nul_len(&buf); sys::eprint(b"[MB5] "); sys::eprint(&buf[..n]); sys::eprint(b"\n"); }
    else { sys::eprint(b"[MB5] log message exceeds 511 bytes\n"); }
}

#[link(name = "mb5vm", kind = "static")]
extern "C" {
    fn mb5_vm_new() -> *mut c_void;
    fn mb5_vm_free(vm: *mut c_void);
    fn mb5_vm_eval(vm: *mut c_void, source: *const c_char) -> i32;
    fn mb5_vm_call(vm: *mut c_void, name: *const c_char, argument: *const c_char) -> i32;
    fn mb5_vm_has_proc(vm: *mut c_void, name: *const c_char) -> i32;
    fn mb5_vm_result(vm: *mut c_void) -> *const c_char;
}

pub struct ScriptVm { raw: *mut c_void, enabled: bool }

impl ScriptVm {
    pub fn new(source: &[u8], external: bool) -> Self {
        clear_scheduled_calls();
        if source.iter().all(|byte| byte.is_ascii_whitespace()) {
            return Self { raw: core::ptr::null_mut(), enabled: false };
        }
        // Legacy inline @mainScript blocks keep using the original command
        // parser unless they explicitly opt into proc/fn syntax. A separate
        // scripts/main.mb5 file always uses the Picol-shaped MB5 VM, including
        // top-level initialization commands that precede procedure definitions.
        if !external {
            let first = source.split(|&b| b == b'\n').map(crate::project::trim)
                .map(|s| if s.starts_with(b"#") { &b""[..] } else { s }).find(|s| !s.is_empty()).unwrap_or(&[]);
            if !first.starts_with(b"proc ") && !first.starts_with(b"fn ") {
                return Self { raw: core::ptr::null_mut(), enabled: false };
            }
        }
        let raw = unsafe { mb5_vm_new() };
        if raw.is_null() { sys::eprint(b"MB5 script VM: allocation failed\n"); return Self { raw, enabled: false }; }
        let mut code = [0u8; 32769];
        let len = core::cmp::min(source.len(), code.len() - 1);
        code[..len].copy_from_slice(&source[..len]); code[len] = 0;
        let status = unsafe { mb5_vm_eval(raw, code.as_ptr() as *const c_char) };
        if status != 0 { Self::log_error(raw); }
        Self { raw, enabled: status == 0 }
    }

    fn log_error(raw: *mut c_void) {
        let result = unsafe { mb5_vm_result(raw) };
        if result.is_null() { sys::eprint(b"MB5 script VM: unknown error\n"); return; }
        let mut message = [0u8; 512];
        if copy_c_string(&mut message, result) { let n = nul_len(&message); sys::eprint(b"MB5 script VM: "); sys::eprint(&message[..n]); sys::eprint(b"\n"); }
    }

    pub fn enabled(&self) -> bool { self.enabled && !self.raw.is_null() }
    pub fn has_proc(&self, name: &[u8]) -> bool {
        if !self.enabled() || name.len() >= 64 || name.contains(&0) { return false; }
        let mut buf = [0u8; 64]; buf[..name.len()].copy_from_slice(name);
        unsafe { mb5_vm_has_proc(self.raw, buf.as_ptr() as *const c_char) != 0 }
    }

    pub fn call(&mut self, project: &mut Project, name: &[u8], argument: &[u8]) -> bool {
        if !self.enabled() || name.len() >= 64 || argument.len() >= 512 || name.contains(&0) || argument.contains(&0) { return false; }
        let mut name_buf = [0u8; 64]; let mut arg_buf = [0u8; 512];
        name_buf[..name.len()].copy_from_slice(name); arg_buf[..argument.len()].copy_from_slice(argument);
        unsafe { ACTIVE_PROJECT = project as *mut Project; }
        let status = unsafe { mb5_vm_call(self.raw, name_buf.as_ptr() as *const c_char, arg_buf.as_ptr() as *const c_char) };
        unsafe { ACTIVE_PROJECT = core::ptr::null_mut(); }
        if status != 0 { Self::log_error(self.raw); false } else { true }
    }
}

impl Drop for ScriptVm {
    fn drop(&mut self) { if !self.raw.is_null() { unsafe { mb5_vm_free(self.raw); } self.raw = core::ptr::null_mut(); } }
}
