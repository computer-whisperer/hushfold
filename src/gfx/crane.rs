//! Folded-paper birds: small triangle meshes (built in, see `mesh`), one per part, drawn
//! with the scene pipeline as instances of `kind::CRANE` (the familiars, and the Verse
//! Gallery's hanging cranes).
//!
//! Every part is modelled in one frame (x right, y up, forward -z; metres at a posted
//! familiar's size, wings spread level), so the parts of a bird share its sheet's
//! coordinates and a print runs on from one to the next. A wing turns about the
//! body's ridge, its outer half about the wrist. Paper is two-sided: each facet is
//! drawn facing out on both sides with its own flat normal, so every crease shows as
//! a change in the light.

use super::mesh::{self, Mesh};
use super::{Inst, Vertex, kind};
use glam::{Quat, Vec3};

/// The birds there are folded as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fold {
    /// The classic crane: neck and tail raised, a bent head, broad wings.
    Crane,
    /// Folded for speed, like a swift: a pointed head held forward, long wings swept
    /// back, a forked tail. Smaller and slimmer than a crane.
    Swift,
}

impl Fold {
    const ALL: [Fold; 2] = [Fold::Crane, Fold::Swift];

    /// How far its keel hangs below its centre (at size 1).
    pub fn keel(self) -> f32 {
        -self.shape().keel.y
    }

    fn shape(self) -> &'static Shape {
        match self {
            Fold::Crane => &CRANE,
            Fold::Swift => &SWIFT,
        }
    }
}

/// What a sheet carries on it (see `kind::CRANE`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Print {
    Plain = 0,
    /// A page of writing.
    Text = 1,
    Map = 2,
    StarChart = 3,
    /// Staves of music.
    Music = 4,
    /// Patterned papers, light on a dark ground.
    Dots = 5,
    Waves = 6,
    Stripes = 7,
    Checks = 8,
}

/// The sheet a bird is folded from.
#[derive(Clone, Copy, Debug)]
pub struct Paper {
    pub tint: [f32; 3],
    pub print: Print,
}

/// A bird to draw.
#[derive(Clone, Copy, Debug)]
pub struct Crane {
    pub fold: Fold,
    /// Where its centre is and how it is turned (forward -z), and its size.
    pub at: Vec3,
    pub rot: Quat,
    pub size: f32,
    /// How far its wings are raised about their roots, and their outer halves about
    /// the wrists, in radians (down negative).
    pub raise: f32,
    pub wrist: f32,
    pub paper: Paper,
    /// The light inside it: colour, and strength.
    pub glow: [f32; 4],
}

/// Parts of a bird: its body, then its wings' inner halves (left, right) and outer
/// halves (left, right).
const PARTS: usize = 5;

/// Meshes of every fold's parts, in `Fold::ALL`'s order.
pub const MESHES: usize = PARTS * Fold::ALL.len();

impl Crane {
    /// Its parts, as instances of `kind::CRANE`.
    pub fn parts(&self) -> [Inst; PARTS] {
        let shape = self.fold.shape();
        let k = self.size;
        let [r, g, b] = self.paper.tint.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u32);
        let first = mesh::CRANE + self.fold as u32 * PARTS as u32;
        let param = (self.paper.print as u32) << 4 | (r | g << 8 | b << 16) << 8;
        let inst = |at: Vec3, rot: Quat, part: usize| {
            Inst::new(at, rot, Vec3::splat(k), kind::CRANE, self.glow, param).shaped(first + part as u32)
        };
        let at = |off: Vec3| self.at + self.rot * (off * k);
        let root = Vec3::Y * shape.ridge;
        let mut parts = [inst(self.at, self.rot, 0); PARTS];
        for (side, s) in [-1.0f32, 1.0].into_iter().enumerate() {
            let mirror = Vec3::new(-s, 1.0, 1.0);
            let (w0, w1) = (shape.wrist.0 * mirror, shape.wrist.1 * mirror);
            // Raised about the ridge, and the outer half about the wrist line.
            let inner = Quat::from_rotation_z(s * self.raise);
            let outer = inner * Quat::from_axis_angle((w1 - w0).normalize(), s * self.wrist);
            parts[1 + side] = inst(at(root - inner * root), self.rot * inner, 1 + side);
            parts[3 + side] = inst(at(root + inner * (w0 - root) - outer * w0), self.rot * outer, 3 + side);
        }
        parts
    }
}

/// A fold's shape: how it is built (the left side, mirrored for the right), and the
/// points it is posed by.
struct Shape {
    /// Height of the ridge the wings are hinged along.
    ridge: f32,
    keel: Vec3,
    /// The left wing's wrist line, front to back.
    wrist: (Vec3, Vec3),
    build: fn(&mut Builder),
}

const fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

const CRANE: Shape = Shape {
    ridge: 0.02,
    keel: v(0.0, -0.075, 0.0),
    wrist: (v(-0.162, 0.011, -0.008), v(-0.162, 0.011, 0.058)),
    build: crane,
};

const SWIFT: Shape = Shape {
    ridge: 0.015,
    keel: v(0.0, -0.05, 0.01),
    wrist: (v(-0.14, 0.014, -0.05), v(-0.135, 0.014, 0.012)),
    build: swift,
};

/// The crane: a body puffed out between neck and tail, the neck rising to a head
/// bent forward, the tail raised, and long triangular wings creased down the middle.
fn crane(b: &mut Builder) {
    let s = &CRANE;
    let k = s.keel;
    let (rf, rb) = (v(0.0, s.ridge, -0.06), v(0.0, s.ridge, 0.06));
    let flank = v(-0.034, -0.012, 0.0);
    // Neck and tail: two layers each, joined along the lower edge up from the keel,
    // open along the upper, where they spread a little apart at the body.
    let (neck, tail) = (v(-0.011, 0.012, -0.064), v(-0.011, 0.012, 0.064));
    let (nape, throat) = (v(-0.004, 0.172, -0.163), v(0.0, 0.155, -0.18));
    let (beak, tip) = (v(0.0, 0.132, -0.25), v(0.0, 0.165, 0.215));
    b.part(|b| {
        b.both(|b| {
            b.fan(flank, &[k, neck, rf, rb, tail, k]);
            b.quad(k, throat, nape, neck);
            b.tri(throat, beak, nape);
            b.tri(k, tip, tail);
        });
    });
    let mid = v(0.0, s.ridge, 0.0);
    let (lw, tw) = s.wrist;
    let t = v(-0.36, 0.0, 0.055);
    // The crease runs from the root out to the tip, standing a little proud in each
    // half (on the hinge at the wrist, so the halves stay joined as the wing bends).
    let cw = lw.lerp(tw, 0.5);
    let (inner, outer) = (
        mid.lerp(cw, 0.5) + v(0.0, 0.008, 0.0),
        cw.lerp(t, 0.4) + v(0.0, 0.006, 0.0),
    );
    b.wing(|b| b.fan(inner, &[mid, rf, lw, cw, tw, rb, mid]));
    b.wing(|b| b.fan(outer, &[lw, t, tw, cw, lw]));
}

/// The swift: a slim body, a pointed head held forward, wings long and swept back to
/// a sickle, and a tail forked into two streamers.
fn swift(b: &mut Builder) {
    let s = &SWIFT;
    let k = s.keel;
    let (rf, rb) = (v(0.0, s.ridge, -0.04), v(0.0, s.ridge, 0.03));
    let (front, back) = (v(0.0, -0.012, -0.058), v(0.0, -0.004, 0.048));
    let flank = v(-0.024, -0.008, -0.004);
    let (cheek, beak) = (v(-0.012, 0.0, -0.05), v(0.0, 0.01, -0.19));
    let (root, out, fork) = (v(0.0, 0.002, 0.042), v(-0.018, 0.006, 0.038), v(-0.058, 0.024, 0.235));
    b.part(|b| {
        b.both(|b| {
            b.fan(flank, &[front, rf, rb, back, k, front]);
            b.tri(rf, cheek, beak);
            b.tri(cheek, front, beak);
            b.tri(root, out, fork);
        });
    });
    let mid = v(0.0, s.ridge, -0.005);
    let (lw, tw) = s.wrist;
    let (lead, t) = (v(-0.235, 0.008, 0.02), v(-0.3, 0.0, 0.1));
    // Creased as the crane's wings are.
    let cw = lw.lerp(tw, 0.45);
    let (inner, outer) = (
        mid.lerp(cw, 0.5) + v(0.0, 0.005, 0.0),
        cw.lerp(t, 0.4) + v(0.0, 0.004, 0.0),
    );
    b.wing(|b| b.fan(inner, &[mid, rf, lw, cw, tw, rb, mid]));
    b.wing(|b| b.fan(outer, &[lw, lead, t, tw, cw, lw]));
}

/// Every fold's parts' meshes (`MESHES` of them).
pub(super) fn meshes() -> Vec<Mesh> {
    let mut b = Builder::default();
    for fold in Fold::ALL {
        (fold.shape().build)(&mut b);
        assert_eq!(b.parts.len(), (fold as usize + 1) * PARTS, "{fold:?}");
    }
    b.parts
}

/// The parts being built, of two-sided facets.
#[derive(Default)]
struct Builder {
    parts: Vec<Mesh>,
    /// The part being built.
    part: Mesh,
    /// Building the right side: points are mirrored across the middle.
    right: bool,
}

impl Builder {
    /// A facet, drawn from both sides.
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let m = |p: Vec3| if self.right { Vec3::new(-p.x, p.y, p.z) } else { p };
        let (a, b, c) = (m(a), m(b), m(c));
        let n = (b - a).cross(c - a).normalize_or_zero();
        let part = &mut self.part;
        for (corners, n) in [([a, b, c], n), ([a, c, b], -n)] {
            for p in corners {
                part.idx.push(part.verts.len() as u32);
                part.verts.push(Vertex {
                    pos: p.into(),
                    normal: n.into(),
                });
            }
        }
    }

    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }

    /// Facets from `centre` to each edge along `rim`.
    fn fan(&mut self, centre: Vec3, rim: &[Vec3]) {
        for e in rim.windows(2) {
            self.tri(centre, e[0], e[1]);
        }
    }

    /// The left side's facets as `f` makes them, and their mirror image.
    fn both(&mut self, f: impl Fn(&mut Self)) {
        f(self);
        self.right = true;
        f(self);
        self.right = false;
    }

    /// A part made by `f`.
    fn part(&mut self, f: impl FnOnce(&mut Self)) {
        f(self);
        let part = std::mem::take(&mut self.part);
        self.parts.push(part);
    }

    /// A pair of parts: the left wing's as `f` makes it, then the right's.
    fn wing(&mut self, f: impl Fn(&mut Self)) {
        self.part(&f);
        self.right = true;
        self.part(&f);
        self.right = false;
    }
}
