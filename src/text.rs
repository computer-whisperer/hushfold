//! Lettering set into the world: sign boards and shelf labels. Glyphs come from the
//! serif face the HUD uses, kept as a signed-distance atlas so letters stay crisp up
//! close and smooth far off. Laying text out needs only the map of where each glyph
//! sits in the atlas (cheap, and all the library needs); the atlas's pixels are drawn
//! once, for the renderer (`atlas`).

use ab_glyph::{Font, FontArc, GlyphId, OutlineCurve};
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use std::sync::OnceLock;

/// Atlas pixels to the em.
pub const EM_PX: f32 = 64.0;
/// How far, in atlas pixels, the distance field reaches either side of an outline
/// (`GLYPH_PAD` in the shader).
pub const PAD: f32 = 8.0;
const ATLAS_W: u32 = 1024;
/// Mip levels of the atlas: down to 8 pixels to the em, below which text has faded.
pub const LEVELS: u32 = 4;
/// The characters the atlas holds; any other is set as '?'.
const CHARS: &str =
    " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~–—‘’“”…·";

/// What letters are made of: gold leaf (the colour is the gold's), ink (the ink's),
/// or light (the light's, w its strength).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lettering {
    Gilt = 0,
    Ink = 1,
    Glow = 2,
}

/// A glyph as the renderer draws it: a quad from corner `at` (bottom left as read)
/// along `right` and `up` (its full sides), showing the atlas rectangle `uv` (u0, v0
/// at its top left, then u1, v1), lettered as `lettering` (a `Lettering`) in `color`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Glyph {
    pub at: [f32; 3],
    pub lettering: u32,
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub uv: [f32; 4],
    pub color: [f32; 4],
}

/// How a line is lettered: its size in metres to the em, extra space between letters
/// in ems, and what the letters are made of, in which colour.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub em: f32,
    pub tracking: f32,
    pub lettering: Lettering,
    pub color: [f32; 4],
}

/// Where a character's glyph sits: its advance in ems; and, if it shows anything, its
/// cell in the atlas, `size` pixels from `atlas` (top left), whose bottom left lies
/// `min` pixels from the pen on the baseline (y up).
struct Cell {
    id: GlyphId,
    advance: f32,
    min: [i32; 2],
    size: [u32; 2],
    atlas: [u32; 2],
}

struct Cells {
    font: FontArc,
    upem: f32,
    /// Height of a capital, in ems.
    cap: f32,
    cells: Vec<Cell>,
    /// Kerning from each character to the next, by slot (`kern[a * n + b]`), in ems.
    kern: Vec<f32>,
    height: u32,
}

/// The serif face, for lettering and the HUD alike: Noto Serif, built in so that
/// both read the same everywhere (SIL Open Font License, `assets/fonts/OFL.txt`).
pub const SERIF: &[u8] = include_bytes!("../assets/fonts/NotoSerif-Regular.ttf");

fn font() -> FontArc {
    FontArc::try_from_slice(SERIF).expect("the built-in serif font")
}

fn cells() -> &'static Cells {
    static CELLS: OnceLock<Cells> = OnceLock::new();
    CELLS.get_or_init(|| {
        let font = font();
        let upem = font.units_per_em().unwrap_or(1000.0);
        let bounds = |id: GlyphId| {
            let pts: Vec<Vec2> = outline(&font, id, upem).into_iter().flat_map(|(a, b)| [a, b]).collect();
            (!pts.is_empty()).then(|| pts.iter().fold((pts[0], pts[0]), |(lo, hi), &p| (lo.min(p), hi.max(p))))
        };
        let cap = bounds(font.glyph_id('H')).map_or(0.7, |(_, hi)| hi.y);
        let mut cells: Vec<Cell> = (CHARS.chars())
            .map(|c| {
                let id = font.glyph_id(c);
                let advance = font.h_advance_unscaled(id) / upem;
                let (min, size) = match bounds(id) {
                    Some((lo, hi)) => {
                        let lo = (lo * EM_PX).floor() - PAD;
                        let hi = (hi * EM_PX).ceil() + PAD;
                        ([lo.x as i32, lo.y as i32], [(hi.x - lo.x) as u32, (hi.y - lo.y) as u32])
                    }
                    None => ([0, 0], [0, 0]),
                };
                Cell {
                    id,
                    advance,
                    min,
                    size,
                    atlas: [0, 0],
                }
            })
            .collect();
        // Shelf-pack, tallest first, a pixel apart.
        let mut order: Vec<usize> = (0..cells.len()).filter(|&i| cells[i].size[0] > 0).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(cells[i].size[1]));
        let (mut x, mut y, mut row) = (0, 0, 0);
        for i in order {
            let [w, h] = cells[i].size;
            if x + w > ATLAS_W {
                (x, y, row) = (0, y + row + 1, 0);
            }
            cells[i].atlas = [x, y];
            x += w + 1;
            row = row.max(h);
        }
        let kern = (cells.iter())
            .flat_map(|a| cells.iter().map(|b| font.kern_unscaled(a.id, b.id) / upem))
            .collect();
        Cells {
            font,
            upem,
            cap,
            cells,
            kern,
            height: (y + row).next_power_of_two(),
        }
    })
}

fn slot(c: char) -> usize {
    // CHARS opens with printable ASCII in order.
    if (' '..='~').contains(&c) {
        return c as usize - ' ' as usize;
    }
    CHARS.chars().position(|k| k == c).unwrap_or_else(|| slot('?'))
}

/// A glyph's outline as line segments, in ems (y up).
fn outline(font: &FontArc, id: GlyphId, upem: f32) -> Vec<(Vec2, Vec2)> {
    let Some(o) = font.outline(id) else { return Vec::new() };
    let p = |q: ab_glyph::Point| Vec2::new(q.x, q.y) / upem;
    let mut segs = Vec::new();
    let mut curve = |n: usize, f: &dyn Fn(f32) -> Vec2| {
        let mut a = f(0.0);
        for k in 1..=n {
            let b = f(k as f32 / n as f32);
            segs.push((a, b));
            a = b;
        }
    };
    for c in &o.curves {
        match *c {
            OutlineCurve::Line(a, b) => curve(1, &|t| p(a).lerp(p(b), t)),
            OutlineCurve::Quad(a, b, c) => curve(8, &|t| {
                let (a, b, c) = (p(a), p(b), p(c));
                a.lerp(b, t).lerp(b.lerp(c, t), t)
            }),
            OutlineCurve::Cubic(a, b, c, d) => curve(12, &|t| {
                let (a, b, c, d) = (p(a), p(b), p(c), p(d));
                let (ab, bc, cd) = (a.lerp(b, t), b.lerp(c, t), c.lerp(d, t));
                ab.lerp(bc, t).lerp(bc.lerp(cd, t), t)
            }),
        }
    }
    segs
}

/// Lay `text` out on one line: each character's slot in the atlas with where its pen
/// stands, in ems from the start, and the line's width in ems.
pub fn run(text: &str, tracking: f32) -> (Vec<(usize, f32)>, f32) {
    let c = cells();
    let n = c.cells.len();
    let mut pen = 0.0;
    let mut prev = None;
    let mut placed = Vec::new();
    for ch in text.chars() {
        let i = slot(ch);
        if let Some(p) = prev {
            pen += c.kern[p * n + i];
        }
        placed.push((i, pen));
        pen += c.cells[i].advance + tracking;
        prev = Some(i);
    }
    let w = pen - if placed.is_empty() { 0.0 } else { tracking };
    (placed, w)
}

/// Height of a capital, in ems.
pub fn cap_height() -> f32 {
    cells().cap
}

/// Whether a character's glyph shows anything (a space does not).
pub fn inked(slot: usize) -> bool {
    cells().cells[slot].size[0] > 0
}

/// Where each character's glyph sits in the atlas, for shaders that cut glyphs out
/// themselves: two words per slot, its top left (x | y << 16) and its size and offset
/// from the pen on the baseline (w | h << 8 | (min x + 128) << 16 | (min y + 128) << 24),
/// in atlas pixels.
pub fn cell_words() -> Vec<u32> {
    let bias = |v: i32| (v + 128).clamp(0, 255) as u32;
    (cells().cells.iter())
        .flat_map(|cell| {
            let [w, h] = cell.size;
            [
                cell.atlas[0] | cell.atlas[1] << 16,
                w.min(255) | h.min(255) << 8 | bias(cell.min[0]) << 16 | bias(cell.min[1]) << 24,
            ]
        })
        .collect()
}

/// Set `text` on one line centred on `centre` (the middle of its capitals), reading
/// along `right` with `up` up (both unit), shrunk to fit `width` if it is wider; its
/// glyphs go to `out`.
pub fn set_line(out: &mut Vec<Glyph>, text: &str, style: Style, centre: Vec3, right: Vec3, up: Vec3, width: f32) {
    let c = cells();
    let (placed, w) = run(text, style.tracking);
    let em = style.em.min(width / w.max(1e-6));
    let px = em / EM_PX;
    let origin = centre - right * (w * em * 0.5) - up * (c.cap * em * 0.5);
    let (aw, ah) = (ATLAS_W as f32, c.height as f32);
    for (i, x) in placed {
        let cell = &c.cells[i];
        let [sx, sy] = cell.size;
        if sx == 0 {
            continue;
        }
        let at = origin + right * (x * em + cell.min[0] as f32 * px) + up * (cell.min[1] as f32 * px);
        let [u, v] = cell.atlas;
        out.push(Glyph {
            at: at.into(),
            lettering: style.lettering as u32,
            right: (right * (sx as f32 * px)).into(),
            up: (up * (sy as f32 * px)).into(),
            uv: [u as f32 / aw, v as f32 / ah, (u + sx) as f32 / aw, (v + sy) as f32 / ah],
            color: style.color,
        });
    }
}

/// The atlas: its size and, for each mip level from the full one down, rows of R8
/// texels top down, each the signed distance to the nearest outline (128 on it,
/// higher inside, `PAD` pixels from it at 0 or 255).
pub fn atlas() -> (u32, u32, Vec<Vec<u8>>) {
    let c = cells();
    let (w, h) = (ATLAS_W, c.height);
    let mut full = vec![0u8; (w * h) as usize];
    let jobs: Vec<usize> = (0..c.cells.len()).filter(|&i| c.cells[i].size[0] > 0).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let fields: Vec<(usize, Vec<u8>)> = std::thread::scope(|s| {
        let handles: Vec<_> = (jobs.chunks(jobs.len().div_ceil(threads).max(1)))
            .map(|chunk| s.spawn(move || chunk.iter().map(|&i| (i, field(c, &c.cells[i]))).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("glyph field"))
            .collect()
    });
    for (i, f) in fields {
        let cell = &c.cells[i];
        let [sx, sy] = cell.size;
        for row in 0..sy {
            let at = ((cell.atlas[1] + row) * w + cell.atlas[0]) as usize;
            full[at..at + sx as usize].copy_from_slice(&f[(row * sx) as usize..((row + 1) * sx) as usize]);
        }
    }
    let mut levels = vec![full];
    for l in 1..LEVELS {
        let (pw, ph) = (w >> (l - 1), h >> (l - 1));
        let prev = &levels[l as usize - 1];
        let mut next = vec![0u8; ((pw / 2) * (ph / 2)) as usize];
        for y in 0..ph / 2 {
            for x in 0..pw / 2 {
                let at = |dx: u32, dy: u32| prev[((2 * y + dy) * pw + 2 * x + dx) as usize] as u32;
                next[(y * (pw / 2) + x) as usize] = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
            }
        }
        levels.push(next);
    }
    (w, h, levels)
}

/// One glyph's cell of the distance field, rows top down.
fn field(c: &Cells, cell: &Cell) -> Vec<u8> {
    let segs = outline(&c.font, cell.id, c.upem);
    let [sx, sy] = cell.size;
    let mut out = Vec::with_capacity((sx * sy) as usize);
    for row in 0..sy {
        for col in 0..sx {
            let p = Vec2::new(
                cell.min[0] as f32 + col as f32 + 0.5,
                cell.min[1] as f32 + (sy - 1 - row) as f32 + 0.5,
            ) / EM_PX;
            let mut d2 = f32::MAX;
            let mut winding = 0;
            for &(a, b) in &segs {
                let ab = b - a;
                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
                d2 = d2.min((a + ab * t).distance_squared(p));
                if (a.y <= p.y) != (b.y <= p.y) {
                    let x = a.x + (p.y - a.y) / ab.y * ab.x;
                    if x > p.x {
                        winding += if b.y > a.y { 1 } else { -1 };
                    }
                }
            }
            let d = d2.sqrt() * EM_PX * if winding != 0 { 1.0 } else { -1.0 };
            out.push((128.0 + d * 127.0 / PAD).round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_character_has_a_glyph() {
        assert!((' '..='~').zip(CHARS.chars()).all(|(a, b)| a == b));
        assert_eq!(slot('–'), 95);
        let c = cells();
        for (ch, cell) in CHARS.chars().zip(&c.cells) {
            assert!(cell.advance > 0.0, "{ch:?} has no advance");
            assert_eq!(cell.size[0] == 0, ch == ' ', "{ch:?} cell {:?}", cell.size);
        }
        assert!((0.6..0.8).contains(&c.cap), "cap height {}", c.cap);
        assert!(c.height <= 1024, "atlas {}x{}", ATLAS_W, c.height);
    }

    #[test]
    fn lines_centre_and_fit() {
        let style = Style {
            em: 0.2,
            tracking: 0.05,
            lettering: Lettering::Gilt,
            color: [1.0; 4],
        };
        let extent = |g: &[Glyph]| {
            g.iter().fold((f32::MAX, f32::MIN), |(lo, hi), g| {
                (lo.min(g.at[0]), hi.max(g.at[0] + g.right[0]))
            })
        };
        let mut wide = Vec::new();
        set_line(
            &mut wide,
            "The Alchemical Wing",
            style,
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            10.0,
        );
        assert_eq!(wide.len(), 17);
        let (lo, hi) = extent(&wide);
        assert!((lo + hi).abs() < 0.05 && hi - lo > 2.0, "{lo}..{hi}");
        let mut narrow = Vec::new();
        set_line(
            &mut narrow,
            "The Alchemical Wing",
            style,
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            1.0,
        );
        let (lo, hi) = extent(&narrow);
        // Within the width but for the cells' padding.
        assert!(hi - lo < 1.0 + 4.0 * PAD / EM_PX * 0.2 && hi - lo > 0.9, "{lo}..{hi}");
    }

    /// Inside an 'o''s ring is ink; in its middle and round it, none.
    #[test]
    fn fields_are_signed() {
        let c = cells();
        let cell = &c.cells[slot('o')];
        let f = field(c, cell);
        let [sx, sy] = cell.size;
        let at = |x: u32, y: u32| f[(y * sx + x) as usize];
        // The middle row, from the left edge in.
        let row: Vec<u8> = (0..sx).map(|x| at(x, sy / 2)).collect();
        assert!(row[0] < 20 && at(sx / 2, sy / 2) < 128, "{row:?}");
        assert!(row.iter().any(|&v| v > 150), "{row:?}");
    }
}
