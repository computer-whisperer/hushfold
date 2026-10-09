//! Loose books: everything that is not shelved or carried lies where it fell.
//!
//! Semi-physics rather than rigid bodies. A thrown book flies ballistically with
//! spin and bounces off walls and furniture; when it lands it settles onto a
//! heightfield of whatever is below and slides downhill while its support is
//! steeper than books rest on, so piles form and spill. Taking books out of a pile
//! re-drops whatever rested on them. Furniture is a solid column from its level's
//! floor: tables hold books, taller pieces (cases, walls, railings, closed gates)
//! shed them off their tops.
//!
//! Each level has its own heightfield and index. Where a level has no floor (a well
//! or a stairwell) its heightfield is a hole: a book flies up into the level above
//! through a hole in that level's floor, and falls through holes to the level below.

use crate::library::Library;
use crate::world::{SLAB, Stairs, Stops, World};
use glam::{Mat3, Quat, Vec2, Vec3};

/// Heightfield resolution.
const CELL: f32 = 0.04;
/// Heightfield cells along a tile's side.
const TILE: usize = 32;
/// Spatial index resolution.
const BUCKET: f32 = 0.5;
/// Steepest support (rise over run) a book stays on; on anything steeper it slides.
const REPOSE: f32 = 0.6;
/// Steepest tilt a resting book takes.
const MAX_TILT: f32 = 0.45;
const SLIDE_STEP: f32 = 0.05;
const MAX_SLIDE_STEPS: usize = 14;
/// Furniture taller than this (above its floor) sheds books off its top rather than
/// holding them.
const SHED_HEIGHT: f32 = 0.9;
pub const GRAVITY: f32 = 9.0;
/// A flying book's collision radius.
const FLY_R: f32 = 0.05;
const SUBSTEPS: usize = 3;
/// A support change smaller than this leaves a resting book exactly where it was.
const KEEP_EPS: f32 = 0.003;
/// Heightfield value where a level has no floor.
const HOLE: f32 = f32::NEG_INFINITY;

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
    level: usize,
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

/// A tile of a level's base heightfield (floor, stairs and furniture).
enum Base {
    Flat(f32),
    Cells(Box<[f32]>),
}

/// One level: its heightfield (in tiles), the index of the books resting on it, and
/// what stands on it.
struct Grid {
    y: f32,
    roof: f32,
    min: Vec2,
    max: Vec2,
    fw: usize,
    fh: usize,
    tw: usize,
    base: Vec<Base>,
    /// `base` plus resting books; `None` where it is just `base`.
    top: Vec<Option<Box<[f32]>>>,
    bw: usize,
    bh: usize,
    /// Resting books by the 0.5 m cells their footprints overlap.
    buckets: Vec<Vec<u32>>,
    slabs: Vec<(Vec2, Vec2)>,
    stairs: Vec<Stairs>,
    /// Where the space reaches higher than `roof` (a skylit well).
    raised: Vec<(Vec2, Vec2, f32)>,
    /// Furniture standing on this level (min, max), rows of tall pieces merged.
    solids: Vec<(Vec3, Vec3)>,
    /// Closed gates on this level: (gate, min, max).
    gates: Vec<(usize, Vec3, Vec3)>,
}

fn xz(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.z)
}

impl Grid {
    fn new(world: &World, l: usize) -> Grid {
        let lv = &world.levels[l];
        let size = lv.max - lv.min;
        let fw = (size.x / CELL).ceil() as usize;
        let fh = (size.y / CELL).ceil() as usize;
        let (tw, th) = (fw.div_ceil(TILE), fh.div_ceil(TILE));
        let bw = (size.x / BUCKET).ceil() as usize;
        let bh = (size.y / BUCKET).ceil() as usize;
        let solids = (world.solids_on(l))
            .filter(|o| o.stops.has(Stops::BOOKS))
            .map(|o| (o.min, o.max))
            .collect();
        let mut g = Grid {
            y: lv.y,
            roof: lv.roof,
            min: lv.min,
            max: lv.max,
            fw,
            fh,
            tw,
            base: Vec::new(),
            top: (0..tw * th).map(|_| None).collect(),
            bw,
            bh,
            buckets: vec![Vec::new(); bw * bh],
            slabs: (world.slabs.iter())
                .filter(|s| s.level == l)
                .map(|s| (s.min, s.max))
                .collect(),
            stairs: world.stairs().iter().filter(|f| f.level == l).copied().collect(),
            raised: (world.raised_roofs.iter())
                .filter(|r| r.0 == l)
                .map(|&(_, mn, mx, y)| (mn, mx, y))
                .collect(),
            solids: merge_solids(solids, lv.y),
            gates: (world.closed_gates())
                .filter(|(_, g)| g.level == l)
                .map(|(i, g)| (i, g.min, g.max))
                .collect(),
        };
        g.base = (0..tw * th).map(|t| g.build_base(t)).collect();
        g
    }

    /// How high this level's space reaches at `p` where no floor above caps it.
    fn roof_at(&self, p: Vec2) -> f32 {
        (self.raised.iter())
            .filter(|(mn, mx, _)| p.cmpge(*mn).all() && p.cmple(*mx).all())
            .map(|r| r.2)
            .fold(self.roof, f32::max)
    }

    fn tall(&self, mx: Vec3) -> bool {
        mx.y - self.y > SHED_HEIGHT
    }

    /// Furniture and closed gates.
    fn all_solids(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        (self.solids.iter().copied()).chain(self.gates.iter().map(|&(_, a, b)| (a, b)))
    }

    /// The base heightfield of tile `t`, from the floor, stairs and furniture over it.
    fn build_base(&self, t: usize) -> Base {
        let (tx, tz) = (t % self.tw, t / self.tw);
        let lo = self.cell_center(tx * TILE, tz * TILE) - Vec2::splat(CELL);
        let hi = lo + Vec2::splat((TILE + 1) as f32 * CELL);
        let over = |mn: Vec2, mx: Vec2| mn.x <= hi.x && mx.x >= lo.x && mn.y <= hi.y && mx.y >= lo.y;
        let slabs: Vec<(Vec2, Vec2)> = self.slabs.iter().copied().filter(|&(a, b)| over(a, b)).collect();
        let stairs: Vec<&Stairs> = self.stairs.iter().filter(|f| over(f.min, f.max)).collect();
        let solids: Vec<(Vec3, Vec3)> = self.all_solids().filter(|&(a, b)| over(xz(a), xz(b))).collect();
        let mut cells = vec![HOLE; TILE * TILE];
        for dz in 0..TILE {
            for dx in 0..TILE {
                let c = self.cell_center(tx * TILE + dx, tz * TILE + dz);
                let mut h = if slabs.iter().any(|&(a, b)| c.cmpge(a).all() && c.cmple(b).all()) {
                    self.y
                } else {
                    HOLE
                };
                for f in &stairs {
                    if f.contains(c) {
                        h = h.max(f.tread(c));
                    }
                }
                for &(mn, mx) in &solids {
                    if c.x >= mn.x && c.x <= mx.x && c.y >= mn.z && c.y <= mx.z {
                        h = h.max(mx.y);
                    }
                }
                cells[dz * TILE + dx] = h;
            }
        }
        if cells.iter().all(|&v| v == cells[0]) {
            Base::Flat(cells[0])
        } else {
            Base::Cells(cells.into_boxed_slice())
        }
    }

    fn cell_xy(&self, p: Vec2) -> (usize, usize) {
        let x = ((p.x - self.min.x) / CELL).floor().clamp(0.0, (self.fw - 1) as f32) as usize;
        let z = ((p.y - self.min.y) / CELL).floor().clamp(0.0, (self.fh - 1) as f32) as usize;
        (x, z)
    }

    fn cell_center(&self, x: usize, z: usize) -> Vec2 {
        Vec2::new(
            self.min.x + (x as f32 + 0.5) * CELL,
            self.min.y + (z as f32 + 0.5) * CELL,
        )
    }

    fn cell(&self, p: Vec2) -> usize {
        let (x, z) = self.cell_xy(p);
        z * self.fw + x
    }

    /// Tile and offset within it of cell `k`.
    fn tile_of(&self, k: usize) -> (usize, usize) {
        let (x, z) = (k % self.fw, k / self.fw);
        ((z / TILE) * self.tw + x / TILE, (z % TILE) * TILE + x % TILE)
    }

    fn base_k(&self, k: usize) -> f32 {
        let (t, o) = self.tile_of(k);
        match &self.base[t] {
            Base::Flat(v) => *v,
            Base::Cells(c) => c[o],
        }
    }

    fn top_k(&self, k: usize) -> f32 {
        let (t, o) = self.tile_of(k);
        match &self.top[t] {
            Some(c) => c[o],
            None => self.base_k(k),
        }
    }

    /// Raise cell `k` to at least `v`.
    fn raise(&mut self, k: usize, v: f32) {
        let (t, o) = self.tile_of(k);
        let top = match &mut self.top[t] {
            Some(c) => c,
            slot => slot.insert(match &self.base[t] {
                Base::Flat(v) => vec![*v; TILE * TILE].into_boxed_slice(),
                Base::Cells(c) => c.clone(),
            }),
        };
        top[o] = top[o].max(v);
    }

    /// Reset the cells `x0..=x1` × `z0..=z1` to the base.
    fn reset(&mut self, x0: usize, z0: usize, x1: usize, z1: usize) {
        for tz in z0 / TILE..=z1 / TILE {
            for tx in x0 / TILE..=x1 / TILE {
                let t = tz * self.tw + tx;
                let (ax, az) = (tx * TILE, tz * TILE);
                if x0 <= ax && z0 <= az && x1 >= ax + TILE - 1 && z1 >= az + TILE - 1 {
                    self.top[t] = None;
                    continue;
                }
                let Some(top) = &mut self.top[t] else { continue };
                for z in z0.max(az)..=z1.min(az + TILE - 1) {
                    for x in x0.max(ax)..=x1.min(ax + TILE - 1) {
                        let o = (z - az) * TILE + (x - ax);
                        top[o] = match &self.base[t] {
                            Base::Flat(v) => *v,
                            Base::Cells(c) => c[o],
                        };
                    }
                }
            }
        }
    }

    /// Height of the surface (floor, stairs, furniture or books) at `p`.
    fn height(&self, p: Vec2) -> f32 {
        self.top_k(self.cell(p))
    }

    fn base_at(&self, p: Vec2) -> f32 {
        self.base_k(self.cell(p))
    }

    fn is_hole(&self, p: Vec2) -> bool {
        self.base_at(p) == HOLE
    }

    fn bucket_xy(&self, p: Vec2) -> (usize, usize) {
        let x = ((p.x - self.min.x) / BUCKET).floor().clamp(0.0, (self.bw - 1) as f32) as usize;
        let z = ((p.y - self.min.y) / BUCKET).floor().clamp(0.0, (self.bh - 1) as f32) as usize;
        (x, z)
    }

    /// Bucket indices overlapping the rectangle `mn..mx`.
    fn buckets_in(&self, mn: Vec2, mx: Vec2) -> impl Iterator<Item = usize> + use<> {
        let (x0, z0) = self.bucket_xy(mn);
        let (x1, z1) = self.bucket_xy(mx);
        let bw = self.bw;
        (z0..=z1).flat_map(move |z| (x0..=x1).map(move |x| z * bw + x))
    }

    /// Heightfield cells whose centres lie inside a footprint with half extents `h`,
    /// with each centre in the footprint's frame.
    fn cells_under(&self, r: Rest, h: Vec2) -> impl Iterator<Item = (usize, Vec2)> + use<> {
        let v = r.v();
        let ext = r.u.abs() * h.x + v.abs() * h.y;
        let (x0, z0) = self.cell_xy(r.xz() - ext);
        let (x1, z1) = self.cell_xy(r.xz() + ext);
        let (fw, min) = (self.fw, self.min);
        (z0..=z1)
            .flat_map(move |z| (x0..=x1).map(move |x| (x, z)))
            .filter_map(move |(x, z)| {
                let c = Vec2::new(min.x + (x as f32 + 0.5) * CELL, min.y + (z as f32 + 0.5) * CELL);
                let l = r.local(c);
                (l.x.abs() <= h.x && l.y.abs() <= h.y).then_some((z * fw + x, l))
            })
    }

    /// True if the segment `a`–`b` crosses the footprint of a tall solid or a closed gate.
    fn crosses_tall(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        let inv = Vec2::new(1.0 / d.x, 1.0 / d.y);
        self.all_solids().filter(|&(_, mx)| self.tall(mx)).any(|(mn, mx)| {
            let t1 = (xz(mn) - a) * inv;
            let t2 = (xz(mx) - a) * inv;
            let tmin = t1.min(t2).max_element().max(0.0);
            let tmax = t1.max(t2).min_element().min(1.0);
            tmax >= tmin
        })
    }
}

pub struct Loose {
    size: Vec<Vec3>,
    rest: Vec<Rest>,
    /// The level each book rests on.
    level: Vec<usize>,
    /// Pose of each resting book, cached from `rest`.
    pose: Vec<(Vec3, Quat)>,
    resting: Vec<bool>,
    /// Left its generated pose (only these are saved).
    moved: Vec<bool>,
    grids: Vec<Grid>,
    /// Every gate: (level, min, max).
    gate_boxes: Vec<(usize, Vec3, Vec3)>,
    flying: Vec<Flying>,
    easing: Vec<Ease>,
    /// In `easing`.
    eased: Vec<bool>,
    /// Bumped whenever how a book is drawn may have changed (it came to rest, left
    /// its rest, or began or ended a fall), so the renderer can redraw just those.
    version: Vec<u32>,
    mark: Vec<u32>,
    stamp: u32,
}

impl Loose {
    /// Drop every book where the library's generator put it (gates as the world has
    /// them).
    pub fn new(lib: &Library) -> Self {
        let n = lib.books.len();
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
            level: vec![0; n],
            pose: vec![(Vec3::ZERO, Quat::IDENTITY); n],
            resting: vec![false; n],
            moved: vec![false; n],
            grids: (0..lib.world.levels.len()).map(|l| Grid::new(&lib.world, l)).collect(),
            gate_boxes: lib.world.gates().iter().map(|g| (g.level, g.min, g.max)).collect(),
            flying: Vec::new(),
            easing: Vec::new(),
            eased: vec![false; n],
            version: vec![0; n],
            mark: vec![0; n],
            stamp: 0,
        };
        for &(b, lv, at, u) in &lib.drops {
            // A pile spilling over an edge settles on the floor below.
            let mut lv = lv;
            let r = loop {
                match loose.drop(lv, b, at, u) {
                    Ok(r) => break r,
                    Err(_) if lv > 0 => lv -= 1,
                    Err(_) => panic!("book {b} dropped where the ground floor has no floor"),
                }
            };
            loose.rest[b as usize] = r;
            loose.level[b as usize] = lv;
            loose.index(b);
        }
        loose
    }

    /// Close or open gate `i`: a closed gate is a wall to books.
    pub fn set_gate(&mut self, i: usize, closed: bool) {
        let (lv, mn, mx) = self.gate_boxes[i];
        let g = &mut self.grids[lv];
        match (closed, g.gates.iter().position(|x| x.0 == i)) {
            (true, None) => g.gates.push((i, mn, mx)),
            (false, Some(j)) => {
                g.gates.remove(j);
            }
            _ => return,
        }
        let (x0, z0) = g.cell_xy(xz(mn));
        let (x1, z1) = g.cell_xy(xz(mx));
        for tz in z0 / TILE..=z1 / TILE {
            for tx in x0 / TILE..=x1 / TILE {
                let t = tz * g.tw + tx;
                g.base[t] = g.build_base(t);
            }
        }
        self.rebuild(lv, xz(mn) - Vec2::splat(CELL), xz(mx) + Vec2::splat(CELL));
    }

    /// The level whose space `p` is in: above its floor, or in a hole through it.
    fn airspace(&self, p: Vec3) -> usize {
        let at = Vec2::new(p.x, p.z);
        (0..self.grids.len())
            .rev()
            .find(|&l| {
                let g = &self.grids[l];
                p.y >= g.y - 0.02 || (p.y >= g.y - SLAB && g.is_hole(at))
            })
            .unwrap_or(0)
    }

    /// Height of the surface (floor, table or books) at `p` on level `lv`.
    pub fn height_at(&self, lv: usize, p: Vec2) -> f32 {
        self.grids[lv].height(p)
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

    /// The level a resting book lies on.
    pub fn level_of(&self, b: u32) -> usize {
        self.level[b as usize]
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

    fn grid_of(&self, b: u32) -> &Grid {
        &self.grids[self.level[b as usize]]
    }

    /// Changes whenever how book `b` is drawn may have.
    pub fn version(&self, b: u32) -> u32 {
        self.version[b as usize]
    }

    /// Resting and drawn where it rests (not partway through a fall).
    pub fn settled(&self, b: u32) -> bool {
        self.resting[b as usize] && !self.eased[b as usize]
    }

    fn touch(&mut self, b: u32) {
        let v = &mut self.version[b as usize];
        *v = v.wrapping_add(1);
    }

    fn start_ease(&mut self, e: Ease) {
        self.eased[e.book as usize] = true;
        self.touch(e.book);
        self.easing.push(e);
    }

    fn stop_ease(&mut self, b: u32) -> Option<Ease> {
        let i = self.easing.iter().position(|e| e.book == b)?;
        self.eased[b as usize] = false;
        self.touch(b);
        Some(self.easing.swap_remove(i))
    }

    /// Add a book at its `rest` (on its `level`) to the index and the heightfield.
    fn index(&mut self, b: u32) {
        let bi = b as usize;
        self.touch(b);
        self.resting[bi] = true;
        self.pose[bi] = self.rest[bi].pose(self.size[bi]);
        let (mn, mx) = self.footprint(b);
        let g = &mut self.grids[self.level[bi]];
        for k in g.buckets_in(mn, mx) {
            g.buckets[k].push(b);
        }
        self.raster(b, None);
    }

    /// Take a book out of the index. The heightfield is left for the caller to rebuild,
    /// and any fall still being drawn carries on (a re-drop picks it up).
    fn unindex(&mut self, b: u32) {
        if !self.resting[b as usize] {
            return;
        }
        self.touch(b);
        self.resting[b as usize] = false;
        let (mn, mx) = self.footprint(b);
        let g = &mut self.grids[self.level[b as usize]];
        for k in g.buckets_in(mn, mx) {
            if let Some(i) = g.buckets[k].iter().position(|&x| x == b) {
                g.buckets[k].swap_remove(i);
            }
        }
    }

    /// A book leaves the piles for good (taken or forgotten): stop drawing its fall.
    fn release(&mut self, b: u32) {
        self.unindex(b);
        self.stop_ease(b);
    }

    /// Raise the heightfield to a book's top, on the cells it covers (only those in the
    /// cell rectangle `clip`, if given).
    fn raster(&mut self, b: u32, clip: Option<(usize, usize, usize, usize)>) {
        let r = self.rest[b as usize];
        let rise = r.rise(self.size[b as usize].x);
        let h = self.half(b);
        let g = &mut self.grids[self.level[b as usize]];
        let fw = g.fw;
        let inside = |k: usize| {
            clip.is_none_or(|(x0, z0, x1, z1)| (x0..=x1).contains(&(k % fw)) && (z0..=z1).contains(&(k / fw)))
        };
        let mut any = false;
        for (k, l) in g.cells_under(r, h) {
            any = true;
            if inside(k) {
                g.raise(k, r.c.y + l.x * r.su + l.y * r.sv + rise);
            }
        }
        // A footprint smaller than a cell stands on the cell under its centre.
        let k = g.cell(r.xz());
        if !any && inside(k) {
            g.raise(k, r.c.y + rise);
        }
    }

    /// Rebuild level `lv`'s heightfield over `lo..hi` from the floor and the books
    /// still resting.
    fn rebuild(&mut self, lv: usize, lo: Vec2, hi: Vec2) {
        let g = &mut self.grids[lv];
        let (x0, z0) = g.cell_xy(lo);
        let (x1, z1) = g.cell_xy(hi);
        g.reset(x0, z0, x1, z1);
        self.stamp += 1;
        let mut books = Vec::new();
        for k in g.buckets_in(lo, hi) {
            for &b in &g.buckets[k] {
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
    fn fit(&self, lv: usize, b: u32, c: Vec2, u: Vec2) -> (Vec2, Rest) {
        let g = &self.grids[lv];
        self.fit_on(lv, b, c, u, |k, _| g.top_k(k))
    }

    /// `fit` against any support surface, read at the heightfield cells whose centres
    /// the book covers (the cells its own top is rastered into). Holes give no support;
    /// a side with none at all tips the book toward it.
    fn fit_on(&self, lv: usize, b: u32, c: Vec2, u: Vec2, support: impl Fn(usize, Vec2) -> f32) -> (Vec2, Rest) {
        let g = &self.grids[lv];
        let h = self.half(b);
        let probe = Rest {
            c: Vec3::new(c.x, 0.0, c.y),
            u,
            su: 0.0,
            sv: 0.0,
        };
        // A footprint smaller than a cell stands on the cell under its centre.
        let fallback = [(g.cell(c), Vec2::ZERO)];
        let small = g.cells_under(probe, h).next().is_none();
        let cells = || {
            g.cells_under(probe, h)
                .chain(fallback.iter().copied().filter(move |_| small))
        };
        // Highest support under each half, along and across.
        let (mut ha, mut hb, mut hc, mut hd) = (HOLE, HOLE, HOLE, HOLE);
        for (k, l) in cells() {
            let s = support(k, l);
            if small {
                (ha, hb, hc, hd) = (s, s, s, s);
            } else {
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
        }
        let slope = |lo: f32, hi: f32, half: f32| match (lo == HOLE, hi == HOLE) {
            (true, true) => 0.0,
            // Nothing under one side: steeply toward it.
            (true, false) => 10.0,
            (false, true) => -10.0,
            _ => (hi - lo) / half,
        };
        let gr = Vec2::new(slope(ha, hb, h.x), slope(hc, hd, h.y));
        let su = gr.x.clamp(-MAX_TILT, MAX_TILT);
        let sv = gr.y.clamp(-MAX_TILT, MAX_TILT);
        let mut y = ((ha + hb) * 0.5).max((hc + hd) * 0.5);
        // Never sink below any support under the tilted plane.
        for (k, l) in cells() {
            y = y.max(support(k, l) - l.x * su - l.y * sv);
        }
        (
            gr,
            Rest {
                c: Vec3::new(c.x, y, c.y),
                u,
                su,
                sv,
            },
        )
    }

    /// Keep a book's centre far enough from the level's walls, tall furniture and
    /// closed gates that it can lie flat beside them.
    fn keep_clear(&self, lv: usize, b: u32, mut c: Vec2) -> Vec2 {
        let g = &self.grids[lv];
        let r = self.half(b).length();
        let walls = |mut c: Vec2| {
            c.x = c.x.clamp(g.min.x + r, g.max.x - r);
            c.y = c.y.clamp(g.min.y + r, g.max.y - r);
            c
        };
        // Pushing out of one piece can push into a wall or another piece; settle it.
        for _ in 0..4 {
            c = walls(c);
            let before = c;
            for (mn, mx) in g.all_solids() {
                if !g.tall(mx) {
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

    /// Where a book dropped at `c` (length along `u`) on level `lv` comes to rest: it
    /// slides downhill while its support is too steep, then fits what is under it.
    /// `Err` is where it slid over an edge (its centre over a hole), to fall from.
    /// A hole is judged by the surface books land on, so another book overhanging the
    /// edge holds it, as the landing test in `step` says.
    fn drop(&self, lv: usize, b: u32, c: Vec2, u: Vec2) -> Result<Rest, Vec2> {
        let g = &self.grids[lv];
        let open = |c: Vec2| g.height(c) == HOLE;
        let mut c = self.keep_clear(lv, b, c);
        let v = Vec2::new(u.y, -u.x);
        for _ in 0..MAX_SLIDE_STEPS {
            if open(c) {
                return Err(c);
            }
            let (gr, _) = self.fit(lv, b, c, u);
            if gr.x.abs().max(gr.y.abs()) <= REPOSE {
                break;
            }
            let dir = -(u * gr.x + v * gr.y).normalize_or_zero();
            let next = self.keep_clear(lv, b, c + dir * SLIDE_STEP);
            if next.distance(c) < 1e-3 {
                break;
            }
            c = next;
        }
        if open(c) {
            return Err(c);
        }
        Ok(self.fit(lv, b, c, u).1)
    }

    /// True if book `y` lies above book `x` (both on one level): on some heightfield
    /// cell both cover, y's underside is at or above x's top. Whatever lies above a
    /// removed book must be lifted with it, not just what touches it, or a re-dropped
    /// book would see books over it as support.
    fn above(&self, y: u32, x: u32) -> bool {
        let (ry, rx) = (self.rest[y as usize], self.rest[x as usize]);
        let ((ymn, ymx), (xmn, xmx)) = (self.footprint(y), self.footprint(x));
        if ymn.x > xmx.x || xmn.x > ymx.x || ymn.y > xmx.y || xmn.y > ymx.y {
            return false;
        }
        let rise = rx.rise(self.size[x as usize].x);
        let hx = self.half(x);
        let g = self.grid_of(y);
        let fw = g.fw;
        g.cells_under(ry, self.half(y)).any(|(k, _)| {
            let p = g.cell_center(k % fw, k / fw);
            let l = rx.local(p);
            l.x.abs() <= hx.x && l.y.abs() <= hx.y && ry.under(p) >= rx.under(p) + rise - 0.012
        })
    }

    /// Books that rested on `removed` (already out of the index) fall: everything
    /// above them is lifted out, the heightfield rebuilt, and the lot re-dropped, each
    /// after the books below it. A book whose support did not change keeps its exact
    /// pose.
    pub fn resettle(&mut self, removed: &[u32]) {
        for lv in 0..self.grids.len() {
            let on: Vec<u32> = (removed.iter().copied())
                .filter(|&b| self.level[b as usize] == lv)
                .collect();
            if !on.is_empty() {
                self.resettle_on(lv, on);
            }
        }
    }

    fn resettle_on(&mut self, lv: usize, removed: Vec<u32>) {
        self.stamp += 1;
        let stamp = self.stamp;
        let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
        let mut set: Vec<u32> = Vec::new();
        let mut queue = removed;
        while let Some(x) = queue.pop() {
            let (mn, mx) = self.footprint(x);
            lo = lo.min(mn);
            hi = hi.max(mx);
            let g = &self.grids[lv];
            for k in g.buckets_in(mn, mx) {
                for i in 0..g.buckets[k].len() {
                    let y = g.buckets[k][i];
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
        self.rebuild(lv, lo, hi);
        for y in order {
            self.redrop(y);
        }
    }

    /// `set` (books on one level marked with `stamp`, still indexed) ordered so each
    /// book comes after the books in the set below it; ties and cycles go lowest first.
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
            let g = self.grid_of(y);
            for k in g.buckets_in(mn, mx) {
                for &x in &g.buckets[k] {
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
        // Order by height; total_cmp orders floats by value, including negatives.
        let key = |i: usize| Reverse((OrdF32(self.rest[set[i] as usize].c.y), i));
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

    /// Where a book is drawn now: its rest, or partway through a fall.
    fn drawn(&mut self, b: u32) -> (Vec3, Quat) {
        let mut at = self.pose[b as usize];
        if let Some(e) = self.stop_ease(b) {
            let k = e.t.min(1.0);
            at = (e.from_pos.lerp(at.0, k * k), e.from_rot.slerp(at.1, k));
        }
        at
    }

    /// Put a book back into the index, dropping it if its support has changed.
    fn redrop(&mut self, b: u32) {
        let bi = b as usize;
        let lv = self.level[bi];
        let old = self.rest[bi];
        let (_, fitted) = self.fit(lv, b, old.xz(), old.u);
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
            let from = self.drawn(b);
            self.moved[bi] = true;
            match self.drop(lv, b, old.xz(), old.u) {
                Ok(r) => self.rest[bi] = r,
                Err(edge) => {
                    // Its support is gone from under it: over the edge it goes.
                    self.fall(b, lv, Vec3::new(edge.x, from.0.y, edge.y), from.1);
                    return;
                }
            }
            // Falls take the time a short drop would.
            let fall = (old.c.y - self.rest[bi].c.y).max(0.02);
            let dur = (2.0 * fall / GRAVITY).sqrt().clamp(0.08, 0.5);
            self.start_ease(Ease {
                book: b,
                from_pos: from.0,
                from_rot: from.1,
                t: 0.0,
                dur,
            });
        }
        self.index(b);
    }

    /// Rebuild every heightfield from the books resting now (after a load).
    pub fn rebuild_all(&mut self) {
        for lv in 0..self.grids.len() {
            let (lo, hi) = (self.grids[lv].min, self.grids[lv].max);
            self.rebuild(lv, lo, hi);
        }
    }

    // ---------------------------------------------------------------- taking

    /// Resting books on level `lv` whose centres lie within `radius` of `at` (with no
    /// wall, case or closed gate between), passing `keep`.
    pub fn near(&mut self, lv: usize, at: Vec2, radius: f32, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        self.stamp += 1;
        let mut out = Vec::new();
        let r = Vec2::splat(radius);
        let g = &self.grids[lv];
        for k in g.buckets_in(at - r, at + r) {
            for &b in &g.buckets[k] {
                if self.mark[b as usize] == self.stamp {
                    continue;
                }
                self.mark[b as usize] = self.stamp;
                let c = self.rest[b as usize].xz();
                if c.distance(at) <= radius && keep(b) && !g.crosses_tall(at, c) {
                    out.push(b);
                }
            }
        }
        out
    }

    /// Lift up to `n` books passing `keep` from within `radius` of `at` on level `lv`,
    /// topmost first; what rested on them falls.
    pub fn gather(&mut self, lv: usize, at: Vec2, radius: f32, n: usize, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let mut found = self.near(lv, at, radius, keep);
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
    /// rates lowest, given the level, the cell's centre at floor height and the books
    /// lying in it (`None` rules a cell out). Returns the level and the top.
    pub fn best_pile(&self, mut score: impl FnMut(usize, Vec3, &[u32]) -> Option<f32>) -> Option<(usize, Vec3)> {
        let mut best: Option<(f32, usize, Vec3)> = None;
        for (lv, c, books) in self.piles() {
            if let Some(s) = score(lv, c, books)
                && best.is_none_or(|(bs, _, _)| s < bs)
            {
                let top = books
                    .iter()
                    .map(|&b| self.pose[b as usize].0)
                    .max_by(|a, b| a.y.total_cmp(&b.y));
                best = Some((s, lv, top.unwrap_or(c)));
            }
        }
        best.map(|(_, lv, top)| (lv, top))
    }

    /// Every 0.5 m cell holding books: its level, its centre at floor height, and
    /// the books lying in it.
    pub fn piles(&self) -> impl Iterator<Item = (usize, Vec3, &[u32])> + '_ {
        (self.grids.iter().enumerate()).flat_map(|(lv, g)| {
            (g.buckets.iter().enumerate())
                .filter(|(_, bucket)| !bucket.is_empty())
                .map(move |(k, bucket)| {
                    let c = Vec3::new(
                        g.min.x + ((k % g.bw) as f32 + 0.5) * BUCKET,
                        g.y,
                        g.min.y + ((k / g.bw) as f32 + 0.5) * BUCKET,
                    );
                    (lv, c, bucket.as_slice())
                })
        })
    }

    /// The nearest resting book a ray from `o` along `d` hits within `max_t`.
    pub fn pick(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<(u32, f32)> {
        let mut best: Option<(u32, f32)> = None;
        for g in &self.grids {
            self.pick_on(g, o, d, max_t, &mut best);
        }
        best
    }

    fn pick_on(&self, g: &Grid, o: Vec3, d: Vec3, max_t: f32, best: &mut Option<(u32, f32)>) {
        let test = |bucket: usize, best: &mut Option<(u32, f32)>| {
            for &b in &g.buckets[bucket] {
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
        let (mut x, mut z) = g.bucket_xy(Vec2::new(o.x, o.z));
        let next = |i: usize, dir: f32, o: f32, min: f32| {
            let edge = min + (i as f32 + if dir > 0.0 { 1.0 } else { 0.0 }) * BUCKET;
            if dir.abs() < 1e-9 {
                f32::INFINITY
            } else {
                (edge - o) / dir
            }
        };
        let mut tx = next(x, d.x, o.x, g.min.x);
        let mut tz = next(z, d.z, o.z, g.min.y);
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
            test(z * g.bw + x, best);
            let t_next = tx.min(tz);
            if t_next > max_t || best.is_some_and(|(_, bt)| bt < t_next) {
                break;
            }
            if tx < tz {
                tx += dtx;
                if d.x > 0.0 {
                    if x + 1 >= g.bw {
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
                    if z + 1 >= g.bh {
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
    }

    // ---------------------------------------------------------------- flight

    /// Launch a book (not resting) from `pos` after `delay` seconds.
    pub fn throw(&mut self, b: u32, pos: Vec3, vel: Vec3, rot: Quat, spin: Vec3, delay: f32) {
        self.flying.push(Flying {
            book: b,
            level: self.airspace(pos),
            pos,
            vel,
            rot,
            spin,
            delay,
            age: 0.0,
        });
    }

    /// A book off an edge on level `lv` falls from `pos`.
    fn fall(&mut self, b: u32, lv: usize, pos: Vec3, rot: Quat) {
        debug_assert!(lv > 0, "the ground floor has no edges to fall off");
        self.flying.push(Flying {
            book: b,
            level: lv,
            pos,
            vel: Vec3::ZERO,
            rot,
            spin: Vec3::ZERO,
            delay: 0.0,
            age: 0.0,
        });
    }

    /// Advance a flying point by `dt` on level `lv` (which it leaves through holes);
    /// true when it has landed.
    fn step(&self, lv: &mut usize, p: &mut Vec3, v: &mut Vec3, dt: f32) -> bool {
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
        // Down through a hole into the level below. A level's space starts at the
        // underside of its floor, through holes as elsewhere.
        while *lv > 0 && p.y < self.grids[*lv].y - SLAB && self.grids[*lv].is_hole(Vec2::new(p.x, p.z)) {
            *lv -= 1;
        }
        // Up: through a hole into the level above, or off its floor's underside.
        let at = Vec2::new(p.x, p.z);
        match self.grids.get(*lv + 1) {
            Some(up) if up.is_hole(at) => {
                if p.y >= up.y - SLAB {
                    *lv += 1;
                }
            }
            Some(up) => bounce(&mut p.y, &mut v.y, f32::MIN, up.y - SLAB - FLY_R),
            None => bounce(&mut p.y, &mut v.y, f32::MIN, self.grids[*lv].roof_at(at) - FLY_R),
        }
        let g = &self.grids[*lv];
        bounce(&mut p.x, &mut v.x, g.min.x + FLY_R, g.max.x - FLY_R);
        bounce(&mut p.z, &mut v.z, g.min.y + FLY_R, g.max.y - FLY_R);
        for (mn, mx) in g.all_solids() {
            let (lo, hi) = (mn - Vec3::splat(FLY_R), mx + Vec3::splat(FLY_R));
            if p.x <= lo.x || p.x >= hi.x || p.z <= lo.z || p.z >= hi.z || p.y >= hi.y {
                continue;
            }
            if prev.y >= hi.y - 1e-4 {
                if !g.tall(mx) {
                    continue; // a table top: the landing test below catches it
                }
                // Off the top of a tall piece, toward its nearest edge that has open
                // floor (or a drop) beyond it, not a wall or more furniture.
                p.y = hi.y;
                v.y = v.y.abs() * 0.25;
                let mut edges = [
                    (p.x - lo.x, Vec2::NEG_X, Vec2::new(lo.x - 0.1, p.z)),
                    (hi.x - p.x, Vec2::X, Vec2::new(hi.x + 0.1, p.z)),
                    (p.z - lo.z, Vec2::NEG_Y, Vec2::new(p.x, lo.z - 0.1)),
                    (hi.z - p.z, Vec2::Y, Vec2::new(p.x, hi.z + 0.1)),
                ];
                edges.sort_by(|a, b| a.0.total_cmp(&b.0));
                let open = |q: Vec2| {
                    q.cmpgt(g.min + 0.2).all() && q.cmplt(g.max - 0.2).all() && g.base_at(q) - g.y <= SHED_HEIGHT
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
        let at = Vec2::new(p.x, p.z);
        let on_tall = (g.all_solids()).any(|(mn, mx)| g.tall(mx) && at.cmpge(xz(mn)).all() && at.cmple(xz(mx)).all());
        !on_tall && p.y - FLY_R <= g.height(at) && v.y <= 0.0
    }

    /// The arc a toss from `from` at `vel` would follow (points every 1/20 s) and where it lands.
    pub fn predict(&self, from: Vec3, vel: Vec3, out: &mut Vec<Vec3>) -> Option<Vec3> {
        let (mut p, mut v) = (from, vel);
        let mut lv = self.airspace(from);
        let dt = 1.0 / 60.0 / SUBSTEPS as f32;
        for i in 0..60 * 4 * SUBSTEPS {
            if self.step(&mut lv, &mut p, &mut v, dt) {
                let at = Vec2::new(p.x, p.z);
                return Some(Vec3::new(p.x, self.grids[lv].height(at), p.z));
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
                if self.step(&mut f.level, &mut f.pos, &mut f.vel, sub) {
                    down = true;
                    break;
                }
            }
            f.rot = (Quat::from_scaled_axis(f.spin * dt) * f.rot).normalize();
            pos[f.book as usize] = f.pos;
            rot[f.book as usize] = f.rot;
            if down || f.age > 8.0 {
                landed.push((f.book, f.level, f.pos, f.vel, f.rot));
                return false;
            }
            true
        });
        self.flying = flying;
        let mut landings = Vec::with_capacity(landed.len());
        for (b, lv, p, v, r) in landed {
            self.land(b, lv, p, v, r);
            landings.push(Landing {
                at: p,
                speed: v.length(),
            });
        }

        let mut done = Vec::new();
        for e in &mut self.easing {
            e.t += dt / e.dur;
            let b = e.book as usize;
            let (to_pos, to_rot) = self.pose[b];
            let k = e.t.min(1.0);
            pos[b] = e.from_pos.lerp(to_pos, k * k);
            rot[b] = e.from_rot.slerp(to_rot, k);
            if e.t >= 1.0 {
                done.push(e.book);
            }
        }
        for b in done {
            self.stop_ease(b);
        }
        landings
    }

    /// A flying book touches down at `p` on level `lv`: it skids a little along its
    /// travel (never through a wall, case or closed gate), then settles.
    fn land(&mut self, b: u32, lv: usize, p: Vec3, vel: Vec3, rot: Quat) {
        let from = Vec2::new(p.x, p.z);
        let skid = (Vec2::new(vel.x, vel.z) * 0.04).clamp_length_max(0.35);
        let at = if self.grids[lv].crosses_tall(from, from + skid) {
            from
        } else {
            from + skid
        };
        let along = rot * Vec3::Y;
        let mut u = Vec2::new(along.x, along.z);
        if u.length() < 0.2 {
            let across = rot * Vec3::Z;
            u = Vec2::new(across.x, across.z);
        }
        let u = u.normalize_or(Vec2::X);
        let bi = b as usize;
        self.moved[bi] = true;
        match self.drop(lv, b, at, u) {
            Ok(r) => {
                self.rest[bi] = r;
                self.level[bi] = lv;
                self.start_ease(Ease {
                    book: b,
                    from_pos: p,
                    from_rot: rot,
                    t: 0.0,
                    dur: 0.1,
                });
                self.index(b);
            }
            // It skidded or slid off an edge.
            Err(edge) => self.fall(b, lv, Vec3::new(edge.x, p.y, edge.y), rot),
        }
    }

    // ---------------------------------------------------------------- saving

    /// How a resting book lies.
    #[cfg(test)]
    pub fn rest(&self, b: u32) -> Rest {
        self.rest[b as usize]
    }

    /// Resting books that left their generated pose (only these need saving), with
    /// their levels.
    pub fn moved(&self) -> impl Iterator<Item = (u32, usize, Rest)> + '_ {
        (0..self.rest.len() as u32)
            .filter(|&b| self.resting[b as usize] && self.moved[b as usize])
            .map(|b| (b, self.level[b as usize], self.rest[b as usize]))
    }

    /// Books in the air, and where they are.
    pub fn airborne(&self) -> impl Iterator<Item = (u32, Vec3)> + '_ {
        self.flying.iter().map(|f| (f.book, f.pos))
    }

    /// Lay a book down exactly as saved. Call `rebuild_all` once all are placed.
    pub fn restore(&mut self, b: u32, lv: usize, r: Rest) {
        self.forget(b);
        let bi = b as usize;
        // Saved directions are unit already; only repair one that is not.
        let u = if (r.u.length() - 1.0).abs() < 1e-4 {
            r.u
        } else {
            r.u.normalize_or(Vec2::X)
        };
        self.rest[bi] = Rest { u, ..r };
        self.level[bi] = lv.min(self.grids.len() - 1);
        self.moved[bi] = true;
        self.index(b);
    }
}

/// An f32 ordered by `total_cmp`, for heaps.
#[derive(Clone, Copy, PartialEq)]
struct OrdF32(f32);

impl Eq for OrdF32 {}

impl PartialOrd for OrdF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrdF32 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Join tall furniture boxes (on a level whose floor is at `floor`) that stand edge
/// to edge, a row of bookcases, into one, so a book on top of the row isn't pushed
/// from one case into the next.
fn merge_solids(mut boxes: Vec<(Vec3, Vec3)>, floor: f32) -> Vec<(Vec3, Vec3)> {
    const GAP: f32 = 0.06;
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    loop {
        let mut joined = None;
        'search: for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let ((amn, amx), (bmn, bmx)) = (boxes[i], boxes[j]);
                if amx.y - floor <= SHED_HEIGHT || !near(amx.y, bmx.y) || !near(amn.y, bmn.y) {
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
    use crate::library::{HALL_X, HALL_Z, VEIL_X};

    impl Loose {
        /// How far a resting book sits above the support it would settle on now, from
        /// the floor and the books below it: positive means floating, negative sunk.
        fn support_gap(&self, b: u32) -> f32 {
            let r = self.rest[b as usize];
            let (mn, mx) = self.footprint(b);
            let lv = self.level[b as usize];
            let g = &self.grids[lv];
            let below: Vec<u32> = g
                .buckets_in(mn, mx)
                .flat_map(|k| g.buckets[k].iter().copied())
                .filter(|&x| x != b && !self.above(x, b))
                .collect();
            // As the heightfield sees it: a book supports the cells whose centres it covers.
            let support = |k: usize, _| {
                let c = g.cell_center(k % g.fw, k / g.fw);
                below
                    .iter()
                    .filter(|&&x| self.inside(x, c))
                    .map(|&x| {
                        let r = self.rest[x as usize];
                        r.under(c) + r.rise(self.size[x as usize].x)
                    })
                    .fold(g.base_k(k), f32::max)
            };
            let (_, fitted) = self.fit_on(lv, b, r.xz(), r.u, support);
            r.c.y - fitted.c.y
        }

        /// Heightfield cells that differ from a rebuild from scratch.
        pub fn heightfield_errors(&mut self) -> usize {
            let snapshot = |l: &Loose| -> Vec<Vec<f32>> {
                (l.grids.iter())
                    .map(|g| (0..g.fw * g.fh).map(|k| g.top_k(k)).collect())
                    .collect()
            };
            let kept = snapshot(self);
            self.rebuild_all();
            let now = snapshot(self);
            (kept.iter().flatten())
                .zip(now.iter().flatten())
                .filter(|(a, b)| a != b && (*a - *b).abs() > 1e-5)
                .count()
        }

        /// Distance from a book's centre into the nearest tall furniture footprint
        /// (positive = overlapping by that much).
        fn furniture_overlap(&self, b: u32) -> f32 {
            let c = self.rest[b as usize].xz();
            let r = self.half(b).length();
            let g = self.grid_of(b);
            g.all_solids()
                .filter(|&(_, mx)| g.tall(mx))
                .map(|(mn, mx)| {
                    let q = Vec2::new(c.x.clamp(mn.x, mx.x), c.y.clamp(mn.z, mx.z));
                    r - q.distance(c)
                })
                .fold(f32::MIN, f32::max)
        }
    }

    fn library() -> (Library, Loose) {
        let lib = Library::generate(crate::library::Layout::Hall, crate::game::SEED);
        let loose = Loose::new(&lib);
        (lib, loose)
    }

    /// The nearest pile west of the veil to `from`.
    fn west_pile(loose: &Loose, from: Vec2) -> Vec3 {
        let (_, top) = loose
            .best_pile(|_, c, _| (c.x < VEIL_X).then(|| xz(c).distance(from)))
            .unwrap();
        top
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
            let g = loose.grid_of(b);
            assert!(
                r.xz().cmpge(g.min + h - 1e-3).all() && r.xz().cmple(g.max - h + 1e-3).all(),
                "book {b} outside its level: {r:?}"
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
            .filter(|&b| lib.category_of_book(b) == cat)
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
        let top = west_pile(&loose, Vec2::new(-16.0, 0.0));
        let books = loose.gather(0, Vec2::new(top.x, top.z), 1.5, 120, |_| true);
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
            let top = west_pile(&loose, Vec2::new(x, z));
            let p = Vec2::new(top.x, top.z);
            loose.gather(0, p, 0.6, 20, |_| true);
            let cat = i * 2;
            loose.gather(0, p, 1.8, 40, |b| lib.category_of_book(b) == cat);
        }
        assert_eq!(loose.heightfield_errors(), 0);
    }

    /// Books thrown at the closed veil, or skidding into it, stay on the thrower's side;
    /// once it opens they fly through.
    #[test]
    fn the_closed_veil_holds() {
        let (lib, mut loose) = library();
        let top = west_pile(&loose, Vec2::new(8.0, 12.0));
        let books = loose.gather(0, Vec2::new(top.x, top.z), 1.5, 30, |_| true);
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        for open in [false, true] {
            if open {
                loose.take(&books);
                loose.set_gate(0, false);
            }
            for (i, &b) in books.iter().enumerate() {
                let from = Vec3::new(VEIL_X - 0.12, 0.2 + (i % 5) as f32 * 0.4, (i as f32 - 15.0) * 0.3);
                let vel = Vec3::new(4.0 + (i % 4) as f32 * 3.0, -1.0 + (i % 3) as f32, 0.5);
                loose.throw(b, from, vel, Quat::IDENTITY, Vec3::ONE, 0.0);
            }
            for _ in 0..600 {
                loose.update(1.0 / 60.0, &mut pos, &mut rot);
            }
            let behind = books.iter().filter(|&&b| loose.rest[b as usize].c.x > VEIL_X).count();
            assert!(books.iter().all(|&b| loose.is_resting(b)));
            if open {
                assert!(behind > books.len() / 2, "only {behind} went through the open veil");
            } else {
                assert_eq!(behind, 0, "books went through the closed veil");
            }
        }
        assert_eq!(loose.heightfield_errors(), 0);
    }

    /// Books landing on tall furniture (the fireplace, the seam between two
    /// bookcases) come off it and rest on the floor in the hall.
    #[test]
    fn tall_furniture_sheds_books() {
        let (lib, mut loose) = library();
        loose.set_gate(0, false);
        let (_, top) = loose
            .best_pile(|_, c, _| Some(xz(c).distance(Vec2::new(-16.0, 0.0))))
            .unwrap();
        let books = loose.gather(0, Vec2::new(top.x, top.z), 1.5, 40, |_| true);
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
            let top = west_pile(&loose, Vec2::new(x, z));
            let p = Vec2::new(top.x, top.z);
            for cat in [0, 1] {
                loose.gather(0, p, 1.8, 30, |b| lib.category_of_book(b) == cat);
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
            let Some((lv, top)) = loose.best_pile(|_, c, _| Some(xz(c).distance(Vec2::new(x, z)))) else {
                continue;
            };
            let o = top + Vec3::new(0.05, 3.0, 0.03);
            let (b, t) = loose.pick(o, -Vec3::Y, 10.0).expect("a pile under the ray");
            let hit = o - Vec3::Y * t;
            let surface = loose.height_at(lv, Vec2::new(hit.x, hit.z));
            // The ray meets the topmost book (its box is inflated a little for aiming).
            assert!((hit.y - surface).abs() < 0.06, "hit {hit} surface {surface} book {b}");
            hits += 1;
        }
        assert!(hits >= 3);
    }

    #[test]
    fn gathering_takes_the_top() {
        let (_, mut loose) = library();
        let (_, top) = loose
            .best_pile(|_, c, _| Some(xz(c).distance(Vec2::new(-16.0, 0.0))))
            .unwrap();
        let at = Vec2::new(top.x, top.z);
        let before: Vec<(u32, f32)> = loose
            .near(0, at, 0.5, |_| true)
            .into_iter()
            .map(|b| (b, loose.rest[b as usize].c.y))
            .collect();
        let taken = loose.gather(0, at, 0.5, 6, |_| true);
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

    /// A book dropped onto another that overhangs the top of a stairwell comes to rest
    /// (regression: it landed and fell again every frame, forever).
    #[test]
    fn books_settle_on_an_overhang() {
        let lib = Library::generate(crate::library::Layout::Great, crate::game::SEED);
        let mut loose = Loose::new(&lib);
        let (lv, top) = loose.best_pile(|l, c, _| (l == 0).then(|| xz(c).length())).unwrap();
        let books = loose.gather(lv, Vec2::new(top.x, top.z), 1.0, 30, |_| true);
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        // A book lying on the first floor at the top of the west stairwell, its middle
        // on the floor and its end out over the stairwell (z > -5.5 is open).
        let ledge = books[0];
        loose.take(&[ledge]);
        let u = Vec2::Y;
        let half = lib.books[ledge as usize].size.y * 0.5;
        loose.restore(
            ledge,
            1,
            Rest {
                c: Vec3::new(-10.2, 6.0, -5.5 - half * 0.4),
                u,
                su: 0.0,
                sv: 0.0,
            },
        );
        loose.rebuild_all();
        // Then the rest dropped on its overhanging end, one after another.
        for (i, &b) in books[1..].iter().enumerate() {
            let at = Vec3::new(-10.2 + (i % 3) as f32 * 0.03 - 0.03, 6.5, -5.5 + half * 0.4);
            loose.throw(
                b,
                at,
                Vec3::ZERO,
                Quat::from_rotation_y(i as f32),
                Vec3::ZERO,
                i as f32 * 0.3,
            );
        }
        let mut last = 0;
        for f in 0..60 * 20 {
            let landed = loose.update(1.0 / 60.0, &mut pos, &mut rot).len();
            if f >= 60 * 19 {
                last += landed;
            }
        }
        assert_eq!(last, 0, "books still landing after 19 s");
        assert!(books.iter().all(|&b| loose.is_resting(b)));
        assert_eq!(loose.heightfield_errors(), 0);
    }

    /// A sift doesn't reach through a bookcase to books on its other side.
    #[test]
    fn gathering_stops_at_cases() {
        let (lib, mut loose) = library();
        // Land books in the aisles either side of the row of cases at z = -4 (faces at
        // z = -4.49 and -3.51), level with x = -16.
        let top = west_pile(&loose, Vec2::new(-16.0, 0.0));
        let books = loose.gather(0, Vec2::new(top.x, top.z), 1.5, 20, |_| true);
        assert_eq!(books.len(), 20);
        for (i, &b) in books.iter().enumerate() {
            let z = if i % 2 == 0 { -4.85 } else { -3.15 };
            let x = -16.0 + (i / 2) as f32 * 0.12;
            loose.throw(b, Vec3::new(x, 0.4, z), Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0.0);
        }
        let n = lib.books.len();
        let (mut pos, mut rot) = (vec![Vec3::ZERO; n], vec![Quat::IDENTITY; n]);
        for _ in 0..120 {
            loose.update(1.0 / 60.0, &mut pos, &mut rot);
        }
        let north = |b: u32| loose.rest[b as usize].c.z < -4.0;
        assert_eq!(books.iter().filter(|&&b| north(b)).count(), 10);
        // From the north aisle, 2 m reaches every one of them across the case, but
        // takes only the north side.
        let at = Vec2::new(-15.7, -4.75);
        assert!(books.iter().all(|&b| loose.rest[b as usize].xz().distance(at) < 2.0));
        let got = loose.gather(0, at, 2.0, 1000, |b| books.contains(&b));
        assert_eq!(got.len(), 10);
        assert!(
            got.iter().all(|&b| loose.rest[b as usize].c.z < -4.0),
            "reached through the case"
        );
    }
}
