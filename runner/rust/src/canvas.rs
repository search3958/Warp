//! 画素を自前で管理するフレームバッファ。
//!
//! 座標は論理座標（ポイント）で扱い、1 論理ピクセルを s×s のデバイス画素に展開する
//! （Retina では s=2）。画素は [R, G, B, 0] の4バイト（kCGImageAlphaNoneSkipLast と一致）。
//! 図形の描画はすべてこのコードが画素単位で行う。
//! 文字は macOS では OS 標準フォント（CoreGraphics 経由）、それ以外では内蔵の簡易フォントを使う。

use crate::font::glyph;
use crate::sys::{write_all, write_num, STDOUT};

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, PartialEq)]
pub enum TextStyle {
    Title,
    Heading,
    Body,
    Small,
    Button,
}

pub struct Canvas {
    px: *mut u8,
    /// 1 論理ピクセルあたりのデバイス画素数（1 または 2）
    pub s: usize,
    pub w: i32,
    pub h: i32,
    /// 描画を許可する領域 (x0, y0, x1, y1)、論理座標
    clip: (i32, i32, i32, i32),
}

impl Canvas {
    /// px は (w*s) * (h*s) * 4 バイト以上の領域を指すこと。
    ///
    /// # Safety
    /// 呼び出し元が上記の大きさと有効性を保証すること。
    pub unsafe fn new(px: *mut u8, s: usize, w: i32, h: i32) -> Self {
        Canvas { px, s, w, h, clip: (0, 0, w, h) }
    }

    pub fn set_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.w);
        let y1 = (y + h).min(self.h);
        self.clip = (x0, y0, x1, y1);
    }

    pub fn clear_clip(&mut self) {
        self.clip = (0, 0, self.w, self.h);
    }

    /// 1 論理ピクセルを描く（クリップ内のみ）。
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: Rgb) {
        let (x0, y0, x1, y1) = self.clip;
        if x < x0 || y < y0 || x >= x1 || y >= y1 {
            return;
        }
        let s = self.s;
        let dw = self.w as usize * s;
        for dy in 0..s {
            for dx in 0..s {
                let px_x = x as usize * s + dx;
                let px_y = y as usize * s + dy;
                let i = (px_y * dw + px_x) * 4;
                // SAFETY: new() で保証された範囲内の画素のみを書く
                unsafe {
                    *self.px.add(i) = c[0];
                    *self.px.add(i + 1) = c[1];
                    *self.px.add(i + 2) = c[2];
                    *self.px.add(i + 3) = 0;
                }
            }
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        let (x0, y0, x1, y1) = self.clip;
        let xa = x.max(x0);
        let ya = y.max(y0);
        let xb = (x + w).min(x1);
        let yb = (y + h).min(y1);
        for yy in ya..yb {
            for xx in xa..xb {
                self.put(xx, yy, c);
            }
        }
    }

    /// 角丸の矩形を塗る（角は半径 r の円弧で切り落とす）。
    pub fn rrect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: Rgb) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.min(w / 2).min(h / 2).max(0);
        for yy in y..y + h {
            for xx in x..x + w {
                let cx = if xx < x + r {
                    x + r - 1
                } else if xx >= x + w - r {
                    x + w - r
                } else {
                    xx
                };
                let cy = if yy < y + r {
                    y + r - 1
                } else if yy >= y + h - r {
                    y + h - r
                } else {
                    yy
                };
                let dx = (xx - cx) as i64;
                let dy = (yy - cy) as i64;
                if dx * dx + dy * dy > (r as i64) * (r as i64) {
                    continue;
                }
                self.put(xx, yy, c);
            }
        }
    }

    /// 幅・高さとも 1 以上であること。
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        self.fill(x, y, w, 1, c);
        self.fill(x, y + h - 1, w, 1, c);
        self.fill(x, y, 1, h, c);
        self.fill(x + w - 1, y, 1, h, c);
    }

    /// 文字列を描画し、描画幅（論理ピクセル）を返す。
    #[cfg(target_os = "macos")]
    pub fn text(&mut self, x: i32, y: i32, s: &[u8], st: TextStyle, c: Rgb) -> i32 {
        crate::mac::draw_text(x, y, s, st, c, self.clip)
    }

    /// 文字列の描画幅（論理ピクセル）を返す。
    #[cfg(target_os = "macos")]
    pub fn measure(&self, s: &[u8], st: TextStyle) -> i32 {
        crate::mac::measure(s, st)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn text(&mut self, x: i32, y: i32, s: &[u8], st: TextStyle, c: Rgb) -> i32 {
        self.fallback_text(true, x, y, s, st, c)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn measure(&self, s: &[u8], st: TextStyle) -> i32 {
        // 描画しない版。c は使わない
        let mut dummy = Canvas { px: self.px, s: self.s, w: self.w, h: self.h, clip: self.clip };
        dummy.fallback_text(false, 0, 0, s, st, [0, 0, 0])
    }

    /// 内蔵の 5x7 フォントで描く（draw=false のときは幅の計算のみ）。
    #[cfg(not(target_os = "macos"))]
    fn fallback_text(&mut self, draw: bool, mut x: i32, y: i32, s: &[u8], st: TextStyle, c: Rgb) -> i32 {
        let sc: i32 = match st {
            TextStyle::Title | TextStyle::Heading => 2,
            _ => 1,
        };
        let start = x;
        let mut i = 0;
        while i < s.len() {
            let b = s[i];
            if b < 0x20 {
                x += 6 * sc;
                i += 1;
            } else if b < 0x80 {
                if draw {
                    let rows = glyph(b);
                    for (r, bits) in rows.iter().enumerate() {
                        for col in 0..5 {
                            if bits & (0x10 >> col) != 0 {
                                self.fill(x + col * sc, y + r as i32 * sc, sc, sc, c);
                            }
                        }
                    }
                }
                x += 6 * sc;
                i += 1;
            } else {
                if draw {
                    self.frame(x, y, 10 * sc, 7 * sc, c);
                }
                x += 12 * sc;
                i += utf8_len(b);
            }
        }
        x - start
    }

    /// 画像を PPM (P6) 形式で標準出力に書き出す（デバイス画素単位）。
    pub fn write_ppm(&self) -> bool {
        let dw = self.w as usize * self.s;
        let dh = self.h as usize * self.s;
        if !(write_all(STDOUT, b"P6\n")
            && write_num(STDOUT, dw)
            && write_all(STDOUT, b" ")
            && write_num(STDOUT, dh)
            && write_all(STDOUT, b"\n255\n"))
        {
            return false;
        }
        let mut row = [0u8; 4096 * 3];
        if dw * 3 > row.len() {
            return false;
        }
        for y in 0..dh {
            for x in 0..dw {
                let i = (y * dw + x) * 4;
                // SAFETY: new() で保証された範囲内
                unsafe {
                    row[x * 3] = *self.px.add(i);
                    row[x * 3 + 1] = *self.px.add(i + 1);
                    row[x * 3 + 2] = *self.px.add(i + 2);
                }
            }
            if !write_all(STDOUT, &row[..dw * 3]) {
                return false;
            }
        }
        true
    }
}

/// UTF-8 の先頭バイトから、その文字のバイト数を返す。
pub fn utf8_len(lead: u8) -> usize {
    if lead >= 0xF0 {
        4
    } else if lead >= 0xE0 {
        3
    } else if lead >= 0xC0 {
        2
    } else {
        1
    }
}
