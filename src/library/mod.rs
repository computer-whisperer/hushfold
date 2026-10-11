//! The library's generation: its world (floors, stairs, solids, gates), bookcase
//! faces and slot assignment, where the loose books start, static geometry and
//! lights. `hall` and `great` lay out the two libraries from the pieces in `kit`;
//! each piece is declared once, as `decor` (drawn only), `solid` (drawn and in the
//! way) or drawn parts with one `collide` box standing for them.

pub mod celebrate;
mod great;
mod hall;
mod kit;
pub mod motion;
mod rooms;
mod upstairs;

#[cfg(test)]
pub use hall::{HALL_X, HALL_Z, VEIL_X};

use crate::catalog::{self, Catalogue, Series};
use crate::gfx::mesh::{self, Mesh, Shape, circle};
use crate::gfx::{Inst, kind};
use crate::text::{self, Glyph, Lettering, Style};
use crate::world::{Gate, Level, NOSING, SLAB, Slab, Stairs, Stops, World, ray_box};
pub use celebrate::{Awaits, Change, Cue, Finish, Sparks};
use glam::{Mat3, Quat, Vec2, Vec3};
use motion::{Motion, Mover};
use rand::prelude::*;
use std::collections::HashSet;
use std::f32::consts::TAU;
use std::ops::Range;

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

/// Gold leaf for the names on signs.
const GOLD_LEAF: [f32; 4] = [1.0, 0.76, 0.36, 1.0];
/// A category's name on its sign.
const SIGN_LETTERS: Style = Style {
    em: 0.2,
    tracking: 0.01,
    lettering: Lettering::Gilt,
    color: GOLD_LEAF,
};
/// A category's name on the sign over one part of its cases, with room for the
/// part's filing keys (`PART_KEYS`) under it.
const PART_LETTERS: Style = Style {
    em: 0.17,
    ..SIGN_LETTERS
};
const PART_KEYS: Style = Style {
    em: 0.1,
    ..SIGN_LETTERS
};
/// A shelf's range of filing keys, inked on its label.
const SHELF_LETTERS: Style = Style {
    em: 0.034,
    tracking: 0.0,
    lettering: Lettering::Ink,
    color: [0.05, 0.03, 0.015, 1.0],
};

/// `rgb` with an alpha (or a glow) of `w`.
fn rgba(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// A flame (`kind::FIRE`, a candle's or a lamp's) standing on `foot`, `w` across and
/// `h` tall: the box the shader draws its teardrop in.
fn flame(foot: Vec3, (w, h): (f32, f32), color: [f32; 4]) -> Inst {
    Inst::new(
        foot + Vec3::Y * (h * 0.5),
        Quat::IDENTITY,
        Vec3::new(w, h, w),
        kind::FIRE,
        color,
        0,
    )
}

/// How far apart the middles of two of a category's faces, one after the other, stand
/// in parts apart: across a nave, a doorway or a window, but not across a gate in a run
/// of cases or round a corner, where its one sign is still in view.
const PARTS_APART: f32 = 6.0;

/// The parts of a category's `faces` (in reading order) that stand apart, each
/// wanting a sign of its own.
fn parts_apart(faces: &[(Vec3, Vec3)]) -> Vec<Range<usize>> {
    let mut parts = Vec::new();
    let mut start = 0;
    for i in 1..=faces.len() {
        if i == faces.len() || faces[i].0.distance(faces[i - 1].0) > PARTS_APART {
            parts.push(start..i);
            start = i;
        }
    }
    parts
}

/// Where the lantern over a sign hangs, over a case whose face's front plane has its
/// centre at `center` (at floor level) and faces `normal`.
fn lantern_at(center: Vec3, normal: Vec3) -> Vec3 {
    center + Vec3::Y * (UNIT_H + 0.95) + normal * 0.05
}

/// Where a sign goes over `faces` (in reading order): over the middle one, or between
/// the middle two (over the later, should they face apart).
fn middle(faces: &[(Vec3, Vec3)]) -> (Vec3, Vec3) {
    let n = faces.len();
    let ((p, m), (q, k)) = (faces[(n - 1) / 2], faces[n / 2]);
    match (m + k).try_normalize() {
        Some(normal) if n.is_multiple_of(2) => ((p + q) * 0.5, normal),
        _ => faces[n / 2],
    }
}

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
    /// Where it lines up before sliding into its slot (`slot_front`).
    pub front: Vec3,
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
    /// The mover it rides (`motion`), its position being where it is at rest.
    pub rides: Option<u32>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LabelKind {
    /// A category's sign over its cases.
    Category(usize),
    /// A department's sign over its door.
    Department(usize),
}

pub struct Label {
    pub pos: Vec3,
    pub normal: Vec3,
    pub text: String,
    /// The height of the sign's letters (an em), and half the height of its board,
    /// in metres.
    pub em: f32,
    pub half: f32,
    pub kind: LabelKind,
    pub level: usize,
}

/// The Great Library's departments that ask something of their own besides finding
/// and filing (see `upstairs` and `game`), by number.
pub mod dept {
    /// Some of its books are restless once it opens: they scurry from anyone walking near.
    pub const BESTIARY: usize = 3;
    /// Open together, each heaped with the other's books; their tallest books are folios.
    pub const MAP_ROOM: usize = 4;
    pub const CHRONICLE_HALL: usize = 5;
    /// Holds half the Music Room's books, a floor below it.
    pub const VERSE_GALLERY: usize = 6;
    /// Its ward quiets magic inside it, less as it is put in order.
    pub const ARCANUM: usize = 7;
    /// Its stopped clocks hold books in the air until their series are filed.
    pub const CLOCKWORKS: usize = 8;
    pub const MUSIC_ROOM: usize = 9;
    /// A draught blows through it.
    pub const WEATHERHOUSE: usize = 10;
    /// Its books were left about the rooms upstairs.
    pub const HEARTH_LIBRARY: usize = 11;
}

/// The first Great Library room whose heaps generation 5 relaid (`upstairs`): the
/// first floor's first. Nothing below it moved, so a save of generation 4 with it
/// still veiled loads.
pub const RELAID: usize = 4;

/// A stopped clock in the Clockworks, floating over the floor with books held still
/// in the air round it (`Library::frozen`) until its key series is filed whole.
pub struct StoppedClock {
    /// The middle of its face, out of the face, and the face's radius.
    pub face: Vec3,
    pub n: Vec3,
    pub r: f32,
    /// The series that starts it again, lettered on its face.
    pub key: u32,
    /// The books it holds.
    pub books: Vec<u32>,
    /// Its hands (hour, minute) as they would stand at twelve, and the time it stopped
    /// at in hours; drawn per frame, turning about `n` through `face`.
    pub hands: [Inst; 2],
    pub stopped_at: f32,
}

/// Where a department's books are shelved: a rectangle of one level (the Hall's
/// rooms are its wings), and the doorway into it./// Where a department's books are shelved: a rectangle of one level (the Hall's
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
    pub layout: Layout,
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
    /// Its own meshes (`shape`), the first `mesh::LIBRARY`.
    pub meshes: Vec<Mesh>,
    /// Each of them by a hash of its contents, so one made twice is kept once.
    mesh_ids: std::collections::HashMap<u64, u32>,
    /// Things that move (`motion`), and their parts as they are at rest.
    pub movers: Vec<Mover>,
    pub moving_insts: Vec<Inst>,
    /// Translucent static geometry (moonbeams, clear glass, flames, mist).
    pub static_trans: Vec<Inst>,
    /// Lettering on signs and labels.
    pub text: Vec<Glyph>,
    pub lights: Vec<LightDef>,
    /// Signs, for the Signposts to point out.
    pub labels: Vec<Label>,
    pub category_books: Vec<u32>,
    pub category_center: Vec<Vec3>,
    /// (category, position) of each category lantern; drawn per frame since they change.
    pub lanterns: Vec<(usize, Vec3)>,
    /// (department, gem) of each gem by a department's door; drawn per frame, since
    /// they brighten as it nears done.
    pub door_gems: Vec<(usize, Inst)>,
    /// Where a new game starts: (feet, yaw).
    pub spawn: (Vec3, f32),
    /// The heap in the middle of the library that the Gathering Charm sends books to.
    pub hub: Vec3,
    /// The Clockworks' stopped clocks, and the books they hold in the air (book, where,
    /// how turned).
    pub stopped: Vec<StoppedClock>,
    pub frozen: Vec<(u32, Vec3, Quat)>,
    /// The Weatherhouse's vane: the mover that turns it, and its arrow's direction at
    /// rest (it points into the wind).
    pub vane: Option<(u32, Vec3)>,
    /// Sparks and sounds waiting for a part of the library to be finished (`celebrate`).
    pub sparks: Vec<celebrate::Sparks>,
    pub cues: Vec<celebrate::Cue>,
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
            layout,
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
            meshes: Vec::new(),
            mesh_ids: Default::default(),
            movers: Vec::new(),
            moving_insts: Vec::new(),
            static_trans: Vec::new(),
            text: Vec::new(),
            lights: Vec::new(),
            labels: Vec::new(),
            category_books: vec![0; n],
            category_center: vec![Vec3::ZERO; n],
            lanterns: Vec::new(),
            door_gems: Vec::new(),
            spawn: (Vec3::ZERO, 0.0),
            hub: Vec3::ZERO,
            stopped: Vec::new(),
            frozen: Vec::new(),
            vane: None,
            sparks: Vec::new(),
            cues: Vec::new(),
        };
        match layout {
            Layout::Hall => hall::build(&mut lib, &mut rng),
            Layout::Great => great::build(&mut lib, &mut rng),
        }
        lib.line_up();
        lib
    }

    /// Where books line up before sliding into their slots: straight out from each,
    /// SLOT_FRONT clear of its face; but where something stands across that (a case
    /// meeting the end of the face in an alcove's corner), the nearest point close in
    /// along the face that is open to the room, so the book glides in along the face,
    /// and the aura, homing and familiars reach it as they reach any other. Run once
    /// everything that stops books stands.
    fn line_up(&mut self) {
        // Open: reached straight on from LOOK_OUT further out, and clear on from there
        // to the mouth of the slot.
        const LOOK_OUT: f32 = 0.5;
        let solids: Vec<(Vec3, Vec3)> = (self.world.solids().iter())
            .filter(|s| s.stops.has(Stops::BOOKS))
            .map(|s| (s.min, s.max))
            .collect();
        let mut on_face = vec![Vec::new(); self.faces.len()];
        for (b, info) in self.books.iter().enumerate() {
            on_face[info.face as usize].push(b);
        }
        for (f, books) in on_face.into_iter().enumerate() {
            let (center, normal, right) = (self.faces[f].center, self.faces[f].normal, self.faces[f].right);
            // What stands before the face, clear of its own case, within reach of a
            // front or a glide along the face.
            let reach = UNIT_W * 0.5 + 1.3;
            let corners = [-reach, reach].map(|a| {
                [0.05, SLOT_FRONT + LOOK_OUT]
                    .map(|o| [0.0, UNIT_H].map(|y| center + right * a + normal * o + Vec3::Y * y))
            });
            let corners = corners.as_flattened().as_flattened();
            let lo = corners.iter().fold(Vec3::MAX, |m, &p| m.min(p));
            let hi = corners.iter().fold(Vec3::MIN, |m, &p| m.max(p));
            let near: Vec<(Vec3, Vec3)> = (solids.iter())
                .filter(|(mn, mx)| mn.cmplt(hi).all() && mx.cmpgt(lo).all())
                .copied()
                .collect();
            if near.is_empty() {
                continue;
            }
            let inside = |p: Vec3| near.iter().any(|(mn, mx)| p.cmpgt(*mn).all() && p.cmplt(*mx).all());
            let clear = |a: Vec3, b: Vec3| {
                let (d, len) = ((b - a).normalize(), a.distance(b));
                near.iter()
                    .all(|&(mn, mx)| ray_box(a, d, mn, mx).is_none_or(|t| t >= len))
            };
            for b in books {
                let slot = self.books[b].slot_pos;
                let deep = (center - slot).dot(normal);
                let at = |along: f32, out: f32| slot + right * along + normal * (deep + out);
                let mouth = at(0.0, 0.06);
                let open = |p: Vec3| !inside(p) && clear(p + normal * LOOK_OUT, p) && clear(p, mouth);
                if open(self.books[b].front) {
                    continue;
                }
                // Along the face either way, out from it no further than it can glide,
                // a step clear of where it first opens.
                let mut best: Option<(f32, Vec3)> = None;
                for out in [SLOT_FRONT, 0.3, 0.2, 0.12] {
                    for e in [-1.0f32, 1.0] {
                        let Some(k) = (1..=12).find(|&k| open(at(e * k as f32 * 0.1, out))) else {
                            continue;
                        };
                        let k = if open(at(e * (k + 1) as f32 * 0.1, out)) {
                            k + 1
                        } else {
                            k
                        };
                        let along = k as f32 * 0.1;
                        let cost = along + (SLOT_FRONT - out);
                        if best.is_none_or(|(c, _)| cost < c) {
                            best = Some((cost, at(e * along, out)));
                        }
                    }
                }
                if let Some((_, p)) = best {
                    self.books[b].front = p;
                }
            }
        }
    }

    pub fn dept_of(&self, category: usize) -> usize {
        self.catalog.categories[category].dept
    }

    pub fn category_of_book(&self, b: u32) -> usize {
        self.series[self.books[b as usize].series as usize].category
    }

    pub fn dept_of_book(&self, b: u32) -> usize {
        self.dept_of(self.category_of_book(b))
    }

    /// Department `d` is the Great Library's `which` (one of `dept`).
    pub fn is_dept(&self, d: usize, which: usize) -> bool {
        self.layout == Layout::Great && d == which
    }

    /// A folio: one of the Map Room's or the Chronicle Hall's tallest series (about a
    /// tenth of their books). It fills `FOLIO` places in the arms, flies half as far
    /// tossed, and is too heavy for familiars.
    pub fn is_folio(&self, b: u32) -> bool {
        let d = self.dept_of_book(b);
        (self.is_dept(d, dept::MAP_ROOM) || self.is_dept(d, dept::CHRONICLE_HALL))
            && self.books[b as usize].size.y >= 0.34
    }

    /// One of the Bestiary's restless books: a quarter of its series, picked by a hash
    /// of the series.
    pub fn is_restless(&self, b: u32) -> bool {
        let s = self.books[b as usize].series;
        self.is_dept(self.dept_of_book(b), dept::BESTIARY) && (s.wrapping_mul(0x9e37_79b9) >> 28).is_multiple_of(4)
    }

    /// Where book `b` lines up before sliding into its slot: in front of its face,
    /// clear of the case (see `line_up`).
    pub fn slot_front(&self, b: u32) -> Vec3 {
        self.books[b as usize].front
    }

    /// Whether book `b` can fly home from `from`: from before its face, with nothing
    /// that stops books between there and the front of its slot. Not through the
    /// back of its case, nor another case.
    pub fn reaches_front(&self, b: u32, from: Vec3) -> bool {
        let face = &self.faces[self.books[b as usize].face as usize];
        (from - face.center).dot(face.normal) > 0.0 && self.world.clear(from, self.slot_front(b), Stops::BOOKS)
    }

    /// The range of filing keys on a shelf ("Ab – Al"), as its label gives it; none
    /// for an empty shelf.
    pub fn shelf_keys(&self, face: usize, row: usize) -> Option<String> {
        let runs = &self.runs[face][row];
        let key = |r: &Run| self.series[r.series as usize].shelf_key();
        Some(format!("{} – {}", key(runs.first()?), key(runs.last()?)))
    }

    /// The range of filing keys across `faces` (one category's, in reading order), as
    /// a sign over them gives it: from their first shelf's first to their last shelf's
    /// last.
    pub fn faces_keys(&self, faces: Range<usize>) -> Option<String> {
        let shelves = || faces.clone().flat_map(|f| self.runs[f].iter());
        let key = |r: &Run| self.series[r.series as usize].shelf_key();
        let first = shelves().find_map(|row| row.first())?;
        let last = shelves().rev().find_map(|row| row.last())?;
        Some(format!("{} – {}", key(first), key(last)))
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

    /// Letter `text` upright on a surface facing `normal`, centred on `centre` (in the
    /// surface), at most `width` wide.
    fn inscribe(&mut self, text: &str, style: Style, centre: Vec3, normal: Vec3, width: f32) {
        // Lifted off the surface by a hair, which the depth bias does the rest of.
        let at = centre + normal * 0.0015;
        text::set_line(&mut self.text, text, style, at, Vec3::Y.cross(normal), Vec3::Y, width);
    }

    /// A mesh of its own (fitted to the unit box; one made before is shared), and the
    /// size it was made at.
    pub fn shape(&mut self, m: Mesh) -> Shape {
        use std::hash::{Hash, Hasher};
        let (m, size, middle) = m.fitted();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytemuck::cast_slice::<_, u8>(&m.verts).hash(&mut h);
        m.idx.hash(&mut h);
        let first = mesh::LIBRARY + self.meshes.len() as u32;
        let id = *self.mesh_ids.entry(h.finish()).or_insert(first);
        if id == first {
            self.meshes.push(m);
        } else {
            debug_assert!(
                self.meshes[(id - mesh::LIBRARY) as usize] == m,
                "two meshes hashed alike"
            );
        }
        Shape::new(id, size, middle)
    }

    /// A rug or carpet on the floor: books lie on it rather than under it; it is in
    /// no one's way.
    fn rug(&mut self, rug: Inst) {
        self.solid(rug, Stops::BOOKS);
    }

    /// Drawn only: small, or out of reach, so in nothing's way.
    fn decor(&mut self, i: Inst) {
        self.static_insts.push(i);
    }

    /// Drawn only, in the translucent pass, over what is behind it: clear glass,
    /// flames, mist (`kind::translucent`).
    fn translucent(&mut self, i: Inst) {
        debug_assert!(kind::translucent(i.kind), "kind {} is opaque", i.kind);
        self.static_trans.push(i);
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
            rides: None,
        });
    }

    /// A light carried by mover `m`, at `pos` when the mover is at rest.
    fn light_riding(&mut self, m: u32, pos: Vec3, radius: f32, color: [f32; 3], intensity: f32, kind: LightKind) {
        self.light(pos, radius, color, intensity, kind);
        self.lights.last_mut().expect("just lit").rides = Some(m);
    }

    /// Shelve category `cat` across `faces` (front plane centre at floor level,
    /// outward normal; see `bookcase`), in reading order, and hang its sign and
    /// lantern over the middle one; or, where its cases stand in parts apart (either
    /// side of a nave, a door or a window), over the middle of each part, with the
    /// part's filing keys under the name.
    fn add_category(&mut self, rng: &mut StdRng, cat: usize, faces: &[(Vec3, Vec3)], titles: &mut HashSet<String>) {
        let first_face = self.faces.len();
        for (ci, &(center, normal)) in faces.iter().enumerate() {
            self.faces.push(Face {
                center,
                normal,
                right: Vec3::Y.cross(normal),
                category: cat,
                case_idx: ci,
            });
            self.runs.push(Default::default());
        }
        self.fill_category(rng, cat, first_face..first_face + faces.len(), titles);
        let parts = parts_apart(faces);
        if parts.len() == 1 {
            let (center, normal) = faces[faces.len() / 2];
            self.hang_sign(cat, center, normal, None);
        } else {
            for part in parts {
                let (center, normal) = middle(&faces[part.clone()]);
                let keys = self.faces_keys(first_face + part.start..first_face + part.end);
                self.hang_sign(cat, center, normal, keys);
            }
        }
        // One light for the category, from the lantern over its middle case (where
        // its one sign hangs, or would): more would crowd out other lights from the
        // nearest the renderer draws, darkening the rooms round about.
        let (center, normal) = faces[faces.len() / 2];
        self.light(
            lantern_at(center, normal) + normal * 0.5,
            7.5,
            [1.0, 0.7, 0.4],
            0.35,
            LightKind::Lantern(cat),
        );
        self.category_center[cat] = center + normal * 1.5 + Vec3::Y * 1.0;
    }

    /// Category `cat`'s sign, and its lantern above it, over a case whose face's front
    /// plane has its centre at `center` (at floor level) and faces `normal`; `keys`,
    /// the filing keys of a part of the category's cases, lettered under the name.
    fn hang_sign(&mut self, cat: usize, center: Vec3, normal: Vec3, keys: Option<String>) {
        let rot = Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(normal), Vec3::Y, normal));
        let sign_pos = center + Vec3::Y * (UNIT_H + 0.42) + normal * 0.02;
        let ccol = catalog::category_color(&self.catalog.categories[cat]);
        let board = Vec3::new(2.0, 0.42, 0.05);
        let plaque = Inst::new(sign_pos, rot, board, kind::PLAQUE, [ccol[0], ccol[1], ccol[2], 0.08], 0);
        self.decor(plaque);
        self.decor(Inst::aabox(
            sign_pos - Vec3::Y * 0.25 - normal * 0.02,
            Vec3::new(0.06, 0.12, 0.06),
            kind::WOOD,
            BRASS,
        ));
        let name = self.catalog.categories[cat].name.clone();
        let face = sign_pos + normal * 0.025;
        let (em, text) = match keys {
            None => {
                self.inscribe(&name, SIGN_LETTERS, face, normal, 1.86);
                (SIGN_LETTERS.em, name)
            }
            Some(keys) => {
                self.inscribe(&name, PART_LETTERS, face + Vec3::Y * 0.06, normal, 1.86);
                self.inscribe(&keys, PART_KEYS, face - Vec3::Y * 0.1, normal, 1.0);
                (PART_LETTERS.em, format!("{name} · {keys}"))
            }
        };
        self.labels.push(Label {
            pos: sign_pos + normal * 0.04,
            normal,
            text,
            em,
            half: board.y * 0.5,
            kind: LabelKind::Category(cat),
            level: self.world.level_at(center),
        });
        let lantern = lantern_at(center, normal);
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
    }

    /// Generate this category's series and lay them out across its faces.
    fn fill_category(&mut self, rng: &mut StdRng, cat: usize, faces: Range<usize>, titles: &mut HashSet<String>) {
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
                        front: pos + fnormal * ((fc - pos).dot(fnormal) + SLOT_FRONT),
                    });
                    self.series_books[sid as usize].push(id);
                    self.category_books[cat] += 1;
                    x += t;
                }
                self.runs[face_i][row].push(Run { series: sid, x0, x1: x });
                x += gap;
            }
        }
        // Each shelf's range of filing keys, on its label.
        for f in faces {
            let (center, normal) = (self.faces[f].center, self.faces[f].normal);
            for row in 0..ROWS {
                let Some(keys) = self.shelf_keys(f, row) else { continue };
                let ((y0, y1), (_, out)) = kit::label_holder(row);
                let at = center + Vec3::Y * ((y0 + y1) * 0.5) + normal * out;
                self.inscribe(&keys, SHELF_LETTERS, at, normal, kit::LABEL_W - 0.01);
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
                center + Vec2::new(libm::cosf(a), libm::sinf(a)) * r,
                Vec2::new(libm::cosf(yaw), libm::sinf(yaw)),
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
    /// overlapping where they can be seen from inside the library. (Meshes are left
    /// out: their bounds are faces only where they are flat.)
    fn coplanar(layout: Layout) -> Vec<String> {
        let lib = Library::generate(layout, 7);
        let boxes: Vec<(Vec3, Vec3, u32)> = (lib.static_insts.iter())
            .filter(|i| i.mesh == mesh::BOX)
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

    /// Every slot is open to what sends books home from a distance (the aura, the rush,
    /// homing, the familiars): from a little before where its book lines up, there is
    /// nothing in the way. (Before `line_up`, the corners of the Alchemical Wing's
    /// alcoves and of two runs in the Hearth Library had some 500 shut.)
    #[test]
    fn every_slot_is_open_to_the_room() {
        for layout in Layout::ALL {
            let lib = Library::generate(layout, 7);
            let shut: Vec<u32> = (0..lib.books.len() as u32)
                .filter(|&b| {
                    let face = &lib.faces[lib.books[b as usize].face as usize];
                    !lib.reaches_front(b, lib.slot_front(b) + face.normal * 0.5)
                })
                .collect();
            assert!(
                shut.is_empty(),
                "{layout:?}: {} slots shut, as {:?}",
                shut.len(),
                shut.first()
            );
        }
    }

    /// No case stands far from a sign naming its category, however its category's
    /// cases are split up (across a nave, a door, a window): no farther than `NEAR`
    /// across the floor. (With one sign to a category, as there was, a case of Feast
    /// Days stood 19 m from it.)
    #[test]
    fn every_case_has_its_sign_near() {
        const NEAR: f32 = 9.0;
        for layout in Layout::ALL {
            let lib = Library::generate(layout, 7);
            let mut worst = (0.0f32, "");
            for f in &lib.faces {
                let near = (lib.labels.iter())
                    .filter(|l| matches!(l.kind, LabelKind::Category(c) if c == f.category))
                    .map(|l| (l.pos - f.center).with_y(0.0).length())
                    .fold(f32::MAX, f32::min);
                if near > worst.0 {
                    worst = (near, lib.catalog.categories[f.category].name.as_str());
                }
            }
            assert!(
                worst.0 <= NEAR,
                "{layout:?}: a case of {} stands {:.1} m from its nearest sign",
                worst.1,
                worst.0
            );
        }
    }
}
