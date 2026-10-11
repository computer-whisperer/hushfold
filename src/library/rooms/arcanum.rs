//! The Arcanum: department 7, first floor, west side. A dim room hung with dark
//! drapes, its walls lined with cases; in the middle a ward of cases stands square
//! round a rune circle set glowing in the floor, a gate in each side and a round
//! stone pillar at each corner; crystals float over the circle, bobbing and turning,
//! the lesser ones circling the great. Grimoires lie chained to lecterns in the
//! aisles.
//!
//! The ward's strands go out across the door as the room fills (`game::rooms`); put in
//! order, it draws the ward in: the light streams to the great crystal and circles it
//! as three rings, runes spread across the floor, and the grimoires slip their chains
//! and fall open. Each of the ward's corner crystals kindles as its cases fill.

use super::*;
use crate::audio::Sfx;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// The ward's middle, its sides' distance from it (to their cases' middles), and
/// where its cases stand along a side (either side of the gate).
const WARD: f32 = 23.6;
const SIDE: f32 = 4.37;
const UNIT: f32 = 2.54;
/// Its corner pillars span this far from the middle.
const PILLAR: (f32, f32) = (3.87, 4.87);
/// The cases on the side walls and on the near wall either side of the door.
const SIDE_CASES: [f32; 5] = [17.65, 20.35, 23.05, 25.75, 28.45];
const NEAR_LEFT: [f32; 3] = [-10.6, -7.9, -5.2];
const NEAR_RIGHT: [f32; 2] = [7.0, 9.7];
const RUNE: [f32; 4] = [0.75, 0.35, 1.0, 1.6];
const CRYSTAL_RGBA: [f32; 4] = [0.5, 0.26, 0.88, 1.1];
const DRAPE: [f32; 3] = [0.1, 0.04, 0.13];
const IRON: [f32; 4] = [0.16, 0.16, 0.18, 1.0];
/// The ward's light, as its strands across the door are drawn (`game::rooms`).
const STRAND: [f32; 4] = [0.62, 0.36, 1.0, 1.5];
const PAGES: [f32; 4] = [0.72, 0.66, 0.53, 1.0];

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    walls(lib, frame);
    shelves(lib, frame, rng, titles);
    pillars(lib, frame);
    rune_circle(lib, frame);
    outer_runes(lib, frame);
    crystals(lib, frame);
    ward_drawn_in(lib, frame);
    for (i, (s, o)) in [(-1.0f32, 19.9f32), (-1.0, 27.3), (1.0, 19.9), (1.0, 27.3)]
        .into_iter()
        .enumerate()
    {
        lectern(lib, frame, s, o, i);
    }
}

/// The wall cases, read round the room from the door's left as one chain (near
/// wall, left wall, far wall, right wall, near wall) cut into four categories of six;
/// and the ward, whose inner faces make two categories of four (each reading round
/// a corner of the court) and whose outer faces make two more.
fn shelves(lib: &mut Library, f: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    let (out, across) = (f.out(), f.across());
    let ah = RoomFrame::A_HALF;
    let near_left = NEAR_LEFT.map(|a| lib.wall_bookcase(f.at(a, RoomFrame::O_NEAR, 0.0), out));
    let left = SIDE_CASES.map(|o| lib.wall_bookcase(f.at(-ah, o, 0.0), across));
    let far: Vec<(Vec3, Vec3)> = (0..9)
        .map(|k| lib.wall_bookcase(f.at(-10.8 + 2.7 * k as f32, RoomFrame::O_FAR, 0.0), -out))
        .collect();
    let right = SIDE_CASES.map(|o| lib.wall_bookcase(f.at(ah, o, 0.0), -across));
    let near_right = NEAR_RIGHT.map(|a| lib.wall_bookcase(f.at(a, RoomFrame::O_NEAR, 0.0), out));
    let chain = [
        left_to_right(near_left.to_vec()),
        left_to_right(left.to_vec()),
        left_to_right(far),
        left_to_right(right.to_vec()),
        left_to_right(near_right.to_vec()),
    ]
    .concat();
    // The ward's sides (far, right, near, left as `a` and `o` run), each side's faces
    // looking into the court and out of it.
    let mut inner: [Vec<(Vec3, Vec3)>; 4] = Default::default();
    let mut outer: [Vec<(Vec3, Vec3)>; 4] = Default::default();
    let middle = f.at(0.0, WARD, 0.0);
    let along_a = !f.out_along_x();
    for (i, (u, v, along)) in [
        (0.0f32, SIDE, along_a),
        (SIDE, 0.0, !along_a),
        (0.0, -SIDE, along_a),
        (-SIDE, 0.0, !along_a),
    ]
    .into_iter()
    .enumerate()
    {
        for k in [-1.0f32, 1.0] {
            let (a, o) = if u == 0.0 {
                (k * UNIT, WARD + v)
            } else {
                (u, WARD + k * UNIT)
            };
            for face in lib.bookcase(f.at(a, o, 0.0), along) {
                if (middle - face.0).dot(face.1) > 0.0 {
                    inner[i].push(face);
                } else {
                    outer[i].push(face);
                }
            }
        }
    }
    let [far_in, right_in, near_in, left_in] = inner.map(left_to_right);
    let [far_out, right_out, near_out, left_out] = outer.map(left_to_right);
    let mut groups: Vec<Vec<(Vec3, Vec3)>> = chain.chunks(6).map(|c| c.to_vec()).collect();
    groups.push([far_out, left_out].concat());
    groups.push([near_out, right_out].concat());
    groups.push([far_in, right_in].concat());
    groups.push([near_in, left_in].concat());
    for (cat, faces) in f.categories.clone().zip(groups) {
        lib.add_category(rng, cat, &faces, titles);
    }
}

/// Dark beams, drapes hung above the cases all round, and one tall window in the far
/// wall between heavy curtains, its moonlight falling toward the ward.
fn walls(lib: &mut Library, f: &RoomFrame) {
    let h = RoomFrame::HEIGHT;
    let ah = RoomFrame::A_HALF;
    let (near, far) = (RoomFrame::O_NEAR, RoomFrame::O_FAR);
    for o in [16.0f32, 19.6, 23.6, 27.6, 30.8] {
        slab(
            lib,
            f,
            (-ah, ah),
            (o - 0.14, o + 0.14),
            (h - 0.3, h),
            kind::WOOD,
            [0.06, 0.03, 0.02],
        );
    }
    // Swagged drapes over the side cases, a fold to each case.
    for s in [-1.0f32, 1.0] {
        let wall = |o: f32| Spot::new(f, s * ah, o, -s * FRAC_PI_2);
        for &o in &SIDE_CASES {
            folds(lib, &wall(o), 2.7, 3.4);
        }
    }
    for k in 0..9 {
        let a = -10.8 + 2.7 * k as f32;
        if k != 4 {
            folds(lib, &Spot::new(f, a, far, 0.0), 2.7, 3.4);
        }
    }
    // The window over the middle of the far wall, its curtains half drawn.
    let at = Spot::new(f, 0.0, far, 0.0);
    let stone = [0.3, 0.28, 0.26, 1.0];
    at.block(
        lib,
        Vec3::new(0.0, 4.5, -0.02),
        Vec3::new(1.0, 1.6, 0.08),
        kind::GLASS,
        [0.32, 0.42, 0.85, 1.0],
    );
    at.block(
        lib,
        Vec3::new(0.0, 3.64, 0.06),
        Vec3::new(1.4, 0.12, 0.12),
        kind::STONE,
        stone,
    );
    for x in [-1.0f32, 1.0] {
        at.block(
            lib,
            Vec3::new(x * 0.58, 4.5, 0.05),
            Vec3::new(0.16, 1.6, 0.1),
            kind::STONE,
            stone,
        );
        for (i, dx) in [0.0f32, 0.18, 0.36].into_iter().enumerate() {
            at.block(
                lib,
                Vec3::new(x * (0.78 + dx), 4.42, 0.14 + i as f32 * 0.04),
                Vec3::new(0.22, 2.36 - i as f32 * 0.08, 0.06),
                kind::CLOTH,
                rgba(DRAPE, 1.0),
            );
        }
    }
    at.block(
        lib,
        Vec3::new(0.0, 5.45, 0.23),
        Vec3::new(3.0, 0.3, 0.1),
        kind::CLOTH,
        rgba(DRAPE, 1.0),
    );
    let c = f.at(0.0, far - 0.1, 4.5);
    let dir = (-f.out() * 1.0 - Vec3::Y * 0.55).normalize();
    let len = 9.0;
    lib.static_trans.push(Inst::new(
        c + dir * len * 0.5,
        Quat::from_rotation_arc(Vec3::Y, dir),
        Vec3::new(0.9, len, 0.9),
        kind::SHAFT,
        [0.35, 0.45, 0.9, 0.03],
        0,
    ));
    lib.light(c - f.out() * 1.5, 7.0, [0.45, 0.55, 1.0], 0.5, LightKind::Static);
    // The near wall, bare but for panelling either side of the door.
    let d = RoomFrame::DOOR_HALF + 0.3;
    let t = 0.07;
    lib.wainscot(f.y, f.rect(NEAR_LEFT[2] + 1.4, f.door - d, near, near + t), f.side.out);
    lib.wainscot(f.y, f.rect(f.door + d, NEAR_RIGHT[0] - 1.4, near, near + t), f.side.out);
}

/// Drapes hung over a case `w` wide on a wall at `at`, from under the ceiling down to
/// `y0`, gathered in folds and swagged up at the middle.
fn folds(lib: &mut Library, at: &Spot, w: f32, y0: f32) {
    let top = RoomFrame::HEIGHT - 0.02;
    let n = 3;
    for i in 0..n {
        let x = -w * 0.5 + (i as f32 + 0.5) * w / n as f32;
        let middle = i == 1;
        let bottom = if middle { y0 + 0.5 } else { y0 };
        at.block(
            lib,
            Vec3::new(x, (top + bottom) * 0.5, 0.06 + 0.03 * (i % 2) as f32),
            Vec3::new(w / n as f32 - 0.02, top - bottom, 0.05),
            kind::CLOTH,
            rgba(DRAPE, 1.0),
        );
    }
    at.block(
        lib,
        Vec3::new(0.0, top - 0.15, 0.14),
        Vec3::new(w, 0.26, 0.06),
        kind::CLOTH,
        [0.16, 0.06, 0.18, 1.0],
    );
}

/// The ward's corner pillars, round on square plinths under square abaci (in the
/// way as the square blocks they stand in), each with a small crystal floating over
/// it, dull until the category whose cases meet at that corner is done, then
/// kindling.
fn pillars(lib: &mut Library, f: &RoomFrame) {
    // Where each category's cases are, on the whole: one of the ward's at each corner.
    let middles: Vec<(usize, Vec3)> = (f.categories.clone())
        .map(|c| {
            let at: Vec<Vec3> = lib.faces_of(c).map(|i| lib.faces[i].center).collect();
            (c, at.iter().sum::<Vec3>() / at.len() as f32)
        })
        .collect();
    let (p0, p1) = PILLAR;
    let stone = [0.3, 0.28, 0.27];
    let shaft = [
        (0.0, 0.0),
        (0.44, 0.0),
        (0.44, 0.06),
        (0.4, 0.1),
        (0.4, 0.14),
        (0.36, 0.18),
        (0.355, 0.5),
        (0.35, 1.5),
        (0.33, 2.4),
        (0.32, 2.62),
        (0.36, 2.66),
        (0.36, 2.7),
        (0.0, 2.7),
    ];
    let shaft = lib.shape(Mesh::lathe(&shaft.map(Vec2::from), 48));
    let capital = [
        (0.0, 0.0),
        (0.32, 0.0),
        (0.34, 0.05),
        (0.42, 0.2),
        (0.48, 0.32),
        (0.5, 0.4),
        (0.0, 0.4),
    ];
    let capital = lib.shape(Mesh::lathe(&capital.map(Vec2::from), 48));
    for (k, (sa, so)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)]
        .into_iter()
        .enumerate()
    {
        let (a0, a1) = (sa * p0, sa * p1);
        let (o0, o1) = (WARD + so * p0, WARD + so * p1);
        let (lo, hi) = f.boxed(a0, a1, o0, o1, 0.0, 3.4);
        lib.collide(lo, hi, Stops::ALL);
        let (lo, hi) = f.boxed(a0, a1, o0, o1, 0.0, 0.3);
        lib.decor(Inst::minmax(lo, hi, kind::STONE, stone));
        let (am, om) = ((a0 + a1) * 0.5, (o0 + o1) * 0.5);
        let foot = f.at(am, om, 0.3);
        lib.decor(shaft.at(foot, Quat::IDENTITY, kind::STONE, rgba(stone, 1.0)));
        lib.decor(capital.at(foot + Vec3::Y * 2.7, Quat::IDENTITY, kind::STONE, rgba(stone, 1.0)));
        slab(
            lib,
            f,
            (am - 0.6, am + 0.6),
            (om - 0.6, om + 0.6),
            (3.4, 3.55),
            kind::STONE,
            [0.36, 0.34, 0.32],
        );
        let gem = crystal(lib, f, Vec3::new(am, 4.05, om), 0.6, sa * so * 0.3, k, None);
        let corner = f.at(am, om, 0.0);
        let (cat, _) = (middles.iter())
            .min_by(|p, q| p.1.distance(corner).total_cmp(&q.1.distance(corner)))
            .expect("categories");
        let done = Finish::Category(*cat);
        lib.awaiting(gem, Awaits::new(done, Change::Kindles { dim: 0.4 }).over(3.0));
        lib.sparks(Sparks {
            rgba: STRAND,
            rate: 40.0,
            ..Sparks::burst(done, f.at(am, om, 4.35))
        });
    }
}

/// A crystal `size` long floating at `p` (a, height, o), leaning by `lean`: a
/// six-sided prism pointed at both ends, bobbing gently and turning slowly on its own
/// axis (each in its own time, by `k`), riding `orbit` if given. Returns the mover it
/// floats on.
fn crystal(lib: &mut Library, f: &RoomFrame, p: Vec3, size: f32, lean: f32, k: usize, orbit: Option<u32>) -> u32 {
    let at = f.at(p.x, p.z, p.y);
    let tilt = f.turn() * Quat::from_rotation_z(lean) * Quat::from_rotation_y(lean * 2.0);
    let (r, tip) = (size * 0.15, size * 0.24);
    let gem = [(0.0, 0.0), (r, tip), (r, size - tip), (0.0, size)];
    let gem = lib.shape(Mesh::lathe(&gem.map(Vec2::from), 6));
    let k = k as f32;
    let float = Motion::Bob {
        amp: 0.04 + 0.01 * (k % 3.0),
        period: 6.5 + 0.8 * k,
        rock: 0.04,
    };
    let bob = lib.mover(at, float, k * 1.7, orbit);
    let turn = Motion::Spin {
        axis: tilt * Vec3::Y,
        rate: (0.12 + 0.03 * (k % 4.0)) * if k % 2.0 == 0.0 { 1.0 } else { -1.0 },
    };
    let m = lib.mover(at, turn, 0.0, Some(bob));
    lib.moving(m, gem.inst(at, tilt, kind::CRYSTAL, CRYSTAL_RGBA));
    bob
}

/// The great crystal over a low round dais in the middle of the circle, and its
/// lesser ones circling it slowly.
fn crystals(lib: &mut Library, f: &RoomFrame) {
    guard(lib, f, (-0.65, 0.65), (WARD - 0.65, WARD + 0.65), 0.4);
    let stone = [0.3, 0.28, 0.27, 1.0];
    let dais = [
        (0.0, 0.0),
        (0.62, 0.0),
        (0.62, 0.12),
        (0.58, 0.16),
        (0.52, 0.18),
        (0.5, 0.3),
        (0.44, 0.34),
        (0.4, 0.38),
        (0.0, 0.38),
    ];
    let dais = lib.shape(Mesh::lathe(&dais.map(Vec2::from), 48));
    let middle = f.at(0.0, WARD, 0.0);
    lib.decor(dais.at(middle, Quat::IDENTITY, kind::STONE, stone));
    let seal = lib.shape(Mesh::cylinder(32));
    lib.decor(seal.sized(Vec3::new(0.3, 0.01, 0.3)).inst(
        middle + Vec3::Y * 0.384,
        Quat::IDENTITY,
        kind::EMISSIVE,
        RUNE,
    ));
    crystal(lib, f, Vec3::new(0.0, 3.3, WARD), 1.1, 0.08, 4, None);
    let circling = Motion::Spin {
        axis: Vec3::Y,
        rate: TAU / 150.0,
    };
    let orbit = lib.mover(middle, circling, 0.0, None);
    for (k, (r, y, size)) in [
        (1.5f32, 2.75f32, 0.45f32),
        (1.3, 3.7, 0.4),
        (1.7, 3.1, 0.5),
        (1.2, 2.6, 0.35),
    ]
    .into_iter()
    .enumerate()
    {
        let t = k as f32 / 4.0 * TAU + 0.4;
        crystal(
            lib,
            f,
            Vec3::new(libm::cosf(t) * r, y, WARD + libm::sinf(t) * r),
            size,
            0.25 + k as f32 * 0.2,
            5 + k,
            Some(orbit),
        );
    }
    lib.light(f.at(0.0, WARD, 1.4), 7.5, [0.65, 0.4, 1.0], 1.1, LightKind::Static);
}

/// The rune circle in the floor of the court: two rings with runes between them, a
/// seven-pointed star inside, a small ring at its heart, all glowing faintly.
fn rune_circle(lib: &mut Library, f: &RoomFrame) {
    let at = Spot::new(f, 0.0, WARD, 0.0);
    let line = |lib: &mut Library, p: Vec2, q: Vec2, w: f32, y: f32| {
        let d = q - p;
        let m = (p + q) * 0.5;
        at.part(
            lib,
            Vec3::new(m.x, y * 0.5, m.y),
            Vec3::new(d.length(), y, w),
            Quat::from_rotation_y(-libm::atan2f(d.y, d.x)),
            kind::EMISSIVE,
            RUNE,
        );
    };
    for (r, w, h) in [(2.9f32, 0.05f32, 0.004f32), (2.35, 0.035, 0.005), (1.0, 0.03, 0.006)] {
        let circle = |r: f32| -> Vec<Vec2> { (0..96).map(|k| Vec2::from_angle(k as f32 / 96.0 * TAU) * r).collect() };
        let ring = lib.shape(Mesh::ring(&circle(r + w * 0.5), &circle(r - w * 0.5), h));
        lib.decor(ring.inst(
            at.at + Vec3::Y * (h * 0.5),
            Quat::from_rotation_x(FRAC_PI_2),
            kind::EMISSIVE,
            RUNE,
        ));
    }
    // The star: each point joined to the point two on.
    for k in 0..7 {
        let p = Vec2::from_angle(k as f32 / 7.0 * TAU + FRAC_PI_2) * 2.3;
        let q = Vec2::from_angle((k + 2) as f32 / 7.0 * TAU + FRAC_PI_2) * 2.3;
        line(lib, p, q, 0.028, 0.007);
    }
    // Runes: a stave and a stroke or two each, standing round between the rings.
    let mut r = StdRng::seed_from_u64(0x5275_6e65);
    for k in 0..14 {
        let t = (k as f32 + 0.5) / 14.0 * TAU;
        let c = Vec2::from_angle(t) * 2.62;
        let up = Vec2::from_angle(t) * 0.15;
        let side = Vec2::new(-up.y, up.x);
        line(lib, c - up, c + up, 0.022, 0.006);
        for _ in 0..r.gen_range(1..3) {
            let a = r.gen_range(-1.0..1.0f32);
            let b = r.gen_range(-1.0..1.0f32);
            let s = if r.gen_bool(0.5) { 1.0 } else { -1.0 };
            line(lib, c + up * a, c + up * b + side * s * 0.6, 0.02, 0.0065);
        }
    }
}

/// A ring of runes round the old circle, an inlay of light laid as one piece: once the
/// room is in order it spreads out from the dais across the floor and settles there.
fn outer_runes(lib: &mut Library, f: &RoomFrame) {
    let h = 0.0028;
    let circle = |r: f32| -> Vec<Vec2> { (0..128).map(|k| Vec2::from_angle(k as f32 / 128.0 * TAU) * r).collect() };
    let mut runes = Mesh::default();
    let ring = Mesh::ring(&circle(3.47), &circle(3.43), h);
    runes.add(&ring, Vec3::ZERO, Quat::from_rotation_x(FRAC_PI_2), Vec3::ONE);
    let cube = Mesh::cube();
    let mut line = |p: Vec2, q: Vec2, w: f32| {
        let (d, c) = (q - p, (p + q) * 0.5);
        let turn = Quat::from_rotation_y(-libm::atan2f(d.y, d.x));
        runes.add(&cube, Vec3::new(c.x, 0.0, c.y), turn, Vec3::new(d.length(), h, w));
    };
    let mut r = StdRng::seed_from_u64(0x4f75_7465_7252);
    for k in 0..20 {
        let t = (k as f32 + 0.5) / 20.0 * TAU;
        let c = Vec2::from_angle(t) * 3.2;
        let up = Vec2::from_angle(t) * 0.14;
        let side = up.perp();
        line(c - up, c + up, 0.02);
        for _ in 0..r.gen_range(1..3) {
            let a = r.gen_range(-1.0..1.0f32);
            let b = r.gen_range(-1.0..1.0f32);
            let s = if r.gen_bool(0.5) { 1.0 } else { -1.0 };
            line(c + up * a, c + up * b + side * s * 0.6, 0.018);
        }
    }
    let middle = f.at(0.0, WARD, h * 0.5 + 0.0002);
    let spreads = Change::Arrives {
        from: Vec3::ZERO,
        grow: 0.0,
    };
    let runes = lib.shape(runes).at(middle, f.turn(), kind::EMISSIVE, RUNE);
    lib.waiting(
        middle,
        Awaits::new(Finish::Room(f.dept), spreads).after(0.4).over(2.4),
        [runes],
    );
}

/// The ward, drawn in: once the room is in order its light streams in from over the
/// doorway to the great crystal, and three rings of it (the three strands that barred
/// the door) grow out of the crystal one after another and turn slowly about it, a
/// light riding them. Motes drift in the court after.
fn ward_drawn_in(lib: &mut Library, f: &RoomFrame) {
    let fin = Finish::Room(f.dept);
    let heart = f.at(0.0, WARD, 3.85);
    // From the doorway's head, over anyone's head, straight to the crystal.
    let door = f.at(f.door, RoomFrame::O_NEAR + 0.3, 2.95);
    lib.sparks(Sparks {
        rgba: [0.62, 0.36, 1.0, 3.0],
        size: Vec3::splat(0.02),
        spread: f.size(1.2, 0.25, 0.05),
        vel: (heart - door).normalize() * 3.4,
        jitter: Vec3::splat(0.08),
        lift: 0.0,
        drag: 0.0,
        wander: 0.05,
        life: 2.7,
        rate: 50.0,
        lasts: Some(2.0),
        ..Sparks::burst(fin, door)
    });
    let rings = [
        (0.7f32, 1.15f32, TAU / 26.0),
        (0.85, 1.3, -TAU / 34.0),
        (1.0, 1.0, TAU / 43.0),
    ];
    for (k, (r, tilt, rate)) in rings.into_iter().enumerate() {
        let ring = lib.shape(Mesh::torus(0.024 / r, 96, 8)).scaled(r);
        let lean = Quat::from_rotation_y(k as f32 * TAU / 3.0) * Quat::from_rotation_x(tilt);
        let m = lib.mover(heart, Motion::Spin { axis: Vec3::Y, rate }, 0.0, None);
        let grows = Change::Arrives {
            from: Vec3::ZERO,
            grow: 0.0,
        };
        lib.awaiting(m, Awaits::new(fin, grows).after(1.6 + 0.8 * k as f32).over(2.2).still());
        lib.moving(m, ring.inst(heart, lean, kind::EMISSIVE, STRAND));
        if k == 0 {
            lib.light_riding(m, heart, 6.5, [0.62, 0.36, 1.0], 0.8, LightKind::Static);
        }
    }
    lib.sparks(Sparks {
        delay: 1.6,
        rgba: [0.62, 0.36, 1.0, 3.0],
        rate: 90.0,
        spread: Vec3::splat(0.5),
        ..Sparks::burst(fin, heart)
    });
    lib.sparks(Sparks {
        rgba: [0.7, 0.45, 1.0, 1.4],
        ..Sparks::motes(fin, f.at(0.0, WARD, 2.6), Vec3::new(2.8, 1.0, 2.8))
    });
    lib.cue(Cue {
        finish: fin,
        delay: 1.6,
        sound: Sfx::Veil,
        at: heart,
        volume: 0.7,
    });
}

/// A heavy lectern in the aisle on side `s` at `o`, facing the ward, a grimoire on it
/// chained to its post; `k` picks the grimoire.
fn lectern(lib: &mut Library, f: &RoomFrame, s: f32, o: f32, k: usize) {
    let a = s * 8.5;
    guard(lib, f, (a - 0.4, a + 0.4), (o - 0.4, o + 0.4), 1.3);
    // Facing the ward: local z toward the middle of the room.
    let at = Spot::new(f, a, o, -s * FRAC_PI_2);
    let wood = [0.08, 0.04, 0.025, 1.0];
    at.block(lib, Vec3::Y * 0.05, Vec3::new(0.66, 0.1, 0.6), kind::WOOD, wood);
    for x in [-1.0f32, 1.0] {
        at.part(
            lib,
            Vec3::new(x * 0.2, 0.5, 0.0),
            Vec3::new(0.08, 0.95, 0.08),
            Quat::from_rotation_z(-x * 0.12),
            kind::WOOD,
            wood,
        );
    }
    at.block(lib, Vec3::Y * 0.5, Vec3::new(0.36, 0.06, 0.12), kind::METAL, IRON);
    let slope = Quat::from_rotation_x(0.45);
    let desk = Vec3::Y * 1.05;
    at.part(lib, desk, Vec3::new(0.66, 0.05, 0.5), slope, kind::WOOD, wood);
    at.part(
        lib,
        desk + slope * Vec3::new(0.0, 0.04, 0.26),
        Vec3::new(0.66, 0.08, 0.03),
        slope,
        kind::METAL,
        IRON,
    );
    // The grimoire, shut, its boards bound in iron and a sigil on its cover. Once the
    // room is in order its chain slips away and it falls open: the front board and half
    // the pages swing over the spine, the book sliding across to lie open on the desk.
    let fin = Finish::Room(f.dept);
    let covers = [
        [0.2, 0.05, 0.08],
        [0.06, 0.08, 0.16],
        [0.16, 0.1, 0.05],
        [0.08, 0.12, 0.08],
    ];
    let cover = rgba(covers[k % 4], 1.0);
    let book = desk + slope * Vec3::new(0.0, 0.1, -0.02);
    let turn = at.q * slope;
    let point = |p: Vec3| at.at + at.q * (book + slope * p);
    // Each half (`s` below or above where it opens): its board, its pages (written on
    // their faces), its half of the spine and of the iron straps, each by its middle
    // (across, and up from where it opens) and size.
    let half = |s: f32| {
        [
            (0.0, 0.0345, Vec3::new(0.42, 0.031, 0.34), kind::CLOTH, cover),
            (0.005, 0.0095, Vec3::new(0.4, 0.019, 0.33), kind::PARCHMENT, PAGES),
            (-0.204, 0.0095, Vec3::new(0.012, 0.019, 0.34), kind::CLOTH, cover),
            (-0.19, 0.0275, Vec3::new(0.04, 0.055, 0.35), kind::METAL, IRON),
            (0.19, 0.0275, Vec3::new(0.04, 0.055, 0.35), kind::METAL, IRON),
        ]
        .map(|(x, y, size, kind, color)| Inst::new(point(Vec3::new(x, s * y, 0.0)), turn, size, kind, color, 0))
    };
    let opens = 4.2 + 0.5 * k as f32;
    let slides = Change::Shifts {
        by: turn * Vec3::X * 0.21,
        turn: Quat::IDENTITY,
        scale: 1.0,
    };
    let base = lib.waiting(
        point(Vec3::ZERO),
        Awaits::new(fin, slides).after(opens).over(2.6),
        half(-1.0),
    );
    let spine = point(Vec3::new(-0.21, 0.0, 0.0));
    let swings = Change::Shifts {
        by: Vec3::ZERO,
        turn: Quat::from_axis_angle(turn * Vec3::Z, PI),
        scale: 1.0,
    };
    let front = lib.mover(spine, Motion::Still, 0.0, Some(base));
    lib.awaiting(front, Awaits::new(fin, swings).after(opens).over(2.6));
    for part in half(1.0) {
        lib.moving(front, part);
    }
    lib.moving(
        front,
        Inst::new(
            point(Vec3::new(0.0, 0.052, 0.0)),
            turn * Quat::from_rotation_y(FRAC_PI_4),
            Vec3::new(0.1, 0.004, 0.1),
            kind::EMISSIVE,
            [0.8, 0.4, 1.0, 1.2],
            0,
        ),
    );
    // The chain, from the book's spine down to a ring on the post; it drops away and
    // is gone a moment before the book opens.
    let from = point(Vec3::new(-0.21, 0.0, -0.1));
    let to = at.at + at.q * Vec3::new(-0.19, 0.6, 0.07);
    let links = 8;
    let dir = (to - from).normalize();
    let link = lib.shape(Mesh::torus(0.45, 12, 6));
    let slips = Change::Shifts {
        by: Vec3::Y * -0.3,
        turn: Quat::IDENTITY,
        scale: 0.0,
    };
    let chain = lib.mover(from.lerp(to, 0.5), Motion::Still, 0.0, None);
    lib.awaiting(chain, Awaits::new(fin, slips).after(opens - 1.4).over(1.4));
    for i in 0..links {
        let t = (i as f32 + 0.5) / links as f32;
        // A little sag in the middle; each link turned square to the last.
        let p = from.lerp(to, t) - Vec3::Y * (t * (1.0 - t) * 0.25);
        let twist =
            Quat::from_rotation_arc(Vec3::Z, dir) * Quat::from_rotation_z(if i % 2 == 0 { 0.0 } else { FRAC_PI_2 });
        lib.moving(
            chain,
            link.sized(Vec3::new(0.04, 0.01, 0.075))
                .inst(p, twist, kind::METAL, [0.3, 0.3, 0.32, 1.0]),
        );
    }
    lib.decor(link.sized(Vec3::new(0.06, 0.012, 0.06)).inst(
        to - at.q * Vec3::Z * 0.02,
        at.q * Quat::from_rotation_x(FRAC_PI_2),
        kind::METAL,
        IRON,
    ));
    if k.is_multiple_of(2) {
        // A guttering candle on an iron pricket, its flame. As the book opens beside it,
        // it rises off the desk to float over the pages, bobbing a little.
        let c = Vec3::new(0.26, 1.15, -0.05);
        let pricket = [
            (0.0, 0.0),
            (0.045, 0.0),
            (0.045, 0.01),
            (0.015, 0.02),
            (0.012, 0.05),
            (0.04, 0.06),
            (0.0, 0.065),
        ];
        let pricket = lib.shape(Mesh::lathe(&pricket.map(Vec2::from), 16));
        let foot = at.at + at.q * (c - Vec3::Y * 0.1);
        let floats = Motion::Bob {
            amp: 0.02,
            period: 4.5,
            rock: 0.0,
        };
        let candle = lib.mover(foot, floats, 0.0, None);
        let rises = Change::Shifts {
            by: at.q * Vec3::new(-0.08, 0.28, -0.08),
            turn: Quat::IDENTITY,
            scale: 1.0,
        };
        lib.awaiting(candle, Awaits::new(fin, rises).after(opens - 0.8).over(2.2).still());
        lib.moving(candle, pricket.at(foot, at.q, kind::METAL, IRON));
        let wax = [
            (0.0, 0.0),
            (0.026, 0.0),
            (0.026, 0.1),
            (0.022, 0.112),
            (0.015, 0.11),
            (0.0, 0.106),
        ];
        let wax = lib.shape(Mesh::lathe(&wax.map(Vec2::from), 16));
        lib.moving(
            candle,
            wax.at(foot + Vec3::Y * 0.06, at.q, kind::PLAIN, [0.75, 0.7, 0.58, 1.0]),
        );
        let drop = [
            (0.0, 0.0),
            (0.22, 0.06),
            (0.34, 0.2),
            (0.3, 0.42),
            (0.16, 0.72),
            (0.0, 1.0),
        ];
        let drop = lib.shape(Mesh::lathe(&drop.map(Vec2::from), 12));
        lib.moving(
            candle,
            drop.scaled(0.05).at(
                foot + Vec3::Y * 0.17,
                Quat::IDENTITY,
                kind::FIRE,
                [1.0, 0.55, 0.22, 4.0],
            ),
        );
        lib.light_riding(
            candle,
            at.at + at.q * (c + Vec3::Y * 0.4),
            4.0,
            [1.0, 0.6, 0.3],
            0.6,
            LightKind::Flicker,
        );
    }
}
