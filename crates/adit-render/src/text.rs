//! Text clips (SPEC V-3): CPU rasterisation with ab_glyph onto a transparent image.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont, point};
use adit_core::{TextAlign, TextSpec};
use std::collections::HashMap;
use std::path::PathBuf;

/// Loaded fonts by (family, bold, italic); `None` = nothing found, so it is not searched again.
#[derive(Default)]
pub(crate) struct Fonts(HashMap<(String, bool, bool), Option<FontVec>>);

fn font_dirs() -> Vec<PathBuf> {
    let mut d = Vec::new();
    if let Some(w) = std::env::var_os("WINDIR") {
        d.push(PathBuf::from(w).join("Fonts"));
    }
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        d.push(PathBuf::from(l).join("Microsoft/Windows/Fonts"));
    }
    for p in [
        "/usr/share/fonts/truetype/dejavu",
        "/usr/share/fonts/TTF",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
    ] {
        d.push(PathBuf::from(p));
    }
    d
}

// shortcut: a family is found by file name (`<family>.ttf`/`.otf`, as given or lowercased without
// spaces) and bold/italic only switch files for the default fonts; use a font database
// (fontdb) if real family/style matching is needed.
fn candidates(family: &str, bold: bool, italic: bool) -> Vec<String> {
    let mut c = Vec::new();
    if !family.is_empty() {
        let squashed = family.to_lowercase().replace(' ', "");
        for n in [family.to_string(), squashed] {
            c.push(format!("{n}.ttf"));
            c.push(format!("{n}.otf"));
        }
    }
    let (arial, dejavu, mac) = match (bold, italic) {
        (false, false) => ("arial.ttf", "DejaVuSans.ttf", "Arial.ttf"),
        (true, false) => ("arialbd.ttf", "DejaVuSans-Bold.ttf", "Arial Bold.ttf"),
        (false, true) => ("ariali.ttf", "DejaVuSans-Oblique.ttf", "Arial Italic.ttf"),
        (true, true) => (
            "arialbi.ttf",
            "DejaVuSans-BoldOblique.ttf",
            "Arial Bold Italic.ttf",
        ),
    };
    c.extend(
        [
            arial,
            dejavu,
            mac,
            "arial.ttf",
            "DejaVuSans.ttf",
            "Arial.ttf",
        ]
        .map(String::from),
    );
    c
}

impl Fonts {
    fn get(&mut self, family: &str, bold: bool, italic: bool) -> Option<&FontVec> {
        self.0
            .entry((family.to_string(), bold, italic))
            .or_insert_with(|| {
                let dirs = font_dirs();
                candidates(family, bold, italic).iter().find_map(|name| {
                    dirs.iter().find_map(|d| {
                        let bytes = std::fs::read(d.join(name)).ok()?;
                        FontVec::try_from_vec(bytes).ok()
                    })
                })
            })
            .as_ref()
    }

    /// `w`x`h` straight-alpha RGBA: the text in `t.color`, its layout box centred.
    /// No usable font → fully transparent (the frame still renders).
    pub fn render(&mut self, t: &TextSpec, w: u32, h: u32) -> Vec<u8> {
        let mut img = vec![0u8; w as usize * h as usize * 4];
        let Some(font) = self.get(&t.font, t.bold, t.italic) else {
            return img;
        };
        if !(t.size.is_finite() && t.size > 0.0) {
            return img;
        }
        // `size` is the em size; ab_glyph's PxScale is ascent-to-descent height.
        let em = font.units_per_em().unwrap_or(1000.0);
        let scale = PxScale::from((t.size as f32) * font.height_unscaled() / em);
        let f = font.as_scaled(scale);
        let line_h = f.height() + f.line_gap();
        let lines: Vec<(Vec<(ab_glyph::GlyphId, f32)>, f32)> = t
            .content
            .split('\n')
            .map(|line| {
                let mut x = 0.0;
                let mut prev = None;
                let mut glyphs = Vec::new();
                for ch in line.chars() {
                    let id = f.glyph_id(ch);
                    if let Some(p) = prev {
                        x += f.kern(p, id);
                    }
                    glyphs.push((id, x));
                    x += f.h_advance(id);
                    prev = Some(id);
                }
                (glyphs, x)
            })
            .collect();
        let block_w = lines.iter().map(|l| l.1).fold(0.0, f32::max);
        let block_h = line_h * (lines.len() as f32 - 1.0) + f.height();
        let left = (w as f32 - block_w) / 2.0;
        let top = (h as f32 - block_h) / 2.0;
        let color = t.color.map(|c| c.clamp(0.0, 1.0));
        let rgb = [0, 1, 2].map(|i| (color[i] * 255.0).round() as u8);
        for (i, (glyphs, lw)) in lines.iter().enumerate() {
            let dx = match t.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => (block_w - lw) / 2.0,
                TextAlign::Right => block_w - lw,
            };
            let baseline = top + f.ascent() + line_h * i as f32;
            for &(id, x) in glyphs {
                let g = id.with_scale_and_position(scale, point(left + dx + x, baseline));
                let Some(o) = font.outline_glyph(g) else {
                    continue;
                };
                let b = o.px_bounds();
                o.draw(|gx, gy, cov| {
                    let (x, y) = (b.min.x as i64 + gx as i64, b.min.y as i64 + gy as i64);
                    if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                        return;
                    }
                    let k = ((y as usize) * w as usize + x as usize) * 4;
                    let a = (cov.clamp(0.0, 1.0) * color[3] * 255.0).round() as u8;
                    // Overlapping glyph edges keep the stronger coverage.
                    if a > img[k + 3] {
                        img[k..k + 3].copy_from_slice(&rgb);
                        img[k + 3] = a;
                    }
                });
            }
        }
        img
    }
}
