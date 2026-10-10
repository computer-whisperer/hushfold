//! Meshes drawn in place of the unit box: an instance's `mesh` picks one (0, the box,
//! by default). Built in are the box and the paper birds' parts (`crane`); a library
//! makes its own (`Library::shape`): turned (`lathe`), extruded (`prism`, `ring`,
//! `gear`), swept along a path (`sweep`), or merged from those (`add`).
//!
//! A library's meshes are fitted to the unit box (`fitted`), so an instance's scale is
//! its size, as a box's is, and materials read its points in the box's coordinates.
//! Normals are smooth but where the surface turns sharply (`CREASE`). The renderer
//! sorts each set of instances by mesh (`Batch`) and draws a run per mesh.

use super::{GrowBuf, Inst, Vertex, crane};
use glam::{Quat, Vec2, Vec3};
use std::f32::consts::{PI, TAU};
use std::ops::Range;

/// The box.
pub const BOX: u32 = 0;
/// The first of the paper birds' parts (`crane::meshes`).
pub const CRANE: u32 = 1;
/// The first of a library's own.
pub const LIBRARY: u32 = CRANE + crane::MESHES as u32;

/// Corners turning more than this (radians, ~34°) stay sharp.
const CREASE: f32 = 0.6;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub idx: Vec<u32>,
}

/// A library's mesh (`Library::shape`), the size to draw it at (at first the size it
/// was made at), and where the middle of its bounds was in the coordinates it was made
/// in: instances of it, placed by that middle or by those coordinates' origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub mesh: u32,
    pub size: Vec3,
    made: Vec3,
    middle: Vec3,
}

impl Shape {
    pub fn new(mesh: u32, made: Vec3, middle: Vec3) -> Shape {
        Shape {
            mesh,
            size: made,
            made,
            middle,
        }
    }

    /// It with the middle of its bounds at `centre`, turned by `rot`.
    pub fn inst(self, centre: Vec3, rot: Quat, kind: u32, color: [f32; 4]) -> Inst {
        Inst::new(centre, rot, self.size, kind, color, 0).shaped(self.mesh)
    }

    /// It with the origin it was made about at `origin` (a lathe's foot on its axis, an
    /// arc's centre), turned by `rot`.
    pub fn at(self, origin: Vec3, rot: Quat, kind: u32, color: [f32; 4]) -> Inst {
        self.inst(origin + rot * (self.middle * self.size / self.made), rot, kind, color)
    }

    /// It (made along y, as a cylinder is) stretched from `a` to `b`, `d` across.
    pub fn between(self, a: Vec3, b: Vec3, d: f32, kind: u32, color: [f32; 4]) -> Inst {
        let v = b - a;
        let turn = Quat::from_rotation_arc(Vec3::Y, v.normalize());
        self.sized(Vec3::new(d, v.length(), d))
            .inst((a + b) * 0.5, turn, kind, color)
    }

    /// It `k` times the size.
    pub fn scaled(self, k: f32) -> Shape {
        self.sized(self.size * k)
    }

    pub fn sized(self, size: Vec3) -> Shape {
        Shape { size, ..self }
    }
}

/// A point of a 2D outline and its normal, for each end of each of its edges.
type Edge = [(Vec2, Vec2); 2];

/// The edges of an outline (`closed`: from its last point back to its first), each
/// end's normal smoothed with the next edge's unless they meet at a crease. A normal
/// is the direction's right hand, (y, -x): outward from an outline run anticlockwise.
fn edges(pts: &[Vec2], closed: bool) -> Vec<Edge> {
    let n = pts.len();
    let count = if closed { n } else { n - 1 };
    let normal = |k: usize| {
        let d = pts[(k + 1) % n] - pts[k];
        Vec2::new(d.y, -d.x).normalize_or_zero()
    };
    let norms: Vec<Vec2> = (0..count).map(normal).collect();
    // The normal at point k for edge e (one of the two edges meeting there).
    let at = |k: usize, e: usize| {
        let (before, after) = match (k, closed) {
            (0, false) => return norms[e],
            (k, false) if k == n - 1 => return norms[e],
            (k, _) => (norms[(k + count - 1) % count], norms[k % count]),
        };
        let turn = libm::acosf(before.dot(after).clamp(-1.0, 1.0));
        if turn > CREASE || before == Vec2::ZERO || after == Vec2::ZERO {
            norms[e]
        } else {
            (before + after).normalize()
        }
    };
    (0..count)
        .map(|e| {
            let k1 = (e + 1) % n;
            [(pts[e], at(e, e)), (pts[k1], at(k1, e))]
        })
        .collect()
}

/// `n` points round a circle of radius `r` in xy, anticlockwise from +x.
pub fn circle(r: f32, n: usize) -> Vec<Vec2> {
    (0..n)
        .map(|k| {
            let a = k as f32 / n as f32 * TAU;
            Vec2::new(libm::cosf(a), libm::sinf(a)) * r
        })
        .collect()
}

/// Twice the signed area of an outline (positive anticlockwise).
fn area2(pts: &[Vec2]) -> f32 {
    (0..pts.len()).map(|k| pts[k].perp_dot(pts[(k + 1) % pts.len()])).sum()
}

/// Triangles covering a simple polygon run anticlockwise (ear clipping).
fn triangulate(pts: &[Vec2]) -> Vec<[usize; 3]> {
    let mut left: Vec<usize> = (0..pts.len()).collect();
    let mut out = Vec::new();
    let inside = |p: Vec2, a: Vec2, b: Vec2, c: Vec2| {
        (b - a).perp_dot(p - a) > 0.0 && (c - b).perp_dot(p - b) > 0.0 && (a - c).perp_dot(p - c) > 0.0
    };
    while left.len() > 3 {
        let m = left.len();
        let ear = (0..m).find(|&k| {
            let (a, b, c) = (left[(k + m - 1) % m], left[k], left[(k + 1) % m]);
            let (pa, pb, pc) = (pts[a], pts[b], pts[c]);
            (pb - pa).perp_dot(pc - pb) > 0.0
                && !(left.iter()).any(|&o| o != a && o != b && o != c && inside(pts[o], pa, pb, pc))
        });
        // A degenerate outline leaves what it can't clip unfilled.
        let Some(k) = ear else { break };
        out.push([left[(k + m - 1) % m], left[k], left[(k + 1) % m]]);
        left.remove(k);
    }
    if left.len() == 3 {
        out.push([left[0], left[1], left[2]]);
    }
    out
}

impl Mesh {
    /// A vertex, and its index.
    pub fn vert(&mut self, p: Vec3, n: Vec3) -> u32 {
        self.verts.push(Vertex {
            pos: p.into(),
            normal: n.into(),
        });
        self.verts.len() as u32 - 1
    }

    /// A triangle, wound to face the way its corners' normals do; none if it has no area.
    pub fn tri(&mut self, a: u32, b: u32, c: u32) {
        let p = |i: u32| Vec3::from(self.verts[i as usize].pos);
        let n = |i: u32| Vec3::from(self.verts[i as usize].normal);
        let face = (p(b) - p(a)).cross(p(c) - p(a));
        if face.length_squared() < 1e-14 {
            return;
        }
        if face.dot(n(a) + n(b) + n(c)) >= 0.0 {
            self.idx.extend_from_slice(&[a, b, c]);
        } else {
            self.idx.extend_from_slice(&[a, c, b]);
        }
    }

    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }

    /// The unit box, a normal to each face.
    pub fn cube() -> Mesh {
        let mut m = Mesh::default();
        for axis in 0..3 {
            for s in [-1.0f32, 1.0] {
                let n = Vec3::AXES[axis] * s;
                let (a, b) = (Vec3::AXES[(axis + 1) % 3], Vec3::AXES[(axis + 2) % 3]);
                let c = (n + a * -1.0 + b * -1.0) * 0.5;
                let v: Vec<u32> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                    .into_iter()
                    .map(|(sa, sb)| m.vert(c + (a * (sa + 1.0) + b * (sb + 1.0)) * 0.5, n))
                    .collect();
                m.quad(v[0], v[1], v[2], v[3]);
            }
        }
        m
    }

    /// A solid turned about y from `profile`, (radius, height) points in order up its
    /// outside: from the axis at the bottom, out and up, back to the axis at the top. A
    /// profile that ends where it began, run anticlockwise, turns into a ring of that
    /// section. `segs` round. Points are `Vec2`s or `(radius, height)` pairs.
    pub fn lathe<P: Into<Vec2> + Copy>(profile: &[P], segs: usize) -> Mesh {
        let mut pts: Vec<Vec2> = profile.iter().map(|&p| p.into()).collect();
        let closed = pts.len() > 2 && pts[0].distance(pts[pts.len() - 1]) < 1e-6;
        if closed {
            pts.pop();
            if area2(&pts) < 0.0 {
                pts.reverse();
            }
        }
        let mut m = Mesh::default();
        let round = |p: Vec2, th: f32| Vec3::new(p.x * libm::cosf(th), p.y, p.x * libm::sinf(th));
        for [(p0, n0), (p1, n1)] in edges(&pts, closed) {
            let first = m.verts.len() as u32;
            for j in 0..=segs {
                let th = j as f32 / segs as f32 * TAU;
                m.vert(round(p0, th), round(n0, th));
                m.vert(round(p1, th), round(n1, th));
            }
            for j in 0..segs as u32 {
                let k = first + 2 * j;
                m.quad(k, k + 2, k + 3, k + 1);
            }
        }
        m
    }

    // The primitives fill the unit box, centred, so they can be merged (`add`) at a
    // size as instances are drawn at one.

    /// A cylinder about y.
    pub fn cylinder(segs: usize) -> Mesh {
        let p = [(0.0, -0.5), (0.5, -0.5), (0.5, 0.5), (0.0, 0.5)];
        Mesh::lathe(&p.map(Vec2::from), segs)
    }

    /// A cone about y, its point up.
    pub fn cone(segs: usize) -> Mesh {
        let p = [(0.0, -0.5), (0.5, -0.5), (0.0, 0.5)];
        Mesh::lathe(&p.map(Vec2::from), segs)
    }

    pub fn sphere(segs: usize) -> Mesh {
        let rows = (segs / 2).max(2);
        let arc: Vec<Vec2> = (0..=rows)
            .map(|k| {
                let a = k as f32 / rows as f32 * PI;
                Vec2::new(libm::sinf(a), -libm::cosf(a)) * 0.5
            })
            .collect();
        Mesh::lathe(&arc, segs)
    }

    /// Half a sphere on a flat base (drawn half as high as it is wide, it is round).
    pub fn dome(segs: usize) -> Mesh {
        let rows = (segs / 4).max(2);
        let mut p = vec![Vec2::new(0.0, -0.5)];
        p.extend((0..=rows).map(|k| {
            let a = k as f32 / rows as f32 * PI * 0.5;
            Vec2::new(0.5 * libm::cosf(a), libm::sinf(a) - 0.5)
        }));
        Mesh::lathe(&p, segs)
    }

    /// A ring about y whose round section is `tube` of its radius (to the section's
    /// middle) across.
    pub fn torus(tube: f32, segs: usize, sides: usize) -> Mesh {
        let r = tube * 0.5;
        let mut p: Vec<Vec2> = (0..sides)
            .map(|k| {
                let a = k as f32 / sides as f32 * TAU;
                Vec2::new(1.0 + r * libm::cosf(a), r * libm::sinf(a))
            })
            .collect();
        p.push(p[0]);
        Mesh::lathe(&p, segs)
    }

    /// The sides of an outline run anticlockwise (holes clockwise) in xy, from z = -d/2
    /// to d/2.
    fn walls(&mut self, outline: &[Vec2], depth: f32) {
        let z = depth * 0.5;
        for [(p0, n0), (p1, n1)] in edges(outline, true) {
            let at = |p: Vec2, z: f32| Vec3::new(p.x, p.y, z);
            let n = |n: Vec2| Vec3::new(n.x, n.y, 0.0);
            let a = self.vert(at(p0, -z), n(n0));
            let b = self.vert(at(p1, -z), n(n1));
            let c = self.vert(at(p1, z), n(n1));
            let d = self.vert(at(p0, z), n(n0));
            self.quad(a, b, c, d);
        }
    }

    /// An outline in xy (a simple polygon) made `depth` thick along z.
    pub fn prism(outline: &[Vec2], depth: f32) -> Mesh {
        let mut pts = outline.to_vec();
        if area2(&pts) < 0.0 {
            pts.reverse();
        }
        let mut m = Mesh::default();
        m.walls(&pts, depth);
        for s in [-1.0f32, 1.0] {
            let first = m.verts.len() as u32;
            for p in &pts {
                m.vert(Vec3::new(p.x, p.y, s * depth * 0.5), Vec3::Z * s);
            }
            for [a, b, c] in triangulate(&pts) {
                m.tri(first + a as u32, first + b as u32, first + c as u32);
            }
        }
        m
    }

    /// The band between two closed outlines in xy with as many points each (`inner[k]`
    /// across from `outer[k]`), made `depth` thick along z: a wheel's rim, a frame. (An
    /// open band, an arch, is a `prism` of its outline.)
    pub fn ring(outer: &[Vec2], inner: &[Vec2], depth: f32) -> Mesh {
        assert_eq!(outer.len(), inner.len(), "a ring's outlines pair up");
        let flip = area2(outer) < 0.0;
        let (mut outer, mut inner) = (outer.to_vec(), inner.to_vec());
        if flip {
            outer.reverse();
            inner.reverse();
        }
        let mut m = Mesh::default();
        m.walls(&outer, depth);
        let hole: Vec<Vec2> = inner.iter().rev().copied().collect();
        m.walls(&hole, depth);
        let n = outer.len() as u32;
        for s in [-1.0f32, 1.0] {
            let first = m.verts.len() as u32;
            for p in outer.iter().chain(&inner) {
                m.vert(Vec3::new(p.x, p.y, s * depth * 0.5), Vec3::Z * s);
            }
            for k in 0..n {
                let k1 = (k + 1) % n;
                m.quad(first + k, first + k1, first + n + k1, first + n + k);
            }
        }
        m
    }

    /// A toothed wheel in xy, 1 to the roots of its teeth, `depth` thick along z:
    /// `teeth` standing `tooth` (of the radius) proud, a rim, `spokes` from a hub (or
    /// solid, with none).
    pub fn gear(teeth: usize, tooth: f32, depth: f32, spokes: usize) -> Mesh {
        let at = |a: f32, r: f32| Vec2::new(libm::cosf(a), libm::sinf(a)) * r;
        let step = TAU / teeth as f32;
        let mut outer = Vec::new();
        for k in 0..teeth {
            let a = k as f32 * step;
            // Tapered teeth: broad at the root, narrower at the tip.
            for (f, r) in [(0.0, 1.0), (0.12, 1.0 + tooth), (0.38, 1.0 + tooth), (0.5, 1.0)] {
                outer.push(at(a + f * step, r));
            }
            outer.push(at(a + 0.75 * step, 1.0));
        }
        if spokes == 0 {
            return Mesh::prism(&outer, depth);
        }
        let rim = 0.8;
        let inner: Vec<Vec2> = outer.iter().map(|p| p.normalize() * rim).collect();
        let mut m = Mesh::ring(&outer, &inner, depth);
        let hub = Mesh::cylinder(16);
        let spoke = Mesh::cube();
        for k in 0..spokes {
            let a = (k as f32 + 0.5) / spokes as f32 * TAU;
            let dir = at(a, 1.0).extend(0.0);
            m.add(
                &spoke,
                dir * (rim * 0.5 + 0.05),
                Quat::from_rotation_z(a),
                Vec3::new(rim - 0.1, 0.12, depth * 0.8),
            );
        }
        m.add(
            &hub,
            Vec3::ZERO,
            Quat::from_rotation_x(PI * 0.5),
            Vec3::new(0.36, depth * 1.4, 0.36),
        );
        m
    }

    /// `section` (an outline) swept along `path`, scaled by `scale` from end to end (a
    /// value for each point, or a few spread evenly along it: `[0.1, 0.02]` tapers);
    /// `caps` closes the ends (of a convex section). Its frame turns as little as it can
    /// along the way.
    pub fn sweep(path: &[Vec3], scale: &[f32], section: &[Vec2], caps: bool) -> Mesh {
        let n = path.len();
        let size = |k: usize| {
            let f = k as f32 / (n - 1) as f32 * (scale.len() - 1) as f32;
            let i = (f as usize).min(scale.len().saturating_sub(2));
            match scale.len() {
                1 => scale[0],
                _ => scale[i] + (scale[i + 1] - scale[i]) * (f - i as f32),
            }
        };
        let tangent = |k: usize| (path[(k + 1).min(n - 1)] - path[k.saturating_sub(1)]).normalize();
        let mut frames = Vec::with_capacity(n);
        let t0 = tangent(0);
        let mut u = t0.any_orthonormal_vector();
        let mut prev = t0;
        for k in 0..n {
            let t = tangent(k);
            u = Quat::from_rotation_arc(prev, t) * u;
            u = (u - t * u.dot(t)).normalize();
            prev = t;
            frames.push((u, t.cross(u), t));
        }
        let mut sec = section.to_vec();
        if area2(&sec) < 0.0 {
            sec.reverse();
        }
        let mut m = Mesh::default();
        for [(s0, n0), (s1, n1)] in edges(&sec, true) {
            let first = m.verts.len() as u32;
            for k in 0..n {
                let (u, v, _) = frames[k];
                let at = |s: Vec2| path[k] + (u * s.x + v * s.y) * size(k);
                m.vert(at(s0), u * n0.x + v * n0.y);
                m.vert(at(s1), u * n1.x + v * n1.y);
            }
            for k in 0..n as u32 - 1 {
                let a = first + 2 * k;
                m.quad(a, a + 1, a + 3, a + 2);
            }
        }
        if caps {
            for (k, s) in [(0, -1.0f32), (n - 1, 1.0)] {
                let (u, v, t) = frames[k];
                let centre = m.vert(path[k], t * s);
                let ring: Vec<u32> = (sec.iter())
                    .map(|p| m.vert(path[k] + (u * p.x + v * p.y) * size(k), t * s))
                    .collect();
                for e in 0..ring.len() {
                    m.tri(centre, ring[e], ring[(e + 1) % ring.len()]);
                }
            }
        }
        m
    }

    /// A cylinder `d` across from `a` to `b`, `segs` round (for merging).
    pub fn rod(a: Vec3, b: Vec3, d: f32, segs: usize) -> Mesh {
        let mut m = Mesh::default();
        let v = b - a;
        let rot = Quat::from_rotation_arc(Vec3::Y, v.normalize());
        m.add(&Mesh::cylinder(segs), (a + b) * 0.5, rot, Vec3::new(d, v.length(), d));
        m
    }

    /// `other` merged in, placed at `at`, turned by `rot` and stretched by `scale`.
    pub fn add(&mut self, other: &Mesh, at: Vec3, rot: Quat, scale: Vec3) {
        let first = self.verts.len() as u32;
        for v in &other.verts {
            let p = at + rot * (Vec3::from(v.pos) * scale);
            let n = (rot * (Vec3::from(v.normal) / scale)).normalize();
            self.vert(p, n);
        }
        self.idx.extend(other.idx.iter().map(|i| i + first));
    }

    /// Seen from both sides: an open shell (a lamp's shade, a bowl). Not for clear
    /// glass: the translucent pass draws both sides of every surface already, so a
    /// two-sided glass would be drawn twice over, twice as thick.
    pub fn two_sided(mut self) -> Mesh {
        let first = self.verts.len() as u32;
        let back: Vec<Vertex> = (self.verts.iter())
            .map(|v| Vertex {
                pos: v.pos,
                normal: (-Vec3::from(v.normal)).into(),
            })
            .collect();
        self.verts.extend(back);
        let flipped: Vec<u32> = (self.idx.chunks_exact(3))
            .flat_map(|t| [t[0] + first, t[2] + first, t[1] + first])
            .collect();
        self.idx.extend(flipped);
        self
    }

    /// Fitted to the unit box (its bounds centred on the origin and stretched to fill
    /// the box), and the size and middle its bounds had.
    pub fn fitted(mut self) -> (Mesh, Vec3, Vec3) {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &self.verts {
            lo = lo.min(v.pos.into());
            hi = hi.max(v.pos.into());
        }
        let size = hi - lo;
        assert!(size.min_element() > 1e-6, "a mesh has no thickness: {size}");
        let centre = (lo + hi) * 0.5;
        for v in &mut self.verts {
            v.pos = ((Vec3::from(v.pos) - centre) / size).into();
            v.normal = (Vec3::from(v.normal) * size).normalize().into();
        }
        (self, size, centre)
    }
}

/// Every mesh's vertices and indices in one pair of buffers (the built-in ones, then
/// the library's), and where each one's are: its indices and its first vertex.
pub(super) struct Meshes {
    vb: GrowBuf<Vertex>,
    ib: GrowBuf<u32>,
    builtin: Vec<Mesh>,
    ranges: Vec<(Range<u32>, i32)>,
}

impl Meshes {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut builtin = vec![Mesh::cube()];
        builtin.extend(crane::meshes());
        assert_eq!(builtin.len(), LIBRARY as usize);
        let mut m = Meshes {
            vb: GrowBuf::with(device, "mesh vertices", 4096, wgpu::BufferUsages::VERTEX),
            ib: GrowBuf::index(device, "mesh indices", 4096),
            builtin,
            ranges: Vec::new(),
        };
        m.set_library(device, queue, &[]);
        m
    }

    /// The library's own meshes, after the built-in ones.
    pub(super) fn set_library(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, library: &[Mesh]) {
        let (mut verts, mut idx) = (Vec::new(), Vec::new());
        self.ranges.clear();
        for m in self.builtin.iter().chain(library) {
            let first = idx.len() as u32;
            self.ranges
                .push((first..first + m.idx.len() as u32, verts.len() as i32));
            verts.extend_from_slice(&m.verts);
            idx.extend_from_slice(&m.idx);
        }
        self.vb.upload(device, queue, &verts);
        self.ib.upload(device, queue, &idx);
    }

    /// Bind them; the box's indices are then the first 36.
    pub(super) fn bind(&self, rp: &mut wgpu::RenderPass) {
        rp.set_vertex_buffer(0, self.vb.buf.slice(..));
        rp.set_index_buffer(self.ib.buf.slice(..), wgpu::IndexFormat::Uint32);
    }

    /// Draw a batch's runs (with them bound).
    pub(super) fn draw(&self, rp: &mut wgpu::RenderPass, batch: &Batch) {
        for (mesh, run) in &batch.runs {
            let Some((range, base)) = self.ranges.get(*mesh as usize) else {
                continue;
            };
            let from = (run.start as usize * std::mem::size_of::<Inst>()) as u64;
            rp.set_vertex_buffer(1, batch.buf.buf.slice(from..));
            rp.draw_indexed(range.clone(), *base, 0..run.len() as u32);
        }
    }
}

/// Instances on the GPU sorted by mesh (the box's first, in the order they came), and
/// each mesh's run of them.
pub(super) struct Batch {
    buf: GrowBuf,
    sorted: Vec<Inst>,
    runs: Vec<(u32, Range<u32>)>,
}

impl Batch {
    pub(super) fn new(device: &wgpu::Device, label: &'static str, cap: usize) -> Self {
        Batch {
            buf: GrowBuf::new(device, label, cap),
            sorted: Vec::new(),
            runs: Vec::new(),
        }
    }

    pub(super) fn upload<'a>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        insts: impl IntoIterator<Item = &'a Inst>,
    ) {
        self.sorted.clear();
        self.sorted.extend(insts);
        self.sorted.sort_by_key(|i| i.mesh);
        self.runs.clear();
        for (k, i) in self.sorted.iter().enumerate() {
            match self.runs.last_mut() {
                Some((m, run)) if *m == i.mesh => run.end = k as u32 + 1,
                _ => self.runs.push((i.mesh, k as u32..k as u32 + 1)),
            }
        }
        self.buf.upload(device, queue, &self.sorted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shapes() -> Vec<(&'static str, Mesh)> {
        let v = |x: f32, y: f32| Vec2::new(x, y);
        let star: Vec<Vec2> = (0..10)
            .map(|k| {
                let a = k as f32 * TAU / 10.0;
                v(libm::cosf(a), libm::sinf(a)) * if k % 2 == 0 { 1.0 } else { 0.45 }
            })
            .collect();
        let arc = |r: f32| -> Vec<Vec2> {
            (0..=12)
                .map(|k| v(libm::cosf(k as f32 * PI / 12.0), libm::sinf(k as f32 * PI / 12.0)) * r)
                .collect()
        };
        let horn: Vec<Vec3> = (0..10)
            .map(|k| Vec3::new(k as f32 * 0.1, (k * k) as f32 * 0.01, 0.0))
            .collect();
        let round: Vec<Vec2> = (0..8)
            .map(|k| v(libm::cosf(k as f32 * TAU / 8.0), libm::sinf(k as f32 * TAU / 8.0)))
            .collect();
        vec![
            ("cube", Mesh::cube()),
            ("cylinder", Mesh::cylinder(24)),
            ("cone", Mesh::cone(12)),
            ("sphere", Mesh::sphere(16)),
            ("dome", Mesh::dome(16)),
            ("torus", Mesh::torus(0.2, 24, 8)),
            ("star", Mesh::prism(&star, 0.1)),
            (
                "arch",
                Mesh::prism(&[arc(1.0), arc(0.8).into_iter().rev().collect()].concat(), 0.2),
            ),
            (
                "frame",
                Mesh::ring(&star, &star.iter().map(|p| *p * 0.7).collect::<Vec<_>>(), 0.05),
            ),
            ("gear", Mesh::gear(12, 0.15, 0.1, 0)),
            ("wheel", Mesh::gear(24, 0.1, 0.1, 3)),
            ("horn", Mesh::sweep(&horn, &[0.1, 0.02], &round, true)),
            (
                "vase",
                Mesh::lathe(&[v(0.0, 0.0), v(0.3, 0.0), v(0.5, 0.4), v(0.2, 0.9), v(0.25, 1.0)], 16).two_sided(),
            ),
        ]
    }

    /// Every triangle faces the way its corners' normals do, and indices are in range.
    #[test]
    fn meshes_face_out() {
        for (name, m) in shapes() {
            assert!(!m.idx.is_empty(), "{name}: empty");
            for t in m.idx.chunks_exact(3) {
                let v: Vec<&Vertex> = t.iter().map(|&i| &m.verts[i as usize]).collect();
                let p = |k: usize| Vec3::from(v[k].pos);
                let face = (p(1) - p(0)).cross(p(2) - p(0)).normalize();
                for x in &v {
                    let n = Vec3::from(x.normal);
                    assert!((n.length() - 1.0).abs() < 1e-3, "{name}: normal {n}");
                    assert!(
                        face.dot(n) > -0.2,
                        "{name}: a triangle faces {face} against its normal {n}"
                    );
                }
            }
        }
    }

    /// Fitted meshes fill the unit box, and their normals still face out.
    #[test]
    fn fitted_fills_the_box() {
        for (name, m) in shapes() {
            let (f, size, _) = m.clone().fitted();
            assert!(size.min_element() > 0.0, "{name}");
            let (mut lo, mut hi) = (Vec3::splat(9.0), Vec3::splat(-9.0));
            for v in &f.verts {
                lo = lo.min(v.pos.into());
                hi = hi.max(v.pos.into());
            }
            assert!(
                lo.abs_diff_eq(Vec3::splat(-0.5), 1e-5) && hi.abs_diff_eq(Vec3::splat(0.5), 1e-5),
                "{name}: {lo} {hi}"
            );
            // Stretched back to its size, a fitted normal is the original's.
            for (a, b) in m.verts.iter().zip(&f.verts) {
                let back = (Vec3::from(b.normal) / size).normalize();
                assert!(
                    back.abs_diff_eq(a.normal.into(), 1e-3),
                    "{name}: {back} vs {:?}",
                    a.normal
                );
            }
        }
    }

    /// Placed by the origin it was made about, a shape's points are drawn where they
    /// were made, however it is sized and turned: a wheel with an odd number of teeth,
    /// whose bounds are off its axis, turns about its axis.
    #[test]
    fn placed_by_origin() {
        let m = Mesh::gear(9, 0.13, 0.1, 0);
        let made: Vec<Vec3> = m.verts.iter().map(|v| v.pos.into()).collect();
        let (f, size, middle) = m.fitted();
        assert!(
            middle.length() > 1e-4,
            "nine teeth leave the middle of the bounds off the axis"
        );
        let stretch = Vec3::new(2.0, 2.0, 3.0);
        let (origin, rot) = (Vec3::new(1.0, 2.0, 3.0), Quat::from_rotation_y(0.7));
        let i = Shape::new(LIBRARY, size, middle)
            .sized(size * stretch)
            .at(origin, rot, 0, [1.0; 4]);
        for (p, v) in made.iter().zip(&f.verts) {
            // As the vertex shader draws it.
            let drawn = Vec3::from(i.pos) + rot * (Vec3::from(v.pos) * Vec3::from(i.scale));
            let want = origin + rot * (*p * stretch);
            assert!(drawn.abs_diff_eq(want, 1e-4), "{drawn} vs {want}");
        }
    }

    /// Closed solids have every edge shared by two triangles running it opposite ways.
    #[test]
    fn solids_are_closed() {
        for (name, m) in shapes().into_iter().filter(|(n, _)| !["vase", "wheel"].contains(n)) {
            let key = |i: u32| {
                let p = m.verts[i as usize].pos;
                [
                    (p[0] * 1e4).round() as i64,
                    (p[1] * 1e4).round() as i64,
                    (p[2] * 1e4).round() as i64,
                ]
            };
            let mut count = std::collections::HashMap::new();
            for t in m.idx.chunks_exact(3) {
                for e in 0..3 {
                    let (a, b) = (key(t[e]), key(t[(e + 1) % 3]));
                    *count.entry((a, b)).or_insert(0i32) += 1;
                    *count.entry((b, a)).or_insert(0i32) -= 1;
                }
            }
            let open = count.values().filter(|&&c| c != 0).count();
            assert_eq!(open, 0, "{name}: {open} unmatched edges");
        }
    }
}
