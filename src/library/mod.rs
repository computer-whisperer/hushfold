//! The library's generation: its world (floors, stairs, solids, gates), bookcase
//! faces and slot assignment, where the loose books start, static geometry and
//! lights. `hall` and `great` lay out the two libraries from the pieces in `kit`;
//! each piece is declared once, as `decor` (drawn only), `solid` (drawn and in the
//! way) or drawn parts with one `collide` box standing for them.

mod great;
mod hall;
mod kit;

#[cfg(test)]
pub use hall::{HALL_X, HALL_Z, VEIL_X};

use crate::catalog::{self, Catalogue, Series};
use crate::gfx::{Inst, kind};
use crate::world::{Gate, Level, NOSING, SLAB, Slab, Stairs, Stops, World};
use glam::{Mat3, Quat, Vec2, Vec3};
use rand::prelude::*;
use std::collections::HashSet;
use std::f32::consts::TAU;

pub const ROWS: usize = 6;
pub const UNIT_W: f32 = 2.6;
pub const UNIT_PITCH: f32 = 2.7;
pub const SIDE_T: f32 = 0.08;
pub const USABLE_W: f32 = 2.36;
pub const FACE_DEPTH: f32 = 0.43;
pub const UNIT_DEPTH: f32 = 0.9;
pub const UNIT_H: f32 = 3.1;
pub const SHELF_BASE: f32 = 0.14;
pub const SHELF_STEP: f32 = 0.48;
pub const BOARD_T: f32 = 0.03;
/// How far before its face a book flying home lines up with its slot.
pub const SLOT_FRONT: f32 = 0.4;
pub const TABLE_H: f32 = 0.78;

const WALNUT: [f32; 3] = [0.16, 0.08, 0.035];
const DARK_WOOD: [f32; 3] = [0.07, 0.035, 0.018];
const STONE: [f32; 3] = [0.36, 0.33, 0.29];
const BRASS: [f32; 3] = [0.55, 0.36, 0.12];

/// Height of the shelf surface above the floor for reading-order row (0 = top).
pub fn row_floor_y(row: usize) -> f32 {
    SHELF_BASE + (ROWS - 1 - row) as f32 * SHELF_STEP
}

pub struct Face {
    /// Centre of the front plane, at floor level.
    pub center: Vec3,
    pub normal: Vec3,
    pub right: Vec3,
    pub category: usize,
    pub case_idx: usize,
}

impl Face {
    pub fn rot(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(self.right, Vec3::Y, self.normal))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Run {
    pub series: u32,
    pub x0: f32,
    pub x1: f32,
}

pub struct BookInfo {
    pub series: u32,
    pub volume: u32,
    /// (thickness, height, depth)
    pub size: Vec3,
    pub face: u32,
    pub row: u8,
    pub slot_pos: Vec3,
    pub slot_rot: Quat,
}

/// Which library to build.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    #[default]
    /// The original hall: ~11.7k books, two wings.
    Hall,
    /// Three storeys round an atrium, twelve department rooms, ~120k books.
    Great,
}

impl Layout {
    pub const ALL: [Layout; 2] = [Layout::Hall, Layout::Great];

    pub fn name(self) -> &'static str {
        match self {
            Layout::Hall => "The Hall",
            Layout::Great => "The Great Library",
        }
    }

    /// How long a library takes, roughly.
    pub fn blurb(self) -> &'static str {
        match self {
            Layout::Hall => "one hall, ~12,000 books, an hour or two",
            Layout::Great => "three storeys, ~120,000 books, a long while",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum LightKind {
    Static,
    Flicker,
    Lantern(usize),
}

pub struct LightDef {
    pub pos: Vec3,
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub kind: LightKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LabelKind {
    /// A shelf's range of filing keys.
    Shelf,
    /// A category's sign over its cases.
    Category(usize),
    /// A department's sign over its door.
    Department(usize),
}

pub struct Label {
    pub pos: Vec3,
    pub normal: Vec3,
    pub text: String,
    pub kind: LabelKind,
    pub level: usize,
}

/// Where a department's books are shelved: a rectangle of one level (the Hall's
/// rooms are its wings), and the doorway into it.
pub struct Room {
    pub level: usize,
    pub min: Vec2,
    pub max: Vec2,
    /// In the doorway, on the floor.
    pub door: Vec3,
    /// Where couriers set its books down, inside it, clear of the way in.
    pub drop: Vec3,
}

pub struct Library {
    pub catalog: Catalogue,
    pub series: Vec<Series>,
    pub series_books: Vec<Vec<u32>>,
    pub books: Vec<BookInfo>,
    pub faces: Vec<Face>,
    pub runs: Vec<[Vec<Run>; ROWS]>,
    pub world: World,
    /// Each department's room, by department.
    pub rooms: Vec<Room>,
    /// Where each book is first dropped, in order: (book, level, point, direction of
    /// its length).
    pub drops: Vec<(u32, usize, Vec2, Vec2)>,
    pub static_insts: Vec<Inst>,
    /// Translucent static geometry (moonbeams).
    pub static_trans: Vec<Inst>,
    pub lights: Vec<LightDef>,
    pub labels: Vec<Label>,
    pub category_books: Vec<u32>,
    pub category_center: Vec<Vec3>,
    /// (category, position) of each category lantern; drawn per frame since they change.
    pub lanterns: Vec<(usize, Vec3)>,
    /// Where a new game starts: (feet, yaw).
    pub spawn: (Vec3, f32),
    /// The heap in the middle of the library that the Gathering Charm sends books to.
    pub hub: Vec3,
}

impl Library {
    pub fn generate(layout: Layout, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let catalog = match layout {
            Layout::Hall => Catalogue::hall(),
            Layout::Great => Catalogue::great(),
        };
        let n = catalog.categories.len();
        let mut lib = Library {
            catalog,
            series: Vec::new(),
            series_books: Vec::new(),
            books: Vec::new(),
            faces: Vec::new(),
            runs: Vec::new(),
            world: World::default(),
            rooms: Vec::new(),
            drops: Vec::new(),
            static_insts: Vec::new(),
            static_trans: Vec::new(),
            lights: Vec::new(),
            labels: Vec::new(),
            category_books: vec![0; n],
            category_center: vec![Vec3::ZERO; n],
            lanterns: Vec::new(),
            spawn: (Vec3::ZERO, 0.0),
            hub: Vec3::ZERO,
        };
        match layout {
            Layout::Hall => hall::build(&mut lib, &mut rng),
            Layout::Great => great::build(&mut lib, &mut rng),
        }
        lib
    }

    pub fn dept_of(&self, category: usize) -> usize {
        self.catalog.categories[category].dept
    }

    pub fn category_of_book(&self, b: u32) -> usize {
        self.series[self.books[b as usize].series as usize].category
    }

    /// Where book `b` lines up before sliding into its slot: in front of its face,
    /// clear of the case.
    pub fn slot_front(&self, b: u32) -> Vec3 {
        let info = &self.books[b as usize];
        let face = &self.faces[info.face as usize];
        info.slot_pos + face.normal * ((face.center - info.slot_pos).dot(face.normal) + SLOT_FRONT)
    }

    /// Whether book `b` can fly home from `from`: from before its face, with nothing
    /// that stops books between there and the front of its slot. Not through the
    /// back of its case, nor another case.
    pub fn reaches_front(&self, b: u32, from: Vec3) -> bool {
        let face = &self.faces[self.books[b as usize].face as usize];
        (from - face.center).dot(face.normal) > 0.0 && self.world.clear(from, self.slot_front(b), Stops::BOOKS)
    }

    /// The department whose room `p` is in, if any.
    pub fn room_at(&self, p: Vec3) -> Option<usize> {
        let l = self.world.level_at(p);
        let q = Vec2::new(p.x, p.z);
        (self.rooms.iter()).position(|r| r.level == l && q.cmpge(r.min).all() && q.cmplt(r.max).all())
    }

    /// The departments a gate counts, by name ("West Wing", "Astronomy and Botany").
    pub fn gate_counts_names(&self, g: usize) -> String {
        let names: Vec<&str> = (self.world.gates()[g].counts.iter())
            .map(|&d| self.catalog.departments[d].name.as_str())
            .collect();
        match names.as_slice() {
            [] => String::new(),
            [one] => one.to_string(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        }
    }

    /// Drawn only: small, or out of reach, so in nothing's way.
    fn decor(&mut self, i: Inst) {
        self.static_insts.push(i);
    }

    /// Drawn, and its box is in the way of what `stops` says.
    fn solid(&mut self, i: Inst, stops: Stops) {
        let (min, max) = i.axis_box().expect("a solid is an axis-aligned box");
        self.decor(i);
        self.collide(min, max, stops);
    }

    /// In the way of what `stops` says, drawn by its own helper's other calls: one box
    /// for a piece drawn in parts (a bookcase), or an unseen guard (a railing's).
    fn collide(&mut self, min: Vec3, max: Vec3, stops: Stops) {
        self.world.add_solid(min, max, stops);
    }

    fn light(&mut self, pos: Vec3, radius: f32, color: [f32; 3], intensity: f32, kind: LightKind) {
        self.lights.push(LightDef {
            pos,
            radius,
            color,
            intensity,
            kind,
        });
    }

    /// A category's shelves: `cases` (front-plane centre at floor level, in reading
    /// order) facing along `normal`; generates its series, lays them out, and hangs
    /// its sign and lantern over the middle case.
    fn add_category(
        &mut self,
        rng: &mut StdRng,
        cat: usize,
        cases: &[Vec3],
        normal: Vec3,
        titles: &mut HashSet<String>,
    ) {
        let right = Vec3::Y.cross(normal);
        let first_face = self.faces.len();
        for (ci, &center) in cases.iter().enumerate() {
            self.faces.push(Face {
                center,
                normal,
                right,
                category: cat,
                case_idx: ci,
            });
            self.runs.push(Default::default());
        }
        self.fill_category(rng, cat, first_face..first_face + cases.len(), titles);
        // Sign + lantern above the middle case.
        let mid = &self.faces[first_face + cases.len() / 2];
        let (mid_center, mid_rot) = (mid.center, mid.rot());
        let sign_pos = mid_center + Vec3::Y * (UNIT_H + 0.38) + normal * 0.02;
        let ccol = catalog::category_color(&self.catalog.categories[cat]);
        let plaque = Inst::new(
            sign_pos,
            mid_rot,
            Vec3::new(1.5, 0.34, 0.05),
            kind::PLAQUE,
            [ccol[0], ccol[1], ccol[2], 0.08],
            0,
        );
        self.decor(plaque);
        self.decor(Inst::aabox(
            sign_pos - Vec3::Y * 0.21 - normal * 0.02,
            Vec3::new(0.06, 0.12, 0.06),
            kind::WOOD,
            BRASS,
        ));
        self.labels.push(Label {
            pos: sign_pos + normal * 0.04,
            normal,
            text: self.catalog.categories[cat].name.clone(),
            kind: LabelKind::Category(cat),
            level: self.world.level_at(mid_center),
        });
        let lantern = mid_center + Vec3::Y * (UNIT_H + 0.95) + normal * 0.05;
        self.decor(Inst::aabox(
            lantern + Vec3::Y * 0.16,
            Vec3::new(0.22, 0.04, 0.22),
            kind::WOOD,
            BRASS,
        ));
        self.decor(Inst::aabox(
            lantern - Vec3::Y * 0.16,
            Vec3::new(0.2, 0.04, 0.2),
            kind::WOOD,
            BRASS,
        ));
        self.decor(Inst::aabox(
            lantern + Vec3::Y * 0.5,
            Vec3::new(0.02, 0.6, 0.02),
            kind::WOOD,
            BRASS,
        ));
        self.lanterns.push((cat, lantern));
        self.light(
            lantern + normal * 0.5,
            7.5,
            [1.0, 0.7, 0.4],
            0.35,
            LightKind::Lantern(cat),
        );
        self.category_center[cat] = mid_center + normal * 1.5 + Vec3::Y * 1.0;
    }

    /// Generate this category's series and lay them out across its faces.
    fn fill_category(
        &mut self,
        rng: &mut StdRng,
        cat: usize,
        faces: std::ops::Range<usize>,
        titles: &mut HashSet<String>,
    ) {
        let first_face = faces.start;
        let shelves = faces.len() * ROWS;
        let capacity = shelves as f32 * USABLE_W;
        let fill = rng.gen_range(0.74..0.84);
        let series = catalog::generate_series(rng, &self.catalog.categories[cat], cat, capacity * fill, titles);
        let gap_min = 0.015;
        let total: f32 = series.iter().map(|s| s.width() + gap_min).sum();
        let per_shelf = total / shelves as f32;

        // Assign each series to the shelf containing its midpoint, then repair overflow.
        let mut assign: Vec<Vec<usize>> = vec![Vec::new(); shelves];
        let mut cum = 0.0;
        for (i, s) in series.iter().enumerate() {
            let w = s.width() + gap_min;
            let shelf = (((cum + w * 0.5) / per_shelf) as usize).min(shelves - 1);
            assign[shelf].push(i);
            cum += w;
        }
        for k in 0..shelves {
            loop {
                let used: f32 = assign[k].iter().map(|&i| series[i].width() + gap_min).sum();
                if used <= USABLE_W || assign[k].len() <= 1 {
                    break;
                }
                let last = assign[k].pop().unwrap();
                if k + 1 < shelves {
                    assign[k + 1].insert(0, last);
                }
            }
        }
        let mut keep = vec![false; series.len()];
        for &i in assign.iter().flatten() {
            keep[i] = true;
        }

        let mut remap = vec![u32::MAX; series.len()];
        for (i, s) in series.into_iter().enumerate() {
            if keep[i] {
                remap[i] = self.series.len() as u32;
                self.series.push(s);
                self.series_books.push(Vec::new());
            }
        }

        for (k, shelf) in assign.iter().enumerate() {
            let face_i = first_face + k / ROWS;
            let row = k % ROWS;
            let ids: Vec<u32> = shelf.iter().map(|&i| remap[i]).collect();
            if ids.is_empty() {
                continue;
            }
            let used: f32 = ids.iter().map(|&s| self.series[s as usize].width()).sum();
            let gap = ((USABLE_W - used) / ids.len() as f32).min(0.35);
            let span = used + gap * ids.len() as f32;
            let mut x = -span * 0.5 + gap * 0.5;
            let (fc, fr) = (self.faces[face_i].center, self.faces[face_i].rot());
            let (fright, fnormal) = (self.faces[face_i].right, self.faces[face_i].normal);
            for sid in ids {
                let x0 = x;
                let s = &self.series[sid as usize];
                let (h, d) = (s.height, s.depth);
                let thick = s.thickness.clone();
                for (v, &t) in thick.iter().enumerate() {
                    let cx = x + t * 0.5;
                    let pos = fc + fright * cx + Vec3::Y * (row_floor_y(row) + h * 0.5) - fnormal * (d * 0.5 + 0.015);
                    let id = self.books.len() as u32;
                    self.books.push(BookInfo {
                        series: sid,
                        volume: v as u32 + 1,
                        size: Vec3::new(t * 0.985, h, d),
                        face: face_i as u32,
                        row: row as u8,
                        slot_pos: pos,
                        slot_rot: fr,
                    });
                    self.series_books[sid as usize].push(id);
                    self.category_books[cat] += 1;
                    x += t;
                }
                self.runs[face_i][row].push(Run { series: sid, x0, x1: x });
                x += gap;
            }
        }
        // Shelf range labels.
        for f in faces {
            for row in 0..ROWS {
                let runs = &self.runs[f][row];
                if runs.is_empty() {
                    continue;
                }
                let key = |s: u32| self.series[s as usize].shelf_key();
                let text = format!("{} – {}", key(runs[0].series), key(runs[runs.len() - 1].series));
                let face = &self.faces[f];
                let pos = face.center + Vec3::Y * (row_floor_y(row) - BOARD_T * 0.5) + face.normal * 0.02;
                let level = self.world.level_at(face.center);
                self.labels.push(Label {
                    pos,
                    normal: face.normal,
                    text,
                    kind: LabelKind::Shelf,
                    level,
                });
            }
        }
    }

    /// `books` (one department's, say) as units to scatter, shuffled: a unit lies in
    /// one pile. Each series of two or more volumes draws how much of it lies
    /// together: about a quarter whole, about two fifths with a core of 40–80% of its
    /// volumes and the rest scattered one by one, the others scattered entirely.
    fn series_units(&self, rng: &mut StdRng, books: &[u32]) -> Vec<Vec<u32>> {
        let mut by_series: Vec<Vec<u32>> = Vec::new();
        let mut at: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for &b in books {
            let s = self.books[b as usize].series;
            let i = *at.entry(s).or_insert_with(|| {
                by_series.push(Vec::new());
                by_series.len() - 1
            });
            by_series[i].push(b);
        }
        let mut units = Vec::new();
        for mut vols in by_series {
            vols.shuffle(rng);
            let together = match rng.gen_range(0.0..1.0f32) {
                x if x < 0.25 => 1.0,
                x if x < 0.65 => rng.gen_range(0.4..0.8),
                _ => 0.0,
            };
            let core = (vols.len() as f32 * together).round() as usize;
            if core >= 2 {
                units.push(vols.drain(..core).collect());
            }
            units.extend(vols.into_iter().map(|b| vec![b]));
        }
        units.shuffle(rng);
        units
    }

    /// A unit from `series_units` that is a whole series of two or more volumes.
    fn whole_set(&self, unit: &[u32]) -> bool {
        let s = self.books[unit[0] as usize].series as usize;
        unit.len() >= 2 && unit.len() == self.series_books[s].len()
    }

    /// Share `units` out between pile `spots` on `level`, roughly evenly by books,
    /// keeping each unit in one pile.
    fn spread(&mut self, rng: &mut StdRng, level: usize, spots: &[Vec2], units: Vec<Vec<u32>>) {
        let mut left: usize = units.iter().map(Vec::len).sum();
        assert!(!spots.is_empty() || left == 0, "nowhere to put {left} books");
        let mut units = units.into_iter();
        for (i, &p) in spots.iter().enumerate() {
            let piles_left = spots.len() - i;
            let want = if piles_left == 1 {
                left
            } else {
                ((left as f32 / piles_left as f32) * rng.gen_range(0.75..1.25)) as usize
            };
            let mut pile = Vec::new();
            while pile.len() < want
                && let Some(u) = units.next()
            {
                pile.extend(u);
            }
            if pile.is_empty() {
                continue;
            }
            left -= pile.len();
            let radius = 0.4 + 0.06 * (pile.len() as f32).sqrt();
            self.add_pile(rng, level, p, radius, pile);
        }
    }

    /// Drop points for a pile: denser toward the middle as it grows, so it heaps up.
    fn add_pile(&mut self, rng: &mut StdRng, level: usize, center: Vec2, radius: f32, books: Vec<u32>) {
        let count = books.len().max(1) as f32;
        for (i, b) in books.into_iter().enumerate() {
            let frac = i as f32 / count;
            let r = radius * (1.0 - 0.5 * frac) * rng.gen_range(0.0..1.0f32).sqrt();
            let a = rng.gen_range(0.0..1.0f32) * TAU;
            let yaw = rng.gen_range(0.0..1.0f32) * TAU;
            self.drops.push((
                b,
                level,
                center + Vec2::new(a.cos(), a.sin()) * r,
                Vec2::new(yaw.cos(), yaw.sin()),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Space the player could see from: cells of `VOX` reached from the spawn point
    /// without touching a static box.
    struct Air {
        min: Vec3,
        dims: [usize; 3],
        open: Vec<bool>,
    }

    const VOX: f32 = 0.2;

    impl Air {
        fn new(boxes: &[(Vec3, Vec3, u32)], seed: Vec3) -> Air {
            let (min, max) = (boxes.iter()).fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(a, b), &(mn, mx, _)| (a.min(mn), b.max(mx)),
            );
            let min = min - Vec3::splat(VOX);
            let d = ((max - min) / VOX).ceil() + 1.0;
            let dims = [d.x as usize, d.y as usize, d.z as usize];
            let idx = |x: usize, y: usize, z: usize| (z * dims[1] + y) * dims[0] + x;
            let mut solid = vec![false; dims[0] * dims[1] * dims[2]];
            // Every cell a box touches is solid, so thin walls hold.
            for &(mn, mx, _) in boxes {
                let lo = ((mn - min) / VOX).floor().max(Vec3::ZERO);
                let hi = ((mx - min) / VOX).ceil() - 1.0;
                for z in lo.z as usize..=hi.z as usize {
                    for y in lo.y as usize..=hi.y as usize {
                        for x in lo.x as usize..=hi.x as usize {
                            solid[idx(x, y, z)] = true;
                        }
                    }
                }
            }
            let mut open = vec![false; solid.len()];
            let s = ((seed - min) / VOX).floor();
            let mut stack = vec![[s.x as usize, s.y as usize, s.z as usize]];
            while let Some([x, y, z]) = stack.pop() {
                let i = idx(x, y, z);
                if open[i] || solid[i] {
                    continue;
                }
                open[i] = true;
                for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                    let (nx, ny, nz) = (x as isize + dx, y as isize + dy, z as isize + dz);
                    if (0..dims[0] as isize).contains(&nx)
                        && (0..dims[1] as isize).contains(&ny)
                        && (0..dims[2] as isize).contains(&nz)
                    {
                        stack.push([nx as usize, ny as usize, nz as usize]);
                    }
                }
            }
            Air { min, dims, open }
        }

        fn at(&self, p: Vec3) -> bool {
            let c = ((p - self.min) / VOX).floor();
            let [x, y, z] = [c.x as usize, c.y as usize, c.z as usize];
            x < self.dims[0]
                && y < self.dims[1]
                && z < self.dims[2]
                && self.open[(z * self.dims[1] + y) * self.dims[0] + x]
        }
    }

    /// Faces of opaque static boxes that lie in the same plane, facing the same way,
    /// overlapping where they can be seen from inside the library.
    fn coplanar(layout: Layout) -> Vec<String> {
        let lib = Library::generate(layout, 7);
        let boxes: Vec<(Vec3, Vec3, u32)> = (lib.static_insts.iter())
            .filter_map(|i| i.axis_box().map(|(a, b)| (a, b, i.kind)))
            .collect();
        let air = Air::new(&boxes, lib.spawn.0 + Vec3::Y * 1.0);
        let mut faces: HashMap<(usize, bool, i64), Vec<usize>> = HashMap::new();
        for (k, &(mn, mx, _)) in boxes.iter().enumerate() {
            for axis in 0..3 {
                for (pos, v) in [(false, mn[axis]), (true, mx[axis])] {
                    faces
                        .entry((axis, pos, (v * 1000.0).round() as i64))
                        .or_default()
                        .push(k);
                }
            }
        }
        let mut out = Vec::new();
        for (&(axis, pos, plane), ks) in &faces {
            let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
            for (i, &a) in ks.iter().enumerate() {
                for &b in &ks[i + 1..] {
                    let (amn, amx, ak) = boxes[a];
                    let (bmn, bmx, bk) = boxes[b];
                    let lo = amn.max(bmn);
                    let hi = amx.min(bmx);
                    if hi[u] - lo[u] < 0.002 || hi[v] - lo[v] < 0.002 {
                        continue;
                    }
                    // Seen if the space just in front of the shared patch can be reached.
                    let mut p = (lo + hi) * 0.5;
                    p[axis] = plane as f32 / 1000.0 + if pos { 1.01 * VOX } else { -1.01 * VOX };
                    let mut q = p;
                    q[axis] = plane as f32 / 1000.0 + if pos { 0.001 } else { -0.001 };
                    let covered = (boxes.iter().enumerate())
                        .any(|(k, &(mn, mx, _))| k != a && k != b && q.cmpgt(mn).all() && q.cmplt(mx).all());
                    if air.at(p) && !covered {
                        out.push(format!(
                            "{layout:?} kinds {ak}/{bk} axis {axis}{} plane {:.3} at {:.2?} size {:.2}x{:.2}",
                            if pos { "+" } else { "-" },
                            plane as f32 / 1000.0,
                            q,
                            hi[u] - lo[u],
                            hi[v] - lo[v],
                        ));
                    }
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn no_coplanar_faces() {
        let found: Vec<String> = Layout::ALL.into_iter().flat_map(coplanar).collect();
        for f in &found {
            println!("{f}");
        }
        assert!(found.is_empty(), "{} coplanar overlapping faces", found.len());
    }
}
