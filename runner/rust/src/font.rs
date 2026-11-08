//! Portable TrueType text rasterizer.
//!
//! Glyphs are rasterized by the vendored no_std stb_truetype implementation
//! and alpha-composited into the same software framebuffer on every target.
//! No AppKit, CoreText, DirectWrite, or system font APIs are used.

use crate::canvas::TextStyle;
use core::mem::MaybeUninit;
use stb_truetype_rust::*;

const MAX_GLYPH_DIM: usize = 64;
const GLYPH_PIXELS: usize = MAX_GLYPH_DIM * MAX_GLYPH_DIM;
const CACHE_CAPACITY: usize = 224;
const GLYPH_ARENA_SIZE: usize = 64 * 1024;

// Visible fallback for ASCII characters not covered by the compact bitmap font.
const BOX: [u8; 7] = [
    0b11111,
    0b10001,
    0b10001,
    0b10001,
    0b10001,
    0b10001,
    0b11111,
];

static FONT_BYTES: &[u8] = include_bytes!("../assets/HarmonyOS_Sans_SC_Regular.ttf");

#[derive(Clone, Copy)]
struct GlyphEntry {
    codepoint: u32,
    pixel_size: u16,
    glyph_id: i32,
    width: u8,
    height: u8,
    advance: i16,
    x_offset: i16,
    y_offset: i16,
    valid: bool,
    bitmap_offset: u32,
}

const EMPTY_GLYPH: GlyphEntry = GlyphEntry {
    codepoint: 0,
    pixel_size: 0,
    glyph_id: 0,
    width: 0,
    height: 0,
    advance: 0,
    x_offset: 0,
    y_offset: 0,
    valid: false,
    bitmap_offset: 0,
};

static mut FONT_INFO: MaybeUninit<stbtt_fontinfo> = MaybeUninit::uninit();
static mut FONT_READY: bool = false;
static mut ASCENT_UNITS: i32 = 0;
static mut GLYPH_CACHE: [GlyphEntry; CACHE_CAPACITY] = [EMPTY_GLYPH; CACHE_CAPACITY];
static mut CACHE_CURSOR: usize = 0;
static mut GLYPH_SCRATCH: [u8; GLYPH_PIXELS] = [0; GLYPH_PIXELS];
static mut GLYPH_ARENA: [u8; GLYPH_ARENA_SIZE] = [0; GLYPH_ARENA_SIZE];
static mut GLYPH_ARENA_CURSOR: usize = 0;

fn style_size(style: TextStyle) -> u16 {
    match style {
        TextStyle::Title => 20,
        TextStyle::SectionTitle => 24,
        TextStyle::SectionDescription => 13,
        TextStyle::Heading => 15,
        TextStyle::Body => 16,
        TextStyle::ListItem => 14,
        TextStyle::Small => 13,
        TextStyle::Button => 18,
    }
}


pub fn init() -> bool {
    unsafe {
        core::ptr::addr_of_mut!(FONT_READY).write(false);
        let info = core::ptr::addr_of_mut!(FONT_INFO).cast::<stbtt_fontinfo>();
        info.write(stbtt_fontinfo::default());
        if stbtt_InitFont(info, FONT_BYTES.as_ptr(), 0) == 0 {
            return false;
        }

        let mut ascent = 0;
        let mut descent = 0;
        let mut line_gap = 0;
        stbtt_GetFontVMetrics(
            info,
            &mut ascent as *mut i32,
            &mut descent as *mut i32,
            &mut line_gap as *mut i32,
        );
        core::ptr::addr_of_mut!(ASCENT_UNITS).write(ascent);
        GLYPH_CACHE = [EMPTY_GLYPH; CACHE_CAPACITY];
        CACHE_CURSOR = 0;
        GLYPH_ARENA_CURSOR = 0;
        core::ptr::addr_of_mut!(FONT_READY).write(true);
        true
    }
}



unsafe fn cached_glyph(ch: char, pixel_size: u16, info: *mut stbtt_fontinfo) -> Option<usize> {
    let codepoint = ch as u32;

    for index in 0..CACHE_CAPACITY {
        let entry = &GLYPH_CACHE[index];
        if entry.valid && entry.codepoint == codepoint && entry.pixel_size == pixel_size {
            return Some(index);
        }
    }

    let glyph_id = stbtt_FindGlyphIndex(info, codepoint as i32);
    let scale = stbtt_ScaleForPixelHeight(info, pixel_size as f32);

    let mut advance_units = 0;
    let mut left_side_bearing = 0;
    stbtt_GetGlyphHMetrics(info, glyph_id, &mut advance_units, &mut left_side_bearing);

    let advance = (advance_units as f32 * scale + 0.5) as i32;
    let mut x0 = 0;
    let mut y0 = 0;
    let mut x1 = 0;
    let mut y1 = 0;
    stbtt_GetGlyphBitmapBox(
        info, glyph_id, scale, scale, &mut x0, &mut y0, &mut x1, &mut y1,
    );

    let width = (x1 - x0).max(0) as usize;
    let height = (y1 - y0).max(0) as usize;
    if width > MAX_GLYPH_DIM || height > MAX_GLYPH_DIM {
        return None;
    }

    let scratch = &mut GLYPH_SCRATCH;
    scratch.fill(0);
    if width > 0 && height > 0 && glyph_id != 0 {
        stbtt_MakeGlyphBitmap(
            info,
            scratch.as_mut_ptr(),
            width as i32,
            height as i32,
            width as i32,
            scale,
            scale,
            glyph_id,
        );
    }

    let bitmap_len = width * height;
    if bitmap_len > GLYPH_ARENA_SIZE {
        return None;
    }
    if GLYPH_ARENA_CURSOR.checked_add(bitmap_len).map_or(true, |end| end > GLYPH_ARENA_SIZE) {
        // The arena is append-only for predictable, allocation-free cache writes.
        // When full, invalidate all cache entries before reusing its storage.
        GLYPH_CACHE = [EMPTY_GLYPH; CACHE_CAPACITY];
        CACHE_CURSOR = 0;
        GLYPH_ARENA_CURSOR = 0;
    }

    let bitmap_offset = GLYPH_ARENA_CURSOR;
    if bitmap_len > 0 {
        for row in 0..height {
            let src = row * width;
            let dst = bitmap_offset + row * width;
            GLYPH_ARENA[dst..dst + width].copy_from_slice(&scratch[src..src + width]);
        }
    }
    GLYPH_ARENA_CURSOR = bitmap_offset + bitmap_len;

    let slot = CACHE_CURSOR % CACHE_CAPACITY;
    CACHE_CURSOR = (CACHE_CURSOR + 1) % CACHE_CAPACITY;
    let entry = &mut GLYPH_CACHE[slot];
    entry.valid = false;
    entry.codepoint = codepoint;
    entry.pixel_size = pixel_size;
    entry.glyph_id = glyph_id;
    entry.width = width as u8;
    entry.height = height as u8;
    entry.advance = advance.clamp(0, i16::MAX as i32) as i16;
    entry.x_offset = x0.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    entry.y_offset = y0.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    entry.bitmap_offset = bitmap_offset as u32;
    entry.valid = true;
    Some(slot)
}

fn fallback_measure(text: &[u8], scale: usize, style: TextStyle) -> i32 {
    let scale = scale.max(1) as i32;
    let glyph_scale = match style {
        TextStyle::Title | TextStyle::SectionTitle | TextStyle::Heading => 2,
        _ => 1,
    };
    let advance = 6 * glyph_scale * scale;
    let mut width = 0i32;
    let mut index = 0usize;
    while index < text.len() {
        let byte = text[index];
        if byte < 0x80 {
            width = width.saturating_add(advance);
            index += 1;
        } else {
            width = width.saturating_add(12 * glyph_scale * scale);
            index += crate::canvas::utf8_len(byte).min(text.len() - index);
        }
    }
    (width + scale - 1) / scale
}

pub fn measure(text: &[u8], style: TextStyle, device_scale: usize) -> i32 {
    if text.is_empty() { return 0; }
    let scale = device_scale.max(1);
    let pixel_size = style_size(style).saturating_mul(scale.min(u16::MAX as usize) as u16).max(1);

    unsafe {
        if !core::ptr::addr_of!(FONT_READY).read() {
            return fallback_measure(text, scale, style);
        }

        let Ok(value) = core::str::from_utf8(text) else {
            return fallback_measure(text, scale, style);
        };

        let mut pen = 0i32;
        let mut previous = 0i32;
        let font = core::ptr::addr_of_mut!(FONT_INFO).cast::<stbtt_fontinfo>();
        let factor = stbtt_ScaleForPixelHeight(font, pixel_size as f32);

        for ch in value.chars() {
            let Some(index) = cached_glyph(ch, pixel_size, font) else { continue; };
            let entry = &GLYPH_CACHE[index];
            if previous != 0 && entry.glyph_id != 0 {
                let kern = stbtt_GetGlyphKernAdvance(font, previous, entry.glyph_id);
                pen = pen.saturating_add((kern as f32 * factor + 0.5) as i32);
            }
            pen = pen.saturating_add(entry.advance as i32);
            previous = entry.glyph_id;
        }
        (pen + scale as i32 - 1) / scale as i32
    }
}

/// Draw text with device-pixel glyph coverage. `plot` receives physical
/// framebuffer coordinates and 8-bit coverage; callers perform clipping/blend.
pub fn draw(
    x: i32,
    y: i32,
    text: &[u8],
    style: TextStyle,
    device_scale: usize,
    mut plot: impl FnMut(i32, i32, u8),
) -> i32 {
    if text.is_empty() { return 0; }
    let scale = device_scale.max(1);
    let pixel_size = style_size(style).saturating_mul(scale.min(u16::MAX as usize) as u16).max(1);

    unsafe {
        if !core::ptr::addr_of!(FONT_READY).read() {
            return draw_fallback(x, y, text, style, scale, &mut plot);
        }
        let Ok(value) = core::str::from_utf8(text) else {
            return draw_fallback(x, y, text, style, scale, &mut plot);
        };

        let font = core::ptr::addr_of_mut!(FONT_INFO).cast::<stbtt_fontinfo>();
        let factor = stbtt_ScaleForPixelHeight(font, pixel_size as f32);
        let ascent_px = (core::ptr::addr_of!(ASCENT_UNITS).read() as f32 * factor + 0.5) as i32;
        let baseline = y.saturating_mul(scale as i32).saturating_add(ascent_px);
        let mut pen_x = x.saturating_mul(scale as i32);
        let mut previous = 0i32;

        for ch in value.chars() {
            let Some(index) = cached_glyph(ch, pixel_size, font) else { continue; };
            let entry = &GLYPH_CACHE[index];
            if previous != 0 && entry.glyph_id != 0 {
                let kern = stbtt_GetGlyphKernAdvance(font, previous, entry.glyph_id);
                pen_x = pen_x.saturating_add((kern as f32 * factor + 0.5) as i32);
            }

            let gx = pen_x.saturating_add(entry.x_offset as i32);
            let gy = baseline.saturating_add(entry.y_offset as i32);
            let width = entry.width as usize;
            let height = entry.height as usize;
            for row in 0..height {
                for col in 0..width {
                    let coverage = GLYPH_ARENA[entry.bitmap_offset as usize + row * width + col];
                    if coverage != 0 {
                        plot(gx + col as i32, gy + row as i32, coverage);
                    }
                }
            }
            pen_x = pen_x.saturating_add(entry.advance as i32);
            previous = entry.glyph_id;
        }

        let used = pen_x.saturating_sub(x.saturating_mul(scale as i32));
        (used + scale as i32 - 1) / scale as i32
    }
}

fn draw_fallback(
    x: i32,
    y: i32,
    text: &[u8],
    style: TextStyle,
    device_scale: usize,
    plot: &mut impl FnMut(i32, i32, u8),
) -> i32 {
    let scale = device_scale.max(1) as i32;
    let glyph_scale = match style {
        TextStyle::Title | TextStyle::SectionTitle | TextStyle::Heading => 2,
        _ => 1,
    };
    let mut pen_x = x * scale;
    let top = y * scale;
    let mut index = 0usize;
    while index < text.len() {
        let byte = text[index];
        if byte < 0x80 {
            let rows = crate::font::glyph(byte);
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0x10 >> col) != 0 {
                        for sy in 0..glyph_scale * scale {
                            for sx in 0..glyph_scale * scale {
                                plot(
                                    pen_x + col as i32 * glyph_scale * scale + sx,
                                    top + row as i32 * glyph_scale * scale + sy,
                                    255,
                                );
                            }
                        }
                    }
                }
            }
            pen_x += 6 * glyph_scale * scale;
            index += 1;
        } else {
            let w = 6 * glyph_scale * scale;
            let h = 7 * glyph_scale * scale;
            for dx in 0..w {
                plot(pen_x + dx, top, 255);
                plot(pen_x + dx, top + h - 1, 255);
            }
            for dy in 0..h {
                plot(pen_x, top + dy, 255);
                plot(pen_x + w - 1, top + dy, 255);
            }
            pen_x += 12 * glyph_scale * scale;
            index += crate::canvas::utf8_len(byte).min(text.len() - index);
        }
    }
    (pen_x - x * scale + scale - 1) / scale
}

pub fn glyph(c: u8) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        b' ' => [0; 7],

        b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        b'3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        b'5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        b'6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        b'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],

        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        b'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        b'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        b'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        b'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        b'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],

        b'!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        b'"' => [0b01010, 0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000],
        b'#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        b'%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        b'\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        b'(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        b')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        b'*' => [0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000],
        b'+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        b',' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00110, 0b00100, 0b01000],
        b'-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        b'.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        b'/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        b':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        b';' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000],
        b'<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        b'=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        b'>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        b'?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        b'@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10110, 0b10000, 0b01111],
        b'[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        b']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        b'_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],

        _ => BOX,
    }
}
