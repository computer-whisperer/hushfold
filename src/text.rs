//! Lettering set into the world: sign boards and shelf labels, and (through the book
//! table, `gfx::books`) the titles on books. Glyphs come from typefaces built into the
//! game (`Face`), kept together as one signed-distance atlas so letters stay crisp up
//! close and smooth far off. Laying text out needs only the map of where each glyph
//! sits in the atlas (cheap, and all the library needs); the atlas's pixels are drawn
//! once, for the renderer (`atlas`).

use ab_glyph::{Font, FontRef, GlyphId, OutlineCurve};
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use std::sync::OnceLock;

/// Atlas pixels to the em.
pub const EM_PX: f32 = 64.0;
/// How far, in atlas pixels, the distance field reaches either side of an outline
/// (`GLYPH_PAD` in the shader).
pub const PAD: f32 = 8.0;
const ATLAS_W: u32 = 2048;
/// Mip levels of the atlas: down to 8 pixels to the em, below which text has faded.
pub const LEVELS: u32 = 4;
/// The characters the atlas holds of every face; any other is set as '?'. The fonts
/// are subsets holding these (assets/fonts/subset.py, which has its own copy).
const CHARS: &str =
    " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~–—‘’“”…·";

/// A typeface lettering is set in, built into the game (assets/fonts/<Family>: each
/// font's licence and where it came from). A series' look chooses its spine's
/// (`catalog::Look`); signs and shelf labels are set in `SIGNS`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    /// EB Garamond: Garamond's roman, Paris, the 1540s.
    Garamond,
    /// Cardo: the Aldine roman, Venice, 1495.
    Cardo,
    /// IM Fell English: the Fell types, Oxford, the 1670s.
    Fell,
    /// Libre Caslon Text: Caslon, London, the 1730s (and the HUD's face).
    Caslon,
    /// Ibarra Real Nova: Ibarra's Don Quixote, Madrid, 1780.
    Ibarra,
    /// Castoro Titling: inscriptional capitals (and capitals only).
    Castoro,
    /// Baskervville: Baskerville, Birmingham, 1757.
    Baskerville,
    /// Bodoni Moda: Bodoni, Parma, the 1790s.
    Bodoni,
    /// Old Standard: the "modern" roman of Victorian books.
    OldStandard,
    /// Coustard: a Clarendon of Victorian cloth.
    Coustard,
    /// Goudy Bookletter 1911: Kennerley Old Style, New York, 1911.
    Kennerley,
    /// Sorts Mill Goudy: Goudy Oldstyle, New York, 1915.
    Goudy,
    /// Texturina: textura, the hand Gutenberg cut.
    Texturina,
    /// UnifrakturMaguntia: fraktur.
    Fraktur,
    /// Uncial Antiqua: uncial.
    Uncial,
    /// Cinzel: Roman inscriptional capitals, with small capitals for lower case.
    Cinzel,
}

/// The library's own lettering: signs and shelf labels.
pub const SIGNS: Face = Face::Cinzel;

/// Caslon's font, which the HUD sets its text in too.
pub static CASLON: &[u8] = include_bytes!("../assets/fonts/LibreCaslonText/LibreCaslonText-Subset.ttf");

impl Face {
    pub const ALL: [Face; 16] = [
        Face::Garamond,
        Face::Cardo,
        Face::Fell,
        Face::Caslon,
        Face::Ibarra,
        Face::Castoro,
        Face::Baskerville,
        Face::Bodoni,
        Face::OldStandard,
        Face::Coustard,
        Face::Kennerley,
        Face::Goudy,
        Face::Texturina,
        Face::Fraktur,
        Face::Uncial,
        Face::Cinzel,
    ];

    fn data(self) -> &'static [u8] {
        match self {
            Face::Garamond => include_bytes!("../assets/fonts/EBGaramond/EBGaramond-Subset.ttf"),
            Face::Cardo => include_bytes!("../assets/fonts/Cardo/Cardo-Subset.ttf"),
            Face::Fell => include_bytes!("../assets/fonts/IMFellEnglish/IMFeENrm28P.ttf"),
            Face::Caslon => CASLON,
            Face::Ibarra => include_bytes!("../assets/fonts/IbarraRealNova/IbarraRealNova-Subset.ttf"),
            Face::Castoro => include_bytes!("../assets/fonts/CastoroTitling/CastoroTitling-Subset.ttf"),
            Face::Baskerville => include_bytes!("../assets/fonts/Baskervville/Baskervville-Subset.ttf"),
            Face::Bodoni => include_bytes!("../assets/fonts/BodoniModa/BodoniModa-Subset.ttf"),
            Face::OldStandard => include_bytes!("../assets/fonts/OldStandardTT/OldStandard-Subset.ttf"),
            Face::Coustard => include_bytes!("../assets/fonts/Coustard/Coustard-Subset.ttf"),
            Face::Kennerley => include_bytes!("../assets/fonts/GoudyBookletter1911/GoudyBookletter1911-Subset.ttf"),
            Face::Goudy => include_bytes!("../assets/fonts/SortsMillGoudy/SortsMillGoudy-Subset.ttf"),
            Face::Texturina => include_bytes!("../assets/fonts/Texturina/Texturina-Subset.ttf"),
            Face::Fraktur => include_bytes!("../assets/fonts/UnifrakturMaguntia/UnifrakturMaguntia-Book.ttf"),
            Face::Uncial => include_bytes!("../assets/fonts/UncialAntiqua/UncialAntiqua-Regular.ttf"),
            Face::Cinzel => include_bytes!("../assets/fonts/Cinzel/Cinzel-Subset.ttf"),
        }
    }

    /// Whether a title may be set in its capitals: fraktur's run together.
    pub fn caps(self) -> bool {
        self != Face::Fraktur
    }
}

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

/// How a line is lettered (in `SIGNS`): its size in metres to the em, extra space
/// between letters in ems, and what the letters are made of, in which colour.
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

/// A face's font and measures.
struct Typeface {
    font: FontRef<'static>,
    upem: f32,
    /// Height of a capital, in ems.
    cap: f32,
    /// Kerning from each character to the next, by character (`kern[a * n + b]`), in ems.
    kern: Vec<f32>,
}

/// Every face's cells, face after face in `Face::ALL`'s order, each face's in `CHARS`'
/// order: a character's slot is its face's index times `CHARS`' length, plus its own.
struct Cells {
    faces: Vec<Typeface>,
    cells: Vec<Cell>,
    height: u32,
}

/// Characters in `CHARS`.
fn chars() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| CHARS.chars().count())
}

fn cells() -> &'static Cells {
    static CELLS: OnceLock<Cells> = OnceLock::new();
    CELLS.get_or_init(|| {
        let mut faces = Vec::new();
        let mut cells = Vec::new();
        for face in Face::ALL {
            let font = FontRef::try_from_slice(face.data()).expect("a built-in font");
            let upem = font.units_per_em().unwrap_or(1000.0);
            let bounds = |id: GlyphId| {
                let pts: Vec<Vec2> = outline(&font, id, upem).into_iter().flat_map(|(a, b)| [a, b]).collect();
                (!pts.is_empty()).then(|| pts.iter().fold((pts[0], pts[0]), |(lo, hi), &p| (lo.min(p), hi.max(p))))
            };
            let cap = bounds(font.glyph_id('H')).map_or(0.7, |(_, hi)| hi.y);
            let ids: Vec<GlyphId> = CHARS.chars().map(|c| font.glyph_id(c)).collect();
            for &id in &ids {
                let (min, size) = match bounds(id) {
                    Some((lo, hi)) => {
                        let lo = (lo * EM_PX).floor() - PAD;
                        let hi = (hi * EM_PX).ceil() + PAD;
                        ([lo.x as i32, lo.y as i32], [(hi.x - lo.x) as u32, (hi.y - lo.y) as u32])
                    }
                    None => ([0, 0], [0, 0]),
                };
                cells.push(Cell {
                    id,
                    advance: font.h_advance_unscaled(id) / upem,
                    min,
                    size,
                    atlas: [0, 0],
                });
            }
            let kern = (ids.iter())
                .flat_map(|&a| ids.iter().map(move |&b| (a, b)))
                .map(|(a, b)| font.kern_unscaled(a, b) / upem)
                .collect();
            faces.push(Typeface { font, upem, cap, kern });
        }
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
        Cells {
            faces,
            cells,
            // Even down to the smallest mip level.
            height: (y + row).next_multiple_of(1 << (LEVELS - 1)),
        }
    })
}

/// A character's place in `CHARS` ('?''s if it has none).
fn index(c: char) -> usize {
    // CHARS opens with printable ASCII in order.
    if (' '..='~').contains(&c) {
        return c as usize - ' ' as usize;
    }
    CHARS.chars().position(|k| k == c).unwrap_or_else(|| index('?'))
}

/// How far a curve may stray from the segments it is cut into, in ems: a twentieth of
/// an atlas pixel.
const FLAT: f32 = 0.05 / EM_PX;

/// A glyph's outline as line segments, in ems (y up). Curves are cut as finely as
/// their bend needs to keep within `FLAT`, into at most 8 pieces (quadratics) or 12
/// (cubics): rough faces' outlines are made of many small curves, nearly straight.
fn outline(font: &FontRef, id: GlyphId, upem: f32) -> Vec<(Vec2, Vec2)> {
    let Some(o) = font.outline(id) else { return Vec::new() };
    let p = |q: ab_glyph::Point| Vec2::new(q.x, q.y) / upem;
    // A quadratic whose control points' second difference is `bend` strays from n even
    // chords by at most bend / (4 n²); a cubic by three times that, of the larger of
    // its two.
    let pieces = |bend: f32, max: usize| ((bend / (4.0 * FLAT)).sqrt().ceil() as usize).clamp(1, max);
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
            OutlineCurve::Quad(a, b, c) => {
                let (a, b, c) = (p(a), p(b), p(c));
                curve(pieces((a - 2.0 * b + c).length(), 8), &|t| {
                    a.lerp(b, t).lerp(b.lerp(c, t), t)
                })
            }
            OutlineCurve::Cubic(a, b, c, d) => {
                let (a, b, c, d) = (p(a), p(b), p(c), p(d));
                let bend = (a - 2.0 * b + c).length().max((b - 2.0 * c + d).length());
                curve(pieces(3.0 * bend, 12), &|t| {
                    let (ab, bc, cd) = (a.lerp(b, t), b.lerp(c, t), c.lerp(d, t));
                    ab.lerp(bc, t).lerp(bc.lerp(cd, t), t)
                })
            }
        }
    }
    segs
}

/// Lay `text` out on one line in `face`: each character's slot in the atlas with where
/// its pen stands, in ems from the start, and the line's width in ems.
pub fn run(face: Face, text: &str, tracking: f32) -> (Vec<(usize, f32)>, f32) {
    let c = cells();
    let f = face as usize;
    let tf = &c.faces[f];
    let n = chars();
    let mut pen = 0.0;
    let mut prev = None;
    let mut placed = Vec::new();
    for ch in text.chars() {
        let i = index(ch);
        if let Some(p) = prev {
            pen += tf.kern[p * n + i];
        }
        let slot = f * n + i;
        placed.push((slot, pen));
        pen += c.cells[slot].advance + tracking;
        prev = Some(i);
    }
    let w = pen - if placed.is_empty() { 0.0 } else { tracking };
    (placed, w)
}

/// Height of a capital in `face`, in ems.
pub fn cap_height(face: Face) -> f32 {
    cells().faces[face as usize].cap
}

/// Whether a slot's glyph shows anything (a space does not).
pub fn inked(slot: usize) -> bool {
    cells().cells[slot].size[0] > 0
}

/// Where each slot's glyph sits in the atlas, for shaders that cut glyphs out
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

/// Set `text` in `SIGNS` on one line centred on `centre` (the middle of its capitals),
/// reading along `right` with `up` up (both unit), shrunk to fit `width` if it is
/// wider; its glyphs go to `out`.
pub fn set_line(out: &mut Vec<Glyph>, text: &str, style: Style, centre: Vec3, right: Vec3, up: Vec3, width: f32) {
    let c = cells();
    let (placed, w) = run(SIGNS, text, style.tracking);
    let em = style.em.min(width / w.max(1e-6));
    let px = em / EM_PX;
    let origin = centre - right * (w * em * 0.5) - up * (cap_height(SIGNS) * em * 0.5);
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
    // Dealt round, so each thread gets some of every face (some outlines are far busier).
    let fields: Vec<(usize, Vec<u8>)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let jobs = &jobs;
                s.spawn(move || {
                    (jobs.iter().skip(t).step_by(threads))
                        .map(|&i| (i, field(c, i)))
                        .collect::<Vec<_>>()
                })
            })
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

/// Slot `slot`'s cell of the distance field, rows top down.
fn field(c: &Cells, slot: usize) -> Vec<u8> {
    let cell = &c.cells[slot];
    let tf = &c.faces[slot / chars()];
    let segs = outline(&tf.font, cell.id, tf.upem);
    let [sx, sy] = cell.size;
    // An outline further off than this leaves a texel saturated, so each segment is
    // measured only from the texels within this of it. A texel's winding counts the
    // crossings of its row to its right.
    let reach = (PAD + 1.0) / EM_PX;
    let x = |col: u32| (cell.min[0] as f32 + col as f32 + 0.5) / EM_PX;
    let col = |x: f32| (x * EM_PX - cell.min[0] as f32 - 0.5).clamp(0.0, sx as f32 - 1.0);
    let mut d2 = vec![f32::MAX; sx as usize];
    let mut crossings = Vec::new();
    let mut out = Vec::with_capacity((sx * sy) as usize);
    for row in 0..sy {
        let y = (cell.min[1] as f32 + (sy - 1 - row) as f32 + 0.5) / EM_PX;
        d2.fill(f32::MAX);
        crossings.clear();
        for &(a, b) in &segs {
            let ab = b - a;
            if (a.y <= y) != (b.y <= y) {
                crossings.push((a.x + (y - a.y) / ab.y * ab.x, if b.y > a.y { 1 } else { -1 }));
            }
            if a.y.min(b.y) - reach > y || y > a.y.max(b.y) + reach {
                continue;
            }
            let (lo, hi) = (col(a.x.min(b.x) - reach).floor(), col(a.x.max(b.x) + reach).ceil());
            for k in lo as u32..=hi as u32 {
                let p = Vec2::new(x(k), y);
                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
                d2[k as usize] = d2[k as usize].min((a + ab * t).distance_squared(p));
            }
        }
        for k in 0..sx {
            let winding: i32 = (crossings.iter()).filter(|&&(cx, _)| cx > x(k)).map(|&(_, w)| w).sum();
            let d = d2[k as usize].sqrt() * EM_PX * if winding != 0 { 1.0 } else { -1.0 };
            out.push((128.0 + d * 127.0 / PAD).round().clamp(0.0, 255.0) as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every face has a glyph for every character, and its measures are sane.
    #[test]
    fn every_face_has_every_glyph() {
        assert!((' '..='~').zip(CHARS.chars()).all(|(a, b)| a == b));
        assert_eq!(index('–'), 95);
        let c = cells();
        let n = chars();
        for face in Face::ALL {
            let f = face as usize;
            for (ch, cell) in CHARS.chars().zip(&c.cells[f * n..(f + 1) * n]) {
                assert!(cell.id.0 != 0, "{face:?} has no {ch:?} (rerun assets/fonts/subset.py)");
                assert!(cell.advance > 0.0, "{face:?} {ch:?} has no advance");
                assert_eq!(cell.size[0] == 0, ch == ' ', "{face:?} {ch:?} cell {:?}", cell.size);
                assert!(
                    cell.size[0] <= 255 && cell.size[1] <= 255,
                    "{face:?} {ch:?} cell {:?}",
                    cell.size
                );
                assert!(
                    cell.min.iter().all(|m| (-128..128).contains(m)),
                    "{face:?} {ch:?} at {:?}",
                    cell.min
                );
            }
            // Uncial's capitals stand tallest, nine tenths of its em.
            assert!(
                (0.55..0.95).contains(&cap_height(face)),
                "{face:?} cap height {}",
                cap_height(face)
            );
        }
        // The book table's glyph words keep 14 bits of slot.
        assert!(c.cells.len() < 1 << 14);
        assert!(c.height <= 4096, "atlas {}x{}", ATLAS_W, c.height);
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
        for face in Face::ALL {
            let slot = face as usize * chars() + index('o');
            let f = field(c, slot);
            let [sx, sy] = c.cells[slot].size;
            let at = |x: u32, y: u32| f[(y * sx + x) as usize];
            // The middle row, from the left edge in.
            let row: Vec<u8> = (0..sx).map(|x| at(x, sy / 2)).collect();
            assert!(row[0] < 20 && at(sx / 2, sy / 2) < 128, "{face:?} {row:?}");
            assert!(row.iter().any(|&v| v > 150), "{face:?} {row:?}");
        }
    }

    /// Kerning pairs are read: capitals that nest close up in faces that kern them.
    #[test]
    fn faces_kern() {
        for face in [Face::Garamond, Face::Caslon, Face::Fell, Face::Cinzel] {
            let apart = run(face, "A", 0.0).1 + run(face, "V", 0.0).1;
            assert!(run(face, "AV", 0.0).1 < apart - 0.02, "{face:?}");
        }
    }
}
