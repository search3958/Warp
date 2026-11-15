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

    /// Figma-style smooth squircle rectangle using the same cubic control
    /// points and radius clamping as the supplied SmoothCornerShape.
    pub fn rrect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: Rgb) {
        if w <= 0 || h <= 0 { return; }
        let scale = self.s.max(1) as i32;
        let left = x.saturating_mul(scale);
        let top = y.saturating_mul(scale);
        let right = x.saturating_add(w).saturating_mul(scale);
        let bottom = y.saturating_add(h).saturating_mul(scale);
        let width = right.saturating_sub(left);
        let height = bottom.saturating_sub(top);
        let radius = (r.max(0) as f32 * scale as f32).min(width as f32 * 0.5).min(height as f32 * 0.5);
        if radius <= 0.0 { self.fill(x, y, w, h, c); return; }

        #[derive(Clone, Copy)]
        struct Pt { x: f32, y: f32 }
        impl Pt { const ZERO: Self = Self { x: 0.0, y: 0.0 }; }
        let mut points = [Pt::ZERO; 160];
        let mut count = 0usize;
        let mut current = Pt { x: right as f32, y: (top + bottom) as f32 * 0.5 };
        points[count] = current; count += 1;
        let lx = (width as f32 * 0.5).min(1.528665 * radius);
        let ly = (height as f32 * 0.5).min(1.528665 * radius);
        let cx3 = 0.63148 * radius; let cx4 = 0.37282 * radius;
        let cx5 = 0.16905 * radius; let cx6 = 0.07491 * radius;
        let cy3 = cx3; let cy4 = cx4; let cy5 = cx5; let cy6 = cx6;
        let d1x = 0.04 * radius + 0.75697 * (lx - radius);
        let d2x = 0.18 * radius + 0.90847 * (lx - radius);
        let d1y = 0.04 * radius + 0.75697 * (ly - radius);
        let d2y = 0.18 * radius + 0.90847 * (ly - radius);
        let ox = left as f32; let oy = top as f32; let w = width as f32; let h = height as f32;
        #[inline]
        fn add_line(points: &mut [Pt; 160], count: &mut usize, p: Pt) {
            if *count < points.len() { points[*count] = p; *count += 1; }
        }
        #[inline]
        fn add_cubic(points: &mut [Pt; 160], count: &mut usize, p0: Pt, p1: Pt, p2: Pt, p3: Pt) {
            const STEPS: usize = 12;
            let mut i = 1usize;
            while i <= STEPS {
                let t = i as f32 / STEPS as f32; let mt = 1.0 - t;
                let a = mt * mt * mt; let b = 3.0 * mt * mt * t;
                let cc = 3.0 * mt * t * t; let d = t * t * t;
                add_line(points, count, Pt { x: a*p0.x + b*p1.x + cc*p2.x + d*p3.x, y: a*p0.y + b*p1.y + cc*p2.y + d*p3.y });
                i += 1;
            }
        }
        // Right -> bottom -> left -> top -> right, matching the Kotlin Path.
        let p = Pt { x: ox + w, y: oy + h - ly }; add_line(&mut points, &mut count, p); current = p;
        let p3 = Pt { x: ox + w - cx6, y: oy + h - cx3 };
        add_cubic(&mut points, &mut count, current, Pt{x:ox+w,y:oy+h-ly+d1y}, Pt{x:ox+w,y:oy+h-ly+d2y}, p3); current=p3;
        let p3 = Pt{x:ox+w-cx3,y:oy+h-cy6};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+w-cx5,y:oy+h-cy4},Pt{x:ox+w-cx4,y:oy+h-cy5},p3); current=p3;
        let p3=Pt{x:ox+w-lx,y:oy+h};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+w-lx+d2x,y:oy+h},Pt{x:ox+w-lx+d1x,y:oy+h},p3); current=p3;
        let p=Pt{x:ox+lx,y:oy+h}; add_line(&mut points,&mut count,p); current=p;
        let p3=Pt{x:ox+cx3,y:oy+h-cy6};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+lx-d1x,y:oy+h},Pt{x:ox+lx-d2x,y:oy+h},p3); current=p3;
        let p3=Pt{x:ox+cx6,y:oy+h-cy3};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+cx4,y:oy+h-cy5},Pt{x:ox+cx5,y:oy+h-cy4},p3); current=p3;
        let p3=Pt{x:ox,y:oy+h-ly};
        add_cubic(&mut points,&mut count,current,Pt{x:ox,y:oy+h-ly+d2y},Pt{x:ox,y:oy+h-ly+d1y},p3); current=p3;
        let p=Pt{x:ox,y:oy+ly}; add_line(&mut points,&mut count,p); current=p;
        let p3=Pt{x:ox+cx6,y:oy+cy3};
        add_cubic(&mut points,&mut count,current,Pt{x:ox,y:oy+ly-d1y},Pt{x:ox,y:oy+ly-d2y},p3); current=p3;
        let p3=Pt{x:ox+cx3,y:oy+cy6};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+cx5,y:oy+cy4},Pt{x:ox+cx4,y:oy+cy5},p3); current=p3;
        let p3=Pt{x:ox+lx,y:oy};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+lx-d2x,y:oy},Pt{x:ox+lx-d1x,y:oy},p3); current=p3;
        let p=Pt{x:ox+w-lx,y:oy}; add_line(&mut points,&mut count,p); current=p;
        let p3=Pt{x:ox+w-cx3,y:oy+cy6};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+w-lx+d1x,y:oy},Pt{x:ox+w-lx+d2x,y:oy},p3); current=p3;
        let p3=Pt{x:ox+w-cx6,y:oy+cy3};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+w-cx4,y:oy+cy5},Pt{x:ox+w-cx5,y:oy+cy4},p3); current=p3;
        let p3=Pt{x:ox+w,y:oy+ly};
        add_cubic(&mut points,&mut count,current,Pt{x:ox+w,y:oy+ly-d2y},Pt{x:ox+w,y:oy+ly-d1y},p3);
        let start_point = points[0];
        add_line(&mut points,&mut count,start_point);

        let clip = self.clip;
        let min_y = (top.max(clip.1.saturating_mul(scale))).max(0);
        let max_y = (bottom.min(clip.3.saturating_mul(scale))).min(self.h.saturating_mul(scale));
        let min_x = left.max(clip.0.saturating_mul(scale)).max(0);
        let max_x = right.min(clip.2.saturating_mul(scale)).min(self.w.saturating_mul(scale));
        let span = (max_x - min_x) as usize;
        if span == 0 || max_y <= min_y || span > 2048 { return; }
        let mut coverage = [0.0f32; 2048];
        let mut intersections = [0.0f32; 160];
        let mut py = min_y;
        while py < max_y {
            coverage[..span].fill(0.0);
            let mut sub = 0usize;
            while sub < 4 {
                let sy = py as f32 + (sub as f32 + 0.5) * 0.25;
                let mut n = 0usize;
                let mut i = 0usize;
                while i + 1 < count {
                    let a = points[i]; let b = points[i+1];
                    if (a.y <= sy && b.y > sy) || (b.y <= sy && a.y > sy) {
                        if n < intersections.len() { intersections[n] = a.x + (sy-a.y) * (b.x-a.x) / (b.y-a.y); n += 1; }
                    }
                    i += 1;
                }
                // Insertion sort; polygon has a small, fixed number of edges.
                let mut si=1usize;
                while si<n { let v=intersections[si]; let mut sj=si; while sj>0 && intersections[sj-1]>v { intersections[sj]=intersections[sj-1]; sj-=1; } intersections[sj]=v; si+=1; }
                let mut pair=0usize;
                while pair+1<n {
                    let a=intersections[pair].max(min_x as f32); let b=intersections[pair+1].min(max_x as f32);
                    if b>a {
                        // a/b are clipped to non-negative framebuffer coordinates, so
                        // truncation is equivalent to floor and ceil can be derived
                        // without libm (this crate is built with no_std).
                        let first=(a as i32).max(min_x);
                        let b_truncated=b as i32;
                        let last=(if b > b_truncated as f32 { b_truncated.saturating_add(1) } else { b_truncated }).min(max_x);
                        let mut px=first;
                        while px<last { let overlap=(b.min(px as f32+1.0)-a.max(px as f32)).max(0.0); coverage[(px-min_x) as usize]+=overlap; px+=1; }
                    }
                    pair+=2;
                }
                sub+=1;
            }
            let mut px=0usize;
            while px<span { let alpha=(coverage[px]*(255.0/4.0)+0.5).min(255.0) as u8; if alpha>0 { self.blend_device(min_x+px as i32,py,c,alpha); } px+=1; }
            py+=1;
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
