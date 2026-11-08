//! Software framebuffer for the MB5 preview runner.
//! Shapes, rounded corners, and text are rasterized without OS drawing APIs.
//! Coordinates use logical pixels; `s` expands each logical pixel to device pixels.


use crate::sys::{write_all, write_num, STDOUT};

pub type Rgb = [u8; 3];

#[derive(Clone, Copy, PartialEq)]
pub enum TextStyle {
    Title,
    SectionTitle,
    SectionDescription,
    Heading,
    Body,
    ListItem,
    Small,
    Button,
}

#[inline]
fn circle_edge_coverage(
    px: i32,
    py: i32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    left_edge: bool,
) -> u8 {
    const SUB_ROWS: i32 = 4;
    let radius_sq = radius * radius;
    let mut coverage = 0.0f32;

    for sub_y in 0..SUB_ROWS {
        let sample_y = py as f32 + (sub_y as f32 + 0.5) / SUB_ROWS as f32;
        let dy = sample_y - center_y;
        let dx = stb_truetype_rust::c_runtime::sqrt((radius_sq - dy * dy).max(0.0));
        let edge_x = if left_edge { center_x - dx } else { center_x + dx };
        let overlap = if left_edge {
            (px as f32 + 1.0 - edge_x).max(0.0).min(1.0)
        } else {
            (edge_x - px as f32).max(0.0).min(1.0)
        };
        coverage += overlap;
    }

    (coverage * (255.0 / SUB_ROWS as f32) + 0.5) as u8
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

    /// Alpha-blend a solid overlay into the existing framebuffer.
    pub fn overlay(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb, alpha: u8) {
        let (x0, y0, x1, y1) = self.clip;
        let xa = x.max(x0);
        let ya = y.max(y0);
        let xb = (x + w).min(x1);
        let yb = (y + h).min(y1);
        let a = alpha as u32;
        let inv = 255 - a;
        let dw = self.w as usize * self.s;
        for yy in ya..yb {
            for xx in xa..xb {
                for dy in 0..self.s {
                    for dx in 0..self.s {
                        let px_x = xx as usize * self.s + dx;
                        let px_y = yy as usize * self.s + dy;
                        let i = (px_y * dw + px_x) * 4;
                        // SAFETY: coordinates are clipped to the framebuffer.
                        unsafe {
                            for channel in 0..3 {
                                let old = *self.px.add(i + channel) as u32;
                                let new = color[channel] as u32;
                                *self.px.add(i + channel) = ((old * inv + new * a + 127) / 255) as u8;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Anti-aliased rounded rectangle using four vertical subpixel samples
    /// and exact fractional horizontal coverage on each scanline.
    ///
    /// The axis-aligned interior is filled directly. Only the four corner
    /// regions need coverage calculations, keeping the cost proportional to
    /// the corner area rather than the full rectangle.
    pub fn rrect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: Rgb) {
        if w <= 0 || h <= 0 {
            return;
        }
        let radius = r.min(w / 2).min(h / 2).max(0);
        if radius == 0 {
            self.fill(x, y, w, h, c);
            return;
        }

        // Axis-aligned center regions are fully covered and need no AA.
        self.fill(x + radius, y, w - radius * 2, h, c);
        self.fill(x, y + radius, radius, h - radius * 2, c);
        self.fill(x + w - radius, y + radius, radius, h - radius * 2, c);

        let scale = self.s.max(1) as i32;
        let left = x.saturating_mul(scale);
        let top = y.saturating_mul(scale);
        let right = x.saturating_add(w).saturating_mul(scale);
        let bottom = y.saturating_add(h).saturating_mul(scale);
        let radius_px = radius.saturating_mul(scale);
        let radius_f = radius_px as f32;
        let left_center = (left + radius_px) as f32;
        let right_center = (right - radius_px) as f32;
        let top_center = (top + radius_px) as f32;
        let bottom_center = (bottom - radius_px) as f32;

        // Top-left / top-right corners.
        for py in top..top + radius_px {
            for px in left..left + radius_px {
                let alpha = circle_edge_coverage(
                    px, py, left_center, top_center, radius_f, true,
                );
                if alpha != 0 { self.blend_device(px, py, c, alpha); }
            }
            for px in right - radius_px..right {
                let alpha = circle_edge_coverage(
                    px, py, right_center, top_center, radius_f, false,
                );
                if alpha != 0 { self.blend_device(px, py, c, alpha); }
            }
        }

        // Bottom-left / bottom-right corners.
        for py in bottom - radius_px..bottom {
            for px in left..left + radius_px {
                let alpha = circle_edge_coverage(
                    px, py, left_center, bottom_center, radius_f, true,
                );
                if alpha != 0 { self.blend_device(px, py, c, alpha); }
            }
            for px in right - radius_px..right {
                let alpha = circle_edge_coverage(
                    px, py, right_center, bottom_center, radius_f, false,
                );
                if alpha != 0 { self.blend_device(px, py, c, alpha); }
            }
        }
    }

    /// Alpha-composite one physical pixel, respecting the logical clip.
    #[inline]
    fn blend_device(&mut self, px: i32, py: i32, color: Rgb, alpha: u8) {
        if alpha == 0 || px < 0 || py < 0 {
            return;
        }
        let scale = self.s.max(1) as i32;
        let lx = px / scale;
        let ly = py / scale;
        let (x0, y0, x1, y1) = self.clip;
        if lx < x0 || ly < y0 || lx >= x1 || ly >= y1 {
            return;
        }
        let dw = self.w.max(0) as usize * self.s.max(1);
        let dh = self.h.max(0) as usize * self.s.max(1);
        if px as usize >= dw || py as usize >= dh {
            return;
        }
        let offset = (py as usize * dw + px as usize) * 4;
        let a = alpha as u32;
        let inv = 255 - a;
        unsafe {
            for channel in 0..3 {
                let old = *self.px.add(offset + channel) as u32;
                let src = color[channel] as u32;
                *self.px.add(offset + channel) = ((src * a + old * inv + 127) / 255) as u8;
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

    /// Draw TrueType text into the software framebuffer. This path is identical
    /// on macOS, Linux, and other supported targets.
    pub fn text(&mut self, x: i32, y: i32, text: &[u8], style: TextStyle, color: Rgb) -> i32 {
        let scale = self.s.max(1);
        crate::font::draw(x, y, text, style, scale, |px, py, coverage| {
            self.blend_device(px, py, color, coverage);
        })
    }

    /// Measure text using the same bundled font metrics used by drawing.
    pub fn measure(&self, text: &[u8], style: TextStyle) -> i32 {
        crate::font::measure(text, style, self.s.max(1))
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
