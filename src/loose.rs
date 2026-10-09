//! Loose books: everything that is not shelved or carried lies where it fell.
//!
//! Semi-physics rather than rigid bodies. A thrown book flies ballistically with
//! spin and bounces off walls and furniture; when it lands it settles onto a
//! heightfield of whatever is below and slides downhill while its support is
//! steeper than books rest on, so piles form and spill. Taking books out of a pile
//! re-drops whatever rested on them. Furniture is a solid column from the floor:
//! tables hold books, taller pieces shed them off their tops.

use crate::library::{HALL_H, HALL_X, HALL_Z, Library, VEIL_X};
use glam::{Mat3, Quat, Vec2, Vec3};

/// Heightfield resolution.
const CELL: f32 = 0.04;
/// Spatial index resolution.
const BUCKET: f32 = 0.5;
/// Steepest support (rise over run) a book stays on; on anything steeper it slides.
const REPOSE: f32 = 0.6;
/// Steepest tilt a resting book takes.
const MAX_TILT: f32 = 0.45;
const SLIDE_STEP: f32 = 0.05;
const MAX_SLIDE_STEPS: usize = 14;
/// Furniture taller than this sheds books off its top rather than holding them.
const SHED_HEIGHT: f32 = 0.9;
pub const GRAVITY: f32 = 9.0;
/// A flying book's collision radius.
const FLY_R: f32 = 0.05;
const SUBSTEPS: usize = 3;
/// A support change smaller than this leaves a resting book exactly where it was.
const KEEP_EPS: f32 = 0.003;

/// How a loose book lies: its underside is a plane through `c`, tilted by `su` along
/// the book's length (`u`) and by `sv` across it (`v`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rest {
    pub c: Vec3,
    pub u: Vec2,
    pub su: f32,
    pub sv: f32,
}

impl Rest {
    fn v(&self) -> Vec2 {
        // Matches the book's depth axis in `pose` (Y × length).
        Vec2::new(self.u.y, -self.u.x)
    }

    fn xz(&self) -> Vec2 {
        Vec2::new(self.c.x, self.c.z)
    }

    /// `p` in the book's footprint frame (along its length, across it).
    fn local(&self, p: Vec2) -> Vec2 {
        let d = p - self.xz();
        Vec2::new(d.dot(self.u), d.dot(self.v()))
    }

    /// Height of the underside's plane above `p`.
    fn under(&self, p: Vec2) -> f32 {
        let l = self.local(p);
        self.c.y + l.x * self.su + l.y * self.sv
    }

    /// How far the top face rises above the underside, vertically, for a book `t`
    /// thick (more than `t` when tilted).
    fn rise(&self, t: f32) -> f32 {
        t * (1.0 + self.su * self.su + self.sv * self.sv).sqrt()
    }

    /// Centre and orientation of a book of `size` (thickness, height, depth) lying so.
    pub fn pose(&self, size: Vec3) -> (Vec3, Quat) {
        let v = self.v();
        let tu = Vec3::new(self.u.x, self.su, self.u.y).normalize();
        let tv = Vec3::new(v.x, self.sv, v.y).normalize();
        let n = tu.cross(tv).normalize();
        let tv = n.cross(tu);
        let rot = Quat::from_mat3(&Mat3::from_cols(n, tu, tv)).normalize();
        (self.c + n * size.x * 0.5, rot)
    }
}

pub struct Landing {
    pub at: Vec3,
    pub speed: f32,
}

struct Flying {
    book: u32,
    pos: Vec3,
    vel: Vec3,
    rot: Quat,
    spin: Vec3,
    /// Seconds before release (a toss leaves the hand as a stream).
    delay: f32,
    age: f32,
}

/// Render-only easing from where a book was to its new rest (a landing or a fall).
struct Ease {
    book: u32,
    from_pos: Vec3,
    from_rot: Quat,
    t: f32,
    dur: f32,
}

pub struct Loose {
    size: Vec<Vec3>,
    rest: Vec<Rest>,
    /// Pose of each resting book, cached from `rest`.
    pose: Vec<(Vec3, Quat)>,
    resting: Vec<bool>,
    /// Left its generated pose (only these are saved).
    moved: Vec<bool>,
    bw: usize,
    bh: usize,
    /// Resting books by the 0.5 m cells their footprints overlap.
    buckets: Vec<Vec<u32>>,
    fw: usize,
    fh: usize,
    /// Floor and furniture tops.
    base: Vec<f32>,
    /// `base` plus resting books.
    top: Vec<f32>,
    /// Furniture boxes (min, max).
    solids: Vec<(Vec3, Vec3)>,
    flying: Vec<Flying>,
    easing: Vec<Ease>,
    mark: Vec<u32>,
    stamp: u32,
    pub east_open: bool,
}

impl Loose {
    /// Drop every book where the library's generator put it.
    pub fn new(lib: &Library) -> Self {
        let n = lib.books.len();
        let bw = (2.0 * HALL_X / BUCKET).ceil() as usize;
        let bh = (2.0 * HALL_Z / BUCKET).ceil() as usize;
        let fw = (2.0 * HALL_X / CELL).ceil() as usize;
        let fh = (2.0 * HALL_Z / CELL).ceil() as usize;
        let solids = merge_solids(lib.obstacles.iter().map(|o| (o.min, o.max)).collect());
        let mut base = vec![0.0f32; fw * fh];
        for &(mn, mx) in &solids {
            let (x0, z0) = Self::cell_xy(Vec2::new(mn.x, mn.z), fw, fh);
            let (x1, z1) = Self::cell_xy(Vec2::new(mx.x, mx.z), fw, fh);
            for z in z0..=z1 {
                for x in x0..=x1 {
                    let c = Self::cell_center(x, z);
                    if c.x >= mn.x && c.x <= mx.x && c.y >= mn.z && c.y <= mx.z {
                        let k = z * fw + x;
                        base[k] = base[k].max(mx.y);
                    }
                }
            }
        }
        let mut loose = Loose {
            size: lib.books.iter().map(|b| b.size).collect(),
            rest: vec![
                Rest {
                    c: Vec3::ZERO,
                    u: Vec2::X,
                    su: 0.0,
                    sv: 0.0
                };
                n
            ],
            pose: vec![(Vec3::ZERO, Quat::IDENTITY); n],
            resting: vec![false; n],
            moved: vec![false; n],
            bw,
            bh,
            buckets: vec![Vec::new(); bw * bh],
            fw,
            fh,
            top: base.clone(),
            base,
            solids,
            flying: Vec::new(),
            easing: Vec::new(),
            mark: vec![0; n],
            stamp: 0,
            east_open: false,
        };
        for &(b, xz, u) in &lib.drops {
            let r = loose.drop(b, xz, u);
            loose.rest[b as usize] = r;
            loose.index(b);
        }
        loose
    }

    // ---------------------------------------------------------------- grids

    fn cell_xy(p: Vec2, w: usize, h: usize) -> (usize, usize) {
        let x = ((p.x + HALL_X) / CELL).floor().clamp(0.0, (w - 1) as f32) as usize;
        let z = ((p.y + HALL_Z) / CELL).floor().clamp(0.0, (h - 1) as f32) as usize;
        (x, z)
    }

    fn cell_center(x: usize, z: usize) -> Vec2 {
        Vec2::new(-HALL_X + (x as f32 + 0.5) * CELL, -HALL_Z + (z as f32 + 0.5) * CELL)
    }

    fn cell(&self, p: Vec2) -> usize {
        let (x, z) = Self::cell_xy(p, self.fw, self.fh);
        z * self.fw + x
    }

    fn bucket_xy(&self, p: Vec2) -> (usize, usize) {
        let x = ((p.x + HALL_X) / BUCKET).floor().clamp(0.0, (self.bw - 1) as f32) as usize;
        let z = ((p.y + HALL_Z) / BUCKET).floor().clamp(0.0, (self.bh - 1) as f32) as usize;
        (x, z)
    }

    /// Bucket indices overlapping the rectangle `mn..mx`.
    fn buckets_in(&self, mn: Vec2, mx: Vec2) -> impl Iterator<Item = usize> + use<> {
        let (x0, z0) = self.bucket_xy(mn);
        let (x1, z1) = self.bucket_xy(mx);
        let bw = self.bw;
        (z0..=z1).flat_map(move |z| (x0..=x1).map(move |x| z * bw + x))
    }

    /// Height of the surface (floor, table or books) at `p`.
    pub fn height_at(&self, p: Vec2) -> f32 {
        self.top[self.cell(p)]
    }

    fn base_at(&self, p: Vec2) -> f32 {
        self.base[self.cell(p)]
    }

    // ---------------------------------------------------------------- books

    pub fn is_resting(&self, b: u32) -> bool {
        self.resting[b as usize]
    }

    #[cfg(test)]
    pub fn is_flying(&self, b: u32) -> bool {
        self.flying.iter().any(|f| f.book == b)
    }

    /// Where a resting book lies (centre, orientation).
    pub fn pose(&self, b: u32) -> (Vec3, Quat) {
        self.pose[b as usize]
    }

    fn half(&self, b: u32) -> Vec2 {
        let s = self.size[b as usize];
        Vec2::new(s.y, s.z) * 0.5
    }

    #[cfg(test)]
    fn inside(&self, b: u32, p: Vec2) -> bool {
        let l = self.rest[b as usize].local(p);
        let h = self.half(b);
        l.x.abs() <= h.x && l.y.abs() <= h.y
    }

    fn footprint(&self, b: u32) -> (Vec2, Vec2) {
        let r = &self.rest[b as usize];
        let h = self.half(b);
        let v = r.v();
        let ext = r.u.abs() * h.x + v.abs() * h.y;
        (r.xz() - ext, r.xz() + ext)
    }

    /// Add a book at its `rest` to the index and the heightfield.
    fn index(&mut self, b: u32) {
        let bi = b as usize;
        self.resting[bi] = true;
        self.pose[bi] = self.rest[bi].pose(self.size[bi]);
        let (mn, mx) = self.footprint(b);
        for k in self.buckets_in(mn, mx) {
            self.buckets[k].push(b);
        }
        self.raster(b, None);
    }

    /// Take a book out of the index. The heightfield is left for the caller to rebuild,
    /// and any fall still being drawn carries on (a re-drop picks it up).
    fn unindex(&mut self, b: u32) {
        if !self.resting[b as usize] {
            return;
        }
        self.resting[b as usize] = false;
        let (mn, mx) = self.footprint(b);
        for k in self.buckets_in(mn, mx) {
            if let Some(i) = self.buckets[k].iter().position(|&x| x == b) {
                self.buckets[k].swap_remove(i);
            }
        }
    }

    /// A book leaves the piles for good (taken or forgotten): stop drawing its fall.
    fn release(&mut self, b: u32) {
        self.unindex(b);
        self.easing.retain(|e| e.book != b);
    }

    /// Raise the heightfield to a book's top, on the cells it covers (only those in the
    /// cell rectangle `clip`, if given).
    fn raster(&mut self, b: u32, clip: Option<(usize, usize, usize, usize)>) {
        let r = self.rest[b as usize];
        let rise = r.rise(self.size[b as usize].x);
        let fw = self.fw;
        let inside = |k: usize| {
            clip.is_none_or(|(x0, z0, x1, z1)| (x0..=x1).contains(&(k % fw)) && (z0..=z1).contains(&(k / fw)))
        };
        let mut any = false;
        for (k, l) in self.cells_under(r, self.half(b)) {
            any = true;
            if inside(k) {
                self.top[k] = self.top[k].max(r.c.y + l.x * r.su + l.y * r.sv + rise);
            }
        }
        // A footprint smaller than a cell stands on the cell under its centre.
        let k = self.cell(r.xz());
        if !any && inside(k) {
            self.top[k] = self.top[k].max(r.c.y + rise);
        }
    }

    /// Rebuild the heightfield over `lo..hi` from the floor and the books still resting.
    fn rebuild(&mut self, lo: Vec2, hi: Vec2) {
        let (x0, z0) = Self::cell_xy(lo, self.fw, self.fh);
        let (x1, z1) = Self::cell_xy(hi, self.fw, self.fh);
        for z in z0..=z1 {
            let row = z * self.fw;
            self.top[row + x0..=row + x1].copy_from_slice(&self.base[row + x0..=row + x1]);
        }
        self.stamp += 1;
        let mut books = Vec::new();
        for k in self.buckets_in(lo, hi) {
            for &b in &self.buckets[k] {
                if self.mark[b as usize] != self.stamp {
                    self.mark[b as usize] = self.stamp;
                    books.push(b);
                }
            }
        }
        // Exactly the cells reset above; filtering by cell centre instead would leave
        // the rim of the range at floor height under books that overlap it.
        for b in books {
            self.raster(b, Some((x0, z0, x1, z1)));
        }
    }

    // ---------------------------------------------------------------- settling

    /// Fit a book's underside to the surface beneath it at `c` with length along `u`:
    /// the raw support slopes (along, across) and the clamped rest.
    fn fit(&self, b: u32, c: Vec2, u: Vec2) -> (Vec2, Rest) {
        self.fit_on(b, c, u, |k, _| self.top[k])
    }

    /// `fit` against any support surface, read at the heightfield cells whose centres
    /// the book covers (the cells its own top is rastered into).
    fn fit_on(&self, b: u32, c: Vec2, u: Vec2, support: impl Fn(usize, Vec2) -> f32) -> (Vec2, Rest) {
        let h = self.half(b);
        let probe = Rest {
            c: Vec3::new(c.x, 0.0, c.y),
            u,
            su: 0.0,
            sv: 0.0,
        };
        // A footprint smaller than a cell stands on the cell under its centre.
        let fallback = [(self.cell(c), Vec2::ZERO)];
        let cells = || {
            let mut it = self.cells_under(probe, h).peekable();
            let none = it.peek().is_none();
            it.chain(fallback.iter().copied().filter(move |_| none))
        };
        // Highest support under each half, along and across.
        let (mut ha, mut hb, mut hc, mut hd) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for (k, l) in cells() {
            let s = support(k, l);
            if l.x < 0.0 {
                ha = ha.max(s);
            } else {
                hb = hb.max(s);
            }
            if l.y < 0.0 {
                hc = hc.max(s);
            } else {
                hd = hd.max(s);
            }
        }
        let g = Vec2::new((hb - ha) / h.x, (hd - hc) / h.y);
        let su = g.x.clamp(-MAX_TILT, MAX_TILT);
        let sv = g.y.clamp(-MAX_TILT, MAX_TILT);
        let mut y = ((ha + hb) * 0.5).max((hc + hd) * 0.5);
        // Never sink below any support under the tilted plane.
        for (k, l) in cells() {
            y = y.max(support(k, l) - l.x * su - l.y * sv);
        }
        (
            g,
            Rest {
                c: Vec3::new(c.x, y, c.y),
                u,
                su,
                sv,
            },
        )
    }

    /// Heightfield cells whose centres lie inside a footprint with half extents `h`,
    /// with each centre in the footprint's frame.
    fn cells_under(&self, r: Rest, h: Vec2) -> impl Iterator<Item = (usize, Vec2)> + use<> {
        let v = r.v();
        let ext = r.u.abs() * h.x + v.abs() * h.y;
        let (x0, z0) = Self::cell_xy(r.xz() - ext, self.fw, self.fh);
        let (x1, z1) = Self::cell_xy(r.xz() + ext, self.fw, self.fh);
        let fw = self.fw;
        (z0..=z1)
            .flat_map(move |z| (x0..=x1).map(move |x| (x, z)))
            .filter_map(move |(x, z)| {
                let l = r.local(Self::cell_center(x, z));
                (l.x.abs() <= h.x && l.y.abs() <= h.y).then_some((z * fw + x, l))
            })
    }

    /// Keep a book's centre far enough from walls, the closed veil and tall furniture
    /// that it can lie flat beside them.
    fn keep_clear(&self, b: u32, mut c: Vec2) -> Vec2 {
        let r = self.half(b).length();
        let walls = |mut c: Vec2| {
            c.x = c.x.clamp(-HALL_X + r, HALL_X - r);
            c.y = c.y.clamp(-HALL_Z + r, HALL_Z - r);
            if !self.east_open {
                // The closed veil is a wall from either side.
                c.x = if c.x < VEIL_X {
                    c.x.min(VEIL_X - r)
                } else {
                    c.x.max(VEIL_X + r)
                };
            }
            c
        };
        // Pushing out of one piece can push into a wall or another piece; settle it.
        for _ in 0..4 {
            c = walls(c);
            let before = c;
            for &(mn, mx) in &self.solids {
                if mx.y <= SHED_HEIGHT {
                    continue;
                }
                let q = Vec2::new(c.x.clamp(mn.x, mx.x), c.y.clamp(mn.z, mx.z));
                let d = c - q;
                let l = d.length();
                if l >= r {
                    continue;
                }
                if l > 1e-5 {
                    c = q + d / l * r;
                } else {
                    // Inside: leave by the nearest side that isn't against a wall.
                    let mut out = [
                        (c.x - mn.x, Vec2::new(mn.x - r, c.y)),
                        (mx.x - c.x, Vec2::new(mx.x + r, c.y)),
                        (c.y - mn.z, Vec2::new(c.x, mn.z - r)),
                        (mx.z - c.y, Vec2::new(c.x, mx.z + r)),
                    ];
                    out.sort_by(|a, b| a.0.total_cmp(&b.0));
                    c = out.iter().find(|o| walls(o.1) == o.1).unwrap_or(&out[0]).1;
                }
            }
            if c == before {
                break;
            }
        }
        walls(c)
    }

    /// Where a book dropped at `c` (length along `u`) comes to rest: it slides
    /// downhill while its support is too steep, then fits what is under it.
    fn drop(&self, b: u32, c: Vec2, u: Vec2) -> Rest {
        let mut c = self.keep_clear(b, c);
        let v = Vec2::new(u.y, -u.x);
        for _ in 0..MAX_SLIDE_STEPS {
            let (g, _) = self.fit(b, c, u);
            if g.x.abs().max(g.y.abs()) <= REPOSE {
                break;
            }
            let dir = -(u * g.x + v * g.y).normalize_or_zero();
            let next = self.keep_clear(b, c + dir * SLIDE_STEP);
            if next.distance(c) < 1e-3 {
                break;
            }
            c = next;
        }
        self.fit(b, c, u).1
    }

    /// True if book `y` lies above book `x`: on some heightfield cell both cover, y's
    /// underside is at or above x's top. Whatever lies above a removed book must be
    /// lifted with it, not just what touches it, or a re-dropped book would see books
    /// over it as support.
    fn above(&self, y: u32, x: u32) -> bool {
        let (ry, rx) = (self.rest[y as usize], self.rest[x as usize]);
        let ((ymn, ymx), (xmn, xmx)) = (self.footprint(y), self.footprint(x));
        if ymn.x > xmx.x || xmn.x > ymx.x || ymn.y > xmx.y || xmn.y > ymx.y {
            return false;
        }
        let rise = rx.rise(self.size[x as usize].x);
        let hx = self.half(x);
        let fw = self.fw;
        self.cells_under(ry, self.half(y)).any(|(k, _)| {
            let p = Self::cell_center(k % fw, k / fw);
            let l = rx.local(p);
            l.x.abs() <= hx.x && l.y.abs() <= hx.y && ry.under(p) >= rx.under(p) + rise - 0.012
        })
    }

    /// Books that rested on `removed` (already out of the index) fall: everything
    /// above them is lifted out, the heightfield rebuilt, and the lot re-dropped, each
    /// after the books below it. A book whose support did not change keeps its exact
    /// pose.
    pub fn resettle(&mut self, removed: &[u32]) {
        if removed.is_empty() {
            return;
        }
        self.stamp += 1;
        let stamp = self.stamp;
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        let mut set: Vec<u32> = Vec::new();
        let mut queue: Vec<u32> = removed.to_vec();
        while let Some(x) = queue.pop() {
            let (mn, mx) = self.footprint(x);
            lo = lo.min(mn);
            hi = hi.max(mx);
            for k in self.buckets_in(mn, mx) {
                for i in 0..self.buckets[k].len() {
                    let y = self.buckets[k][i];
                    if self.mark[y as usize] != stamp && self.above(y, x) {
                        self.mark[y as usize] = stamp;
                        set.push(y);
                        queue.push(y);
                    }
                }
            }
        }
        let order = self.support_order(&set, stamp);
        for &y in &set {
            self.unindex(y);
        }
        self.rebuild(lo, hi);
        for y in order {
            self.redrop(y);
        }
    }

    /// `set` (books marked with `stamp`, still indexed) ordered so each book comes
    /// after the books in the set below it; ties and cycles go lowest first.
    fn support_order(&self, set: &[u32], stamp: u32) -> Vec<u32> {
        use std::cmp::Reverse;
        use std::collections::{BinaryHeap, HashMap, HashSet};
        let at: HashMap<u32, usize> = set.iter().enumerate().map(|(i, &b)| (b, i)).collect();
        let mut above: Vec<Vec<usize>> = vec![Vec::new(); set.len()];
        let mut deps = vec![0u32; set.len()];
        let mut seen: HashSet<u32> = HashSet::new();
        for (iy, &y) in set.iter().enumerate() {
            let (mn, mx) = self.footprint(y);
            seen.clear();
            for k in self.buckets_in(mn, mx) {
                for &x in &self.buckets[k] {
                    if x == y || self.mark[x as usize] != stamp || !seen.insert(x) {
                        continue;
                    }
                    if self.above(y, x) {
                        above[at[&x]].push(iy);
                        deps[iy] += 1;
                    }
                }
            }
        }
        // Heights are non-negative, so their bit patterns order like the floats.
        let key = |i: usize| Reverse((self.rest[set[i] as usize].c.y.to_bits(), i));
        let mut ready: BinaryHeap<_> = (0..set.len()).filter(|&i| deps[i] == 0).map(key).collect();
        let mut done = vec![false; set.len()];
        let mut order = Vec::with_capacity(set.len());
        while order.len() < set.len() {
            let i = match ready.pop() {
                Some(Reverse((_, i))) if !done[i] => i,
                Some(_) => continue,
                // A cycle (two books each within tolerance of the other): lowest first.
                None => (0..set.len()).filter(|&i| !done[i]).min_by_key(|&i| key(i).0).unwrap(),
            };
            done[i] = true;
            order.push(set[i]);
            for &j in &above[i] {
                deps[j] = deps[j].saturating_sub(1);
                if deps[j] == 0 && !done[j] {
                    ready.push(key(j));
                }
            }
        }
        order
    }

    /// Put a book back into the index, dropping it if its support has changed.
    fn redrop(&mut self, b: u32) {
        let bi = b as usize;
        let old = self.rest[bi];
        let (_, fitted) = self.fit(b, old.xz(), old.u);
        // Kept only if the whole underside still fits, not just its centre: a book
        // can tip, or have a neighbour drop in under one edge, at the same height.
        let h = self.half(b);
        let moved = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
            .iter()
            .any(|&(a, c)| {
                let p = old.xz() + old.u * (a * h.x) + old.v() * (c * h.y);
                (fitted.under(p) - old.under(p)).abs() >= KEEP_EPS
            });
        if moved {
            // Fall from where the book is drawn: its old rest, or partway through a fall.
            let mut from = self.pose[bi];
            if let Some(i) = self.easing.iter().position(|e| e.book == b) {
                let e = self.easing.swap_remove(i);
                let k = e.t.min(1.0);
                from = (e.from_pos.lerp(from.0, k * k), e.from_rot.slerp(from.1, k));
            }
            self.rest[bi] = self.drop(b, old.xz(), old.u);
            self.moved[bi] = true;
            // Falls take the time a short drop would.
            let fall = (old.c.y - self.rest[bi].c.y).max(0.02);
            let dur = (2.0 * fall / GRAVITY).sqrt().clamp(0.08, 0.5);
            self.easing.push(Ease {
                book: b,
                from_pos: from.0,
                from_rot: from.1,
                t: 0.0,
                dur,
            });
        }
        self.index(b);
    }

    /// Rebuild the whole heightfield from the books resting now (after a load).
    pub fn rebuild_all(&mut self) {
        let (lo, hi) = (Vec2::new(-HALL_X, -HALL_Z), Vec2::new(HALL_X, HALL_Z));
        self.rebuild(lo, hi);
    }

    // ---------------------------------------------------------------- taking

    /// Resting books whose centres lie within `radius` of `at` (and on its side of the
    /// closed veil), passing `keep`.
    fn near(&mut self, at: Vec2, radius: f32, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let west = at.x < VEIL_X;
        let open = self.east_open;
        let keep = |b: u32, r: &Rest| (open || (r.c.x < VEIL_X) == west) && keep(b);
        self.stamp += 1;
        let mut out = Vec::new();
        let r = Vec2::splat(radius);
        for k in self.buckets_in(at - r, at + r) {
            for &b in &self.buckets[k] {
                if self.mark[b as usize] == self.stamp {
                    continue;
                }
                self.mark[b as usize] = self.stamp;
                let r = &self.rest[b as usize];
                if r.xz().distance(at) <= radius && keep(b, r) {
                    out.push(b);
                }
            }
        }
        out
    }

    /// Lift up to `n` books passing `keep` from within `radius` of `at`, topmost
    /// first; what rested on them falls.
    pub fn gather(&mut self, at: Vec2, radius: f32, n: usize, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let mut found = self.near(at, radius, keep);
        found.sort_by(|&a, &b| self.rest[b as usize].c.y.total_cmp(&self.rest[a as usize].c.y));
        found.truncate(n);
        for &b in &found {
            self.release(b);
        }
        self.resettle(&found);
        found
    }

    /// Lift specific books out (scripts and loading); what rested on them falls.
    pub fn take(&mut self, books: &[u32]) {
        let books: Vec<u32> = books.iter().copied().filter(|&b| self.resting[b as usize]).collect();
        for &b in &books {
            self.release(b);
        }
        self.resettle(&books);
    }

    /// Forget a book without disturbing anything (loading; `rebuild_all` afterwards).
    pub fn forget(&mut self, b: u32) {
        self.release(b);
        self.flying.retain(|f| f.book != b);
    }

    /// The top of the best pile: of the 0.5 m cells holding books, the one `score`
    /// rates lowest, given the cell's centre and how many books lie in it (`None`
    /// rules a cell out).
    pub fn best_pile(&self, mut score: impl FnMut(Vec2, usize) -> Option<f32>) -> Option<Vec3> {
        let mut best: Option<(f32, usize)> = None;
        for (k, bucket) in self.buckets.iter().enumerate() {
            if bucket.is_empty() {
                continue;
            }
            let c = Vec2::new(
                -HALL_X + ((k % self.bw) as f32 + 0.5) * BUCKET,
                -HALL_Z + ((k / self.bw) as f32 + 0.5) * BUCKET,
            );
            if let Some(s) = score(c, bucket.len())
                && best.is_none_or(|(bs, _)| s < bs)
            {
                best = Some((s, k));
            }
        }
        let (_, k) = best?;
        self.buckets[k]
            .iter()
            .map(|&b| self.pose[b as usize].0)
            .max_by(|a, b| a.y.total_cmp(&b.y))
    }

    /// The nearest resting book a ray from `o` along `d` hits within `max_t`.
    pub fn pick(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<(u32, f32)> {
        let mut best: Option<(u32, f32)> = None;
        let test = |bucket: usize, best: &mut Option<(u32, f32)>| {
            for &b in &self.buckets[bucket] {
                let (pos, rot) = self.pose[b as usize];
                let inv = rot.inverse();
                let lo = inv * (o - pos);
                let ld = inv * d;
                let h = self.size[b as usize] * 0.6;
                let inv_d = ld.recip();
                let t1 = (-h - lo) * inv_d;
                let t2 = (h - lo) * inv_d;
                let tmin = t1.min(t2).max_element().max(0.0);
                let tmax = t1.max(t2).min_element();
                if tmax >= tmin && tmin <= max_t && best.is_none_or(|(_, bt)| tmin < bt) {
                    *best = Some((b, tmin));
                }
            }
        };
        // Walk the buckets under the ray (2D DDA in xz).
        let (mut x, mut z) = self.bucket_xy(Vec2::new(o.x, o.z));
        let next = |i: usize, dir: f32, o: f32, half: f32| {
            let edge = -half + (i as f32 + if dir > 0.0 { 1.0 } else { 0.0 }) * BUCKET;
            if dir.abs() < 1e-9 {
                f32::INFINITY
            } else {
                (edge - o) / dir
            }
        };
        let mut tx = next(x, d.x, o.x, HALL_X);
        let mut tz = next(z, d.z, o.z, HALL_Z);
        let dtx = if d.x.abs() < 1e-9 {
            f32::INFINITY
        } else {
            BUCKET / d.x.abs()
        };
        let dtz = if d.z.abs() < 1e-9 {
            f32::INFINITY
        } else {
            BUCKET / d.z.abs()
        };
        loop {
            test(z * self.bw + x, &mut best);
            let t_next = tx.min(tz);
            if t_next > max_t || best.is_some_and(|(_, bt)| bt < t_next) {
                break;
            }
            if tx < tz {
                tx += dtx;
                if d.x > 0.0 {
                    if x + 1 >= self.bw {
                        break;
                    }
                    x += 1;
                } else {
                    if x == 0 {
                        break;
                    }
                    x -= 1;
                }
            } else {
                tz += dtz;
                if d.z > 0.0 {
                    if z + 1 >= self.bh {
                        break;
                    }
                    z += 1;
                } else {
                    if z == 0 {
                        break;
                    }
                    z -= 1;
                }
            }
        }
        best
    }

    // ---------------------------------------------------------------- flight

    /// Launch a book (not resting) from `pos` after `delay` seconds.
    pub fn throw(&mut self, b: u32, pos: Vec3, vel: Vec3, rot: Quat, spin: Vec3, delay: f32) {
        self.flying.push(Flying {
            book: b,
            pos,
            vel,
            rot,
            spin,
            delay,
            age: 0.0,
        });
    }

    /// Advance a flying point by `dt`; true when it has landed.
    fn step(&self, p: &mut Vec3, v: &mut Vec3, dt: f32) -> bool {
        let prev = *p;
        v.y -= GRAVITY * dt;
        *p += *v * dt;
        let bounce = |x: &mut f32, vx: &mut f32, lo: f32, hi: f32| {
            if *x < lo {
                *x = lo;
                *vx = vx.abs() * 0.35;
            } else if *x > hi {
                *x = hi;
                *vx = -vx.abs() * 0.35;
            }
        };
        let (west, east) = match (self.east_open, prev.x < VEIL_X) {
            (true, _) => (-HALL_X + FLY_R, HALL_X - FLY_R),
            (false, true) => (-HALL_X + FLY_R, VEIL_X - FLY_R),
            (false, false) => (VEIL_X + FLY_R, HALL_X - FLY_R),
        };
        bounce(&mut p.x, &mut v.x, west, east);
        bounce(&mut p.z, &mut v.z, -HALL_Z + FLY_R, HALL_Z - FLY_R);
        bounce(&mut p.y, &mut v.y, f32::MIN, HALL_H - FLY_R);
        for &(mn, mx) in &self.solids {
            let (lo, hi) = (mn - Vec3::splat(FLY_R), mx + Vec3::splat(FLY_R));
            if p.x <= lo.x || p.x >= hi.x || p.z <= lo.z || p.z >= hi.z || p.y >= hi.y {
                continue;
            }
            if prev.y >= hi.y - 1e-4 {
                if mx.y <= SHED_HEIGHT {
                    continue; // a table top: the landing test below catches it
                }
                // Off the top of a tall piece, toward its nearest edge that has open
                // floor beyond it (not a wall, the closed veil or more furniture).
                p.y = hi.y;
                v.y = v.y.abs() * 0.25;
                let mut edges = [
                    (p.x - lo.x, Vec2::NEG_X, Vec2::new(lo.x - 0.1, p.z)),
                    (hi.x - p.x, Vec2::X, Vec2::new(hi.x + 0.1, p.z)),
                    (p.z - lo.z, Vec2::NEG_Y, Vec2::new(p.x, lo.z - 0.1)),
                    (hi.z - p.z, Vec2::Y, Vec2::new(p.x, hi.z + 0.1)),
                ];
                edges.sort_by(|a, b| a.0.total_cmp(&b.0));
                let west = prev.x < VEIL_X;
                let open = |q: Vec2| {
                    q.x.abs() < HALL_X - 0.2
                        && q.y.abs() < HALL_Z - 0.2
                        && (self.east_open || (q.x < VEIL_X) == west)
                        && self.base_at(q) <= SHED_HEIGHT
                };
                let out = edges.iter().find(|e| open(e.2)).unwrap_or(&edges[0]).1;
                v.x = v.x * 0.5 + out.x * 1.5;
                v.z = v.z * 0.5 + out.y * 1.5;
                continue;
            }
            // From the side: out along the shallower horizontal axis.
            let px = (p.x - lo.x).min(hi.x - p.x);
            let pz = (p.z - lo.z).min(hi.z - p.z);
            if px < pz {
                if p.x - lo.x < hi.x - p.x {
                    p.x = lo.x;
                    v.x = -v.x.abs() * 0.35;
                } else {
                    p.x = hi.x;
                    v.x = v.x.abs() * 0.35;
                }
                v.z *= 0.7;
            } else {
                if p.z - lo.z < hi.z - p.z {
                    p.z = lo.z;
                    v.z = -v.z.abs() * 0.35;
                } else {
                    p.z = hi.z;
                    v.z = v.z.abs() * 0.35;
                }
                v.x *= 0.7;
            }
        }
        let xz = Vec2::new(p.x, p.z);
        self.base_at(xz) <= SHED_HEIGHT && p.y - FLY_R <= self.height_at(xz) && v.y <= 0.0
    }

    /// The arc a toss from `from` at `vel` would follow (points every 1/20 s) and where it lands.
    pub fn predict(&self, from: Vec3, vel: Vec3, out: &mut Vec<Vec3>) -> Option<Vec3> {
        let (mut p, mut v) = (from, vel);
        let dt = 1.0 / 60.0 / SUBSTEPS as f32;
        for i in 0..60 * 4 * SUBSTEPS {
            if self.step(&mut p, &mut v, dt) {
                return Some(Vec3::new(p.x, self.height_at(Vec2::new(p.x, p.z)), p.z));
            }
            if i % (3 * SUBSTEPS) == 0 {
                out.push(p);
            }
        }
        None
    }

    /// Fly thrown books, ease settling ones, and write render poses.
    pub fn update(&mut self, dt: f32, pos: &mut [Vec3], rot: &mut [Quat]) -> Vec<Landing> {
        let mut landed = Vec::new();
        let mut flying = std::mem::take(&mut self.flying);
        let sub = dt / SUBSTEPS as f32;
        flying.retain_mut(|f| {
            if f.delay > 0.0 {
                // Waiting its turn in the stream, at the launch point.
                f.delay -= dt;
                pos[f.book as usize] = f.pos;
                rot[f.book as usize] = f.rot;
                return true;
            }
            f.age += dt;
            let mut down = false;
            for _ in 0..SUBSTEPS {
                if self.step(&mut f.pos, &mut f.vel, sub) {
                    down = true;
                    break;
                }
            }
            f.rot = (Quat::from_scaled_axis(f.spin * dt) * f.rot).normalize();
            pos[f.book as usize] = f.pos;
            rot[f.book as usize] = f.rot;
            if down || f.age > 8.0 {
                landed.push((f.book, f.pos, f.vel, f.rot));
                return false;
            }
            true
        });
        self.flying = flying;
        let mut landings = Vec::with_capacity(landed.len());
        for (b, p, v, r) in landed {
            self.land(b, p, v, r);
            landings.push(Landing {
                at: p,
                speed: v.length(),
            });
        }

        self.easing.retain_mut(|e| {
            e.t += dt / e.dur;
            let b = e.book as usize;
            let (to_pos, to_rot) = self.pose[b];
            let k = e.t.min(1.0);
            pos[b] = e.from_pos.lerp(to_pos, k * k);
            rot[b] = e.from_rot.slerp(to_rot, k);
            e.t < 1.0
        });
        landings
    }

    /// A flying book touches down at `p`: it skids a little along its travel, then settles.
    fn land(&mut self, b: u32, p: Vec3, vel: Vec3, rot: Quat) {
        let skid = (Vec2::new(vel.x, vel.z) * 0.04).clamp_length_max(0.35);
        let mut at = Vec2::new(p.x, p.z) + skid;
        if !self.east_open {
            // Never skid through the closed veil.
            at.x = if p.x < VEIL_X {
                at.x.min(VEIL_X - 0.01)
            } else {
                at.x.max(VEIL_X + 0.01)
            };
        }
        let along = rot * Vec3::Y;
        let mut u = Vec2::new(along.x, along.z);
        if u.length() < 0.2 {
            let across = rot * Vec3::Z;
            u = Vec2::new(across.x, across.z);
        }
        let u = u.normalize_or(Vec2::X);
        let r = self.drop(b, at, u);
        let bi = b as usize;
        self.rest[bi] = r;
        self.moved[bi] = true;
        self.easing.push(Ease {
            book: b,
            from_pos: p,
            from_rot: rot,
            t: 0.0,
            dur: 0.1,
        });
        self.index(b);
    }

    // ---------------------------------------------------------------- saving

    /// How a resting book lies.
    #[cfg(test)]
    pub fn rest(&self, b: u32) -> Rest {
        self.rest[b as usize]
    }

    /// Resting books that left their generated pose (only these need saving).
    pub fn moved(&self) -> impl Iterator<Item = (u32, Rest)> + '_ {
        (0..self.rest.len() as u32)
            .filter(|&b| self.resting[b as usize] && self.moved[b as usize])
            .map(|b| (b, self.rest[b as usize]))
    }

    /// Books in the air, and where they are.
    pub fn airborne(&self) -> impl Iterator<Item = (u32, Vec3)> + '_ {
        self.flying.iter().map(|f| (f.book, f.pos))
    }

    /// Lay a book down exactly as saved. Call `rebuild_all` once all are placed.
    pub fn restore(&mut self, b: u32, r: Rest) {
        self.forget(b);
        let bi = b as usize;
        // Saved directions are unit already; only repair one that is not.
        let u = if (r.u.length() - 1.0).abs() < 1e-4 {
            r.u
        } else {
            r.u.normalize_or(Vec2::X)
        };
        self.rest[bi] = Rest { u, ..r };
        self.moved[bi] = true;
        self.index(b);
    }
}

/// Join tall furniture boxes that stand edge to edge (a row of bookcases) into one,
/// so a book on top of the row isn't pushed from one case into the next.
fn merge_solids(mut boxes: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    const GAP: f32 = 0.06;
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    loop {
        let mut joined = None;
        'search: for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let ((amn, amx), (bmn, bmx)) = (boxes[i], boxes[j]);
                if amx.y <= SHED_HEIGHT || !near(amx.y, bmx.y) || !near(amn.y, bmn.y) {
                    continue;
                }
                let along_x = near(amn.z, bmn.z) && near(amx.z, bmx.z) && amn.x.max(bmn.x) - amx.x.min(bmx.x) < GAP;
                let along_z = near(amn.x, bmn.x) && near(amx.x, bmx.x) && amn.z.max(bmn.z) - amx.z.min(bmx.z) < GAP;
                if along_x || along_z {
                    joined = Some((i, j));
                    break 'search;
                }
            }
        }
        let Some((i, j)) = joined else { return boxes };
        let (bmn, bmx) = boxes.swap_remove(j);
        boxes[i] = (boxes[i].0.min(bmn), boxes[i].1.max(bmx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Loose {
        /// How far a resting book sits above the support it would settle on now, from
        /// the floor and the books below it: positive means floating, negative sunk.
        fn support_gap(&self, b: u32) -> f32 {
            let r = self.rest[b as usize];
            let (mn, mx) = self.footprint(b);
            let below: Vec<u32> = self
                .buckets_in(mn, mx)
                .flat_map(|k| self.buckets[k].iter().copied())
                .filter(|&x| x != b && !self.above(x, b))
                .collect();
            // As the heightfield sees it: a book supports the cells whose centres it covers.
            let support = |k: usize, _| {
                let c = Self::cell_center(k % self.fw, k / self.fw);
                below
                    .iter()
                    .filter(|&&x| self.inside(x, c))
                    .map(|&x| {
                        let r = self.rest[x as usize];
                        r.under(c) + r.rise(self.size[x as usize].x)
                    })
                    .fold(self.base[k], f32::max)
            };
            let (_, fitted) = self.fit_on(b, r.xz(), r.u, support);
            r.c.y - fitted.c.y
        }

        /// Heightfield cells that differ from a rebuild from scratch.
        pub fn heightfield_errors(&mut self) -> usize {
            let kept = self.top.clone();
            self.rebuild_all();
            let bad = kept
                .iter()
                .zip(&self.top)
                .filter(|(a, b)| (*a - *b).abs() > 1e-5)
                .count();
            self.top = kept;
            bad
        }

        /// Distance from a book's centre into the nearest tall furniture footprint
        /// (positive = overlapping by that much).
        fn furniture_overlap(&self, b: u32) -> f32 {
            let c = self.rest[b as usize].xz();
            let r = self.half(b).length();
            self.solids
                .iter()
                .filter(|(_, mx)| mx.y > SHED_HEIGHT)
                .map(|&(mn, mx)| {
                    let q = Vec2::new(c.x.clamp(mn.x, mx.x), c.y.clamp(mn.z, mx.z));
                    r - q.distance(c)
                })
                .fold(f32::MIN, f32::max)
        }
    }

    fn library() -> (Library, Loose) {
        let lib = Library::generate(crate::game::SEED);
        let loose = Loose::new(&lib);
        (lib, loose)
    }

    fn assert_settled(loose: &mut Loose, books: impl Iterator<Item = u32>) {
        let mut worst = (0.0f32, u32::MAX);
        let mut checked = 0;
        for b in books {
            if !loose.resting[b as usize] {
                continue;
            }
            checked += 1;
            let gap = loose.support_gap(b);
            if gap.abs() > worst.0.abs() {
                worst = (gap, b);
            }
            assert!(loose.furniture_overlap(b) < 0.01, "book {b} overlaps furniture");
            let r = loose.rest[b as usize];
            let h = loose.half(b).length();
            assert!(
                r.c.x.abs() <= HALL_X - h + 1e-3 && r.c.z.abs() <= HALL_Z - h + 1e-3,
                "book {b} outside the hall: {r:?}"
            );
        }
        assert!(checked > 0);
        assert!(
            worst.0.abs() < KEEP_EPS,
            "book {} is {:.3} m off its support: {:?}",
            worst.1,
            worst.0,
            loose.rest[worst.1 as usize]
        );
    }

    #[test]
    fn pose_matches_underside() {
        let size = Vec3::new(0.04, 0.24, 0.17);
        for (su, sv) in [(0.0, 0.0), (0.3, -0.2), (-0.45, 0.45)] {
            let r = Rest {
                c: Vec3::new(1.0, 0.5, -2.0),
                u: Vec2::new(0.6, 0.8),
                su,
                sv,
            };
            let (pos, rot) = r.pose(size);
            for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let corner = pos + rot * (Vec3::new(-size.x, a * size.y, b * size.z) * 0.5);
                let want = r.under(Vec2::new(corner.x, corner.z));
                assert!((corner.y - want).abs() < 1e-4, "{su} {sv}: {} vs {want}", corner.y);
            }
        }
    }

    #[test]
    fn generated_piles_lie_on_something() {
        let (_, mut loose) = library();
        let n = loose.rest.len() as u32;
        assert_settled(&mut loose, (0..n).step_by(3));
    }

    /// Taking books from deep inside piles (as sifting does) leaves nothing hovering.
    #[test]
    fn piles_fall_into_gaps() {
        let (lib, mut loose) = library();
        let cat = 3;
        let ids: Vec<u32> = (0..lib.books.len() as u32)
            .filter(|&b| lib.series[lib.books[b as usize].series as usize].category == cat)
            .collect();
        loose.take(&ids);
        assert!(ids.iter().all(|&b| !loose.is_resting(b)));
        let n = loose.rest.len() as u32;
        assert_settled(&mut loose, 0..n);
    }

    #[test]
    fn tossed_books_land_in_the_open() {
        let (lib, mut loose) = library();
        // Throw books from beside a west-wing pile in every direction, hard and soft,
        // into bookcases, walls and the veil.
        let from = Vec2::new(-16.0, 0.0);
        let top = loose
            .best_pile(|c, _| (c.x < VEIL_X).then(|| c.distance(from)))
            .unwrap();
        let books = loose.gather(Vec2::new(top.x, top.z), 1.5, 120, |_| true);
        assert!(books.len() > 60);
        let from = Vec3::new(top.x, 1.4, top.z + 1.5);
        for (i, &b) in books.iter().enumerate() {
            let a = i as f32 * 0.7;
            let speed = 2.0 + (i % 7) as f32 * 2.0;
            let vel = Vec3::new(a.cos() * speed, 2.0 + (i % 3) as f32 * 2.0, a.sin() * speed);
            loose.throw(b, from, vel, Quat::IDENTITY, Vec3::new(3.0, 7.0, 1.0), i as f32 * 0.01);
        }
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        let mut landed = 0;
        for _ in 0..60 * 10 {
            landed += loose.update(1.0 / 60.0, &mut pos, &mut rot).len();
        }
        assert_eq!(landed, books.len());
        for &b in &books {
            let r = loose.rest[b as usize];
            assert!(loose.is_resting(b));
            assert!(r.c.x < VEIL_X && r.c.x.abs() < HALL_X && r.c.z.abs() < HALL_Z, "{r:?}");
            assert!(r.c.y >= 0.0 && r.c.y < 2.0, "{r:?}");
        }
        assert_settled(&mut loose, books.iter().copied());
    }

    /// Gathers and sifts keep the heightfield exactly what the resting books make
    /// (regression: the rim of each rebuilt area was left at floor height).
    #[test]
    fn heightfield_stays_exact() {
        let (lib, mut loose) = library();
        assert_eq!(loose.heightfield_errors(), 0);
        for (i, (x, z)) in [(-16.0, 0.0), (-20.0, 10.0), (-8.0, -12.0), (-14.0, 14.0)]
            .into_iter()
            .enumerate()
        {
            let at = Vec2::new(x, z);
            let top = loose.best_pile(|c, _| (c.x < VEIL_X).then(|| c.distance(at))).unwrap();
            let p = Vec2::new(top.x, top.z);
            loose.gather(p, 0.6, 20, |_| true);
            let cat = i * 2;
            loose.gather(p, 1.8, 40, |b| {
                lib.series[lib.books[b as usize].series as usize].category == cat
            });
        }
        assert_eq!(loose.heightfield_errors(), 0);
    }

    /// Books thrown at the closed veil, or skidding into it, stay on the thrower's side.
    #[test]
    fn the_closed_veil_holds() {
        let (lib, mut loose) = library();
        let top = loose
            .best_pile(|c, _| (c.x < VEIL_X).then(|| c.distance(Vec2::new(8.0, 12.0))))
            .unwrap();
        let books = loose.gather(Vec2::new(top.x, top.z), 1.5, 30, |_| true);
        for (i, &b) in books.iter().enumerate() {
            let from = Vec3::new(VEIL_X - 0.12, 0.2 + (i % 5) as f32 * 0.4, (i as f32 - 15.0) * 0.3);
            let vel = Vec3::new(4.0 + (i % 4) as f32 * 3.0, -1.0 + (i % 3) as f32, 0.5);
            loose.throw(b, from, vel, Quat::IDENTITY, Vec3::ONE, 0.0);
        }
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        for _ in 0..600 {
            loose.update(1.0 / 60.0, &mut pos, &mut rot);
        }
        for &b in &books {
            assert!(loose.is_resting(b));
            assert!(loose.rest[b as usize].c.x < VEIL_X, "book {b} behind the veil");
        }
    }

    /// Books landing on tall furniture (the fireplace, the seam between two
    /// bookcases) come off it and rest on the floor in the hall.
    #[test]
    fn tall_furniture_sheds_books() {
        let (lib, mut loose) = library();
        loose.east_open = true;
        let top = loose.best_pile(|c, _| Some(c.distance(Vec2::new(-16.0, 0.0)))).unwrap();
        let books = loose.gather(Vec2::new(top.x, top.z), 1.5, 40, |_| true);
        // A row of cases stands at z = -4 (units 2.7 m apart along x); the fireplace
        // top runs z 16.3..18, with the chimney rising from z 17.4.
        let seams = [-17.83f32, -15.13, -20.53];
        for (i, &b) in books.iter().enumerate() {
            let from = match i % 4 {
                0 => Vec3::new(seams[i % 3], 4.0, -4.0),
                1 => Vec3::new(-0.5 + (i % 7) as f32 * 0.2, 3.6, 17.0),
                2 => Vec3::new(2.2, 3.4, 17.8),
                _ => Vec3::new(seams[i % 3] + 0.3, 3.8, -3.8),
            };
            loose.throw(b, from, Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY, Vec3::ZERO, 0.0);
        }
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        for _ in 0..600 {
            loose.update(1.0 / 60.0, &mut pos, &mut rot);
        }
        for &b in &books {
            let r = loose.rest[b as usize];
            assert!(loose.is_resting(b));
            assert!(r.c.y < 1.0, "book {b} stayed up at {r:?}");
        }
        assert_settled(&mut loose, books.iter().copied());
    }

    /// A book whose fall is interrupted by another gather is still drawn where it lies.
    #[test]
    fn falls_finish_where_books_rest() {
        let (lib, mut loose) = library();
        let n = lib.books.len();
        let (mut pos, mut rot): (Vec<Vec3>, Vec<Quat>) = (0..n as u32).map(|b| loose.pose(b)).unzip();
        for (x, z) in [
            (-16.0, 0.0),
            (-20.0, 10.0),
            (-8.0, -12.0),
            (-14.0, 14.0),
            (-6.0, 12.0),
            (-21.0, -14.0),
        ] {
            let at = Vec2::new(x, z);
            let top = loose.best_pile(|c, _| (c.x < VEIL_X).then(|| c.distance(at))).unwrap();
            let p = Vec2::new(top.x, top.z);
            for cat in [0, 1] {
                loose.gather(p, 1.8, 30, |b| {
                    lib.series[lib.books[b as usize].series as usize].category == cat
                });
                loose.update(1.0 / 60.0, &mut pos, &mut rot);
            }
        }
        for _ in 0..120 {
            loose.update(1.0 / 60.0, &mut pos, &mut rot);
        }
        for b in 0..n as u32 {
            if loose.is_resting(b) {
                let d = pos[b as usize].distance(loose.pose(b).0);
                assert!(d < 1e-4, "book {b} drawn {d:.3} m from where it lies");
            }
        }
    }

    #[test]
    fn rays_pick_the_top_book() {
        let (_, loose) = library();
        let mut hits = 0;
        for (x, z) in [(-16.0, 0.0), (-20.0, 10.0), (-8.0, -12.0), (16.0, 5.0)] {
            let Some(top) = loose.best_pile(|c, _| Some(c.distance(Vec2::new(x, z)))) else {
                continue;
            };
            let o = top + Vec3::new(0.05, 3.0, 0.03);
            let (b, t) = loose.pick(o, -Vec3::Y, 10.0).expect("a pile under the ray");
            let hit = o - Vec3::Y * t;
            let surface = loose.height_at(Vec2::new(hit.x, hit.z));
            // The ray meets the topmost book (its box is inflated a little for aiming).
            assert!((hit.y - surface).abs() < 0.06, "hit {hit} surface {surface} book {b}");
            hits += 1;
        }
        assert!(hits >= 3);
    }

    #[test]
    fn gathering_takes_the_top() {
        let (_, mut loose) = library();
        let top = loose.best_pile(|c, _| Some(c.distance(Vec2::new(-16.0, 0.0)))).unwrap();
        let at = Vec2::new(top.x, top.z);
        let before: Vec<(u32, f32)> = loose
            .near(at, 0.5, |_| true)
            .into_iter()
            .map(|b| (b, loose.rest[b as usize].c.y))
            .collect();
        let taken = loose.gather(at, 0.5, 6, |_| true);
        assert_eq!(taken.len(), 6);
        let lowest_taken = taken
            .iter()
            .map(|&b| before.iter().find(|x| x.0 == b).unwrap().1)
            .fold(f32::MAX, f32::min);
        for (b, y) in before {
            if !taken.contains(&b) {
                assert!(y <= lowest_taken + 1e-6, "left book {b} above a taken one");
            }
        }
    }
}
