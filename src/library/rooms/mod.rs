//! What stands inside each Great Library department's room: its bookcases, laid out
//! as suits the department, and its furnishings. The building around it (the walls,
//! the doorway with its sign and gate) is great.rs's. Each department has a file of
//! its own; the helpers they share are here.

use super::great::{ATRIUM, DOOR_HALF, OUTER, ROOM_H, ROOM_HALF, Side, WALL_T, box3};
use super::*;

mod alchemical_wing;
mod arcanum;
mod bestiary;
mod chronicle_hall;
mod clockworks;
mod hearth_library;
mod herbarium;
mod map_room;
mod music_room;
mod observatory;
mod verse_gallery;
mod weatherhouse;

pub(super) use clockworks::stopped_clock;

/// A department's room, in its own frame: `a` runs across the room (along the wall
/// with the door), `o` outward from the atrium's centre, heights from its floor.
pub(super) struct RoomFrame {
    pub dept: usize,
    /// The floor's height.
    pub y: f32,
    pub side: Side,
    /// Where along the near wall the doorway's middle is.
    pub door: f32,
    /// The department's categories (8).
    pub categories: std::ops::Range<usize>,
}

impl RoomFrame {
    /// Inside the walls: `a` within ±A_HALF, `o` from O_NEAR (the wall with the door)
    /// to O_FAR (the far wall).
    pub const A_HALF: f32 = ROOM_HALF - WALL_T * 0.5;
    pub const O_NEAR: f32 = ATRIUM + WALL_T * 0.5;
    pub const O_FAR: f32 = OUTER - WALL_T * 0.5;
    /// Floor to the underside of the floor above.
    pub const HEIGHT: f32 = ROOM_H;
    pub const DOOR_HALF: f32 = DOOR_HALF;

    /// The point `a`, `o` at `dy` above the floor.
    pub fn at(&self, a: f32, o: f32, dy: f32) -> Vec3 {
        self.side.at3(a, o, self.y + dy)
    }

    /// The world box spanning `a0..a1` × `o0..o1`, `dy0..dy1` above the floor.
    pub fn boxed(&self, a0: f32, a1: f32, o0: f32, o1: f32, dy0: f32, dy1: f32) -> (Vec3, Vec3) {
        let (mn, mx) = self.side.rect(a0, a1, o0, o1);
        box3(mn, mx, self.y + dy0, self.y + dy1)
    }

    /// The world rectangle (x, z) spanning `a0..a1` × `o0..o1`.
    pub fn rect(&self, a0: f32, a1: f32, o0: f32, o1: f32) -> (Vec2, Vec2) {
        self.side.rect(a0, a1, o0, o1)
    }

    /// The world direction of +o (away from the atrium) and of +a.
    pub fn out(&self) -> Vec3 {
        self.side.out3()
    }

    pub fn across(&self) -> Vec3 {
        self.side.across3()
    }

    /// Whether +o is along world x (a case running outward then runs along x).
    pub fn out_along_x(&self) -> bool {
        self.side.along_x()
    }

    /// The room's frame as a rotation: x across, y up, z toward the door's wall.
    pub fn turn(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(self.across(), Vec3::Y, -self.out()))
    }

    /// A world size for a box `sa` across, `sy` high and `so` outward.
    pub fn size(&self, sa: f32, sy: f32, so: f32) -> Vec3 {
        if self.out_along_x() {
            Vec3::new(so, sy, sa)
        } else {
            Vec3::new(sa, sy, so)
        }
    }
}

/// Furnish department `frame.dept`'s room.
pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let f = match frame.dept {
        0 => observatory::furnish,
        1 => alchemical_wing::furnish,
        2 => herbarium::furnish,
        3 => bestiary::furnish,
        4 => map_room::furnish,
        5 => chronicle_hall::furnish,
        6 => verse_gallery::furnish,
        7 => arcanum::furnish,
        8 => clockworks::furnish,
        9 => music_room::furnish,
        10 => weatherhouse::furnish,
        11 => hearth_library::furnish,
        d => unreachable!("no room for department {d}"),
    };
    f(lib, frame, rng, titles);
}

/// A flat band round `c` in the plane of `u` and `v` (unit, at right angles), `r` to
/// its middle, `w` wide and `d` thick.
pub(super) fn band(
    lib: &mut Library,
    c: Vec3,
    (u, v): (Vec3, Vec3),
    (r, w, d): (f32, f32, f32),
    kind: u32,
    rgba: [f32; 4],
) -> Inst {
    let segs = if r < 0.5 { 32 } else { 64 };
    let ring = Mesh::ring(&circle(r + w * 0.5, segs), &circle(r - w * 0.5, segs), d);
    let turn = Quat::from_mat3(&Mat3::from_cols(u, v, u.cross(v)));
    lib.shape(ring).at(c, turn, kind, rgba)
}

/// A box over `a` across, `o` outward and `y` up from the floor.
pub(super) fn piece(f: &RoomFrame, a: (f32, f32), o: (f32, f32), y: (f32, f32), kind: u32, rgb: [f32; 3]) -> Inst {
    let (lo, hi) = f.boxed(a.0, a.1, o.0, o.1, y.0, y.1);
    Inst::minmax(lo, hi, kind, rgb)
}

/// Drawn only: the box over `a`, `o`, and heights `y` above the floor.
pub(super) fn slab(
    lib: &mut Library,
    f: &RoomFrame,
    a: (f32, f32),
    o: (f32, f32),
    y: (f32, f32),
    kind: u32,
    rgb: [f32; 3],
) {
    lib.decor(piece(f, a, o, y, kind, rgb));
}

/// A window's glass over `a`, `o` and heights `y` above the floor, glowing `rgba`
/// (`kind::GLASS`). Its leading runs across its own x, so it is turned to lie along
/// the wall it is in.
pub(super) fn pane(lib: &mut Library, f: &RoomFrame, a: (f32, f32), o: (f32, f32), y: (f32, f32), rgba: [f32; 4]) {
    let (lo, hi) = f.boxed(a.0, a.1, o.0, o.1, y.0, y.1);
    let size = hi - lo;
    let (rot, size) = if size.z > size.x {
        (
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(size.z, size.y, size.x),
        )
    } else {
        (Quat::IDENTITY, size)
    };
    lib.decor(Inst::new((lo + hi) * 0.5, rot, size, kind::GLASS, rgba, 0));
}

/// In the way of everything: the floor box over `a`, `o`, `h` high.
pub(super) fn guard(lib: &mut Library, f: &RoomFrame, a: (f32, f32), o: (f32, f32), h: f32) {
    let (lo, hi) = f.boxed(a.0, a.1, o.0, o.1, 0.0, h);
    lib.collide(lo, hi, Stops::ALL);
}

/// A slim box from `p` to `q` (world), `w` × `d` across.
pub(super) fn rod(lib: &mut Library, p: Vec3, q: Vec3, w: f32, d: f32, kind: u32, col: [f32; 4]) {
    let v = q - p;
    let rot = Quat::from_rotation_arc(Vec3::Y, v.normalize());
    lib.decor(Inst::new((p + q) * 0.5, rot, Vec3::new(w, v.length(), d), kind, col, 0));
}

/// `faces` (all facing one way) left to right as you face them, as `add_category`
/// takes them.
pub(super) fn left_to_right(mut faces: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    let right = Vec3::Y.cross(faces[0].1);
    faces.sort_by(|p, q| p.0.dot(right).total_cmp(&q.0.dot(right)));
    faces
}

/// A piece of furniture's place: its origin on the floor and its turn. Its parts are
/// given in its own frame (x across the room, y up, z toward the door's wall, unless
/// it is turned).
pub(super) struct Spot {
    pub at: Vec3,
    pub q: Quat,
}

impl Spot {
    pub fn new(f: &RoomFrame, a: f32, o: f32, yaw: f32) -> Spot {
        Spot {
            at: f.at(a, o, 0.0),
            q: f.turn() * Quat::from_rotation_y(yaw),
        }
    }

    pub fn part(&self, lib: &mut Library, p: Vec3, size: Vec3, q: Quat, kind: u32, color: [f32; 4]) {
        lib.decor(Inst::new(self.at + self.q * p, self.q * q, size, kind, color, 0));
    }

    /// An unturned part.
    pub fn block(&self, lib: &mut Library, p: Vec3, size: Vec3, kind: u32, color: [f32; 4]) {
        self.part(lib, p, size, Quat::IDENTITY, kind, color);
    }
}
