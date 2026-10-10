//! What the book shader (`fs_book`) draws a book from besides its instance: a table
//! of words made once per library, holding each series' binding and where its
//! lettering goes, and a tiling texture of binding materials.
//!
//! The table: a header; the glyph atlas's cells (`text::cell_words`); volume numbers
//! set roman and arabic; a record of `RECORD` words per series; and the glyphs every
//! block of lettering points into. A block is one word, `first | count << 22 | lines
//! << 29`; each of its glyphs a word, `slot | pen << 14 | line << 30`, the pen in
//! 1/1024 ems from the middle of its line biased by 32 ems, so a block's glyphs sort
//! by line, then pen (the shader finds a glyph by bisection).

use crate::catalog::{Binding, Look, Series};
use crate::text;

/// Words before the cells: where the cells, numerals, records and glyphs start, and
/// the height of a capital in ems (as f32 bits).
const HEADER: usize = 8;
/// Words per series: the look; label colour; accent colour; spine lettering's block
/// and its (em, centre); the label's (half length, centre); the front cover's block
/// and its (em, centre); the volume number's (em, centre). Centres are metres down
/// from the head; pairs are two f16 in one word.
const RECORD: usize = 10;
/// Volume numbers set in advance (higher ones show as the last).
const NUMERALS: u32 = 64;

/// The spine's raised bands run from this far below the head to as far above the tail
/// (the shader's `BAND_END`).
const BAND_END: f32 = 0.045;
/// Lines of capitals, and of mixed case, apart in ems (`PITCH_CAPS`, `PITCH`).
const PITCH_CAPS: f32 = 1.2;
const PITCH: f32 = 1.25;

/// How the spine is lettered (look bits 9..10).
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Down the spine.
    Along = 1,
    /// Across it, in capitals on a label.
    Across = 2,
}

/// What the title sits on (look bits 11..12).
#[derive(Clone, Copy, PartialEq)]
enum Label {
    None = 0,
    Leather = 1,
    Paper = 2,
    /// A strip of the label's leather down the middle of the spine.
    Inlay = 3,
}

/// A block of lettering being set: its glyph words go to `glyphs`, and it returns the
/// block's word.
fn block(glyphs: &mut Vec<u32>, lines: &[String], tracking: f32) -> u32 {
    let first = glyphs.len() as u32;
    for (l, line) in lines.iter().enumerate() {
        let (placed, w) = text::run(line, tracking);
        for (slot, pen) in placed.into_iter().filter(|&(s, _)| text::inked(s)) {
            let pen = ((pen - w * 0.5) * 1024.0 + 32768.0).round().clamp(0.0, 65535.0) as u32;
            glyphs.push(slot as u32 | pen << 14 | (l as u32) << 30);
        }
    }
    let count = (glyphs.len() as u32 - first).min(127);
    first | count << 22 | (lines.len() as u32) << 29
}

fn pair(a: f32, b: f32) -> u32 {
    let h = |v: f32| f16_bits(v) as u32;
    h(a) | h(b) << 16
}

/// An f32 as IEEE half-precision bits (for values well inside its range).
fn f16_bits(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = (b >> 13) & 0x3ff;
    if v == 0.0 || exp <= 0 {
        return sign;
    }
    sign | ((exp.min(30) as u16) << 10) | man as u16
}

fn rgb8(c: [f32; 3]) -> u32 {
    let q = |v: f32| (v.max(0.0).sqrt() * 255.0).round().min(255.0) as u32;
    q(c[0]) | q(c[1]) << 8 | q(c[2]) << 16
}

fn roman(mut n: u32) -> String {
    let mut s = String::new();
    for (v, r) in [
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

/// The widths of `words` set with `tracking`, and of a space, in ems.
fn widths(words: &[&str], tracking: f32) -> (Vec<f32>, f32) {
    let space = text::run(" ", tracking).1 + tracking;
    (words.iter().map(|w| text::run(w, tracking).1).collect(), space)
}

/// `words` broken into `n` lines (each a run of words) as evenly as they go: the
/// lines, and the widest one's width in ems.
fn break_lines(words: &[&str], w: &[f32], space: f32, n: usize) -> Option<(Vec<String>, f32)> {
    if n == 0 || n > words.len() {
        return None;
    }
    let width = |a: usize, b: usize| w[a..b].iter().sum::<f32>() + space * (b - a - 1) as f32;
    let mut best: Option<(Vec<usize>, f32)> = None;
    // Every way to cut the words into n runs (titles are a handful of words).
    let mut cuts: Vec<usize> = (1..n).collect();
    loop {
        let bounds: Vec<usize> = std::iter::once(0)
            .chain(cuts.iter().copied())
            .chain([words.len()])
            .collect();
        let widest = bounds.windows(2).map(|p| width(p[0], p[1])).fold(0.0, f32::max);
        if best.as_ref().is_none_or(|b| widest < b.1) {
            best = Some((bounds, widest));
        }
        // Next combination of cuts.
        let mut k = n - 1;
        loop {
            if k == 0 {
                let (bounds, widest) = best?;
                let lines = bounds.windows(2).map(|p| words[p[0]..p[1]].join(" ")).collect();
                return Some((lines, widest));
            }
            k -= 1;
            if cuts[k] < words.len() - (n - 1 - k) {
                cuts[k] += 1;
                for j in k + 1..n - 1 {
                    cuts[j] = cuts[j - 1] + 1;
                }
                break;
            }
        }
    }
}

/// The largest em at which `text` fits a box `w` wide and `h` high in up to `max_lines`
/// lines `pitch` apart: (lines, em).
fn fit(text: &str, tracking: f32, pitch: f32, w: f32, h: f32, max_lines: usize) -> (Vec<String>, f32) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let (ws, space) = widths(&words, tracking);
    let cap = text::cap_height();
    let mut best = (vec![text.to_string()], 0.0);
    for n in 1..=max_lines {
        if let Some((lines, widest)) = break_lines(&words, &ws, space, n) {
            let em = (w / widest.max(1e-3)).min(h / (cap + 0.35 + (n - 1) as f32 * pitch));
            // A line more only for a clearly bigger letter.
            if em > best.1 * 1.12 {
                best = (lines, em);
            }
        }
    }
    best
}

/// Where a series' spine lettering goes: its block's word, (em, centre), the label's
/// (half length, centre), the volume number's (em, centre), and the look bits saying how.
struct Spine {
    block: u32,
    text: u32,
    label: u32,
    numeral: u32,
    bits: u32,
}

fn spine(glyphs: &mut Vec<u32>, s: &Series) -> Spine {
    let look = &s.look;
    let h = s.height;
    let t = s.thickness.iter().copied().fold(f32::MAX, f32::min);
    let numbered = s.volumes > 1;
    let caps = s.title.to_uppercase();
    // The volume number: in the panel under the label's on banded spines, else above
    // the tail's tooling.
    let panel = |k: f32| {
        let step = (h - 2.0 * BAND_END) / (look.bands.max(2) - 1) as f32;
        (BAND_END + step * k, step)
    };
    let (num_at, num_room) = if look.bands > 0 {
        let (a, step) = panel(1.5);
        (a, step * 0.5)
    } else {
        (h - 0.05, 0.016)
    };
    let numeral = pair((0.36 * t).min(num_room).min(0.009), num_at);
    // Labels across a spine too thin for them would carry letters too small to read.
    let across_room = t >= 0.03;
    let along = |glyphs: &mut Vec<u32>, from: f32, to: f32, width: f32| {
        let (lines, em) = fit(&s.title, 0.02, PITCH, to - from - 0.012, width, 2);
        (block(glyphs, &lines, 0.02), pair(em.min(0.016), (from + to) * 0.5))
    };
    let across = |glyphs: &mut Vec<u32>, half: f32, at: f32| {
        let (lines, em) = fit(&caps, 0.03, PITCH_CAPS, t - 0.0075, 2.0 * half - 0.004, 3);
        (block(glyphs, &lines, 0.03), pair(em.min(0.011), at))
    };
    let bottom = if numbered { h - 0.064 } else { h - 0.045 };
    let (mode, label, (block, text), label_geom) = if look.bands > 0 {
        let (a, step) = panel(0.5);
        let half = step * 0.5 - 0.0035;
        (Mode::Across, Label::Leather, across(glyphs, half, a), pair(half, a))
    } else if s.pattern == 2 && across_room {
        // Pattern 2's dark panel near the head is a label.
        (
            Mode::Across,
            Label::Leather,
            across(glyphs, 0.035, 0.07),
            pair(0.035, 0.07),
        )
    } else if s.pattern == 7 {
        // A strip of leather down the spine, lettered along.
        let (from, to) = (0.03, h - if numbered { 0.07 } else { 0.03 });
        let lettered = along(glyphs, from, to, 0.5 * t);
        (
            Mode::Along,
            Label::Inlay,
            lettered,
            pair((to - from) * 0.5, (from + to) * 0.5),
        )
    } else if look.binding == Binding::Boards && across_room {
        (
            Mode::Across,
            Label::Paper,
            across(glyphs, 0.03, 0.075),
            pair(0.03, 0.075),
        )
    } else {
        let from = match s.pattern {
            2 => 0.11,
            4 => 0.092,
            _ => 0.045,
        };
        let lettered = along(glyphs, from, bottom, 0.66 * t);
        if look.binding == Binding::Boards {
            let (from, to) = (from - 0.006, bottom + 0.006);
            (
                Mode::Along,
                Label::Paper,
                lettered,
                pair((to - from) * 0.5, (from + to) * 0.5),
            )
        } else {
            (Mode::Along, Label::None, lettered, 0)
        }
    };
    let roman = matches!(look.binding, Binding::Leather | Binding::Half | Binding::Vellum);
    let numerals = match (numbered, roman) {
        (false, _) => 0,
        (true, true) => 1,
        (true, false) => 2,
    };
    Spine {
        block,
        text,
        label: label_geom,
        numeral,
        bits: (mode as u32) << 9 | (label as u32) << 11 | numerals << 13,
    }
}

fn look_bits(look: &Look) -> u32 {
    look.binding as u32 | look.bands.min(7) << 3 | (look.edges as u32) << 6 | ((look.age * 255.0) as u32) << 24
}

/// The book table for a library's series (see the module's notes).
pub fn table(series: &[Series]) -> Vec<u32> {
    let cells = text::cell_words();
    let mut glyphs = Vec::new();
    let numerals: Vec<u32> = (0..NUMERALS)
        .flat_map(|n| {
            let r = block(&mut glyphs, &[roman(n)], 0.04);
            let a = block(&mut glyphs, &[n.to_string()], 0.0);
            [r, a]
        })
        .collect();
    let mut records = Vec::with_capacity(series.len() * RECORD);
    for s in series {
        let sp = spine(&mut glyphs, s);
        let d = s.depth;
        let (cover, cover_at) = if matches!(s.look.binding, Binding::Cloth | Binding::Boards) {
            let (lines, em) = fit(&s.title.to_uppercase(), 0.06, PITCH_CAPS, 0.6 * d, 0.3 * s.height, 3);
            (block(&mut glyphs, &lines, 0.06), pair(em.min(0.012), 0.3 * s.height))
        } else {
            (0, 0)
        };
        records.extend_from_slice(&[
            look_bits(&s.look) | sp.bits,
            rgb8(s.look.label),
            rgb8(s.look.accent),
            sp.block,
            sp.text,
            sp.label,
            cover,
            cover_at,
            sp.numeral,
            0,
        ]);
    }
    let at_cells = HEADER;
    let at_numerals = at_cells + cells.len();
    let at_series = at_numerals + numerals.len();
    let at_glyphs = at_series + records.len();
    let mut out = vec![
        at_cells as u32,
        at_numerals as u32,
        at_series as u32,
        at_glyphs as u32,
        text::cap_height().to_bits(),
        0,
        0,
        0,
    ];
    out.extend(cells);
    out.extend(numerals);
    out.extend(records);
    out.extend(glyphs);
    out
}

/// Binding materials, tiling every 64 mm (`BOOK_TILE`): r a leather's pebbled grain and
/// creases, g a cloth's plain weave, b marbling's veins (a place in a palette), a a
/// mottle for paper, vellum and wear. The texture's size and its mip levels (RGBA8).
pub fn material() -> (u32, Vec<Vec<u8>>) {
    const N: u32 = 512;
    let rows: Vec<u32> = (0..N).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let full: Vec<u8> = std::thread::scope(|s| {
        let handles: Vec<_> = (rows.chunks(rows.len().div_ceil(threads)))
            .map(|chunk| {
                s.spawn(move || {
                    chunk
                        .iter()
                        .flat_map(|&y| (0..N).flat_map(move |x| texel(x, y, N)))
                        .collect::<Vec<u8>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("book texels"))
            .collect()
    });
    let mut levels = vec![full];
    let mut w = N;
    while w > 1 {
        let prev = levels.last().expect("a level");
        let half = w / 2;
        let mut next = vec![0u8; (half * half * 4) as usize];
        for y in 0..half {
            for x in 0..half {
                for c in 0..4 {
                    let at = |dx: u32, dy: u32| prev[(((2 * y + dy) * w + 2 * x + dx) * 4 + c) as usize] as u32;
                    next[((y * half + x) * 4 + c) as usize] =
                        ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        levels.push(next);
        w = half;
    }
    (N, levels)
}

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h =
        (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0xff_ffff) as f32 / 16_777_216.0
}

/// Value noise with period `p` cells, at a point in cells.
fn vnoise(x: f32, y: f32, p: i32, seed: u32) -> f32 {
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - ix as f32, y - iy as f32);
    let (ux, uy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let at = |dx: i32, dy: i32| hash((ix + dx).rem_euclid(p), (iy + dy).rem_euclid(p), seed);
    let a = at(0, 0) + (at(1, 0) - at(0, 0)) * ux;
    let b = at(0, 1) + (at(1, 1) - at(0, 1)) * ux;
    a + (b - a) * uy
}

/// Octaves of `vnoise` from period `p` cells over the tile (u, v in 0..1), halving.
fn fbm(u: f32, v: f32, p: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut p) = (0.0, 1.0, 0.0, p);
    for o in 0..octaves {
        sum += amp * vnoise(u * p as f32, v * p as f32, p, seed + o);
        norm += amp;
        amp *= 0.5;
        p *= 2;
    }
    sum / norm
}

/// The nearest and second nearest of one jittered point per cell, period `p` cells, and
/// a number for the nearest point's cell.
fn worley(u: f32, v: f32, p: i32, seed: u32) -> (f32, f32, f32) {
    let (x, y) = (u * p as f32, v * p as f32);
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2, mut id) = (9.0f32, 9.0f32, 0.0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (cx, cy) = (ix + dx, iy + dy);
            let (wx, wy) = (cx.rem_euclid(p), cy.rem_euclid(p));
            let px = cx as f32 + 0.15 + 0.7 * hash(wx, wy, seed);
            let py = cy as f32 + 0.15 + 0.7 * hash(wx, wy, seed + 1);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < f1 {
                (f2, f1, id) = (f1, d, hash(wx, wy, seed + 2));
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, id)
}

fn texel(x: u32, y: u32, n: u32) -> [u8; 4] {
    let (u, v) = ((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32);
    let q = |f: f32| (f.clamp(0.0, 1.0) * 255.0).round() as u8;
    // Leather: pebbles about a millimetre across, darker between them, and longer
    // creases where the skin flexed.
    let (f1, f2, _) = worley(u, v, 56, 11);
    let (c1, c2, _) = worley(u + 0.13 * fbm(u, v, 4, 2, 5), v, 9, 23);
    let pebble = 0.55 + 0.45 * (1.0 - f1 * f1) - 0.5 * (1.0 - smooth(0.0, 0.16, f2 - f1));
    let crease = 1.0 - smooth(0.0, 0.05, c2 - c1);
    let leather = pebble * (1.0 - 0.35 * crease) + 0.12 * (vnoise(u * 128.0, v * 128.0, 128, 7) - 0.5);
    // Cloth: a plain weave of threads half a millimetre apart, each its own shade and
    // a little thicker or thinner along its length. Where a thread crosses over, its
    // length runs along x (a weft) or y (a warp).
    let (tx, ty) = (x / 4, y / 4);
    let (fx, fy) = ((x % 4) as f32 + 0.5, (y % 4) as f32 + 0.5);
    let (along, cross, across, thread) = if (tx + ty) % 2 == 0 {
        (x as f32, fx, fy, ty)
    } else {
        (y as f32, fy, fx, tx + 1000)
    };
    let slub = (0.8 + 0.2 * hash(thread as i32, 0, 9)) * (0.7 + 0.3 * vnoise(along / 32.0, 0.0, 16, 3 + thread));
    let profile =
        (std::f32::consts::PI * across / 4.0).sin() * (0.8 + 0.2 * (std::f32::consts::PI * cross / 4.0).sin());
    let cloth = 0.2 + 0.8 * profile * slub;
    // Marbling, Stormont-like: clouds of colour (a place in a palette) under a lace of
    // fine veins (1), warped as the colours drifted on the size.
    let wu = u + 0.08 * fbm(u, v, 3, 3, 41) + 0.03 * (fbm(u, v, 16, 3, 45) - 0.5);
    let wv = v + 0.12 * fbm(u, v + 0.5, 3, 3, 43) + 0.03 * (fbm(u, v, 16, 3, 46) - 0.5);
    let (m1, m2, cell) = worley(wu, wv, 36, 47);
    let vein = (1.0 - ((m2 - m1) / 0.07).min(1.0)) * ((fbm(u, v, 5, 3, 61) - 0.38) * 6.0).clamp(0.0, 1.0);
    let clouds = ((fbm(wu, wv, 4, 4, 51) - 0.25) * 1.6).clamp(0.0, 1.0);
    let marble = (0.85 * clouds + 0.08 * cell).min(1.0) * 0.84 * (1.0 - vein) + vein;
    // Mottle at many scales.
    let mottle = fbm(u, v, 4, 6, 71);
    [q(leather), q(cloth), q(marble), q(mottle)]
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_round_trip() {
        for v in [0.0, 0.0125, 0.07, 0.3, 1.0] {
            let b = f16_bits(v);
            let exp = ((b >> 10) & 31) as i32 - 15;
            let back = if b == 0 {
                0.0
            } else {
                (1.0 + (b & 0x3ff) as f32 / 1024.0) * 2f32.powi(exp)
            };
            assert!((back - v).abs() <= v * 1e-3, "{v} -> {back}");
        }
    }

    #[test]
    fn lines_break_evenly() {
        let words = ["The", "Lantern-lit", "Almanac", "of", "Tides"];
        let (w, space) = widths(&words, 0.0);
        let (lines, widest) = break_lines(&words, &w, space, 2).expect("two lines");
        assert_eq!(lines, ["The Lantern-lit", "Almanac of Tides"]);
        assert!(widest < w.iter().sum::<f32>());
        assert!(break_lines(&words, &w, space, 6).is_none());
        assert_eq!(roman(14), "XIV");
    }

    /// Glyphs of a block sort by line, then pen, as the shader's bisection needs.
    #[test]
    fn blocks_sort() {
        let mut glyphs = Vec::new();
        let b = block(&mut glyphs, &["SALT-WORN".into(), "TIDES & LIGHT".into()], 0.06);
        let (first, count) = (b & 0x3f_ffff, b >> 22 & 127);
        assert_eq!((first, count, b >> 29), (0, 20, 2));
        assert!(glyphs.windows(2).all(|g| g[0] >> 14 < g[1] >> 14));
    }
}
