//! The Great Library: three storeys round a skylit atrium whose well is open from
//! the ground floor to the roof, galleries ringing the well upstairs, and a
//! department room off each side of the atrium on every storey, opened one by one.

use super::*;

const STOREYS: usize = 3;
const STOREY: f32 = 6.0;
/// Clear height of a storey (to the underside of the floor above).
const ROOM_H: f32 = STOREY - SLAB;
/// Half-size of the atrium, and of the open well inside it.
const ATRIUM: f32 = 14.0;
const WELL: f32 = 9.0;
/// Rooms reach from the atrium wall out to `OUTER`, `ROOM_HALF` either side.
const OUTER: f32 = 32.0;
const ROOM_HALF: f32 = 13.0;
const WALL_T: f32 = 0.4;
const DOOR_HALF: f32 = 1.5;
const DOOR_H: f32 = 3.6;
/// A flight of stairs runs along the inner edge of a gallery strip.
const STAIR_W: f32 = 2.4;
const STAIR_HALF_RUN: f32 = 5.5;
const STEPS: usize = 33;
/// Thickness of a stair's wooden tread.
const TREAD_T: f32 = 0.05;
const RAIL_H: f32 = 1.05;
const RAIL_T: f32 = 0.12;
/// The skylight over the well.
const SKY: f32 = 2.0 * STOREY + ROOM_H + 2.5;
/// Rows of cases in a room (across), and cases in a row (outward).
const ROWS_A: [f32; 4] = [-9.0, -3.0, 3.0, 9.0];
const CASES_PER_ROW: usize = 5;
/// The share of the departments already open that must be home to open the next.
const GATE_NEED: f32 = 0.5;
const BALUSTER: [f32; 3] = [0.2, 0.1, 0.045];
const PALE_STONE: [f32; 3] = [0.48, 0.45, 0.4];

/// A side of the atrium, as a frame: `a` runs across it, `o` outward from the centre.
#[derive(Clone, Copy)]
struct Side {
    out: Vec2,
    across: Vec2,
}

/// North, east, south, west. Rooms open in this order on each storey, ground floor first.
const SIDES: [Side; 4] = [
    Side {
        out: Vec2::new(0.0, -1.0),
        across: Vec2::new(1.0, 0.0),
    },
    Side {
        out: Vec2::new(1.0, 0.0),
        across: Vec2::new(0.0, 1.0),
    },
    Side {
        out: Vec2::new(0.0, 1.0),
        across: Vec2::new(-1.0, 0.0),
    },
    Side {
        out: Vec2::new(-1.0, 0.0),
        across: Vec2::new(0.0, -1.0),
    },
];

impl Side {
    fn at(&self, a: f32, o: f32) -> Vec2 {
        self.across * a + self.out * o
    }

    fn at3(&self, a: f32, o: f32, y: f32) -> Vec3 {
        let p = self.at(a, o);
        Vec3::new(p.x, y, p.y)
    }

    /// The world rectangle of the local one `a0..a1` × `o0..o1`.
    fn rect(&self, a0: f32, a1: f32, o0: f32, o1: f32) -> (Vec2, Vec2) {
        let (p, q) = (self.at(a0, o0), self.at(a1, o1));
        (p.min(q), p.max(q))
    }

    /// Cases in this side's rooms run outward: along x for east and west.
    fn along_x(&self) -> bool {
        self.out.x != 0.0
    }

    fn out3(&self) -> Vec3 {
        Vec3::new(self.out.x, 0.0, self.out.y)
    }

    fn across3(&self) -> Vec3 {
        Vec3::new(self.across.x, 0.0, self.across.y)
    }
}

/// Where along its wall (`a`) the door from the atrium into the room on `side` of
/// `level` is: in the middle, unless a flight of stairs stands in front of the wall,
/// when it moves past the foot of the stairs so it can be seen from the well.
fn door_a(level: usize, side: usize) -> f32 {
    if FLIGHTS.iter().any(|f| f.level == level && f.side == side) {
        -10.5
    } else {
        0.0
    }
}

fn floor_y(level: usize) -> f32 {
    level as f32 * STOREY
}

fn box3(min: Vec2, max: Vec2, y0: f32, y1: f32) -> (Vec3, Vec3) {
    (Vec3::new(min.x, y0, min.y), Vec3::new(max.x, y1, max.y))
}

/// `rects` with `hole` cut out of each (pieces left as up to four rectangles).
fn cut(rects: Vec<(Vec2, Vec2)>, (hmn, hmx): (Vec2, Vec2)) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    for (mn, mx) in rects {
        if hmn.x >= mx.x || hmx.x <= mn.x || hmn.y >= mx.y || hmx.y <= mn.y {
            out.push((mn, mx));
            continue;
        }
        // Full-width strips below and above the hole, then the sides beside it.
        if hmn.y > mn.y {
            out.push((mn, Vec2::new(mx.x, hmn.y)));
        }
        if hmx.y < mx.y {
            out.push((Vec2::new(mn.x, hmx.y), mx));
        }
        let (y0, y1) = (hmn.y.max(mn.y), hmx.y.min(mx.y));
        if hmn.x > mn.x {
            out.push((Vec2::new(mn.x, y0), Vec2::new(hmn.x, y1)));
        }
        if hmx.x < mx.x {
            out.push((Vec2::new(hmx.x, y0), Vec2::new(mx.x, y1)));
        }
    }
    out
}

/// A flight of stairs: in the gallery strip on `side` of the well, from `level` up
/// to the next, rising toward +a.
struct Flight {
    level: usize,
    side: usize,
}

const FLIGHTS: [Flight; 4] = [
    Flight { level: 0, side: 3 },
    Flight { level: 0, side: 1 },
    Flight { level: 1, side: 0 },
    Flight { level: 1, side: 2 },
];

impl Flight {
    /// The flight's footprint (the stairwell in the floor above).
    fn rect(&self) -> (Vec2, Vec2) {
        SIDES[self.side].rect(-STAIR_HALF_RUN, STAIR_HALF_RUN, WELL, WELL + STAIR_W)
    }
}

pub(super) fn build(lib: &mut Library, rng: &mut StdRng) {
    let ext = Vec2::splat(OUTER + 0.4);
    for l in 0..STOREYS {
        lib.levels.push(Level {
            y: floor_y(l),
            roof: floor_y(l) + ROOM_H,
            min: -ext,
            max: ext,
        });
    }
    lib.raised_roofs
        .push((STOREYS - 1, Vec2::splat(-WELL), Vec2::splat(WELL), SKY));
    lib.spawn = (Vec3::new(0.0, 0.0, 6.5), 0.0);
    // Newel posts already placed, so railings that meet share one.
    let mut newels = Vec::new();
    lib.build_floors();
    lib.build_stairs(&mut newels);
    lib.build_atrium(&mut newels);
    let mut titles = HashSet::new();
    for room in 0..STOREYS * 4 {
        lib.build_room(rng, room, &mut titles);
    }
    lib.scatter_books(rng);
}

impl Library {
    /// A stone wall on `level`, the full height of the storey (or `h`).
    fn wall(&mut self, level: usize, (mn, mx): (Vec2, Vec2), h: f32) {
        let y = floor_y(level);
        let (a, b) = box3(mn, mx, y, y + h);
        self.push(Inst::minmax(a, b, kind::STONE, STONE));
        self.obstacle(a, b, true);
    }

    /// An invisible barrier on `level`, `h` tall (railings are drawn on their own).
    fn barrier(&mut self, level: usize, (mn, mx): (Vec2, Vec2), h: f32) {
        let y = floor_y(level);
        let (a, b) = box3(mn, mx, y, y + h);
        self.obstacle(a, b, false);
    }

    /// A wooden box along the line `a`→`b`, `w` across and `h` tall, kept upright.
    fn beam(&mut self, a: Vec3, b: Vec3, w: f32, h: f32, rgb: [f32; 3]) {
        let d = b - a;
        let len = d.length();
        let x = d / len;
        let z = x.cross(Vec3::Y).normalize();
        let y = z.cross(x);
        let rot = Quat::from_mat3(&Mat3::from_cols(x, y, z));
        self.push(Inst::new(
            (a + b) * 0.5,
            rot,
            Vec3::new(len, h, w),
            kind::WOOD,
            [rgb[0], rgb[1], rgb[2], 1.0],
            0,
        ));
    }

    /// A wooden post `w` square standing `h` tall on `foot`.
    fn post(&mut self, foot: Vec3, h: f32, w: f32, rgb: [f32; 3]) {
        self.push(Inst::aabox(
            foot + Vec3::Y * (h * 0.5),
            Vec3::new(w, h, w),
            kind::WOOD,
            rgb,
        ));
    }

    /// A newel post with a cap at `foot`, unless one already stands there.
    fn newel(&mut self, foot: Vec3, h: f32, newels: &mut Vec<Vec3>) {
        if newels.iter().any(|n| n.distance(foot) < 0.3) {
            return;
        }
        newels.push(foot);
        self.post(foot, h, 0.15, WALNUT);
        self.push(Inst::aabox(
            foot + Vec3::Y * (h + 0.03),
            Vec3::new(0.2, 0.06, 0.2),
            kind::WOOD,
            [0.12, 0.06, 0.028],
        ));
    }

    /// A railing on the floor from `a` to `b`: a bottom rail, balusters, a handrail,
    /// and newel posts at its ends.
    fn railing(&mut self, a: Vec3, b: Vec3, newels: &mut Vec<Vec3>) {
        let up = Vec3::Y;
        self.beam(a + up * 0.05, b + up * 0.05, 0.1, 0.1, WALNUT);
        let n = (a.distance(b) / 0.28).round().max(1.0) as usize;
        for i in 1..n {
            let p = a.lerp(b, i as f32 / n as f32);
            self.post(p + up * 0.1, RAIL_H - 0.16, 0.045, BALUSTER);
        }
        self.beam(a + up * (RAIL_H - 0.03), b + up * (RAIL_H - 0.03), 0.12, 0.07, WALNUT);
        self.newel(a, RAIL_H + 0.06, newels);
        self.newel(b, RAIL_H + 0.06, newels);
    }

    /// Wooden panelling 1.1 m high on the face of a wall: a strip `r` in front of the
    /// face, facing `toward` the room. Its cap fills the strip; the panel is set back.
    fn wainscot(&mut self, level: usize, (mn, mx): (Vec2, Vec2), toward: Vec2) {
        let y = floor_y(level);
        let back = toward * 0.02;
        let (pmn, pmx) = (mn - back.max(Vec2::ZERO), mx - back.min(Vec2::ZERO));
        let (a, b) = box3(pmn, pmx, y, y + 1.1);
        self.push(Inst::minmax(a, b, kind::WOOD, [0.13, 0.065, 0.03]));
        let (a, b) = box3(mn, mx, y + 1.1, y + 1.16);
        self.push(Inst::minmax(a, b, kind::WOOD, WALNUT));
    }

    /// Floors: solid on the ground; upstairs, everywhere but the well and the
    /// stairwells. Each floor's underside is the ceiling below.
    fn build_floors(&mut self) {
        let ext = Vec2::splat(OUTER + 0.4);
        for l in 0..STOREYS {
            let mut rects = vec![(-ext, ext)];
            if l > 0 {
                rects = cut(rects, (Vec2::splat(-WELL), Vec2::splat(WELL)));
                for f in FLIGHTS.iter().filter(|f| f.level + 1 == l) {
                    rects = cut(rects, f.rect());
                }
            }
            let y = floor_y(l);
            for &(mn, mx) in &rects {
                self.slabs.push(Slab {
                    level: l,
                    min: mn,
                    max: mx,
                });
                self.push(Inst::minmax(
                    Vec3::new(mn.x, y - 0.06, mn.y),
                    Vec3::new(mx.x, y, mx.y),
                    kind::FLOOR,
                    [0.30, 0.16, 0.075],
                ));
                if l > 0 {
                    self.push(Inst::minmax(
                        Vec3::new(mn.x, y - SLAB, mn.y),
                        Vec3::new(mx.x, y - 0.06, mx.y),
                        kind::WOOD,
                        DARK_WOOD,
                    ));
                }
            }
        }
        // The roof over the top storey, and the lantern of the well rising to the skylight.
        let top = floor_y(STOREYS - 1) + ROOM_H;
        let lantern = WELL + 0.2;
        for (mn, mx) in cut(vec![(-ext, ext)], (Vec2::splat(-lantern), Vec2::splat(lantern))) {
            self.push(Inst::minmax(
                Vec3::new(mn.x, top, mn.y),
                Vec3::new(mx.x, top + SLAB, mx.y),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
        for side in SIDES {
            let (mn, mx) = side.rect(-lantern, lantern, WELL, lantern);
            self.push(Inst::minmax(
                Vec3::new(mn.x, top, mn.y),
                Vec3::new(mx.x, SKY, mx.y),
                kind::STONE,
                STONE,
            ));
        }
        let glass = Inst::new(
            Vec3::new(0.0, SKY + 0.04, 0.0),
            Quat::IDENTITY,
            Vec3::new(2.0 * WELL, 0.08, 2.0 * WELL),
            kind::GLASS,
            [0.32, 0.42, 0.85, 1.0],
            0,
        );
        self.push(glass);
        for k in -2..=2 {
            let x = k as f32 * 3.6;
            for (mn, mx) in [
                (Vec3::new(x - 0.12, SKY - 0.2, -WELL), Vec3::new(x + 0.12, SKY, WELL)),
                (Vec3::new(-WELL, SKY - 0.15, x - 0.1), Vec3::new(WELL, SKY, x + 0.1)),
            ] {
                self.push(Inst::minmax(mn, mx, kind::WOOD, DARK_WOOD));
            }
        }
        // Moonlight down the well.
        self.light(
            Vec3::new(0.0, SKY - 2.0, 0.0),
            26.0,
            [0.45, 0.55, 1.0],
            1.6,
            LightKind::Static,
        );
        let beam_dir = Vec3::new(0.25, -1.0, 0.15).normalize();
        for (x, z) in [(-4.0, -3.0), (3.0, -5.0), (-2.0, 4.0), (5.0, 3.0)] {
            let len = 16.0;
            let start = Vec3::new(x, SKY - 0.3, z);
            self.static_trans.push(Inst::new(
                start + beam_dir * len * 0.5,
                Quat::from_rotation_arc(Vec3::Y, beam_dir),
                Vec3::new(2.4, len, 2.4),
                kind::SHAFT,
                [0.35, 0.45, 0.9, 0.035],
                0,
            ));
        }
    }

    /// Four flights in the gallery strips: west and east from the ground floor to the
    /// first, north and south from the first to the second. Each is solid stone with
    /// wooden treads, stringers along its sides and a balustrade either side; the
    /// stairwell it rises through is railed on the floor above.
    fn build_stairs(&mut self, newels: &mut Vec<Vec3>) {
        let r = STAIR_HALF_RUN;
        for f in &FLIGHTS {
            let side = SIDES[f.side];
            let (y0, y1) = (floor_y(f.level), floor_y(f.level + 1));
            let (mn, mx) = f.rect();
            self.ramps.push(Ramp {
                level: f.level,
                min: mn,
                max: mx,
                axis: side.across,
                y0,
                y1,
                steps: STEPS,
            });
            let run = 2.0 * r / STEPS as f32;
            let rise = (y1 - y0) / STEPS as f32;
            // The walking slope and the points of a flight, along it.
            let slope = |a: f32| y0 + (y1 - y0) * (a + r) / (2.0 * r);
            let at = |a: f32, o: f32, y: f32| side.at3(a, o, y);
            for k in 0..STEPS {
                let a0 = -r + k as f32 * run;
                let top = y0 + (k + 1) as f32 * rise;
                let (sl, sh) = side.rect(a0, a0 + run, WELL, WELL + STAIR_W);
                let (lo, hi) = box3(sl, sh, y0, top - TREAD_T);
                self.push(Inst::minmax(lo, hi, kind::STONE, [0.4, 0.37, 0.33]));
                // The tread overhangs its riser a little (as books find it: `Ramp::tread`).
                let (tl, th) = side.rect(a0 - NOSING, a0 + run, WELL, WELL + STAIR_W);
                let (lo, hi) = box3(tl, th, top - TREAD_T, top);
                self.push(Inst::minmax(lo, hi, kind::WOOD, WALNUT));
            }
            for (o_rail, o_string) in [
                (WELL + 0.07, WELL - 0.035),
                (WELL + STAIR_W - 0.07, WELL + STAIR_W + 0.035),
            ] {
                // A stringer over the ends of the steps, stopping short of the floor above.
                let end = r - 0.46;
                self.beam(
                    at(-r, o_string, slope(-r) + 0.05),
                    at(end, o_string, slope(end) + 0.05),
                    0.07,
                    0.42,
                    WALNUT,
                );
                // Balusters on every other tread, a handrail along the slope.
                for k in (0..STEPS).step_by(2) {
                    let a = -r + (k as f32 + 0.5) * run;
                    let foot = y0 + (k + 1) as f32 * rise;
                    let h = slope(a) + 0.92 - foot;
                    self.post(at(a, o_rail, foot), h, 0.045, BALUSTER);
                }
                self.beam(
                    at(-r, o_rail, slope(-r) + 0.95),
                    at(r, o_rail, slope(r) + 0.95),
                    0.1,
                    0.07,
                    WALNUT,
                );
                self.newel(at(-r, o_rail, y0), RAIL_H + 0.06, newels);
                self.newel(at(r, o_rail, y1), RAIL_H + 0.06, newels);
            }
            // What stops a body or a book going over the sides: rails from the floor to
            // a railing's height above the flight, in five steps.
            let segs = 5;
            for k in 0..segs {
                let a0 = -r + k as f32 * 2.0 * r / segs as f32;
                let a1 = a0 + 2.0 * r / segs as f32;
                let top = slope(a1) - y0 + RAIL_H;
                self.barrier(f.level, side.rect(a0, a1, WELL, WELL + RAIL_T), top);
                self.barrier(f.level, side.rect(a0, a1, WELL + STAIR_W - RAIL_T, WELL + STAIR_W), top);
            }
            // The flight's high end is a stone face down to the floor below.
            let (a, b) = side.rect(r, r + 0.1, WELL, WELL + STAIR_W);
            let (lo, hi) = box3(a, b, y0, y1);
            self.obstacle(lo, hi, true);
            // The stairwell on the floor above: railed along its outer side and its low
            // end, its edges faced in wood.
            let up = f.level + 1;
            let edge = WELL + STAIR_W;
            self.barrier(up, side.rect(-r, r, edge, edge + RAIL_T), RAIL_H);
            self.barrier(up, side.rect(-r - RAIL_T, -r, WELL, edge), RAIL_H);
            self.railing(at(-r, edge + 0.06, y1), at(r, edge + 0.06, y1), newels);
            self.railing(at(-r - 0.06, WELL + 0.06, y1), at(-r - 0.06, edge + 0.06, y1), newels);
            for rect in [
                side.rect(-r, r, edge - 0.06, edge),
                side.rect(-r, -r + 0.06, WELL, edge - 0.06),
            ] {
                let (lo, hi) = box3(rect.0, rect.1, y1 - SLAB - 0.05, y1 + 0.03);
                self.push(Inst::minmax(lo, hi, kind::WOOD, WALNUT));
            }
        }
    }

    /// The atrium's walls and doors, the galleries' railings, and the reading floor in
    /// the well.
    fn build_atrium(&mut self, newels: &mut Vec<Vec3>) {
        for l in 0..STOREYS {
            let y = floor_y(l);
            for (si, side) in SIDES.iter().enumerate() {
                // The wall between the atrium and this side's room, with its door.
                let h = WALL_T * 0.5;
                let d = door_a(l, si);
                self.wall(l, side.rect(-ATRIUM - h, d - DOOR_HALF, ATRIUM - h, ATRIUM + h), ROOM_H);
                self.wall(l, side.rect(d + DOOR_HALF, ATRIUM + h, ATRIUM - h, ATRIUM + h), ROOM_H);
                let (a, b) = side.rect(d - DOOR_HALF, d + DOOR_HALF, ATRIUM - h, ATRIUM + h);
                let (lo, hi) = box3(a, b, y + DOOR_H, y + ROOM_H);
                self.push(Inst::minmax(lo, hi, kind::STONE, [0.42, 0.39, 0.35]));
                // A stone frame round the door, on both faces of the wall.
                for (face, out) in [(ATRIUM - h, -1.0f32), (ATRIUM + h, 1.0)] {
                    let o = |d: f32| (face.min(face + out * d), face.max(face + out * d));
                    for (a0, a1) in [
                        (d - DOOR_HALF - 0.3, d - DOOR_HALF),
                        (d + DOOR_HALF, d + DOOR_HALF + 0.3),
                    ] {
                        let (o0, o1) = o(0.07);
                        let (lo, hi) = box3(
                            side.rect(a0, a1, o0, o1).0,
                            side.rect(a0, a1, o0, o1).1,
                            y,
                            y + DOOR_H + 0.3,
                        );
                        self.push(Inst::minmax(lo, hi, kind::STONE, PALE_STONE));
                    }
                    let (o0, o1) = o(0.09);
                    let rect = side.rect(d - DOOR_HALF - 0.36, d + DOOR_HALF + 0.36, o0, o1);
                    let (lo, hi) = box3(rect.0, rect.1, y + DOOR_H, y + DOOR_H + 0.38);
                    self.push(Inst::minmax(lo, hi, kind::STONE, PALE_STONE));
                }
                // Panelling along the atrium face of the wall.
                for (a0, a1) in [
                    (-ATRIUM + h + 0.07, d - DOOR_HALF - 0.3),
                    (d + DOOR_HALF + 0.3, ATRIUM - h - 0.07),
                ] {
                    self.wainscot(l, side.rect(a0, a1, ATRIUM - h - 0.07, ATRIUM - h), -side.out);
                }
                // Sconces either side of the door, facing the atrium.
                for sa in [-7.5, 7.5] {
                    let p = side.at3(sa, ATRIUM - h - 0.15, y + 2.6);
                    self.sconce(p, -side.out3());
                }
                // The gallery railing round the well, except where stairs meet it, and
                // the gallery's edge faced in wood (except over a stairwell).
                if l > 0 {
                    let stairs = FLIGHTS
                        .iter()
                        .any(|f| f.side == si && (f.level == l || f.level + 1 == l));
                    let spans: &[(f32, f32)] = if stairs {
                        &[(-WELL, -STAIR_HALF_RUN), (STAIR_HALF_RUN, WELL)]
                    } else {
                        &[(-WELL, WELL)]
                    };
                    for &(a0, a1) in spans {
                        self.barrier(l, side.rect(a0, a1, WELL, WELL + RAIL_T), RAIL_H);
                        let o = WELL + RAIL_T * 0.5;
                        self.railing(side.at3(a0, o, y), side.at3(a1, o, y), newels);
                    }
                    let stairwell = FLIGHTS.iter().any(|f| f.side == si && f.level + 1 == l);
                    let edges: &[(f32, f32)] = if stairwell {
                        &[(-WELL, -STAIR_HALF_RUN), (STAIR_HALF_RUN, WELL)]
                    } else {
                        &[(-WELL, WELL)]
                    };
                    for &(a0, a1) in edges {
                        let rect = side.rect(a0, a1, WELL - 0.06, WELL);
                        let (lo, hi) = box3(rect.0, rect.1, y - SLAB - 0.05, y + 0.03);
                        self.push(Inst::minmax(lo, hi, kind::WOOD, WALNUT));
                    }
                }
            }
        }
        // Stone piers at the corners of the well, from the ground to the roof.
        let roof = floor_y(STOREYS - 1) + ROOM_H;
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let c = Vec3::new(sx * WELL, 0.0, sz * WELL);
            self.push(Inst::minmax(
                c - Vec3::new(0.35, 0.0, 0.35),
                c + Vec3::new(0.35, roof, 0.35),
                kind::STONE,
                [0.42, 0.39, 0.35],
            ));
            for l in 0..STOREYS {
                let y = floor_y(l);
                let base = c + Vec3::Y * y;
                self.push(Inst::aabox(
                    base + Vec3::Y * 0.15,
                    Vec3::new(0.9, 0.3, 0.9),
                    kind::STONE,
                    PALE_STONE,
                ));
                self.push(Inst::aabox(
                    base + Vec3::Y * (ROOM_H - 0.12),
                    Vec3::new(0.95, 0.24, 0.95),
                    kind::STONE,
                    PALE_STONE,
                ));
                self.obstacle(
                    base - Vec3::new(0.45, 0.0, 0.45),
                    base + Vec3::new(0.45, ROOM_H, 0.45),
                    true,
                );
            }
        }
        // The reading floor at the bottom of the well.
        let mut rug = Inst::aabox(
            Vec3::new(0.0, 0.012, 0.0),
            Vec3::new(15.0, 0.024, 15.0),
            kind::RUG,
            [0.06, 0.12, 0.28],
        );
        rug.param = 70 | (20 << 8) | (24 << 16);
        self.push(rug);
        for (x, z) in [(-5.6f32, -6.4f32), (5.6, -6.4), (-5.6, 6.4), (5.6, 6.4)] {
            self.reading_table(Vec3::new(x, 0.0, z));
        }
        self.chandelier(Vec3::new(0.0, STOREY + 3.0, 0.0), SKY);
    }

    /// Room `r`: storey `r / 4`, off side `r % 4`: four rows of five cases running
    /// out from the door, a category on each side of a row; department `r`.
    fn build_room(&mut self, rng: &mut StdRng, r: usize, titles: &mut HashSet<String>) {
        let (l, side) = (r / 4, SIDES[r % 4]);
        let y = floor_y(l);
        let h = WALL_T * 0.5;
        let d = door_a(l, r % 4);
        self.wall(l, side.rect(-ROOM_HALF - h, -ROOM_HALF + h, ATRIUM, OUTER + h), ROOM_H);
        self.wall(l, side.rect(ROOM_HALF - h, ROOM_HALF + h, ATRIUM, OUTER + h), ROOM_H);
        self.wall(l, side.rect(-ROOM_HALF, ROOM_HALF, OUTER - h, OUTER + h), ROOM_H);
        // Tall moonlit windows in the far wall.
        for a in [-7.0f32, 0.0, 7.0] {
            let c = side.at3(a, OUTER - h - 0.02, y + 3.0);
            let size = if side.along_x() {
                Vec3::new(0.08, 3.4, 1.6)
            } else {
                Vec3::new(1.6, 3.4, 0.08)
            };
            self.push(Inst::aabox(c, size, kind::GLASS, [0.32, 0.42, 0.85]));
            self.static_insts.last_mut().unwrap().color[3] = 1.2;
            self.light(c - side.out3() * 1.5, 8.0, [0.45, 0.55, 1.0], 0.8, LightKind::Static);
            let beam = (-side.out3() * 1.0 - Vec3::Y * 0.6).normalize();
            let len = 7.0;
            self.static_trans.push(Inst::new(
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
                let p = side.at3(sa * (ROOM_HALF - h - 0.15), o, y + 2.6);
                self.sconce(p, -side.across3() * sa);
            }
        }
        // Panelling round the walls, beams across the ceiling.
        let inner = ROOM_HALF - h;
        let t = 0.07;
        self.wainscot(l, side.rect(-inner, -inner + t, ATRIUM + h, OUTER - h - t), side.across);
        self.wainscot(l, side.rect(inner - t, inner, ATRIUM + h, OUTER - h - t), -side.across);
        self.wainscot(l, side.rect(-inner, inner, OUTER - h - t, OUTER - h), -side.out);
        for (a0, a1) in [(-inner + t, d - DOOR_HALF - 0.3), (d + DOOR_HALF + 0.3, inner - t)] {
            self.wainscot(l, side.rect(a0, a1, ATRIUM + h, ATRIUM + h + t), side.out);
        }
        for k in 0..6 {
            let o = ATRIUM + h + 1.6 + k as f32 * 3.0;
            let rect = side.rect(-inner, inner, o - 0.13, o + 0.13);
            let (lo, hi) = box3(rect.0, rect.1, y + ROOM_H - 0.3, y + ROOM_H);
            self.push(Inst::minmax(lo, hi, kind::WOOD, [0.11, 0.055, 0.025]));
        }
        // The department's sign over its door, on the atrium side.
        let dept = r;
        let sign = side.at3(d, ATRIUM - h - 0.03, y + DOOR_H + 0.75);
        let rot = Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(-side.out3()), Vec3::Y, -side.out3()));
        let first = self.catalog.departments[dept].categories.start;
        let dcol = catalog::category_color(&self.catalog.categories[first]);
        self.push(Inst::new(
            sign,
            rot,
            Vec3::new(3.2, 0.5, 0.05),
            kind::PLAQUE,
            [dcol[0], dcol[1], dcol[2], 0.12],
            0,
        ));
        self.labels.push(Label {
            pos: sign - side.out3() * 0.04,
            normal: -side.out3(),
            text: self.catalog.departments[dept].name.clone(),
            kind: LabelKind::Department(dept),
            level: l,
        });
        if dept > 0 {
            let (a, b) = side.rect(d - DOOR_HALF, d + DOOR_HALF, ATRIUM - 0.05, ATRIUM + 0.05);
            let (lo, hi) = box3(a, b, y, y + DOOR_H);
            self.gates.push(Gate {
                level: l,
                min: lo,
                max: hi,
                opens: dept,
                counts: (0..dept).collect(),
                need: GATE_NEED,
            });
        }
        // The cases, then each side of each row as a category.
        let case_o = |k: usize| ATRIUM + h + 2.5 + UNIT_W * 0.5 + k as f32 * UNIT_PITCH;
        for &ra in &ROWS_A {
            for k in 0..CASES_PER_ROW {
                self.build_unit(side.at3(ra, case_o(k), y), side.along_x());
            }
        }
        let mut cat = self.catalog.departments[dept].categories.start;
        for &ra in &ROWS_A {
            for s in [-1.0f32, 1.0] {
                let normal = side.across3() * s;
                let right = Vec3::Y.cross(normal);
                let mut cases: Vec<Vec3> = (0..CASES_PER_ROW)
                    .map(|k| side.at3(ra + s * UNIT_DEPTH * 0.5, case_o(k), y))
                    .collect();
                cases.sort_by(|a, b| a.dot(right).total_cmp(&b.dot(right)));
                self.add_category(rng, cat, &cases, normal, titles);
                cat += 1;
            }
        }
    }

    /// Where the books start. A department's books lie in its own room, the atrium,
    /// and rooms that open before it, so every book is within reach once its
    /// department opens; early rooms are mixed, later ones purer.
    fn scatter_books(&mut self, rng: &mut StdRng) {
        let rooms = STOREYS * 4;
        let mut room_pool: Vec<Vec<u32>> = vec![Vec::new(); rooms];
        let mut atrium_pool: Vec<u32> = Vec::new();
        for d in 0..rooms {
            let mut books: Vec<u32> = (0..self.books.len() as u32)
                .filter(|&b| self.dept_of(self.category_of_book(b)) == d)
                .collect();
            books.shuffle(rng);
            let own = (books.len() as f32 * 0.65) as usize;
            let atrium = (books.len() as f32 * 0.2) as usize;
            room_pool[d].extend(books.drain(..own));
            atrium_pool.extend(books.drain(..atrium));
            for (i, b) in books.into_iter().enumerate() {
                if d == 0 {
                    atrium_pool.push(b);
                } else {
                    room_pool[i % d].push(b);
                }
            }
        }
        for (r, mut pool) in room_pool.into_iter().enumerate() {
            pool.shuffle(rng);
            let (l, side) = (r / 4, SIDES[r % 4]);
            let area = side.rect(-ROOM_HALF + 0.2, ROOM_HALF - 0.2, ATRIUM + 0.2, OUTER - 0.2);
            let spots = self.pile_spots(rng, l, &[area], 0.8, 0.7);
            self.spread(rng, l, &spots, pool);
        }
        // The atrium: heaps on the reading floor (one great one in the middle), piles
        // under the galleries and along them.
        atrium_pool.shuffle(rng);
        let n = atrium_pool.len();
        let great: Vec<u32> = atrium_pool.drain(..n * 3 / 20).collect();
        self.add_pile(rng, 0, Vec2::ZERO, 0.4 + 0.045 * (great.len() as f32).sqrt(), great);
        // Between the well and the atrium walls (on the ground floor, under the
        // galleries; upstairs, the galleries themselves).
        let a = Vec2::splat(ATRIUM - 0.2);
        let ring = cut(vec![(-a, a)], (Vec2::splat(-WELL - 0.3), Vec2::splat(WELL + 0.3)));
        let well = vec![(Vec2::splat(-WELL + 0.6), Vec2::splat(WELL - 0.6))];
        let shares = [
            (0, well, 0.3),
            (0, ring.clone(), 0.25),
            (1, ring.clone(), 0.25),
            (2, ring, 0.2),
        ];
        let total = atrium_pool.len();
        for (i, (l, rects, share)) in shares.into_iter().enumerate() {
            let take = if i == 3 {
                atrium_pool.len()
            } else {
                ((total as f32 * share) as usize).min(atrium_pool.len())
            };
            let books: Vec<u32> = atrium_pool.drain(..take).collect();
            let spots = self.pile_spots(rng, l, &rects, 0.8, 0.6);
            self.spread(rng, l, &spots, books);
        }
    }

    /// Pile centres on `level` within `rects`: clear of furniture (by `r + gap`), of
    /// stairs, doorways, the great heap and the spawn point, and of each other.
    fn pile_spots(&self, rng: &mut StdRng, level: usize, rects: &[(Vec2, Vec2)], r: f32, gap: f32) -> Vec<Vec2> {
        let area: f32 = rects.iter().map(|(a, b)| (b.x - a.x) * (b.y - a.y)).sum();
        let doors: Vec<Vec2> = (SIDES.iter().enumerate())
            .map(|(si, s)| s.at(door_a(level, si), ATRIUM))
            .collect();
        let mut spots: Vec<Vec2> = Vec::new();
        for _ in 0..(area * 30.0) as usize {
            let mut pick = rng.gen_range(0.0..area);
            let &(mn, mx) = rects
                .iter()
                .find(|(a, b)| {
                    let s = (b.x - a.x) * (b.y - a.y);
                    pick -= s;
                    pick < 0.0
                })
                .unwrap_or(&rects[0]);
            if mx.x - mn.x < 2.0 * r || mx.y - mn.y < 2.0 * r {
                continue;
            }
            let p = Vec2::new(rng.gen_range(mn.x + r..mx.x - r), rng.gen_range(mn.y + r..mx.y - r));
            let clear_of = |a: Vec2, b: Vec2, d: f32| p.clamp(a, b).distance(p) >= d;
            let ok = self
                .obstacles
                .iter()
                .filter(|o| o.level == level)
                .all(|o| clear_of(Vec2::new(o.min.x, o.min.z), Vec2::new(o.max.x, o.max.z), r + gap))
                && self.ramps.iter().all(|s| clear_of(s.min, s.max, r + 1.2))
                && doors.iter().all(|&d| d.distance(p) >= r + 2.5)
                && (level > 0
                    || (p.length() >= r + 4.0 && p.distance(Vec2::new(self.spawn.0.x, self.spawn.0.z)) >= r + 2.5))
                && spots.iter().all(|&q| q.distance(p) >= 2.0 * r + gap);
            if ok {
                spots.push(p);
            }
        }
        spots
    }

    /// Share `books` out between pile `spots` on `level`, roughly evenly.
    fn spread(&mut self, rng: &mut StdRng, level: usize, spots: &[Vec2], mut books: Vec<u32>) {
        assert!(
            !spots.is_empty() || books.is_empty(),
            "nowhere to put {} books",
            books.len()
        );
        let count = spots.len();
        for (i, &p) in spots.iter().enumerate() {
            let left = count - i;
            let n = if left == 1 {
                books.len()
            } else {
                ((books.len() as f32 / left as f32) * rng.gen_range(0.75..1.25)) as usize
            };
            let n = n.min(books.len());
            if n == 0 {
                continue;
            }
            let radius = 0.4 + 0.06 * (n as f32).sqrt();
            let pile: Vec<u32> = books.drain(..n).collect();
            self.add_pile(rng, level, p, radius, pile);
        }
    }
}
