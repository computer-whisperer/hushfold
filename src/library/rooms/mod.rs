//! What stands inside each Great Library department's room: its bookcases, laid out
//! as suits the department, and its furnishings. The building around it (the walls,
//! the doorway with its sign and gate) is great.rs's. Each department has a file of
//! its own; `classic` is the room as it first was, for any to start from.

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

/// A department's room, in its own frame: `a` runs across the room (along the wall
/// with the door), `o` outward from the atrium's centre, heights from its floor.
// TEMP(visual pass): `level` and `drop` are for the departments' own layouts.
#[allow(dead_code)]
pub(super) struct RoomFrame {
    pub dept: usize,
    pub level: usize,
    /// The floor's height.
    pub y: f32,
    pub side: Side,
    /// Where along the near wall the doorway's middle is.
    pub door: f32,
    /// Where couriers set books down, just inside the door.
    pub drop: Vec3,
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
        _ => classic,
    };
    f(lib, frame, rng, titles);
}

/// The room as it first was: `classic_walls` and `classic_rows`.
pub(super) fn classic(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic_walls(lib, frame);
    classic_rows(lib, frame, rng, titles);
}

/// Three tall moonlit windows in the far wall, sconces down the sides, panelling
/// round the walls and beams across the ceiling.
pub(super) fn classic_walls(lib: &mut Library, frame: &RoomFrame) {
    let (out, across) = (frame.out(), frame.across());
    for a in [-7.0f32, 0.0, 7.0] {
        let c = frame.at(a, RoomFrame::O_FAR - 0.02, 3.0);
        lib.decor(Inst::aabox(
            c,
            frame.size(1.6, 3.4, 0.08),
            kind::GLASS,
            [0.32, 0.42, 0.85],
        ));
        lib.static_insts.last_mut().unwrap().color[3] = 1.2;
        lib.light(c - out * 1.5, 8.0, [0.45, 0.55, 1.0], 0.8, LightKind::Static);
        let beam = (-out * 1.0 - Vec3::Y * 0.6).normalize();
        let len = 7.0;
        lib.static_trans.push(Inst::new(
            c + beam * len * 0.5,
            Quat::from_rotation_arc(Vec3::Y, beam),
            Vec3::new(1.4, len, 2.6),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.04],
            0,
        ));
    }
    for o in [19.5, 27.0] {
        for sa in [-1.0f32, 1.0] {
            let p = frame.at(sa * (RoomFrame::A_HALF - 0.15), o, 2.6);
            lib.sconce(p, -across * sa);
        }
    }
    let inner = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    let t = 0.07;
    let side = frame.side;
    lib.wainscot(frame.y, frame.rect(-inner, -inner + t, near, far - t), side.across);
    lib.wainscot(frame.y, frame.rect(inner - t, inner, near, far - t), -side.across);
    lib.wainscot(frame.y, frame.rect(-inner, inner, far - t, far), -side.out);
    let d = frame.door;
    for (a0, a1) in [
        (-inner + t, d - RoomFrame::DOOR_HALF - 0.3),
        (d + RoomFrame::DOOR_HALF + 0.3, inner - t),
    ] {
        lib.wainscot(frame.y, frame.rect(a0, a1, near, near + t), side.out);
    }
    for k in 0..6 {
        let o = near + 1.6 + k as f32 * 3.0;
        let (lo, hi) = frame.boxed(
            -inner,
            inner,
            o - 0.13,
            o + 0.13,
            RoomFrame::HEIGHT - 0.3,
            RoomFrame::HEIGHT,
        );
        lib.decor(Inst::minmax(lo, hi, kind::WOOD, [0.11, 0.055, 0.025]));
    }
}

/// Four rows of five double-sided cases running outward; each side of a row is a
/// category.
pub(super) fn classic_rows(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let case_o = |k: usize| RoomFrame::O_NEAR + 2.5 + UNIT_W * 0.5 + k as f32 * UNIT_PITCH;
    let mut cat = frame.categories.start;
    for ra in [-9.0f32, -3.0, 3.0, 9.0] {
        let cases: Vec<[(Vec3, Vec3); 2]> = (0..5)
            .map(|k| lib.bookcase(frame.at(ra, case_o(k), 0.0), frame.out_along_x()))
            .collect();
        // Each side of the row, its faces left to right as you face them.
        for s in 0..2 {
            let mut faces: Vec<(Vec3, Vec3)> = cases.iter().map(|c| c[s]).collect();
            let right = Vec3::Y.cross(faces[0].1);
            faces.sort_by(|a, b| a.0.dot(right).total_cmp(&b.0.dot(right)));
            lib.add_category(rng, cat, &faces, titles);
            cat += 1;
        }
    }
}
