//! MB5 preview UI and small script scheduler.
//! The tester intentionally has no in-content header or board tab bar.

use crate::canvas::{utf8_len, Canvas, Rgb, TextStyle};
use crate::project::{
    Comp, Project, ScriptCall, ScriptCommand, MAX_COMPS, MAX_ITEMS, NO_PARENT,
    CMD_ADD_COMPONENT, CMD_CALL, CMD_DELETE_COMPONENT, CMD_SET_DISPLAY, CMD_WAIT,
};
use crate::script::{take_due_scheduled_call, ScriptVm};

pub const W: i32 = 760;
pub const H: i32 = 640;

const PAD_X: i32 = 32;
const PAD_TOP: i32 = 24;
const GAP: i32 = 16;
const CARD_PAD: i32 = 20;
const BTN_H: i32 = 44;
const MAX_LINES: usize = 20;
const MAX_HITS: usize = MAX_COMPS * (MAX_ITEMS + 2);
const MAX_SCRIPT_DEPTH: usize = 8;

// Warp4 light palette: keep the neutral surfaces quiet and reserve blue for actions.
const BG: Rgb = [245, 245, 246];       // #f5f5f6
const WHITE: Rgb = [255, 255, 255];    // card and list surface
const INK: Rgb = [26, 26, 26];         // warp3_text
const MUTED: Rgb = [93, 93, 93];       // warp3_muted
const BORDER: Rgb = [211, 211, 211];   // warp3_border
const ACCENT: Rgb = [0, 106, 255];     // warp4_primary
const BUTTON_BG: Rgb = [238, 238, 239];

#[derive(Clone, Copy)]
enum Action {
    None,
    Goto(usize),
    Close(usize),
    SelectItem(usize, usize),
    ComponentEvent(usize),
    Toggle(usize),
    Input(usize),
    Slider(usize),
    ExpandDropdown(usize),
}

#[derive(Clone, Copy)]
struct Hit {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    action: Action,
}

#[derive(Clone, Copy)]
struct ScrollRect { x: i32, y: i32, w: i32, h: i32, valid: bool }
const EMPTY_SCROLL_RECT: ScrollRect = ScrollRect { x: 0, y: 0, w: 0, h: 0, valid: false };

const EMPTY_HIT: Hit = Hit { x: 0, y: 0, w: 0, h: 0, action: Action::None };

#[derive(Clone, Copy)]
struct Frame {
    script: usize,
    pc: usize,
}

const EMPTY_FRAME: Frame = Frame { script: 0, pc: 0 };

pub struct App {
    /// Project data is kept in static storage to avoid multi-megabyte stack copies.
    pub proj: &'static mut Project,
    board: usize,
    scroll: i32,
    max_scroll: i32,
    scroll_acc: f64,
    pointer_x: i32,
    pointer_y: i32,
    pointer_down: bool,
    active_slider: Option<Hit>,
    hits: [Hit; MAX_HITS],
    nhits: usize,
    scrollbox_rects: [ScrollRect; MAX_COMPS],
    scrollbox_content_heights: [i32; MAX_COMPS],
    frames: [Frame; MAX_SCRIPT_DEPTH],
    nframes: usize,
    wait_until_ms: u64,
    script_vm: ScriptVm,
    focused_input: Option<usize>,
}

fn line_h(st: TextStyle) -> i32 {
    match st {
        TextStyle::Title => 28,
        TextStyle::SectionTitle => 30,
        TextStyle::SectionDescription => 18,
        TextStyle::Heading => 20,
        TextStyle::Body => 22,
        TextStyle::ListItem => 20,
        TextStyle::Small => 18,
        TextStyle::Button => 24,
    }
}

fn wrap(c: &Canvas, s: &[u8], st: TextStyle, maxw: i32, out: &mut [(usize, usize); MAX_LINES]) -> usize {
    if s.is_empty() || maxw <= 0 { return 0; }
    let mut n = 0;
    let mut start = 0;
    let mut i = 0;
    while i < s.len() {
        let l = utf8_len(s[i]).min(s.len() - i);
        let next = i + l;
        if i > start && n < MAX_LINES - 1 && c.measure(&s[start..next], st) > maxw {
            out[n] = (start, i);
            n += 1;
            start = i;
        }
        i = next;
    }
    if start < s.len() && n < MAX_LINES {
        out[n] = (start, s.len());
        n += 1;
    }
    n
}

fn text_block_height(c: &Canvas, maxw: i32, title: &[u8], body: &[u8]) -> i32 {
    let mut title_lines = [(0usize, 0usize); MAX_LINES];
    let mut body_lines = [(0usize, 0usize); MAX_LINES];
    let nt = wrap(c, title, TextStyle::SectionTitle, maxw, &mut title_lines);
    let nb = wrap(c, body, TextStyle::SectionDescription, maxw, &mut body_lines);
    let gap = if nt > 0 && nb > 0 { 7 } else { 0 };
    nt as i32 * line_h(TextStyle::SectionTitle) + gap + nb as i32 * line_h(TextStyle::SectionDescription)
}

fn text_block(c: &mut Canvas, draw: bool, x: i32, y: i32, maxw: i32, title: &[u8], body: &[u8], foreground: Option<Rgb>) -> i32 {
    let mut title_lines = [(0usize, 0usize); MAX_LINES];
    let mut body_lines = [(0usize, 0usize); MAX_LINES];
    let nt = wrap(c, title, TextStyle::SectionTitle, maxw, &mut title_lines);
    let nb = wrap(c, body, TextStyle::SectionDescription, maxw, &mut body_lines);
    let gap = if nt > 0 && nb > 0 { 7 } else { 0 };
    let height = nt as i32 * line_h(TextStyle::SectionTitle) + gap + nb as i32 * line_h(TextStyle::SectionDescription);
    if draw {
        let mut cy = y;
        for (a, b) in title_lines.iter().take(nt) {
            c.text(x, cy, &title[*a..*b], TextStyle::SectionTitle, foreground.unwrap_or(INK));
            cy += line_h(TextStyle::SectionTitle);
        }
        cy += gap;
        for (a, b) in body_lines.iter().take(nb) {
            c.text(x, cy, &body[*a..*b], TextStyle::SectionDescription, foreground.unwrap_or(MUTED));
            cy += line_h(TextStyle::SectionDescription);
        }
    }
    height
}

fn draw_button(
    c: &mut Canvas,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &[u8],
    primary: bool,
    text_button: bool,
    custom_bg: Option<Rgb>,
    custom_fg: Option<Rgb>,
    accent: Rgb,
    radius: i32,
    hovered: bool,
    pressed: bool,
) {
    let transparent_text = text_button && custom_bg.is_none() && !primary;
    let base_fill = custom_bg.unwrap_or(if primary { accent } else { BUTTON_BG });
    let luminance = (base_fill[0] as u32 * 299 + base_fill[1] as u32 * 587 + base_fill[2] as u32 * 114) / 1000;
    let contrast = if luminance >= 150 { INK } else { WHITE };
    let fg = custom_fg.unwrap_or(if primary || custom_bg.is_some() { contrast } else { accent });
    let overlay = if pressed { 34u16 } else if hovered { 18u16 } else { 0u16 };
    let fill = if overlay == 0 { base_fill } else {
        [
            ((base_fill[0] as u16 * (255 - overlay)) / 255) as u8,
            ((base_fill[1] as u16 * (255 - overlay)) / 255) as u8,
            ((base_fill[2] as u16 * (255 - overlay)) / 255) as u8,
        ]
    };
    if !transparent_text || hovered || pressed {
        let state_fill = if transparent_text {
            if pressed { [210, 226, 255] } else { [232, 240, 255] }
        } else { fill };
        c.rrect(x, y, w, h, radius.min(h / 2), state_fill);
    }
    let tw = c.measure(label, TextStyle::Button);
    let previous_clip = c.clip_rect();
    c.set_clip(x + 8, y + 2, (w - 16).max(1), h - 4);
    c.text(x + (w - tw) / 2, y + (h - line_h(TextStyle::Button)) / 2, label, TextStyle::Button, fg);
    c.restore_clip(previous_clip);
}

fn numeric_value(value: &[u8], fallback: i32) -> i32 {
    if value.is_empty() { return fallback; }
    let mut sign = 1i32; let mut i = 0usize;
    if value.first() == Some(&b'-') { sign = -1; i = 1; }
    let mut n = 0i32; let mut found = false;
    while i < value.len() && value[i].is_ascii_digit() { found = true; n = n.saturating_mul(10).saturating_add((value[i]-b'0') as i32); i += 1; }
    if found { n.saturating_mul(sign) } else { fallback }
}

fn button_height(comp: &Comp) -> i32 {
    match comp.size.get() {
        b"large" => 52,
        b"small" => 36,
        _ => BTN_H,
    }
}

fn button_width(comp: &Comp, available: i32, natural: i32) -> i32 {
    let value = comp.width.get();
    if value.is_empty() { return natural.max(104).min(available); }
    let mut n = 0i32;
    let mut i = 0usize;
    while i < value.len() && value[i].is_ascii_digit() {
        n = n.saturating_mul(10).saturating_add((value[i] - b'0') as i32);
        i += 1;
    }
    if i == 0 { return natural.max(104).min(available); }
    let requested = if value.get(i) == Some(&b'%') { available.saturating_mul(n) / 100 } else { n };
    requested.max(40).min(available)
}

fn button_natural_width(c: &Canvas, comp: &Comp, available: i32) -> i32 {
    let label = if comp.label1.is_empty() { &b"Button"[..] } else { comp.label1.get() };
    let horizontal_padding = if comp.text_button { 28 } else { 48 };
    button_width(comp, available, c.measure(label, TextStyle::Button) + horizontal_padding)
}

fn write_number(value: i32, out: &mut [u8]) -> usize {
    if out.is_empty() { return 0; }
    let mut number = value as i64;
    let negative = number < 0;
    if negative { number = -number; }
    let mut temp = [0u8; 12];
    let mut n = 0usize;
    loop {
        temp[n] = b'0' + (number % 10) as u8;
        n += 1;
        number /= 10;
        if number == 0 { break; }
    }
    let mut len = 0usize;
    if negative && len < out.len() { out[len] = b'-'; len += 1; }
    while n > 0 && len < out.len() { n -= 1; out[len] = temp[n]; len += 1; }
    len
}

fn draw_symbol(c: &mut Canvas, name: &[u8], cx: i32, cy: i32, color: Rgb) {
    match name {
        b"search" => { c.circle(cx - 2, cy - 2, 7, color, false); c.line(cx + 3, cy + 3, cx + 9, cy + 9, color); }
        b"close" | b"x" => { c.line(cx - 7, cy - 7, cx + 7, cy + 7, color); c.line(cx + 7, cy - 7, cx - 7, cy + 7, color); }
        b"add" | b"plus" => { c.line(cx - 8, cy, cx + 8, cy, color); c.line(cx, cy - 8, cx, cy + 8, color); }
        b"remove" | b"minus" => c.line(cx - 8, cy, cx + 8, cy, color),
        b"check" => { c.line(cx - 8, cy, cx - 2, cy + 6, color); c.line(cx - 2, cy + 6, cx + 9, cy - 7, color); }
        b"arrow-left" | b"back" => { c.line(cx + 8, cy, cx - 8, cy, color); c.line(cx - 8, cy, cx - 1, cy - 7, color); c.line(cx - 8, cy, cx - 1, cy + 7, color); }
        b"arrow-right" | b"forward" => { c.line(cx - 8, cy, cx + 8, cy, color); c.line(cx + 8, cy, cx + 1, cy - 7, color); c.line(cx + 8, cy, cx + 1, cy + 7, color); }
        b"menu" => { for offset in [-6, 0, 6] { c.fill(cx - 9, cy + offset, 18, 2, color); } }
        b"home" => {
            c.line(cx - 10, cy - 1, cx, cy - 10, color); c.line(cx, cy - 10, cx + 10, cy - 1, color);
            c.line(cx - 7, cy - 3, cx - 7, cy + 9, color); c.line(cx + 7, cy - 3, cx + 7, cy + 9, color);
            c.line(cx - 7, cy + 9, cx + 7, cy + 9, color); c.line(cx - 2, cy + 9, cx - 2, cy + 2, color); c.line(cx + 2, cy + 9, cx + 2, cy + 2, color);
        }
        b"info" => { c.circle(cx, cy, 9, color, false); c.circle(cx, cy - 4, 1, color, true); c.line(cx, cy - 1, cx, cy + 6, color); }
        b"settings" | b"gear" => { c.circle(cx, cy, 9, color, false); c.circle(cx, cy, 3, color, false); for (dx,dy) in [(0,-12),(0,12),(-12,0),(12,0)] { c.fill(cx + dx - 1, cy + dy - 2, 3, 4, color); } }
        b"refresh" => { c.circle(cx, cy, 8, color, false); c.line(cx + 3, cy - 8, cx + 9, cy - 8, color); c.line(cx + 9, cy - 8, cx + 9, cy - 2, color); }
        _ => { let first = name.first().copied().unwrap_or(b'?'); c.circle(cx, cy, 10, color, false); c.text(cx - 4, cy - 8, &[first], TextStyle::Small, color); }
    }
}

impl App {
    pub fn new(proj: &'static mut Project) -> Self {
        let initial_board = (0..proj.nboards)
            .find(|&i| proj.boards[i].get() == proj.main_board.get())
            .unwrap_or(0);
        let startup = proj.main_script;
        let script_source = proj.script_source.get();
        let external_script = proj.script_source.external;
        let script_vm = ScriptVm::new(script_source, external_script);
        let mut app = Self {
            proj,
            board: initial_board,
            scroll: 0,
            max_scroll: 0,
            scroll_acc: 0.0,
            pointer_x: -1,
            pointer_y: -1,
            pointer_down: false,
            active_slider: None,
            hits: [EMPTY_HIT; MAX_HITS],
            nhits: 0,
            scrollbox_rects: [EMPTY_SCROLL_RECT; MAX_COMPS],
            scrollbox_content_heights: [0; MAX_COMPS],
            frames: [EMPTY_FRAME; MAX_SCRIPT_DEPTH],
            nframes: 0,
            wait_until_ms: 0,
            script_vm,
            focused_input: None,
        };
        if app.script_vm.enabled() {
            if app.script_vm.has_proc(b"main") {
                let _ = app.script_vm.call(&mut *app.proj, b"main", b"");
            }
        } else if !external_script && !startup.is_empty() {
            let _ = app.start_script(startup.get(), b"");
        }
        app
    }

    fn push_hit(&mut self, x: i32, y: i32, w: i32, h: i32, action: Action) {
        if self.nhits < MAX_HITS {
            self.hits[self.nhits] = Hit { x, y, w, h, action };
            self.nhits += 1;
        }
    }

    fn button_state(&self, x: i32, y: i32, w: i32, h: i32) -> (bool, bool) {
        let hovered = self.pointer_x >= x && self.pointer_x < x + w
            && self.pointer_y >= y && self.pointer_y < y + h;
        (hovered, hovered && self.pointer_down)
    }

    pub fn pointer_move(&mut self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() { return false; }
        let next_x = x as i32;
        let next_y = y as i32;
        let mut changed = self.pointer_x != next_x || self.pointer_y != next_y;
        self.pointer_x = next_x;
        self.pointer_y = next_y;
        if self.pointer_down {
            if let Some(hit) = self.active_slider {
                if let Action::Slider(index) = hit.action {
                    changed |= self.set_slider_from_pointer(index, hit.x, hit.w, next_x);
                }
            }
        }
        changed
    }

    pub fn pointer_press(&mut self, x: f64, y: f64) -> bool {
        let moved = self.pointer_move(x, y);
        let changed = !self.pointer_down;
        self.pointer_down = true;
        self.active_slider = None;
        let px = self.pointer_x;
        let py = self.pointer_y;
        self.active_slider = self.hits.iter().take(self.nhits).rev().find_map(|hit| {
            if px < hit.x || px >= hit.x + hit.w || py < hit.y || py >= hit.y + hit.h { return None; }
            match hit.action { Action::Slider(_) => Some(*hit), _ => None }
        });
        moved || changed
    }

    pub fn pointer_release(&mut self, x: f64, y: f64) -> bool {
        let moved = self.pointer_move(x, y);
        let changed = self.pointer_down;
        self.pointer_down = false;
        self.active_slider = None;
        moved || changed
    }

    fn set_slider_from_pointer(&mut self, idx: usize, hit_x: i32, hit_w: i32, pointer_x: i32) -> bool {
        if idx >= self.proj.ncomps || self.proj.comps[idx].kind.get() != b"slider" { return false; }
        let min = self.proj.comps[idx].min_value;
        let max = self.proj.comps[idx].max_value.max(min.saturating_add(1));
        let width = hit_w.max(1);
        let ratio = ((pointer_x - hit_x).clamp(0, width) as i64 * 1000 / width as i64) as i32;
        let value = min.saturating_add(((max as i64 - min as i64) * ratio as i64 / 1000) as i32);
        let mut buffer = [0u8; 16];
        let length = write_number(value, &mut buffer);
        if self.proj.comps[idx].value.get() == &buffer[..length] { return false; }
        self.proj.comps[idx].value.set(&buffer[..length]);
        let _ = self.trigger_component(idx);
        true
    }

    fn find_board(&self, name: &[u8]) -> Option<usize> {
        if name.is_empty() { return None; }
        (0..self.proj.nboards).find(|&i| self.proj.boards[i].get() == name)
    }

    fn find_script(&self, name: &[u8], arg: &[u8]) -> Option<usize> {
        (0..self.proj.nscripts).find(|&i| {
            self.proj.scripts[i].name.get() == name && self.proj.scripts[i].branch.get() == arg
        })
    }

    fn start_script(&mut self, name: &[u8], arg: &[u8]) -> bool {
        if self.script_vm.has_proc(name) {
            return self.script_vm.call(&mut *self.proj, name, arg);
        }
        let idx = match self.find_script(name, arg) { Some(v) => v, None => return false };
        if self.nframes >= MAX_SCRIPT_DEPTH { return false; }
        self.frames[self.nframes] = Frame { script: idx, pc: 0 };
        self.nframes += 1;
        true
    }

    fn run_callback(&mut self, call: ScriptCall) -> bool {
        !call.is_empty() && self.start_script(call.name.get(), call.arg.get())
    }

    fn tick_transitions(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        for comp in self.proj.comps.iter_mut().take(self.proj.ncomps) {
            let duration = comp.transition_ms as u64;
            if duration == 0 { continue; }
            if comp.opacity_transition || comp.bg_transition || comp.fg_transition || comp.gradient_end_transition {
                let elapsed = now_ms.saturating_sub(comp.transition_started_ms);
                if elapsed >= duration {
                    if comp.opacity_transition { comp.opacity = comp.opacity_target; comp.opacity_transition = false; changed = true; }
                    if comp.bg_transition { comp.bg_color = comp.bg_target_color; comp.bg_transition = false; changed = true; }
                    if comp.fg_transition { comp.fg_color = comp.fg_target_color; comp.fg_transition = false; changed = true; }
                    if comp.gradient_end_transition { comp.gradient_end = comp.gradient_end_target; comp.gradient_end_transition = false; changed = true; }
                } else {
                    let t = elapsed as u32 * 1000 / duration as u32;
                    if comp.opacity_transition {
                        let from = comp.opacity_from as i32; let to = comp.opacity_target as i32;
                        let next = (from + (to - from) * t as i32 / 1000).clamp(0, 255) as u8;
                        if next != comp.opacity { comp.opacity = next; changed = true; }
                    }
                    if comp.bg_transition {
                        for channel in 0..3 {
                            let from = comp.bg_from_color[channel] as i32; let to = comp.bg_target_color[channel] as i32;
                            comp.bg_color[channel] = (from + (to - from) * t as i32 / 1000).clamp(0, 255) as u8;
                        }
                        changed = true;
                    }
                    if comp.fg_transition {
                        for channel in 0..3 {
                            let from = comp.fg_from_color[channel] as i32; let to = comp.fg_target_color[channel] as i32;
                            comp.fg_color[channel] = (from + (to - from) * t as i32 / 1000).clamp(0, 255) as u8;
                        }
                        changed = true;
                    }
                    if comp.gradient_end_transition {
                        for channel in 0..3 {
                            let from = comp.gradient_end_from[channel] as i32; let to = comp.gradient_end_target[channel] as i32;
                            comp.gradient_end[channel] = (from + (to - from) * t as i32 / 1000).clamp(0, 255) as u8;
                        }
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    fn trigger_component(&mut self, idx: usize) -> bool {
        if idx >= self.proj.ncomps { return false; }
        let call = self.proj.comps[idx].on_event;
        self.run_callback(call)
    }

    fn set_component_display(&mut self, id: &[u8], visible: bool, target: &[u8]) -> bool {
        let target_board = if target.is_empty() { Some(self.board) } else { self.find_board(target) };
        let mut changed = false;
        for i in 0..self.proj.ncomps {
            let comp = &mut self.proj.comps[i];
            if !id.is_empty()
                && comp.id.get() == id
                && target_board.map_or(false, |board| comp.board == board)
            {
                let next_hidden = !visible;
                if comp.hidden != next_hidden {
                    comp.hidden = next_hidden;
                    changed = true;
                }
            }
        }
        changed
    }

    fn delete_named_components(&mut self, name: &[u8], target: &[u8]) -> bool {
        let target_board = if target.is_empty() { Some(self.board) } else { self.find_board(target) };
        let mut changed = false;
        for i in 0..self.proj.ncomps {
            let c = &mut self.proj.comps[i];
            if (c.struct_name.get() == name || c.name.get() == name)
                && target_board.map_or(false, |b| c.board == b)
                && !c.deleted
            {
                c.deleted = true;
                changed = true;
            }
        }
        changed
    }

    fn add_named_components(&mut self, name: &[u8], target: &[u8]) -> bool {
        let target_board = if target.is_empty() { Some(self.board) } else { self.find_board(target) };
        let board = match target_board { Some(v) => v, None => return false };
        let struct_index = (0..self.proj.nstructs).find(|&i| self.proj.structs[i].name.get() == name);
        let struct_index = match struct_index { Some(v) => v, None => return false };
        let definition = self.proj.structs[struct_index];
        let mut changed = false;
        for ci in 0..definition.ncomps {
            if self.proj.ncomps >= MAX_COMPS { break; }
            let mut instance = definition.comps[ci];
            instance.board = board;
            instance.parent = NO_PARENT;
            instance.struct_name.set(name);
            instance.deleted = false;
            let index = self.proj.ncomps;
            self.proj.comps[index] = instance;
            self.proj.ncomps += 1;
            changed = true;
        }
        changed
    }

    /// Run scheduled script commands until a wait or the execution budget is reached.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        self.tick_with_budget(now_ms, 64)
    }

    /// Run scheduled commands with a caller-controlled per-tick budget.
    pub fn tick_with_budget(&mut self, now_ms: u64, execution_budget: usize) -> bool {
        self.tick_with_profile(now_ms, execution_budget, 1000)
    }

    /// Apply a relative time scale in permille (1000 = normal speed).
    pub fn tick_with_profile(&mut self, now_ms: u64, execution_budget: usize, time_scale_permille: u32) -> bool {
        let mut changed = self.tick_transitions(now_ms);
        // Timed callbacks are dispatched from the event loop; `after` never sleeps.
        // Bound timer callbacks per frame. A callback may schedule another
        // zero-delay callback; the cap prevents an accidental infinite loop
        // from starving input and rendering. Remaining callbacks run next tick.
        for _ in 0..16 {
            let call = match take_due_scheduled_call(now_ms) {
                Some(call) => call,
                None => break,
            };
            let _ = self.script_vm.call(
                &mut *self.proj,
                &call.name[..call.name_len],
                &call.argument[..call.argument_len],
            );
            changed = true;
        }
        if self.wait_until_ms != 0 {
            if now_ms < self.wait_until_ms { return changed; }
            self.wait_until_ms = 0;
        }

        let mut budget = execution_budget.max(1);
        while budget > 0 && self.nframes > 0 {
            budget -= 1;
            let frame_index = self.nframes - 1;
            let script_index = self.frames[frame_index].script;
            if script_index >= self.proj.nscripts {
                self.nframes -= 1;
                continue;
            }
            let command_index = self.frames[frame_index].pc;
            if command_index >= self.proj.scripts[script_index].ncommands {
                self.nframes -= 1;
                continue;
            }
            let cmd: ScriptCommand = self.proj.scripts[script_index].commands[command_index];
            self.frames[frame_index].pc += 1;
            match cmd.kind {
                CMD_WAIT => {
                    let scale = time_scale_permille.max(1) as u64;
                    let scaled_wait = cmd.millis.saturating_mul(scale).saturating_add(999) / 1000;
                    self.wait_until_ms = now_ms.saturating_add(scaled_wait);
                    if cmd.millis > 0 { return changed; }
                }
                CMD_SET_DISPLAY => {
                    changed |= self.set_component_display(cmd.subject.get(), cmd.flag, cmd.target.get());
                }
                CMD_CALL => {
                    let _ = self.start_script(cmd.subject.get(), cmd.arg.get());
                }
                CMD_DELETE_COMPONENT => {
                    changed |= self.delete_named_components(cmd.subject.get(), cmd.target.get());
                }
                CMD_ADD_COMPONENT => {
                    changed |= self.add_named_components(cmd.subject.get(), cmd.target.get());
                }
                _ => {}
            }
        }
        changed
    }

    pub fn render(&mut self, c: &mut Canvas) {
        c.clear_clip();
        c.fill(0, 0, W, H, BG);
        self.nhits = 0;
        self.scrollbox_rects = [EMPTY_SCROLL_RECT; MAX_COMPS];
        self.scrollbox_content_heights = [0; MAX_COMPS];
        c.set_clip(0, 0, W, H);
        let end_y = self.draw_board(c);
        self.max_scroll = core::cmp::max(0, end_y + self.scroll + PAD_TOP - H);
        if self.scroll > self.max_scroll { self.scroll = self.max_scroll; }
        self.draw_scrollbar(c);
        self.draw_dialog(c);
        c.clear_clip();
    }

    fn draw_board(&mut self, c: &mut Canvas) -> i32 {
        if self.proj.nboards == 0 {
            c.text(PAD_X, PAD_TOP, "表示できる board がありません".as_bytes(), TextStyle::Body, MUTED);
            return PAD_TOP + 40;
        }
        let topbar = (0..self.proj.ncomps).find(|&i| {
            let comp = self.proj.comps[i];
            comp.board == self.board && comp.parent == NO_PARENT && comp.kind.get() == b"topbar" && !comp.hidden && !comp.deleted
        });
        let mut y = PAD_TOP;
        if let Some(idx) = topbar {
            let top_height = self.draw_component(c, idx, 0, 0, W).max(56);
            y += top_height + 8;
        }
        y -= self.scroll;
        for i in 0..self.proj.ncomps {
            let comp = self.proj.comps[i];
            if comp.board != self.board || comp.parent != NO_PARENT || comp.deleted || comp.hidden { continue; }
            if comp.kind.get() == b"alert" || comp.kind.get() == b"topbar" { continue; }
            let used = self.draw_component(c, i, PAD_X, y, W - PAD_X * 2);
            if used > 0 { y += used + GAP; }
        }
        y
    }

    fn component_height(&self, c: &Canvas, idx: usize, width: i32) -> i32 {
        if idx >= self.proj.ncomps { return 0; }
        let comp = self.proj.comps[idx];
        if comp.deleted || comp.hidden { return 0; }
        match comp.kind.get() {
            b"text" => text_block_height(c, width, comp.label1.get(), comp.label2.get()),
            b"list" => self.list_height(c, &comp, width),
            b"button" => button_height(&comp),
            b"input" => 48,
            b"checkbox" | b"switch" => 40,
            b"slider" => 48,
            b"progress" => 12,
            b"divider" => 1,
            b"spacing" => comp.spacing.max(0),
            b"navigation" | b"tab" => 44,
            b"dropdown" => if comp.expanded { 48 + comp.nitems.min(MAX_ITEMS) as i32 * 40 } else { 44 },
            b"snackbar" => 0,
            b"icon" => 36,
            b"card" => CARD_PAD * 2 + self.card_content_height(c, idx, width - CARD_PAD * 2),
            b"topbar" => comp.height.max(56).max(self.container_content_height(c, idx, width, true) + 16),
            b"scrollbox" => if comp.height > 0 { comp.height } else { self.vertical_children_height(c, idx, width) },
            b"horizontal" | b"grid" => self.container_content_height(c, idx, width, true),
            b"vertical" | b"gradient" | b"opacity" => self.container_content_height(c, idx, width, false),
            _ => 0,
        }
    }

    fn vertical_children_height(&self, c: &Canvas, idx: usize, width: i32) -> i32 {
        let spacing = self.proj.comps[idx].spacing.max(0);
        let mut total = 0i32;
        let mut visible = 0usize;
        for child in 0..self.proj.ncomps {
            let item = self.proj.comps[child];
            if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                let child_height = self.component_height(c, child, width);
                if child_height > 0 {
                    if visible > 0 { total = total.saturating_add(spacing); }
                    total = total.saturating_add(child_height);
                    visible += 1;
                }
            }
        }
        total
    }

    fn container_content_height(&self, c: &Canvas, idx: usize, width: i32, row_layout: bool) -> i32 {
        let comp = self.proj.comps[idx];
        let kind = comp.kind.get();
        let mut count = 0usize;
        for child in 0..self.proj.ncomps {
            let item = self.proj.comps[child];
            if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden { count += 1; }
        }
        if count == 0 { return comp.height.max(0); }
        let spacing = comp.spacing.max(0);
        if kind == b"horizontal" || kind == b"topbar" {
            let cell_w = ((width - spacing * (count.saturating_sub(1) as i32)) / count as i32).max(1);
            let mut max_h = 0;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    max_h = max_h.max(self.component_height(c, child, cell_w));
                }
            }
            max_h + if count > 1 { 0 } else { 0 }
        } else if kind == b"grid" {
            let cols = comp.columns.max(1) as usize;
            let cell_w = ((width - spacing * (cols.saturating_sub(1) as i32)) / cols as i32).max(1);
            let rows = (count + cols - 1) / cols;
            let mut row_heights = [0i32; MAX_COMPS];
            let mut n = 0usize;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let row = n / cols;
                    row_heights[row] = row_heights[row].max(self.component_height(c, child, cell_w));
                    n += 1;
                }
            }
            let mut total = 0;
            for row in 0..rows { total += row_heights[row]; if row + 1 < rows { total += spacing; } }
            total
        } else {
            let mut total = 0;
            let mut visible = 0usize;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let h = self.component_height(c, child, width);
                    if h > 0 { if visible > 0 { total += spacing; } total += h; visible += 1; }
                }
            }
            if !row_layout && comp.height > 0 { comp.height } else { total }
        }
    }

    fn card_content_height(&self, c: &Canvas, idx: usize, inner: i32) -> i32 {
        let mut total = 0;
        let mut row_width = 0;
        let mut row_height = 0;
        let mut has_button_row = false;
        for child in 0..self.proj.ncomps {
            let item = self.proj.comps[child];
            if item.parent != idx || item.board != self.board || item.deleted || item.hidden { continue; }
            if item.kind.get() == b"button" {
                let gap = if has_button_row { 10 } else { 0 };
                let mut available = (inner - row_width - gap).max(1);
                let mut width = button_natural_width(c, &item, available);
                if has_button_row && row_width + gap + width > inner {
                    total += row_height + 12;
                    row_width = 0;
                    row_height = 0;
                    has_button_row = false;
                    available = inner;
                    width = button_natural_width(c, &item, available);
                }
                if has_button_row { row_width += 10; }
                row_width += width;
                row_height = row_height.max(button_height(&item));
                has_button_row = true;
            } else {
                if has_button_row {
                    total += row_height + 12;
                    row_width = 0;
                    row_height = 0;
                    has_button_row = false;
                }
                let height = self.component_height(c, child, inner);
                if height > 0 { total += height + 12; }
            }
        }
        if has_button_row { total += row_height; }
        else if total > 0 { total -= 12; }
        total
    }

    fn draw_component(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        if idx >= self.proj.ncomps { return 0; }
        let comp = self.proj.comps[idx];
        if comp.deleted || comp.hidden { return 0; }
        let previous_alpha = c.alpha();
        c.set_alpha(((previous_alpha as u16 * comp.opacity as u16 + 127) / 255) as u8);
        let result = match comp.kind.get() {
            b"text" => text_block(c, true, x, y, width, comp.label1.get(), comp.label2.get(), if comp.has_fg_color { Some(comp.fg_color) } else { None }),
            b"list" => self.draw_list(c, idx, x, y, width),
            b"button" => {
                let label = if comp.label1.is_empty() { &b"Button"[..] } else { comp.label1.get() };
                let pad = if comp.text_button { 28 } else { 48 };
                let natural = c.measure(label, TextStyle::Button) + pad;
                let bw = button_width(&comp, width, natural);
                let bh = button_height(&comp);
                let bg = if comp.has_bg_color { Some(comp.bg_color) } else { None };
                let fg = if comp.has_fg_color { Some(comp.fg_color) } else { None };
                let radius = if comp.radius_custom { comp.radius } else { bh / 2 };
                let accent = self.proj.primary_color;
                let (hovered, pressed) = self.button_state(x, y, bw, bh);
                draw_button(c, x, y, bw, bh, label, comp.primary, comp.text_button, bg, fg, accent, radius, hovered, pressed);
                let action = match self.find_board(comp.event.get()) {
                    Some(board) => Action::Goto(board),
                    None if !comp.on_event.is_empty() => Action::ComponentEvent(idx),
                    None => Action::None,
                };
                self.push_hit(x, y, bw, bh, action);
                bh
            }
            b"card" => self.draw_card(c, idx, x, y, width),
            b"horizontal" | b"vertical" | b"grid" | b"scrollbox" | b"topbar" | b"gradient" | b"opacity" => self.draw_container(c, idx, x, y, width),
            b"icon" => {
                let name = if !comp.label1.is_empty() { comp.label1.get() } else if !comp.name.is_empty() { comp.name.get() } else { b"info" };
                let color = if comp.has_fg_color { comp.fg_color } else { self.proj.primary_color };
                draw_symbol(c, name, x + 18, y + 18, color);
                if !comp.on_event.is_empty() { self.push_hit(x, y, width.min(44), 36, Action::ComponentEvent(idx)); }
                36
            }
            b"input" => {
                let focused = self.focused_input == Some(idx);
                c.rrect(x, y, width, 48, 13, [250, 250, 251]);
                c.frame(x, y, width, 48, if focused { self.proj.primary_color } else { [220, 220, 224] });
                let value = if comp.value.is_empty() { comp.placeholder.get() } else { comp.value.get() };
                let color = if comp.value.is_empty() { MUTED } else { INK };
                c.text(x + 14, y + 14, value, TextStyle::Body, color);
                self.push_hit(x, y, width, 48, Action::Input(idx));
                48
            }
            b"checkbox" | b"switch" => {
                let label = if !comp.label1.is_empty() { comp.label1.get() } else { comp.placeholder.get() };
                if comp.kind.get() == b"switch" {
                    let track = if comp.checked { self.proj.primary_color } else { [205, 207, 211] };
                    c.rrect(x, y + 7, 48, 27, 14, track);
                    let knob_x = if comp.checked { x + 25 } else { x + 3 };
                    c.rrect(knob_x, y + 10, 21, 21, 11, WHITE);
                    c.text(x + 60, y + 10, label, TextStyle::Body, INK);
                } else {
                    c.rrect(x, y + 8, 24, 24, 7, if comp.checked { self.proj.primary_color } else { WHITE });
                    c.frame(x, y + 8, 24, 24, if comp.checked { self.proj.primary_color } else { [190, 192, 196] });
                    if comp.checked { c.text(x + 4, y + 8, "✓".as_bytes(), TextStyle::Body, WHITE); }
                    c.text(x + 36, y + 9, label, TextStyle::Body, INK);
                }
                self.push_hit(x, y, width, 40, Action::Toggle(idx));
                40
            }
            b"slider" => {
                let min = comp.min_value; let max = comp.max_value.max(min + 1);
                let val = numeric_value(comp.value.get(), min).clamp(min, max);
                let ratio = (val - min) as f32 / (max - min) as f32;
                let track_x = x + 8; let track_w = (width - 16).max(1);
                c.rrect(track_x, y + 21, track_w, 5, 3, [222, 224, 228]);
                let filled = (track_w as f32 * ratio) as i32;
                c.rrect(track_x, y + 21, filled.max(1), 5, 3, self.proj.primary_color);
                let knob_x = track_x + filled - 9;
                c.rrect(knob_x, y + 13, 18, 18, 9, WHITE);
                c.rrect(knob_x + 1, y + 14, 16, 16, 8, self.proj.primary_color);
                self.push_hit(x, y, width, 48, Action::Slider(idx));
                48
            }
            b"progress" => {
                let val = numeric_value(comp.value.get(), 0).clamp(0, 100);
                let bar_w = width.max(1);
                c.rrect(x, y + 2, bar_w, 8, 4, [226, 228, 232]);
                let filled = (bar_w * val / 100).max(if val > 0 { 8 } else { 0 });
                if filled > 0 {
                    if comp.has_gradient { c.gradient_rrect(x, y + 2, filled, 8, 4, comp.bg_color, comp.gradient_end); }
                    else { c.rrect(x, y + 2, filled, 8, 4, if comp.has_bg_color { comp.bg_color } else { self.proj.primary_color }); }
                }
                12
            }
            b"divider" => { c.fill(x, y, width, 1, [224, 225, 228]); 1 }
            b"spacing" => comp.spacing.max(0),
            b"navigation" | b"tab" | b"dropdown" => self.draw_choice_control(c, idx, x, y, width),
            b"snackbar" => {
                let label = if !comp.label1.is_empty() { comp.label1.get() } else { comp.label2.get() };
                let w = width.min(420); let sx = x + (width - w) / 2; let sy = H - 68;
                c.rrect(sx, sy, w, 48, 15, [43, 46, 54]);
                c.text(sx + 18, sy + 13, label, TextStyle::Body, WHITE);
                0
            }
            _ => 0,
        };
        c.set_alpha(previous_alpha);
        result
    }

    fn draw_container(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        if idx >= self.proj.ncomps { return 0; }
        // Copy the fixed-size kind field before recursive drawing. Keeping a slice
        // borrowed from self.proj here would block mutable access to self in
        // draw_component() while child components are rendered.
        let kind_text = self.proj.comps[idx].kind;
        let kind = kind_text.get();
        let inset = if kind == b"card" { CARD_PAD } else if kind == b"topbar" { 10 } else { 0 };
        let inner_x = x + inset;
        let inner_w = (width - inset * 2).max(1);
        let content_h = if kind == b"scrollbox" { self.vertical_children_height(c, idx, inner_w) }
            else { self.container_content_height(c, idx, width, kind == b"horizontal" || kind == b"topbar" || kind == b"grid") };
        let explicit_height = self.proj.comps[idx].height;
        let height = if kind == b"scrollbox" && explicit_height > 0 { explicit_height }
            else { self.component_height(c, idx, width).max(content_h).max(1) };
        let spacing = self.proj.comps[idx].spacing.max(0);

        if kind == b"scrollbox" {
            let max_offset = (content_h - height).max(0);
            self.proj.comps[idx].scroll_offset = self.proj.comps[idx].scroll_offset.clamp(0, max_offset);
            self.scrollbox_rects[idx] = ScrollRect { x: inner_x, y, w: inner_w, h: height, valid: true };
            self.scrollbox_content_heights[idx] = content_h;
        }

        let comp = self.proj.comps[idx];
        match kind {
            b"card" => {},
            b"topbar" => { c.fill(x, y, width, height, WHITE); c.fill(x, y + height - 1, width, 1, [232, 232, 234]); },
            b"gradient" => {
                let start = if comp.has_bg_color { comp.bg_color } else { self.proj.primary_color };
                let end = if comp.has_gradient { comp.gradient_end } else { [start[0].saturating_add(38), start[1].saturating_add(45), start[2].saturating_add(55)] };
                c.gradient_rrect(x, y, width, height, comp.radius, start, end);
            }
            _ if comp.has_bg_color => {
                if comp.has_gradient { c.gradient_rrect(x, y, width, height, comp.radius, comp.bg_color, comp.gradient_end); }
                else { c.rrect(x, y, width, height, comp.radius, comp.bg_color); }
            }
            _ => {},
        }

        let previous_clip = c.clip_rect();
        let first_child_hit = self.nhits;
        if kind == b"scrollbox" { c.set_clip(inner_x, y, inner_w, height); }
        let vertical_offset = if kind == b"scrollbox" { comp.scroll_offset } else { 0 };
        let origin_y = y + (if kind == b"card" { CARD_PAD } else if kind == b"topbar" { 8 } else { 0 }) - vertical_offset;
        let child_count = self.proj.comps.iter().take(self.proj.ncomps)
            .filter(|item| item.parent == idx && item.board == self.board && !item.deleted && !item.hidden).count();

        if kind == b"horizontal" || kind == b"topbar" {
            let gap_total = spacing.saturating_mul(child_count.saturating_sub(1) as i32);
            let cell_w = ((inner_w - gap_total) / child_count.max(1) as i32).max(1);
            let mut n = 0usize;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let cx = inner_x + n as i32 * (cell_w + spacing);
                    let _ = self.draw_component(c, child, cx, origin_y, cell_w);
                    n += 1;
                }
            }
        } else if kind == b"grid" {
            let cols = comp.columns.max(1) as usize;
            let cell_w = ((inner_w - spacing * (cols.saturating_sub(1) as i32)) / cols as i32).max(1);
            let mut row_heights = [0i32; MAX_COMPS];
            let mut n = 0usize;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let row = n / cols;
                    if row < row_heights.len() { row_heights[row] = row_heights[row].max(self.component_height(c, child, cell_w)); }
                    n += 1;
                }
            }
            let mut col = 0usize;
            let mut row = 0usize;
            let mut row_y = origin_y;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let _ = self.draw_component(c, child, inner_x + col as i32 * (cell_w + spacing), row_y, cell_w);
                    col += 1;
                    if col >= cols { col = 0; row_y += row_heights[row].saturating_add(spacing); row += 1; }
                }
            }
        } else {
            let mut child_y = origin_y;
            let mut drawn = 0usize;
            for child in 0..self.proj.ncomps {
                let item = self.proj.comps[child];
                if item.parent == idx && item.board == self.board && !item.deleted && !item.hidden {
                    let used = self.draw_component(c, child, inner_x, child_y, inner_w);
                    if used > 0 { child_y = child_y.saturating_add(used).saturating_add(spacing); drawn += 1; }
                }
            }
            let _ = drawn;
        }

        if kind == b"scrollbox" {
            let clip_bottom = y + height;
            for hit in self.hits.iter_mut().take(self.nhits).skip(first_child_hit) {
                let left = hit.x.max(inner_x);
                let top = hit.y.max(y);
                let right = (hit.x + hit.w).min(inner_x + inner_w);
                let bottom = (hit.y + hit.h).min(clip_bottom);
                if right <= left || bottom <= top {
                    hit.action = Action::None;
                    hit.w = 0;
                    hit.h = 0;
                } else {
                    hit.x = left; hit.y = top; hit.w = right - left; hit.h = bottom - top;
                }
            }
            c.restore_clip(previous_clip);
        }
        height
    }

    fn draw_choice_control(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        let comp = self.proj.comps[idx];
        let kind = comp.kind.get();
        if kind == b"dropdown" {
            let selected = comp.items.get(comp.selected.min(comp.nitems.saturating_sub(1)));
            let label = selected.map(|v| v.text.get()).unwrap_or(comp.label1.get());
            c.rrect(x, y, width, 44, 12, [248, 248, 249]);
            c.frame(x, y, width, 44, [222, 224, 228]);
            c.text(x + 14, y + 11, if label.is_empty() { b"Choose" } else { label }, TextStyle::Body, INK);
            c.text(x + width - 28, y + 9, if comp.expanded { "⌃".as_bytes() } else { "⌄".as_bytes() }, TextStyle::Heading, MUTED);
            self.push_hit(x, y, width, 44, Action::ExpandDropdown(idx));
            if comp.expanded {
                for item_i in 0..comp.nitems.min(MAX_ITEMS) {
                    let iy = y + 48 + item_i as i32 * 40;
                    c.fill(x, iy, width, 39, WHITE);
                    c.text(x + 14, iy + 9, comp.items[item_i].text.get(), TextStyle::Body, INK);
                    self.push_hit(x, iy, width, 40, Action::SelectItem(idx, item_i));
                }
                return 48 + comp.nitems.min(MAX_ITEMS) as i32 * 40;
            }
            44
        } else {
            let n = comp.nitems.min(MAX_ITEMS).max(1);
            let cell_w = (width / n as i32).max(1);
            for item_i in 0..comp.nitems.min(MAX_ITEMS) {
                let cx = x + item_i as i32 * cell_w;
                if kind == b"tab" {
                    if item_i == comp.selected { c.fill(cx + 8, y + 39, cell_w - 16, 3, self.proj.primary_color); }
                    c.text(cx + 10, y + 9, comp.items[item_i].text.get(), TextStyle::Body, if item_i == comp.selected { self.proj.primary_color } else { MUTED });
                } else {
                    if item_i == comp.selected { c.rrect(cx + 2, y + 2, cell_w - 4, 40, 12, [232, 240, 255]); }
                    c.text(cx + 8, y + 11, comp.items[item_i].text.get(), TextStyle::Small, if item_i == comp.selected { self.proj.primary_color } else { INK });
                }
                self.push_hit(cx, y, cell_w, 44, Action::SelectItem(idx, item_i));
            }
            44
        }
    }

    fn draw_card(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        let card_comp = self.proj.comps[idx];
        let radius = if card_comp.radius_custom { card_comp.radius } else { 16 };
        let inner = width - CARD_PAD * 2;
        let content_h = self.card_content_height(c, idx, inner);
        let h = (content_h + CARD_PAD * 2).max(CARD_PAD * 2 + 8);
        c.rrect(x, y, width, h, radius, WHITE);

        let mut cy = y + CARD_PAD;
        let mut row_x = 0;
        let mut row_h = 0;
        let mut row_active = false;
        for child in 0..self.proj.ncomps {
            let item = self.proj.comps[child];
            if item.parent != idx || item.board != self.board || item.deleted || item.hidden { continue; }
            if item.kind.get() == b"button" {
                let gap = if row_active { 10 } else { 0 };
                let mut available = (inner - row_x - gap).max(1);
                let mut natural_width = button_natural_width(c, &item, available);
                if row_active && row_x + gap + natural_width > inner {
                    cy += row_h + 12;
                    row_x = 0;
                    row_h = 0;
                    row_active = false;
                    available = inner;
                    natural_width = button_natural_width(c, &item, available);
                }
                if row_active { row_x += 10; }
                let used = self.draw_component(c, child, x + CARD_PAD + row_x, cy, inner - row_x);
                row_x += natural_width;
                row_h = row_h.max(used);
                row_active = true;
            } else {
                if row_active {
                    cy += row_h + 12;
                    row_x = 0;
                    row_h = 0;
                    row_active = false;
                }
                let used = self.draw_component(c, child, x + CARD_PAD, cy, inner);
                if used > 0 { cy += used + 12; }
            }
        }
        h
    }

    fn list_height(&self, c: &Canvas, comp: &Comp, width: i32) -> i32 {
        let head = text_block_height(c, width, comp.label1.get(), comp.label2.get());
        let rows = comp.nitems as i32 * 44;
        if head > 0 && comp.nitems > 0 { head + 14 + rows }
        else { head + rows }
    }

    /// A list is a set of rows, never a card. Model IDs resolve to event handlers.
    fn draw_list(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        let comp: Comp = self.proj.comps[idx];
        let head = text_block(c, true, x, y, width, comp.label1.get(), comp.label2.get(), if comp.has_fg_color { Some(comp.fg_color) } else { None });
        let mut cy = y + head;
        if head > 0 && comp.nitems > 0 { cy += 14; }
        let row_h = 44;
        let row_count = comp.nitems.min(MAX_ITEMS);
        if row_count > 0 {
            // One continuous list surface: only the first top and last bottom corners are rounded.
            c.rrect(x, cy, width, row_count as i32 * row_h, 12, WHITE);
        }
        for item_i in 0..row_count {
            let item = comp.items[item_i];
            // Keep rows flat; separators are inset so they do not cut through the rounded edge.
            if item_i + 1 < row_count {
                c.fill(x + 12, cy + row_h - 1, (width - 24).max(0), 1, [229, 229, 231]);
            }
            c.text(x + 14, cy + (row_h - line_h(TextStyle::ListItem)) / 2, item.text.get(), TextStyle::ListItem, INK);
            if !item.call.is_empty() {
                let arrow_x = x + width - 16;
                c.text(arrow_x, cy + (row_h - line_h(TextStyle::Heading)) / 2, "›".as_bytes(), TextStyle::Heading, self.proj.primary_color);
            }
            let action = Action::SelectItem(idx, item_i);
            self.push_hit(x, cy, width, row_h, action);
            cy += row_h;
        }
        cy - y
    }

    /// Draw the compact Warp4-style vertical scrollbar only when the document overflows.
    fn draw_scrollbar(&self, c: &mut Canvas) {
        if self.max_scroll <= 0 { return; }
        let track_h = (H - 4).max(1);
        let thumb_h = ((track_h as i64 * H as i64) / (H as i64 + self.max_scroll as i64))
            .max(18)
            .min(track_h as i64) as i32;
        let max_thumb_y = track_h - thumb_h;
        let thumb_y = if self.max_scroll > 0 {
            (max_thumb_y as i64 * self.scroll as i64 / self.max_scroll as i64) as i32
        } else {
            0
        };
        let x = W - 9;
        c.rrect(x, 2, 6, track_h, 3, [241, 241, 241]);
        c.rrect(x, 2 + thumb_y, 6, thumb_h, 3, [184, 184, 184]);
    }

    fn draw_dialog(&mut self, c: &mut Canvas) {
        let mut selected = None;
        for i in 0..self.proj.ncomps {
            let comp = self.proj.comps[i];
            if comp.board == self.board && comp.kind.get() == b"alert" && !comp.hidden && !comp.deleted {
                selected = Some(i);
                break;
            }
        }
        let idx = match selected { Some(v) => v, None => return };

        c.overlay(0, 0, W, H, [17, 22, 32], 132);
        self.push_hit(0, 0, W, H, Action::None); // Modal backdrop blocks clicks behind the dialog.
        let comp = self.proj.comps[idx];
        let dialog_w = 520.min(W - 48);
        let inner_w = dialog_w - 56;
        let title_w = dialog_w - 112;
        let mut title_lines = [(0usize, 0usize); MAX_LINES];
        let mut body_lines = [(0usize, 0usize); MAX_LINES];
        let title_count = wrap(c, comp.label1.get(), TextStyle::SectionTitle, title_w, &mut title_lines);
        let body_count = wrap(c, comp.label2.get(), TextStyle::SectionDescription, inner_w, &mut body_lines);
        let title_h = title_count as i32 * line_h(TextStyle::SectionTitle);
        let header_h = title_h.max(42);
        let body_h = body_count as i32 * line_h(TextStyle::SectionDescription);
        let dialog_h = (28 + header_h + 12 + body_h + 24 + BTN_H + 28).max(208).min(H - 48);
        let dx = (W - dialog_w) / 2;
        let dy = (H - dialog_h) / 2;

        c.rrect(dx, dy + 5, dialog_w, dialog_h, 18, [226, 228, 232]);
        c.rrect(dx, dy, dialog_w, dialog_h, 18, BORDER);
        c.rrect(dx + 1, dy + 1, dialog_w - 2, dialog_h - 2, 17, WHITE);
        c.rrect(dx + 28, dy + 27, 42, 42, 13, [232, 242, 255]);
        c.text(dx + 41, dy + 38, "!".as_bytes(), TextStyle::Heading, self.proj.primary_color);
        let title_y = dy + 34;
        let mut title_line_y = title_y;
        for (a, b) in title_lines.iter().take(title_count) {
            c.text(dx + 84, title_line_y, &comp.label1.get()[*a..*b], TextStyle::SectionTitle, INK);
            title_line_y += line_h(TextStyle::SectionTitle);
        }
        let body_y = dy + 28 + header_h + 12;
        let mut body_line_y = body_y;
        for (a, b) in body_lines.iter().take(body_count) {
            c.text(dx + 28, body_line_y, &comp.label2.get()[*a..*b], TextStyle::SectionDescription, MUTED);
            body_line_y += line_h(TextStyle::SectionDescription);
        }

        let close_label = if comp.close_label.is_empty() { b"OK" as &[u8] } else { comp.close_label.get() };
        let button_w = c.measure(close_label, TextStyle::Button) + 48;
        let bx = dx + dialog_w - 28 - button_w;
        let by = dy + dialog_h - 28 - BTN_H;
        let (hovered, pressed) = self.button_state(bx, by, button_w, BTN_H);
        draw_button(c, bx, by, button_w, BTN_H, close_label, true, false, None, None, self.proj.primary_color, BTN_H / 2, hovered, pressed);
        // The visual button remains compact, but the effective target is at least 64px high
        // and has a 10px forgiving margin on every side.
        let hit_left = (bx - 10).max(dx + 8);
        let hit_top = (by - 10).max(dy + 8);
        let hit_right = (bx + button_w + 10).min(dx + dialog_w - 8);
        let hit_bottom = (by + BTN_H + 10).min(dy + dialog_h - 8);
        self.push_hit(hit_left, hit_top, (hit_right - hit_left).max(button_w), (hit_bottom - hit_top).max(64), Action::Close(idx));
    }

    pub fn click(&mut self, x: f64, y: f64) -> bool {
        let (x, y) = (x as i32, y as i32);
        let mut i = self.nhits;
        while i > 0 {
            i -= 1;
            let hit = self.hits[i];
            if x < hit.x || x >= hit.x + hit.w || y < hit.y || y >= hit.y + hit.h { continue; }
            match hit.action {
                Action::None => return false,
                Action::Goto(board) => { self.board = board; self.scroll = 0; self.focused_input = None; return true; }
                Action::Close(idx) => {
                    if idx < self.proj.ncomps {
                        let call = self.proj.comps[idx].close_event;
                        if self.run_callback(call) { return true; }
                        self.proj.comps[idx].hidden = true;
                    }
                    return true;
                }
                Action::SelectItem(comp_idx, item_idx) => {
                    if comp_idx < self.proj.ncomps && item_idx < self.proj.comps[comp_idx].nitems {
                        if self.proj.comps[comp_idx].kind.get() == b"navigation" || self.proj.comps[comp_idx].kind.get() == b"dropdown" || self.proj.comps[comp_idx].kind.get() == b"tab" {
                            self.proj.comps[comp_idx].selected = item_idx;
                            self.proj.comps[comp_idx].expanded = false;
                            let call = self.proj.comps[comp_idx].items[item_idx].call;
                            if !self.run_callback(call) { let _ = self.trigger_component(comp_idx); }
                        } else {
                            let call = self.proj.comps[comp_idx].items[item_idx].call;
                            let _ = self.run_callback(call);
                        }
                    }
                    return true;
                }
                Action::ComponentEvent(idx) => { let _ = self.trigger_component(idx); return true; }
                Action::Toggle(idx) => {
                    if idx < self.proj.ncomps {
                        let comp = &mut self.proj.comps[idx];
                        comp.checked = !comp.checked;
                        comp.value.set(if comp.checked { b"true" } else { b"false" });
                        let _ = self.trigger_component(idx);
                    }
                    return true;
                }
                Action::Input(idx) => { self.focused_input = Some(idx); return true; }
                Action::Slider(idx) => {
                    let _ = self.set_slider_from_pointer(idx, hit.x, hit.w, x);
                    return true;
                }
                Action::ExpandDropdown(idx) => {
                    if idx < self.proj.ncomps { self.proj.comps[idx].expanded = !self.proj.comps[idx].expanded; }
                    return true;
                }
            }
        }
        false
    }

    /// Update the currently focused input. The platform event adapter passes
    /// UTF-8 text or requests one grapheme-safe backspace operation.
    pub fn text_input(&mut self, text: &[u8], backspace: bool) -> bool {
        let idx = match self.focused_input { Some(v) if v < self.proj.ncomps => v, _ => return false };
        if self.proj.comps[idx].kind.get() != b"input" { self.focused_input = None; return false; }
        let old = self.proj.comps[idx].value.get();
        let mut next = [0u8; 192];
        let mut len = old.len().min(next.len() - 1);
        next[..len].copy_from_slice(&old[..len]);
        if backspace {
            if len > 0 {
                len -= 1;
                while len > 0 && (next[len] & 0xC0) == 0x80 { len -= 1; }
            }
        } else {
            let mut add_len = 0usize;
            while add_len < text.len() && add_len < next.len() - 1 - len {
                let byte = text[add_len];
                if byte >= 0x20 && byte != 0x7f { next[len] = byte; len += 1; }
                add_len += 1;
            }
        }
        self.proj.comps[idx].value.set(&next[..len]);
        let _ = self.trigger_component(idx);
        true
    }

    /// Route a wheel gesture to the scrollbox under the pointer, falling back
    /// to the board document when no scrollbox contains the pointer.
    pub fn scroll_at(&mut self, x: f64, y: f64, dy: f64) -> bool {
        if !x.is_finite() || !y.is_finite() || !dy.is_finite() { return false; }
        let px = x as i32;
        let py = y as i32;
        let mut index = self.proj.ncomps;
        while index > 0 {
            index -= 1;
            let rect = self.scrollbox_rects[index];
            if rect.valid && px >= rect.x && px < rect.x + rect.w && py >= rect.y && py < rect.y + rect.h {
                let content_height = self.scrollbox_content_heights[index];
                let delta = dy.max(-4096.0).min(4096.0);
                let step = delta as i32;
                let max_offset = (content_height - rect.h).max(0);
                let comp = &mut self.proj.comps[index];
                let next = (comp.scroll_offset as i64 - step as i64).max(0).min(max_offset as i64) as i32;
                if next != comp.scroll_offset { comp.scroll_offset = next; return true; }
                return false;
            }
        }
        self.scroll(dy)
    }

    /// Apply a normalized vertical scroll delta in logical pixels.
    ///
    /// Platform event adapters should convert their native wheel/trackpad
    /// units to logical-pixel deltas before calling this method.
    pub fn scroll(&mut self, dy: f64) -> bool {
        if !dy.is_finite() || self.max_scroll <= 0 {
            return false;
        }

        // Bound a single event so malformed or unusually large backend values
        // cannot overflow the accumulator or jump beyond the document.
        let delta = dy.max(-4096.0).min(4096.0);
        self.scroll_acc = (self.scroll_acc - delta).max(-4096.0).min(4096.0);
        let step = self.scroll_acc as i32;
        self.scroll_acc -= step as f64;
        if step == 0 {
            return false;
        }

        let next = (self.scroll as i64 + step as i64)
            .max(0)
            .min(self.max_scroll as i64) as i32;
        if next == self.scroll {
            return false;
        }
        self.scroll = next;
        true
    }

    pub fn title(&self) -> &[u8] { self.proj.title.get() }
}
