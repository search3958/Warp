//! MB5 preview UI and small script scheduler.
//! The tester intentionally has no in-content header or board tab bar.

use crate::canvas::{utf8_len, Canvas, Rgb, TextStyle};
use crate::project::{
    Comp, Project, ScriptCommand, MAX_COMPS, MAX_ITEMS, NO_PARENT,
    CMD_ADD_COMPONENT, CMD_CALL, CMD_DELETE_COMPONENT, CMD_SET_DISPLAY, CMD_WAIT,
};

pub const W: i32 = 760;
pub const H: i32 = 640;

const PAD_X: i32 = 36;
const PAD_TOP: i32 = 28;
const GAP: i32 = 18;
const CARD_PAD: i32 = 24;
const BTN_H: i32 = 42;
const MAX_LINES: usize = 20;
const MAX_HITS: usize = MAX_COMPS * (MAX_ITEMS + 2);
const MAX_SCRIPT_DEPTH: usize = 8;

const BG: Rgb = [246, 247, 250];
const WHITE: Rgb = [255, 255, 255];
const INK: Rgb = [27, 29, 34];
const MUTED: Rgb = [112, 117, 127];
const BORDER: Rgb = [220, 223, 230];
const ACCENT: Rgb = [0, 112, 240];
const SELECTED: Rgb = [231, 241, 255];

#[derive(Clone, Copy)]
enum Action {
    None,
    Goto(usize),
    Close(usize),
    SelectItem(usize, usize),
}

#[derive(Clone, Copy)]
struct Hit {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    action: Action,
}

const EMPTY_HIT: Hit = Hit { x: 0, y: 0, w: 0, h: 0, action: Action::None };

#[derive(Clone, Copy)]
struct Frame {
    script: usize,
    pc: usize,
}

const EMPTY_FRAME: Frame = Frame { script: 0, pc: 0 };

pub struct App {
    pub proj: Project,
    board: usize,
    scroll: i32,
    max_scroll: i32,
    scroll_acc: f64,
    selected_items: [i16; MAX_COMPS],
    hits: [Hit; MAX_HITS],
    nhits: usize,
    frames: [Frame; MAX_SCRIPT_DEPTH],
    nframes: usize,
    wait_until_ms: u64,
}

fn line_h(st: TextStyle) -> i32 {
    match st {
        TextStyle::Title => 32,
        TextStyle::Heading => 26,
        TextStyle::Body => 22,
        TextStyle::Small => 18,
        TextStyle::Button => 20,
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
    let nt = wrap(c, title, TextStyle::Heading, maxw, &mut title_lines);
    let nb = wrap(c, body, TextStyle::Body, maxw, &mut body_lines);
    let gap = if nt > 0 && nb > 0 { 7 } else { 0 };
    nt as i32 * line_h(TextStyle::Heading) + gap + nb as i32 * line_h(TextStyle::Body)
}

fn text_block(c: &mut Canvas, draw: bool, x: i32, y: i32, maxw: i32, title: &[u8], body: &[u8]) -> i32 {
    let mut title_lines = [(0usize, 0usize); MAX_LINES];
    let mut body_lines = [(0usize, 0usize); MAX_LINES];
    let nt = wrap(c, title, TextStyle::Heading, maxw, &mut title_lines);
    let nb = wrap(c, body, TextStyle::Body, maxw, &mut body_lines);
    let gap = if nt > 0 && nb > 0 { 7 } else { 0 };
    let height = nt as i32 * line_h(TextStyle::Heading) + gap + nb as i32 * line_h(TextStyle::Body);
    if draw {
        let mut cy = y;
        for (a, b) in title_lines.iter().take(nt) {
            c.text(x, cy, &title[*a..*b], TextStyle::Heading, INK);
            cy += line_h(TextStyle::Heading);
        }
        cy += gap;
        for (a, b) in body_lines.iter().take(nb) {
            c.text(x, cy, &body[*a..*b], TextStyle::Body, INK);
            cy += line_h(TextStyle::Body);
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
    custom_bg: Option<Rgb>,
    custom_fg: Option<Rgb>,
    radius: i32,
) {
    let fill = custom_bg.unwrap_or(if primary { ACCENT } else { BORDER });
    let fg = custom_fg.unwrap_or(if primary || custom_bg.is_some() { WHITE } else { INK });
    c.rrect(x, y, w, h, radius.min(h / 2), fill);
    if !primary && custom_bg.is_none() {
        c.rrect(x + 1, y + 1, w - 2, h - 2, (radius - 1).max(1), WHITE);
    }
    let tw = c.measure(label, TextStyle::Button);
    c.set_clip(x + 6, y + 2, (w - 12).max(1), h - 4);
    c.text(x + (w - tw) / 2, y + (h - line_h(TextStyle::Button)) / 2, label, TextStyle::Button, fg);
    c.clear_clip();
}

fn button_height(comp: &Comp) -> i32 {
    match comp.size.get() {
        b"large" => 50,
        b"small" => 34,
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
    button_width(comp, available, c.measure(label, TextStyle::Button) + 44)
}

impl App {
    pub fn new(proj: Project) -> Self {
        let initial_board = (0..proj.nboards)
            .find(|&i| proj.boards[i].get() == proj.main_board.get())
            .unwrap_or(0);
        let startup = proj.main_script;
        let mut app = Self {
            proj,
            board: initial_board,
            scroll: 0,
            max_scroll: 0,
            scroll_acc: 0.0,
            selected_items: [-1; MAX_COMPS],
            hits: [EMPTY_HIT; MAX_HITS],
            nhits: 0,
            frames: [EMPTY_FRAME; MAX_SCRIPT_DEPTH],
            nframes: 0,
            wait_until_ms: 0,
        };
        if !startup.is_empty() {
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
        let idx = match self.find_script(name, arg) {
            Some(v) => v,
            None => return false,
        };
        if self.nframes >= MAX_SCRIPT_DEPTH { return false; }
        self.frames[self.nframes] = Frame { script: idx, pc: 0 };
        self.nframes += 1;
        true
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
        if self.wait_until_ms != 0 {
            if now_ms < self.wait_until_ms { return false; }
            self.wait_until_ms = 0;
        }

        let mut changed = false;
        let mut budget = 64usize;
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
                    self.wait_until_ms = now_ms.saturating_add(cmd.millis);
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
        c.set_clip(0, 0, W, H);
        let end_y = self.draw_board(c);
        self.max_scroll = core::cmp::max(0, end_y + self.scroll + PAD_TOP - H);
        if self.scroll > self.max_scroll { self.scroll = self.max_scroll; }
        self.draw_dialog(c);
        c.clear_clip();
    }

    fn draw_board(&mut self, c: &mut Canvas) -> i32 {
        if self.proj.nboards == 0 {
            c.text(PAD_X, PAD_TOP, "表示できる board がありません".as_bytes(), TextStyle::Body, MUTED);
            return PAD_TOP + 40;
        }
        let mut y = PAD_TOP - self.scroll;
        for i in 0..self.proj.ncomps {
            let comp = self.proj.comps[i];
            if comp.board != self.board || comp.parent != NO_PARENT || comp.deleted || comp.hidden { continue; }
            if comp.kind.get() == b"alert" { continue; }
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
            b"text" => {
                text_block_height(c, width, comp.label1.get(), comp.label2.get())
            }
            b"list" => self.list_height(c, &comp, width),
            b"button" => button_height(&comp),
            b"card" => CARD_PAD * 2 + self.card_content_height(c, idx, width - CARD_PAD * 2),
            _ => 0,
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
        let comp: Comp = self.proj.comps[idx];
        if comp.deleted || comp.hidden { return 0; }
        match comp.kind.get() {
            b"text" => {
                text_block(c, true, x, y, width, comp.label1.get(), comp.label2.get())
            }
            b"list" => self.draw_list(c, idx, x, y, width),
            b"button" => {
                let label = if comp.label1.is_empty() { &b"Button"[..] } else { comp.label1.get() };
                let natural = c.measure(label, TextStyle::Button) + 44;
                let button_width = button_width(&comp, width, natural);
                let button_h = button_height(&comp);
                let bg = if comp.has_bg_color { Some(comp.bg_color) } else { None };
                let fg = if comp.has_fg_color { Some(comp.fg_color) } else { None };
                draw_button(c, x, y, button_width, button_h, label, comp.primary, bg, fg, comp.radius.max(12));
                let action = match self.find_board(comp.event.get()) {
                    Some(board) => Action::Goto(board),
                    None => Action::None,
                };
                self.push_hit(x, y, button_width, button_h, action);
                button_h
            }
            b"card" => self.draw_card(c, idx, x, y, width),
            _ => 0,
        }
    }

    fn draw_card(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        let radius = self.proj.comps[idx].radius.max(12);
        let inner = width - CARD_PAD * 2;
        let content_h = self.card_content_height(c, idx, inner);
        let h = (content_h + CARD_PAD * 2).max(CARD_PAD * 2 + 8);
        c.rrect(x, y + 2, width, h, radius, [226, 229, 235]);
        c.rrect(x, y, width, h, radius, WHITE);
        c.frame(x, y, width, h, BORDER);

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
        let rows = comp.nitems as i32 * 46;
        if head > 0 && comp.nitems > 0 { head + 14 + rows }
        else { head + rows }
    }

    /// A list is a set of rows, never a card. Model IDs resolve to event handlers.
    fn draw_list(&mut self, c: &mut Canvas, idx: usize, x: i32, y: i32, width: i32) -> i32 {
        let comp: Comp = self.proj.comps[idx];
        let head = text_block(c, true, x, y, width, comp.label1.get(), comp.label2.get());
        let mut cy = y + head;
        if head > 0 && comp.nitems > 0 { cy += 14; }
        for item_i in 0..comp.nitems.min(MAX_ITEMS) {
            let item = comp.items[item_i];
            let row_h = 46;
            if self.selected_items[idx] == item_i as i16 {
                c.rrect(x - 8, cy, width + 16, row_h, 13, SELECTED);
            }
            c.fill(x + 2, cy + row_h - 1, width - 4, 1, [232, 234, 239]);
            c.text(x + 8, cy + 13, "•".as_bytes(), TextStyle::Body, MUTED);
            c.text(x + 28, cy + 12, item.text.get(), TextStyle::Body, INK);
            if !item.call.is_empty() {
                let arrow_x = x + width - 18;
                c.text(arrow_x, cy + 11, "›".as_bytes(), TextStyle::Heading, MUTED);
            }
            let action = Action::SelectItem(idx, item_i);
            self.push_hit(x - 8, cy, width + 16, row_h, action);
            cy += row_h;
        }
        cy - y
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

        c.overlay(0, 0, W, H, [17, 22, 32], 142);
        self.push_hit(0, 0, W, H, Action::None); // Modal backdrop blocks clicks behind the dialog.
        let comp = self.proj.comps[idx];
        let dialog_w = 520.min(W - 48);
        let inner_w = dialog_w - 56;
        let body_h = text_block_height(c, inner_w, comp.label1.get(), comp.label2.get());
        let dialog_h = (body_h + 56 + 18 + BTN_H + 28).max(208).min(H - 48);
        let dx = (W - dialog_w) / 2;
        let dy = (H - dialog_h) / 2;

        c.rrect(dx, dy + 5, dialog_w, dialog_h, 24, [205, 209, 218]);
        c.rrect(dx, dy, dialog_w, dialog_h, 24, WHITE);
        c.frame(dx, dy, dialog_w, dialog_h, BORDER);
        c.rrect(dx + 28, dy + 27, 42, 42, 14, [232, 242, 255]);
        c.text(dx + 41, dy + 38, "!".as_bytes(), TextStyle::Heading, ACCENT);
        let title_y = dy + 34;
        c.text(dx + 84, title_y, comp.label1.get(), TextStyle::Heading, INK);
        let body_y = title_y + 40;
        text_block(c, true, dx + 28, body_y, inner_w, b"", comp.label2.get());

        let close_label = if comp.close_label.is_empty() { b"OK" as &[u8] } else { comp.close_label.get() };
        let button_w = c.measure(close_label, TextStyle::Button) + 42;
        let bx = dx + dialog_w - 28 - button_w;
        let by = dy + dialog_h - 28 - BTN_H;
        draw_button(c, bx, by, button_w, BTN_H, close_label, true, None, None, 14);
        self.push_hit(bx, by, button_w, BTN_H, Action::Close(idx));
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
                Action::Goto(board) => {
                    self.board = board;
                    self.scroll = 0;
                    return true;
                }
                Action::Close(idx) => {
                    let call = self.proj.comps[idx].close_event;
                    if !call.is_empty() && self.start_script(call.name.get(), call.arg.get()) {
                        return true;
                    }
                    self.proj.comps[idx].hidden = true;
                    return true;
                }
                Action::SelectItem(comp_idx, item_idx) => {
                    if comp_idx < MAX_COMPS { self.selected_items[comp_idx] = item_idx as i16; }
                    if comp_idx < self.proj.ncomps && item_idx < self.proj.comps[comp_idx].nitems {
                        let call = self.proj.comps[comp_idx].items[item_idx].call;
                        if !call.is_empty() { let _ = self.start_script(call.name.get(), call.arg.get()); }
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn scroll(&mut self, dy: f64) -> bool {
        self.scroll_acc -= dy;
        let step = self.scroll_acc as i32;
        self.scroll_acc -= step as f64;
        if step == 0 { return false; }
        let next = core::cmp::max(0, core::cmp::min(self.max_scroll, self.scroll + step));
        if next == self.scroll { return false; }
        self.scroll = next;
        true
    }

    pub fn title(&self) -> &[u8] { self.proj.title.get() }
}
