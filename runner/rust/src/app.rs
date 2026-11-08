//! 画面（UI）の状態・配置・描画・クリック処理。
//!
//! 構成:
//!   - @mainBoard で指定された board の内容を表示
//!   - board タブや独自ヘッダーは表示しない
//!   - board の内容はホイールでスクロール
//!
//! 命令ごとの表示:
//!   text   : 見出し（label1）と本文（label2）のカード
//!   alert  : 見出し・本文と「閉じる」ボタンのカード。閉じると非表示になる
//!   list   : 見出し・本文と項目一覧のカード
//!   button : 角丸のボタン。moveboard(名前) を持つ場合はクリックで board を切り替える

use crate::canvas::{utf8_len, Canvas, Rgb, TextStyle};
use crate::project::{Comp, Project, MAX_COMPS};

/// ウィンドウの論理サイズ（ポイント）。
pub const W: i32 = 760;
pub const H: i32 = 640;

const PAD: i32 = 20;
const CARD_X: i32 = 40;
const CARD_W: i32 = W - 2 * CARD_X;
const CARD_PAD: i32 = 18;
const GAP: i32 = 14;
const RADIUS: i32 = 12;
const BTN_H: i32 = 36;
const BTN_GAP: i32 = 10;
const MAX_LINES: usize = 16;
const MAX_HITS: usize = MAX_COMPS * 2;

const BG: Rgb = [242, 243, 246];
const WHITE: Rgb = [255, 255, 255];
const INK: Rgb = [28, 28, 32];
const MUTED: Rgb = [110, 112, 122];
const BORDER: Rgb = [214, 216, 224];
const ACCENT: Rgb = [0, 112, 240];

#[derive(Clone, Copy, PartialEq)]
enum Action {
    None,
    Goto(usize),
    Close(usize),
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

pub struct App {
    pub proj: Project,
    board: usize,
    scroll: i32,
    max_scroll: i32,
    scroll_acc: f64,
    closed: [bool; MAX_COMPS],
    hits: [Hit; MAX_HITS],
    nhits: usize,
}

fn line_h(st: TextStyle) -> i32 {
    match st {
        TextStyle::Title => 28,
        TextStyle::Heading => 22,
        TextStyle::Body => 19,
        TextStyle::Small => 16,
        TextStyle::Button => 18,
    }
}

/// 文字列を幅 maxw に収まるよう折り返す。各行のバイト範囲を out に入れ、行数を返す。
fn wrap(c: &Canvas, s: &[u8], st: TextStyle, maxw: i32, out: &mut [(usize, usize); MAX_LINES]) -> usize {
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

/// 角丸のカード（枠線つき白地）。
fn card(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.rrect(x, y, w, h, RADIUS, BORDER);
    c.rrect(x + 1, y + 1, w - 2, h - 2, RADIUS - 1, WHITE);
}

/// 角丸のボタン。primary は塗りつぶし、それ以外は枠線つき。
fn pill(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, label: &[u8], primary: bool, st: TextStyle) {
    let (fill, fg) = if primary { (ACCENT, WHITE) } else { (BORDER, INK) };
    c.rrect(x, y, w, h, h / 2, fill);
    if !primary {
        c.rrect(x + 1, y + 1, w - 2, h - 2, h / 2 - 1, WHITE);
    }
    let tw = c.measure(label, st);
    c.text(x + (w - tw) / 2, y + (h - 16) / 2, label, st, fg);
}

/// Comp に指定された色・角丸を反映したコンポーネントボタン。
fn component_pill(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, label: &[u8], comp: &Comp) {
    let fill = if comp.has_bg_color {
        comp.bg_color
    } else if comp.primary {
        ACCENT
    } else {
        BORDER
    };
    let fg = if comp.has_fg_color {
        comp.fg_color
    } else if comp.primary || comp.has_bg_color {
        WHITE
    } else {
        INK
    };
    let radius = (comp.radius as i32).max(0).min(h / 2);
    c.rrect(x, y, w, h, radius, fill);
    if !comp.primary && !comp.has_bg_color {
        c.rrect(x + 1, y + 1, w - 2, h - 2, (radius - 1).max(0), WHITE);
    }
    let tw = c.measure(label, TextStyle::Button);
    c.text(x + (w - tw) / 2, y + (h - 16) / 2, label, TextStyle::Button, fg);
}

/// 見出し・本文の行を描き、使った高さを返す（カードの中身の高さ計算にも使う）。
/// draw=false のときは描かず、高さだけ返す。
fn text_block(
    c: &mut Canvas,
    draw: bool,
    x: i32,
    y: i32,
    maxw: i32,
    l1: &[u8],
    l2: &[u8],
) -> i32 {
    let mut lines1 = [(0usize, 0usize); MAX_LINES];
    let mut lines2 = [(0usize, 0usize); MAX_LINES];
    let n1 = wrap(c, l1, TextStyle::Heading, maxw, &mut lines1);
    let n2 = wrap(c, l2, TextStyle::Body, maxw, &mut lines2);
    let gap = if n1 > 0 && n2 > 0 { 6 } else { 0 };
    let h1 = n1 as i32 * line_h(TextStyle::Heading);
    let h2 = n2 as i32 * line_h(TextStyle::Body);
    if draw {
        let mut ty = y;
        for k in 0..n1 {
            let (a, b) = lines1[k];
            c.text(x, ty, &l1[a..b], TextStyle::Heading, INK);
            ty += line_h(TextStyle::Heading);
        }
        ty += gap;
        for k in 0..n2 {
            let (a, b) = lines2[k];
            c.text(x, ty, &l2[a..b], TextStyle::Body, INK);
            ty += line_h(TextStyle::Body);
        }
    }
    h1 + gap + h2
}

impl App {
    pub fn new(proj: Project) -> Self {
        let mut closed = [false; MAX_COMPS];
        for i in 0..proj.ncomps {
            closed[i] = proj.comps[i].hidden;
        }
        let board = proj.main_board.min(proj.nboards.saturating_sub(1));
        App {
            proj,
            board,
            scroll: 0,
            max_scroll: 0,
            scroll_acc: 0.0,
            closed,
            hits: [EMPTY_HIT; MAX_HITS],
            nhits: 0,
        }
    }

    fn push_hit(&mut self, x: i32, y: i32, w: i32, h: i32, action: Action) {
        if self.nhits < MAX_HITS {
            self.hits[self.nhits] = Hit { x, y, w, h, action };
            self.nhits += 1;
        }
    }

    fn find_board(&self, name: &[u8]) -> Option<usize> {
        if name.is_empty() {
            return None;
        }
        (0..self.proj.nboards).find(|&b| self.proj.boards[b].get() == name)
    }

    /// 画面全体を描く。
    pub fn render(&mut self, c: &mut Canvas) {
        c.clear_clip();
        c.fill(0, 0, W, H, BG);
        self.nhits = 0;

        c.set_clip(0, 0, W, H);
        let y_end = self.draw_board(c);
        c.clear_clip();

        // スクロール上限（描いた内容の高さから計算）
        let total = y_end + self.scroll;
        self.max_scroll = core::cmp::max(0, total + PAD - H);
        if self.scroll > self.max_scroll {
            self.scroll = self.max_scroll;
        }
    }

    /// 現在の board の内容を描く。戻り値は最後の要素の下端（スクロール後の座標）。
    fn draw_board(&mut self, c: &mut Canvas) -> i32 {
        if self.proj.nboards == 0 {
            c.text(CARD_X, 24, "表示できる board がありません".as_bytes(), TextStyle::Body, MUTED);
            return 60;
        }

        let board = self.board;
        let mut y = PAD - self.scroll;
        // ボタン行の状態: (次の x, 行の y, 行内の最大高さ)
        let mut row: Option<(i32, i32, i32)> = None;

        for i in 0..self.proj.ncomps {
            let comp: Comp = self.proj.comps[i];
            if comp.board != board || comp.hidden {
                continue;
            }

            if comp.kind.get() == b"button" {
                let label: &[u8] = if comp.label1.get().is_empty() {
                    "ボタン".as_bytes()
                } else {
                    comp.label1.get()
                };
                let tw = c.measure(label, TextStyle::Button);
                let natural_width = core::cmp::max(tw + 36, 96);
                let requested_width = if comp.width_px > 0 {
                    if comp.width_percent {
                        (CARD_W * comp.width_px as i32) / 100
                    } else {
                        comp.width_px as i32
                    }
                } else {
                    natural_width
                };
                let bw = core::cmp::min(core::cmp::max(requested_width, 32), CARD_W);
                let bh = if comp.size.get() == b"small" {
                    30
                } else if comp.size.get() == b"large" {
                    44
                } else {
                    BTN_H
                };
                let (bx, by, row_height) = match row {
                    Some((nx, ry, rh)) if nx + bw <= CARD_X + CARD_W => (nx, ry, core::cmp::max(rh, bh)),
                    Some((_, ry, rh)) => (CARD_X, ry + rh + BTN_GAP, bh),
                    None => (CARD_X, y, bh),
                };
                row = Some((bx + bw + BTN_GAP, by, row_height));

                let action = match self.find_board(comp.event.get()) {
                    Some(target) => Action::Goto(target),
                    None => Action::None,
                };
                component_pill(c, bx, by, bw, bh, label, &comp);
                self.push_hit(bx, by, bw, bh, action);
                continue;
            }

            // ボタン以外が来たら、直前のボタン行を閉じる
            if let Some((_, ry, rh)) = row.take() {
                y = ry + rh + GAP;
            }

            let used = if comp.kind.get() == b"text" {
                self.draw_text_card(c, y, &comp)
            } else if comp.kind.get() == b"alert" {
                if self.closed[i] {
                    0
                } else {
                    self.draw_alert(c, y, i, &comp)
                }
            } else if comp.kind.get() == b"list" {
                self.draw_list(c, y, &comp)
            } else {
                0
            };
            if used > 0 {
                y += used + GAP;
            }
        }

        if let Some((_, ry, rh)) = row {
            y = ry + rh + GAP;
        }
        y
    }

    fn draw_text_card(&mut self, c: &mut Canvas, y: i32, comp: &Comp) -> i32 {
        let l1 = comp.label1.get();
        let l2 = comp.label2.get();
        if l1.is_empty() && l2.is_empty() {
            return 0;
        }
        let inner = CARD_W - 2 * CARD_PAD;
        let h = text_block(c, false, 0, 0, inner, l1, l2) + 2 * CARD_PAD;
        card(c, CARD_X, y, CARD_W, h);
        text_block(c, true, CARD_X + CARD_PAD, y + CARD_PAD, inner, l1, l2);
        h
    }

    fn draw_alert(&mut self, c: &mut Canvas, y: i32, idx: usize, comp: &Comp) -> i32 {
        let l1 = comp.label1.get();
        let l2 = comp.label2.get();
        let stripe = 8;
        let inner = CARD_W - 2 * CARD_PAD - stripe;
        let close_label: &[u8] = if comp.close_label.get().is_empty() {
            "閉じる".as_bytes()
        } else {
            comp.close_label.get()
        };
        let cw = core::cmp::max(c.measure(close_label, TextStyle::Button) + 36, 88);
        let h = text_block(c, false, 0, 0, inner, l1, l2) + 2 * CARD_PAD + 14 + BTN_H;

        card(c, CARD_X, y, CARD_W, h);
        c.fill(CARD_X + 1, y + 12, 5, h - 24, ACCENT);
        let tx = CARD_X + CARD_PAD + stripe;
        text_block(c, true, tx, y + CARD_PAD, inner, l1, l2);

        let bx = CARD_X + CARD_W - CARD_PAD - cw;
        let by = y + h - CARD_PAD - BTN_H;
        pill(c, bx, by, cw, BTN_H, close_label, false, TextStyle::Button);
        self.push_hit(bx, by, cw, BTN_H, Action::Close(idx));
        h
    }

    fn draw_list(&mut self, c: &mut Canvas, y: i32, comp: &Comp) -> i32 {
        let l1 = comp.label1.get();
        let l2 = comp.label2.get();
        if l1.is_empty() && l2.is_empty() && comp.nitems == 0 {
            return 0;
        }
        let inner = CARD_W - 2 * CARD_PAD;
        let head = text_block(c, false, 0, 0, inner, l1, l2);
        let bullet_w = 16;
        let mut items_h = 0;
        for k in 0..comp.nitems {
            let mut lines = [(0usize, 0usize); MAX_LINES];
            let n = wrap(c, comp.items[k].get(), TextStyle::Body, inner - bullet_w, &mut lines);
            items_h += n as i32 * line_h(TextStyle::Body) + 4;
        }
        let gap = if head > 0 && items_h > 0 { 10 } else { 0 };
        let h = 2 * CARD_PAD + head + gap + items_h;
        card(c, CARD_X, y, CARD_W, h);

        text_block(c, true, CARD_X + CARD_PAD, y + CARD_PAD, inner, l1, l2);
        let mut iy = y + CARD_PAD + head + gap;
        for k in 0..comp.nitems {
            let item = comp.items[k].get();
            let mut lines = [(0usize, 0usize); MAX_LINES];
            let n = wrap(c, item, TextStyle::Body, inner - bullet_w, &mut lines);
            c.text(CARD_X + CARD_PAD, iy, "•".as_bytes(), TextStyle::Body, MUTED);
            for m in 0..n {
                let (a, b) = lines[m];
                c.text(CARD_X + CARD_PAD + bullet_w, iy, &item[a..b], TextStyle::Body, INK);
                iy += line_h(TextStyle::Body);
            }
            iy += 4;
        }
        h
    }

    /// クリック（論理座標、左上原点）。画面が変わったら true を返す。
    pub fn click(&mut self, x: f64, y: f64) -> bool {
        let (x, y) = (x as i32, y as i32);
        let mut i = self.nhits;
        while i > 0 {
            i -= 1;
            let h = self.hits[i];
            if x >= h.x && x < h.x + h.w && y >= h.y && y < h.y + h.h {
                return match h.action {
                    Action::None => false,
                    Action::Goto(b) => {
                        self.board = b;
                        self.scroll = 0;
                        true
                    }
                    Action::Close(k) => {
                        self.closed[k] = true;
                        true
                    }
                };
            }
        }
        false
    }

    /// ホイールのスクロール量（論理ポイント、上方向が正）。変化したら true。
    pub fn scroll(&mut self, dy: f64) -> bool {
        // トラックパッドの小さな量も失わないよう、端数を溜めてから整数分だけ動かす
        self.scroll_acc -= dy;
        let step = self.scroll_acc as i32;
        self.scroll_acc -= step as f64;
        if step == 0 {
            return false;
        }
        let next = core::cmp::max(0, core::cmp::min(self.max_scroll, self.scroll + step));
        if next == self.scroll {
            return false;
        }
        self.scroll = next;
        true
    }

    pub fn title(&self) -> &[u8] {
        self.proj.title.get()
    }
}
