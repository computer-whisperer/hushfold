//! The Great Library: three storeys round a skylit atrium whose well is open from
//! the ground floor to the roof, galleries ringing the well upstairs, and a
//! department room off each side of the atrium on every storey, opened one by one.

use super::kit::RAIL_T;
use super::rooms;
use super::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

const STOREYS: usize = 3;
const STOREY: f32 = 6.0;
/// Clear height of a storey (to the underside of the floor above).
pub(super) const ROOM_H: f32 = STOREY - SLAB;
/// Half-size of the atrium, and of the open well inside it.
pub(super) const ATRIUM: f32 = 14.0;
const WELL: f32 = 9.0;
/// Rooms reach from the atrium wall out to `OUTER`, `ROOM_HALF` either side.
pub(super) const OUTER: f32 = 32.0;
pub(super) const ROOM_HALF: f32 = 13.0;
pub(super) const WALL_T: f32 = 0.4;
pub(super) const DOOR_HALF: f32 = 1.5;
pub(super) const DOOR_H: f32 = 3.6;
/// A flight of stairs runs along the inner edge of a gallery strip.
const STAIR_W: f32 = 2.4;
const STAIR_HALF_RUN: f32 = 5.5;
const STEPS: usize = 33;
/// The skylight over the well.
const SKY: f32 = 2.0 * STOREY + ROOM_H + 2.5;
/// The share of the departments already open that must be home to open the next;
/// the first veil lifts sooner, to show more of the library early.
const GATE_NEED: f32 = 0.5;
const FIRST_GATE_NEED: f32 = 0.25;

/// A stretch from one value to another along an axis.
type Span = (f32, f32);

/// The atrium face of the walls round it.
const FACE: f32 = ATRIUM - WALL_T * 0.5;
/// Where pilasters may stand along an atrium wall; one near the door gives way to its
/// frame.
const PILASTERS: [f32; 5] = [-9.0, -4.5, 0.0, 4.5, 9.0];
/// How far either side of a door from the atrium its stone frame reaches.
const PORTAL_HALF: f32 = 2.65;
/// Heights in a storey: where the wall's arches spring, and the top of the capitals.
const SPRING: f32 = 2.5;
const CAPITAL: f32 = 4.95;
/// The atrium's dressed stone, and a paler one for keystones and caps.
const DRESSED: [f32; 3] = [0.46, 0.43, 0.38];
const PALE: [f32; 3] = [0.56, 0.53, 0.47];
/// Gallery fronts: the dark of their friezes and the lighter wood of their panels.
const FRIEZE: [f32; 3] = [0.11, 0.055, 0.025];
const PANEL: [f32; 3] = [0.2, 0.1, 0.045];
const GILT: [f32; 3] = [0.62, 0.45, 0.16];
/// A department's name, in capitals on the sign over its door.
const DEPT_LETTERS: Style = Style {
    em: 0.22,
    tracking: 0.12,
    lettering: Lettering::Gilt,
    color: GOLD_LEAF,
};

/// A side of the atrium, as a frame: `a` runs across it, `o` outward from the centre.
#[derive(Clone, Copy)]
pub(super) struct Side {
    pub(super) out: Vec2,
    pub(super) across: Vec2,
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
    pub(super) fn at(&self, a: f32, o: f32) -> Vec2 {
        self.across * a + self.out * o
    }

    pub(super) fn at3(&self, a: f32, o: f32, y: f32) -> Vec3 {
        let p = self.at(a, o);
        Vec3::new(p.x, y, p.y)
    }

    /// The world rectangle of the local one `a0..a1` × `o0..o1`.
    pub(super) fn rect(&self, a0: f32, a1: f32, o0: f32, o1: f32) -> (Vec2, Vec2) {
        let (p, q) = (self.at(a0, o0), self.at(a1, o1));
        (p.min(q), p.max(q))
    }

    /// Cases in this side's rooms run outward: along x for east and west.
    pub(super) fn along_x(&self) -> bool {
        self.out.x != 0.0
    }

    pub(super) fn out3(&self) -> Vec3 {
        Vec3::new(self.out.x, 0.0, self.out.y)
    }

    pub(super) fn across3(&self) -> Vec3 {
        Vec3::new(self.across.x, 0.0, self.across.y)
    }
}

/// Where along its wall (`a`) the door from the atrium into the room on `side` of
/// `level` is: in the middle, unless a flight of stairs stands in front of the wall,
/// when it moves past the foot of the stairs so it can be seen from the well.
fn door_a(level: usize, side: usize) -> f32 {
    if STAIRWAYS.iter().any(|f| f.level == level && f.side == side) {
        -10.5
    } else {
        0.0
    }
}

fn floor_y(level: usize) -> f32 {
    level as f32 * STOREY
}

/// The two lengths (across) of the atrium wall whose door is at `d`, either side of it.
fn wall_pieces(d: f32) -> [Span; 2] {
    let h = WALL_T * 0.5;
    [(-ATRIUM - h, d - DOOR_HALF), (d + DOOR_HALF, ATRIUM + h)]
}

/// The pilasters along the wall whose door is at `d`.
fn pilasters(d: f32) -> impl Iterator<Item = f32> {
    PILASTERS.into_iter().filter(move |p| (p - d).abs() > PORTAL_HALF + 0.3)
}

pub(super) fn box3(min: Vec2, max: Vec2, y0: f32, y1: f32) -> (Vec3, Vec3) {
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

/// Where a flight of stairs stands: in the gallery strip on `side` of the well, from
/// `level` up to the next, rising toward +a.
struct Stairway {
    level: usize,
    side: usize,
}

const STAIRWAYS: [Stairway; 4] = [
    Stairway { level: 0, side: 3 },
    Stairway { level: 0, side: 1 },
    Stairway { level: 1, side: 0 },
    Stairway { level: 1, side: 2 },
];

impl Stairway {
    /// The flight's footprint (the stairwell in the floor above).
    fn rect(&self) -> (Vec2, Vec2) {
        SIDES[self.side].rect(-STAIR_HALF_RUN, STAIR_HALF_RUN, WELL, WELL + STAIR_W)
    }
}

pub(super) fn build(lib: &mut Library, rng: &mut StdRng) {
    let ext = Vec2::splat(OUTER + 0.4);
    for l in 0..STOREYS {
        lib.world.levels.push(Level {
            y: floor_y(l),
            roof: floor_y(l) + ROOM_H,
            min: -ext,
            max: ext,
        });
    }
    (lib.world.raised_roofs).push((STOREYS - 1, Vec2::splat(-WELL), Vec2::splat(WELL), SKY));
    lib.spawn = (Vec3::new(0.0, 0.0, 6.5), 0.0);
    // The great heap on the atrium floor, under the well.
    lib.hub = Vec3::ZERO;
    // Newel posts already placed, so railings that meet share one.
    let mut newels = Vec::new();
    // Static boxes are drawn in the order they are laid down, so what stands in front
    // goes first and hides what is behind it: the rooms' faces of the atrium's walls
    // (hiding the whole atrium from inside the rooms), then the stairs and the
    // atrium's dressing before the atrium's faces of its walls, the floors and the
    // ceilings.
    lib.atrium_wall_backs();
    lib.build_stairs(&mut newels);
    lib.build_atrium(&mut newels);
    lib.build_floors();
    let mut titles = HashSet::new();
    for room in 0..STOREYS * 4 {
        // Each room's books from its own generator, so changing one room's layout
        // leaves the others' books as they were.
        let mut room_rng = StdRng::seed_from_u64(rng.next_u64());
        lib.build_room(&mut room_rng, room, &mut titles);
    }
    lib.scatter_books(rng);
}

impl Library {
    /// A stone wall on `level`, the full height of the storey (or `h`).
    fn wall(&mut self, level: usize, (mn, mx): (Vec2, Vec2), h: f32) {
        let y = floor_y(level);
        let (a, b) = box3(mn, mx, y, y + h);
        self.solid(Inst::minmax(a, b, kind::STONE, STONE), Stops::ALL);
    }

    /// Floors: solid on the ground; upstairs, everywhere but the well and the
    /// stairwells. Each floor's underside is the ceiling below.
    fn build_floors(&mut self) {
        let ext = Vec2::splat(OUTER + 0.4);
        for l in 0..STOREYS {
            let mut rects = vec![(-ext, ext)];
            if l > 0 {
                rects = cut(rects, (Vec2::splat(-WELL), Vec2::splat(WELL)));
                for f in STAIRWAYS.iter().filter(|f| f.level + 1 == l) {
                    rects = cut(rects, f.rect());
                }
            }
            let y = floor_y(l);
            for &(mn, mx) in &rects {
                self.world.slabs.push(Slab {
                    level: l,
                    min: mn,
                    max: mx,
                });
                self.decor(Inst::minmax(
                    Vec3::new(mn.x, y - 0.06, mn.y),
                    Vec3::new(mx.x, y, mx.y),
                    kind::FLOOR,
                    [0.30, 0.16, 0.075],
                ));
                if l > 0 {
                    self.decor(Inst::minmax(
                        Vec3::new(mn.x, y - SLAB, mn.y),
                        Vec3::new(mx.x, y - 0.06, mx.y),
                        kind::WOOD,
                        DARK_WOOD,
                    ));
                }
            }
        }
        // The roof over the top storey, and the lantern of the well rising to the
        // skylight (what stands under the skylight and on the lantern's walls first).
        self.skylight();
        let top = floor_y(STOREYS - 1) + ROOM_H;
        let lantern = WELL + 0.2;
        for (mn, mx) in cut(vec![(-ext, ext)], (Vec2::splat(-lantern), Vec2::splat(lantern))) {
            self.decor(Inst::minmax(
                Vec3::new(mn.x, top, mn.y),
                Vec3::new(mx.x, top + SLAB, mx.y),
                kind::WOOD,
                DARK_WOOD,
            ));
        }
        for side in SIDES {
            let (mn, mx) = side.rect(-lantern, lantern, WELL, lantern);
            self.decor(Inst::minmax(
                Vec3::new(mn.x, top, mn.y),
                Vec3::new(mx.x, SKY, mx.y),
                kind::STONE,
                STONE,
            ));
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
    /// first, north and south from the first to the second; the stairwell each rises
    /// through is railed on the floor above, its edges faced in wood.
    fn build_stairs(&mut self, newels: &mut Vec<Vec3>) {
        let r = STAIR_HALF_RUN;
        for st in &STAIRWAYS {
            let side = SIDES[st.side];
            let (y0, y1) = (floor_y(st.level), floor_y(st.level + 1));
            let (min, max) = st.rect();
            let flight = Stairs {
                level: st.level,
                min,
                max,
                axis: side.across,
                y0,
                y1,
                steps: STEPS,
            };
            self.stair_panelling(side, y0, y1);
            self.stairs(flight, newels);
            let edge = WELL + STAIR_W;
            // Railings on the slab, their guards' faces flush with the stairwell's edges.
            let at = |a: f32, o: f32| side.at3(a, o, y1);
            let (lip, out) = (WELL + RAIL_T * 0.5, edge + RAIL_T * 0.5);
            self.railing(at(-r, out), at(r, out), newels);
            self.railing(at(-r - RAIL_T * 0.5, lip), at(-r - RAIL_T * 0.5, out), newels);
            for rect in [
                side.rect(-r, r, edge - 0.06, edge),
                side.rect(-r, -r + 0.06, WELL, edge - 0.06),
            ] {
                let (lo, hi) = box3(rect.0, rect.1, y1 - SLAB - 0.05, y1 + 0.03);
                self.decor(Inst::minmax(lo, hi, kind::WOOD, WALNUT));
            }
        }
    }

    /// The rooms' halves of the atrium's walls (through their thickness).
    fn atrium_wall_backs(&mut self) {
        for l in 0..STOREYS {
            let y = floor_y(l);
            for (si, side) in SIDES.iter().enumerate() {
                for (a0, a1) in wall_pieces(door_a(l, si)) {
                    let (mn, mx) = side.rect(a0, a1, ATRIUM, ATRIUM + WALL_T * 0.5);
                    let (lo, hi) = box3(mn, mx, y, y + ROOM_H);
                    self.decor(Inst::minmax(lo, hi, kind::STONE, STONE));
                }
            }
        }
    }

    /// The atrium's walls with their doors (each in a stone frame with its
    /// department's sign), the galleries' fronts, railings and lamps, the beams under
    /// its ceilings, the well's piers and its floating lanterns, and the reading floor
    /// at the bottom of the well.
    fn build_atrium(&mut self, newels: &mut Vec<Vec3>) {
        for l in 0..STOREYS {
            let y = floor_y(l);
            for (si, side) in SIDES.iter().copied().enumerate() {
                // The wall between the atrium and this side's room, with its door:
                // what stands on it, then its atrium half (`atrium_wall_backs` drew
                // the other), and what is in the way, the whole wall.
                let h = WALL_T * 0.5;
                let d = door_a(l, si);
                self.portal(l, side, d, l * 4 + si);
                self.doorway(side.at3(d, ATRIUM, y), side.across, DOOR_HALF, DOOR_H, WALL_T, ROOM_H);
                self.dress_wall(l, si);
                self.ceiling_beams(l, si);
                for (a0, a1) in wall_pieces(d) {
                    let (mn, mx) = side.rect(a0, a1, ATRIUM - h, ATRIUM);
                    let (lo, hi) = box3(mn, mx, y, y + ROOM_H);
                    self.decor(Inst::minmax(lo, hi, kind::STONE, STONE));
                    let (mn, mx) = side.rect(a0, a1, ATRIUM - h, ATRIUM + h);
                    let (lo, hi) = box3(mn, mx, y, y + ROOM_H);
                    self.collide(lo, hi, Stops::ALL);
                }
                // The gallery railing round the well, except where stairs meet it, and
                // the gallery's front (except over a stairwell).
                if l > 0 {
                    let stairs = STAIRWAYS
                        .iter()
                        .any(|f| f.side == si && (f.level == l || f.level + 1 == l));
                    let spans: &[Span] = if stairs {
                        &[(-WELL, -STAIR_HALF_RUN), (STAIR_HALF_RUN, WELL)]
                    } else {
                        &[(-WELL, WELL)]
                    };
                    for &(a0, a1) in spans {
                        let o = WELL + RAIL_T * 0.5;
                        self.railing(side.at3(a0, o, y), side.at3(a1, o, y), newels);
                    }
                    // Lamps on the railing: a pair along it, or one in the middle of
                    // each length either side of the stairs.
                    let lamps = if stairs { [-7.25, 7.25] } else { [-4.5, 4.5] };
                    for a in lamps {
                        self.lamp_standard(side.at3(a, WELL + RAIL_T * 0.5, y));
                    }
                    let stairwell = STAIRWAYS.iter().any(|f| f.side == si && f.level + 1 == l);
                    let edges: &[Span] = if stairwell {
                        &[(-WELL, -STAIR_HALF_RUN), (STAIR_HALF_RUN, WELL)]
                    } else {
                        &[(-WELL, WELL)]
                    };
                    self.gallery_front(side, y, edges);
                }
            }
            for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                self.corner_pier(sx, sz, y);
            }
        }
        // The well's crown under the lantern, and stone piers at its corners from the
        // ground to the roof.
        let roof = floor_y(STOREYS - 1) + ROOM_H;
        for side in SIDES {
            self.gallery_front(side, roof + SLAB, &[(-WELL, WELL)]);
        }
        let floors: Vec<f32> = (0..STOREYS).map(floor_y).collect();
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.pier(Vec3::new(sx * WELL, 0.0, sz * WELL), roof, &floors, ROOM_H);
        }
        self.floating_lanterns();
        // The reading floor at the bottom of the well.
        let mut rug = Inst::aabox(
            Vec3::new(0.0, 0.012, 0.0),
            Vec3::new(15.0, 0.024, 15.0),
            kind::RUG,
            [0.06, 0.12, 0.28],
        );
        rug.param = 70 | (20 << 8) | (24 << 16);
        self.rug(rug);
        for (x, z) in [(-5.6f32, -6.4f32), (5.6, -6.4), (-5.6, 6.4), (5.6, 6.4)] {
            self.reading_table(Vec3::new(x, 0.0, z));
        }
        self.chandelier(Vec3::new(0.0, STOREY + 3.0, 0.0), SKY);
    }

    /// Department `r`'s room, on storey `r / 4` off side `r % 4`: its walls (the one it
    /// shares with the atrium, with the door's frame and sign, is `build_atrium`'s),
    /// the gate in its door, and then what stands inside, which is the department's
    /// own (`rooms::furnish`).
    fn build_room(&mut self, rng: &mut StdRng, r: usize, titles: &mut HashSet<String>) {
        let (l, side) = (r / 4, SIDES[r % 4]);
        let y = floor_y(l);
        let h = WALL_T * 0.5;
        let d = door_a(l, r % 4);
        self.wall(l, side.rect(-ROOM_HALF - h, -ROOM_HALF + h, ATRIUM, OUTER + h), ROOM_H);
        self.wall(l, side.rect(ROOM_HALF - h, ROOM_HALF + h, ATRIUM, OUTER + h), ROOM_H);
        self.wall(l, side.rect(-ROOM_HALF, ROOM_HALF, OUTER - h, OUTER + h), ROOM_H);
        let dept = r;
        let (mn, mx) = side.rect(-ROOM_HALF, ROOM_HALF, ATRIUM, OUTER);
        self.rooms.push(Room {
            level: l,
            min: mn,
            max: mx,
            door: side.at3(d, ATRIUM + 0.5, y),
            drop: side.at3(d + 2.6, ATRIUM + 1.5, y),
        });
        if dept > 0 {
            let (a, b) = side.rect(d - DOOR_HALF, d + DOOR_HALF, ATRIUM - 0.05, ATRIUM + 0.05);
            let (lo, hi) = box3(a, b, y, y + DOOR_H);
            self.world.add_gate(Gate {
                level: l,
                min: lo,
                max: hi,
                opens: dept,
                counts: (0..dept).collect(),
                need: if dept == 1 { FIRST_GATE_NEED } else { GATE_NEED },
            });
        }
        let frame = rooms::RoomFrame {
            dept,
            y,
            side,
            door: d,
            categories: self.catalog.departments[dept].categories.clone(),
        };
        rooms::furnish(self, &frame, rng, titles);
    }

    /// Where the books start. A department's books lie in its own room, the atrium,
    /// and rooms that open before it, so every book is within reach once its
    /// department opens; early rooms are mixed, later ones purer. They go as units
    /// (`series_units`): a series lying whole goes to its own room (else the atrium);
    /// other units 65% to its room, 20% to the atrium, 15% to rooms that open earlier
    /// (the atrium for the first room).
    fn scatter_books(&mut self, rng: &mut StdRng) {
        let rooms = STOREYS * 4;
        let mut room_pool: Vec<Vec<Vec<u32>>> = vec![Vec::new(); rooms];
        let mut atrium_pool: Vec<Vec<u32>> = Vec::new();
        for d in 0..rooms {
            let books: Vec<u32> = (0..self.books.len() as u32)
                .filter(|&b| self.dept_of(self.category_of_book(b)) == d)
                .collect();
            let mut earlier = 0;
            for unit in self.series_units(rng, &books) {
                let (own, atrium) = if self.whole_set(&unit) {
                    (0.85, 0.15)
                } else {
                    (0.65, 0.2)
                };
                let x = rng.gen_range(0.0..1.0f32);
                if x < own {
                    room_pool[d].push(unit);
                } else if x < own + atrium || d == 0 {
                    atrium_pool.push(unit);
                } else {
                    room_pool[earlier % d].push(unit);
                    earlier += 1;
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
        let total: usize = atrium_pool.iter().map(Vec::len).sum();
        let take = |pool: &mut Vec<Vec<u32>>, books: usize| -> Vec<Vec<u32>> {
            let mut out = Vec::new();
            let mut n = 0;
            while n < books
                && let Some(u) = pool.pop()
            {
                n += u.len();
                out.push(u);
            }
            out
        };
        let great: Vec<u32> = take(&mut atrium_pool, total * 3 / 20).into_iter().flatten().collect();
        self.add_pile(rng, 0, Vec2::ZERO, 0.4 + 0.045 * (great.len() as f32).sqrt(), great);
        // Between the well and the atrium walls (on the ground floor, under the
        // galleries; upstairs, the galleries themselves).
        let a = Vec2::splat(ATRIUM - 0.2);
        let ring = cut(vec![(-a, a)], (Vec2::splat(-WELL - 0.3), Vec2::splat(WELL + 0.3)));
        let well = vec![(Vec2::splat(-WELL + 0.6), Vec2::splat(WELL - 0.6))];
        let rest: usize = atrium_pool.iter().map(Vec::len).sum();
        let shares = [
            (0, well, 0.3),
            (0, ring.clone(), 0.25),
            (1, ring.clone(), 0.25),
            (2, ring, 0.2),
        ];
        for (i, (l, rects, share)) in shares.into_iter().enumerate() {
            let units = if i == 3 {
                std::mem::take(&mut atrium_pool)
            } else {
                take(&mut atrium_pool, (rest as f32 * share) as usize)
            };
            let spots = self.pile_spots(rng, l, &rects, 0.8, 0.6);
            self.spread(rng, l, &spots, units);
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
            let ok = (self.world.solids_on(level))
                .filter(|o| o.stops.has(Stops::BODIES))
                .all(|o| clear_of(Vec2::new(o.min.x, o.min.z), Vec2::new(o.max.x, o.max.z), r + gap))
                && self.world.stairs().iter().all(|s| clear_of(s.min, s.max, r + 1.2))
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
}

/// A box in `side`'s frame: `a` across, `o` out from the atrium's centre, `y` up.
fn piece(side: Side, a: Span, o: Span, y: Span, kind: u32, rgb: [f32; 3]) -> Inst {
    let (mn, mx) = side.rect(a.0, a.1, o.0, o.1);
    let (lo, hi) = box3(mn, mx, y.0, y.1);
    Inst::minmax(lo, hi, kind, rgb)
}

/// A box in the plane of a wall on `side`: centred `at` (across, up), `o` its span off
/// the wall, `size` (along, up), turned `angle` from level.
fn in_wall(side: Side, at: (f32, f32), o: Span, size: (f32, f32), angle: f32, look: (u32, [f32; 4])) -> Inst {
    let across = side.across3();
    let x = across * libm::cosf(angle) + Vec3::Y * libm::sinf(angle);
    let up = Vec3::Y * libm::cosf(angle) - across * libm::sinf(angle);
    let rot = Quat::from_mat3(&Mat3::from_cols(x, up, x.cross(up)));
    let c = side.at3(at.0, (o.0 + o.1) * 0.5, at.1);
    Inst::new(c, rot, Vec3::new(size.0, size.1, (o.1 - o.0).abs()), look.0, look.1, 0)
}

fn rgba(rgb: [f32; 3], w: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], w]
}

/// The turn laying a piece made in xy (x across, y up) flat on the wall on `side`, its
/// z out from the wall's face toward the middle of the atrium.
fn wall_rot(side: Side) -> Quat {
    let across = side.across3();
    Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, across.cross(Vec3::Y)))
}

/// Turned to stand a lathe's axis (y) out of the wall on `side`.
fn out_of_wall(side: Side) -> Quat {
    wall_rot(side) * Quat::from_rotation_x(FRAC_PI_2)
}

/// `n` + 1 points along an arc `r` round the origin in xy, from angle `a0` to `a1`.
fn arc(r: f32, (a0, a1): Span, n: usize) -> impl Iterator<Item = Vec2> {
    (0..=n).map(move |k| {
        let a = a0 + (a1 - a0) * k as f32 / n as f32;
        Vec2::new(libm::cosf(a), libm::sinf(a)) * r
    })
}

/// A round of `n` points `r` from the origin in xy.
fn round(r: f32, n: usize) -> Vec<Vec2> {
    arc(r, (0.0, TAU), n).take(n).collect()
}

/// A lathe's profile, (radius, height) up its outside.
fn profile(points: &[(f32, f32)]) -> Vec<Vec2> {
    points.iter().map(|&p| Vec2::from(p)).collect()
}

/// A small flame, a teardrop on its foot (the origin), 1 tall.
fn flame() -> Mesh {
    let p = [
        (0.0, 0.0),
        (0.3, 0.06),
        (0.5, 0.22),
        (0.44, 0.42),
        (0.24, 0.7),
        (0.0, 1.0),
    ];
    Mesh::lathe(&profile(&p), 12)
}

/// The atrium's dressing: what stands on its walls, under its ceilings, along its
/// galleries, round its doors, beside its stairs and over the well.
impl Library {
    fn part(&mut self, side: Side, a: Span, o: Span, y: Span, kind: u32, rgb: [f32; 3]) {
        self.decor(piece(side, a, o, y, kind, rgb));
    }

    /// The foot of something standing on the floor against an atrium wall, out from
    /// it by less than a body's radius: drawn, and in the way of books. Bodies, held
    /// that far off the wall, never reach it, so it changes nothing about where piles
    /// go (they keep clear of what is in bodies' way).
    fn against_wall(&mut self, i: Inst) {
        let (min, max) = i.axis_box().expect("an upright box");
        self.decor(i);
        self.collide(min, max, Stops::BOOKS);
    }

    /// The atrium face of the wall on side `si` of `level`: pilasters on pedestals, a
    /// blind arch between each pair with a banner in the colour of the wall's
    /// department hung in it, panelling between the pedestals, sconces by the door and
    /// a cornice under the ceiling.
    fn dress_wall(&mut self, level: usize, si: usize) {
        let side = SIDES[si];
        let y = floor_y(level);
        let d = door_a(level, si);
        let first = self.catalog.departments[level * 4 + si].categories.start;
        let colour = catalog::category_color(&self.catalog.categories[first]);
        let piers: Vec<f32> = pilasters(d).collect();
        for &p in &piers {
            self.atrium_pilaster(side, p, y);
        }
        // The bays between the corner piers' and the pilasters' faces; the door's bay
        // has the door's frame instead.
        let mut faces = vec![-FACE + 0.5];
        for &p in &piers {
            faces.extend([p - 0.25, p + 0.25]);
        }
        faces.push(FACE - 0.5);
        for (b, bay) in faces.chunks(2).enumerate() {
            let (l, r) = (bay[0], bay[1]);
            if r > d - PORTAL_HALF && l < d + PORTAL_HALF {
                continue;
            }
            self.banner(side, (l + r) * 0.5, y + 4.15, colour, (level * 4 + si) * 7 + b);
            self.blind_arch(side, (l, r), y);
        }
        // Panelling between the pedestals (the corner piers', the pilasters' and the
        // door frame's), each length running a little into what it meets.
        let mut stops = vec![(-FACE, -FACE + 0.62), (d - 2.48, d + 2.48), (FACE - 0.62, FACE)];
        stops.extend(piers.iter().map(|&p| (p - 0.33, p + 0.33)));
        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in stops.windows(2) {
            let (a0, a1) = (w[0].1 - 0.01, w[1].0 + 0.01);
            self.wainscot(y, side.rect(a0, a1, FACE - 0.07, FACE), -side.out);
        }
        // Sconces either side of the door; where it is near the corner, on the corner
        // pier's flank.
        for sa in [d - 3.45, d + 3.45] {
            if sa < -FACE + 0.7 {
                self.sconce(side.at3(-FACE + 0.65, FACE - 0.25, y + 2.6), side.across3());
            } else {
                self.sconce(side.at3(sa, FACE - 0.15, y + 2.6), -side.out3());
            }
        }
        // The cornice in steps. Each side's runs into the corner at its -a end and
        // stops at the next side's at the other, so no two cover the same corner.
        for (y0, y1, depth) in [(5.02, 5.14, 0.1), (5.38, 5.5, 0.2), (5.5, ROOM_H, 0.3)] {
            let (a, o) = ((-FACE, FACE - depth), (FACE - depth, FACE));
            self.part(side, a, o, (y + y0, y + y1), kind::STONE, DRESSED);
        }
    }

    /// A pilaster on the wall on `side` at `a`, on storey floor `y`: a pedestal (in
    /// the way, like furniture against the wall), its shaft, and a capital.
    fn atrium_pilaster(&mut self, side: Side, a: f32, y: f32) {
        let o = |depth: f32| (FACE - depth, FACE);
        let pedestal = piece(side, (a - 0.33, a + 0.33), o(0.24), (y, y + 1.24), kind::STONE, DRESSED);
        self.against_wall(pedestal);
        for (w, depth, y0, y1, rgb) in [
            (0.37, 0.28, 1.24, 1.32, PALE),
            (0.25, 0.16, 1.32, CAPITAL - 0.25, DRESSED),
            (0.3, 0.2, CAPITAL - 0.25, CAPITAL - 0.15, PALE),
            (0.36, 0.27, CAPITAL - 0.15, CAPITAL, PALE),
        ] {
            self.part(side, (a - w, a + w), o(depth), (y + y0, y + y1), kind::STONE, rgb);
        }
    }

    /// A square stone pier in the atrium's corner toward (`sx`, `sz`), on storey floor
    /// `y`, built up as the pilasters are.
    fn corner_pier(&mut self, sx: f32, sz: f32, y: f32) {
        let s = Vec2::new(sx, sz);
        for (k, (w, y0, y1, rgb)) in [
            (0.62, 0.0, 1.24, DRESSED),
            (0.66, 1.24, 1.32, PALE),
            (0.5, 1.32, CAPITAL - 0.25, DRESSED),
            (0.55, CAPITAL - 0.25, CAPITAL - 0.15, PALE),
            (0.61, CAPITAL - 0.15, CAPITAL, PALE),
        ]
        .into_iter()
        .enumerate()
        {
            let (p, q) = (s * FACE, s * (FACE - w));
            let (lo, hi) = box3(p.min(q), p.max(q), y + y0, y + y1);
            self.decor(Inst::minmax(lo, hi, kind::STONE, rgb));
            if k == 0 {
                // The pedestal is in the way of books; of bodies, only its core, small
                // enough to change nothing about where piles go (they keep 1.4 m off
                // both walls, so more than that from it).
                self.collide(lo, hi, Stops::BOOKS);
                let (p, q) = (s * FACE, s * (FACE - 0.4));
                let (lo, hi) = box3(p.min(q), p.max(q), y + y0, y + y1);
                self.collide(lo, hi, Stops::ALL);
            }
        }
    }

    /// A blind arch on the wall on `side` over the bay `l..r` (between pilasters'
    /// faces) of the storey with its floor at `y`, springing from imposts: a ring of
    /// voussoirs with a keystone, round a darker recess with a round head.
    fn blind_arch(&mut self, side: Side, (l, r): Span, y: f32) {
        const RECESS: [f32; 3] = [0.2, 0.18, 0.16];
        let spring = y + SPRING;
        let (c, ri) = ((l + r) * 0.5, (r - l) * 0.5);
        let n = 9;
        let step = PI / n as f32;
        let wall = wall_rot(side);
        // A voussoir: the ring's wedge from `r0` to `r1` round the arch's centre, its
        // middle along x, 1 thick (each is sized to how proud it stands).
        let wedge = |lib: &mut Self, r0: f32, r1: f32| {
            let half = (-step * 0.5, step * 0.5);
            let mut outline: Vec<Vec2> = arc(r1, half, 4).collect();
            outline.extend(arc(r0, (half.1, half.0), 4));
            lib.shape(Mesh::prism(&outline, 1.0))
        };
        let plain = wedge(self, ri, ri + 0.28);
        let key = wedge(self, ri - 0.06, ri + 0.42);
        for k in 0..n {
            let t = (k as f32 + 0.5) * step;
            // Alternate stones stand a little prouder, so the joints show.
            let (shape, proud, rgb) = if k == n / 2 {
                (key, 0.15, PALE)
            } else {
                (plain, 0.1 + 0.012 * (k % 2) as f32, DRESSED)
            };
            let s = shape.sized(Vec3::new(shape.size.x, shape.size.y, proud));
            let at = side.at3(c, FACE - proud * 0.5, spring);
            self.decor(s.at(at, wall * Quat::from_rotation_z(t), kind::STONE, rgba(rgb, 1.0)));
        }
        for a in [(l - 0.04, l + 0.12), (r - 0.12, r + 0.04)] {
            self.part(side, a, (FACE - 0.2, FACE), (spring - 0.16, spring), kind::STONE, PALE);
        }
        // The recess: from the panelling up to the springing and round, its edges
        // tucked behind the pilasters and the ring.
        let (rr, foot) = (ri + 0.04, y + 1.16 - spring);
        let mut outline = vec![Vec2::new(-rr, foot), Vec2::new(rr, foot)];
        outline.extend(arc(rr, (0.0, PI), 32));
        let recess = self.shape(Mesh::prism(&outline, 0.012));
        let at = side.at3(c, FACE - 0.006, spring);
        self.decor(recess.at(at, wall, kind::STONE, rgba(RECESS, 1.0)));
    }

    /// A banner on the wall on `side`, centred at `c` across, hung from a gilt rod at
    /// `top`: a field of `colour` in a gold border, coming to a point with a tassel,
    /// a gold band across its head and a lozenge on it. It stirs in the air of the
    /// well, its foot lifting a little off the wall and settling back, slowly, each
    /// banner (`k`) in its own time.
    fn banner(&mut self, side: Side, c: f32, top: f32, colour: [f32; 3], k: usize) {
        const BORDER: [f32; 3] = [0.45, 0.32, 0.1];
        // How far the rod's axis stands off the wall.
        const ROD: f32 = 0.075;
        let (w, h) = (1.7, 1.9);
        // Made about the rod's axis: x across, y up, z out from the wall.
        let (y0, y1) = (-0.05 - h, -0.05);
        let tip = y0 - w * 0.5;
        // A layer standing `front`..`back` off the wall: its middle and thickness.
        let layer = |(front, back): Span| (Vec3::Z * ((front + back) * 0.5 - ROD), front - back);
        // Cloth `hw` either side, from `head` down to `foot` and on to a point.
        let pennant = |hw: f32, head: f32, foot: f32| {
            [(-hw, head), (-hw, foot), (0.0, foot - hw), (hw, foot), (hw, head)].map(Vec2::from)
        };
        let lozenge = Vec3::Y * (y0 + 0.75);
        let turned = Quat::from_rotation_z(FRAC_PI_4);
        let one = Vec3::ONE;
        // The border, the band across its head and the lozenge's frame, one cloth.
        let mut border = Mesh::default();
        let (at, d) = layer((0.076, 0.05));
        border.add(&Mesh::prism(&pennant(w * 0.5, y1, y0), d), at, Quat::IDENTITY, one);
        let (at, d) = layer((0.083, 0.06));
        let band = Vec3::new(w - 0.2, 0.06, d);
        border.add(&Mesh::cube(), at + Vec3::Y * (y1 - 0.33), Quat::IDENTITY, band);
        let (at, d) = layer((0.086, 0.065));
        border.add(&Mesh::cube(), at + lozenge, turned, Vec3::new(0.5, 0.5, d));
        let border = self.shape(border);
        let mut field = Mesh::default();
        let (at, d) = layer((0.08, 0.055));
        field.add(
            &Mesh::prism(&pennant(w * 0.5 - 0.1, y1 - 0.1, y0), d),
            at,
            Quat::IDENTITY,
            one,
        );
        let field = self.shape(field);
        let tassel = [
            (0.0, 0.0),
            (0.022, 0.0),
            (0.03, 0.03),
            (0.024, 0.07),
            (0.012, 0.085),
            (0.016, 0.1),
            (0.008, 0.12),
            (0.0, 0.12),
        ];
        let tassel = self.shape(Mesh::lathe(&profile(&tassel), 12));
        // The rod with a knob at each end, on brackets from the wall; it stays put.
        let mut rod = Mesh::default();
        let reach = w * 0.5 + 0.12;
        let along = Quat::from_rotation_z(FRAC_PI_2);
        rod.add(
            &Mesh::cylinder(12),
            Vec3::ZERO,
            along,
            Vec3::new(0.044, 2.0 * reach, 0.044),
        );
        for s in [-1.0f32, 1.0] {
            rod.add(
                &Mesh::sphere(12),
                Vec3::X * s * reach,
                Quat::IDENTITY,
                Vec3::splat(0.06),
            );
            let bracket = Vec3::new(s * (w * 0.5 + 0.03), -0.0075, 0.04 - ROD);
            rod.add(&Mesh::cube(), bracket, Quat::IDENTITY, Vec3::new(0.05, 0.085, 0.08));
        }
        let rod = self.shape(rod);
        let wall = wall_rot(side);
        let pivot = side.at3(c, FACE - ROD, top);
        let gilt = rgba(GILT, 1.0);
        self.decor(rod.at(pivot, wall, kind::METAL, gilt));
        // Swinging about the rod, a turn about `lift` taking its foot off the wall; laid
        // down lifted as far as it swings, so it swings between hanging flat and twice
        // that.
        let lift = -side.across3();
        let (amp, period) = (0.016 + 0.006 * (k % 3) as f32, 9.0 + 1.7 * (k % 4) as f32);
        let m = self.mover(
            pivot,
            Motion::Swing {
                axis: lift,
                amp,
                period,
            },
            k as f32 * 2.3,
            None,
        );
        let rot = Quat::from_axis_angle(lift, amp) * wall;
        self.moving(m, border.at(pivot, rot, kind::CLOTH, rgba(BORDER, 1.0)));
        let fill = colour.map(|v| v * 0.6);
        self.moving(m, field.at(pivot, rot, kind::CLOTH, rgba(fill, 1.0)));
        let (at, d) = layer((0.09, 0.07));
        let size = Vec3::new(0.24, 0.24, d);
        let cloth = rgba(colour, 1.0);
        self.moving(
            m,
            Inst::new(pivot + rot * (at + lozenge), rot * turned, size, kind::CLOTH, cloth, 0),
        );
        let foot = pivot + rot * Vec3::new(0.0, tip - 0.1, 0.069 - ROD);
        self.moving(m, tassel.at(foot, rot, kind::METAL, gilt));
    }

    /// Beams under the ceiling over the gallery (or the walk under it) on side `si` of
    /// `level`: across from the well's edge to each pilaster (or to the stairwell's
    /// edge), and along the edges.
    fn ceiling_beams(&mut self, level: usize, si: usize) {
        let side = SIDES[si];
        let top = floor_y(level) + ROOM_H;
        let stairwell = STAIRWAYS.iter().any(|f| f.level == level && f.side == si);
        let (r, e) = (STAIR_HALF_RUN, WELL + STAIR_W);
        let mut at: Vec<f32> = pilasters(door_a(level, si)).collect();
        for a in [-WELL, WELL] {
            if !at.contains(&a) {
                at.push(a);
            }
        }
        for a in at {
            // Over a stairwell, from inside the beam along its edge.
            let o0 = if stairwell && a.abs() < r + 0.3 { e + 0.1 } else { WELL };
            let beam = (a - 0.15, a + 0.15);
            self.part(side, beam, (o0, FACE), (top - 0.3, top), kind::WOOD, WALNUT);
        }
        let edges: &[(Span, Span)] = if stairwell {
            &[
                ((-WELL, -r), (WELL, WELL + 0.36)),
                ((r, WELL), (WELL, WELL + 0.36)),
                ((-r, r), (e, e + 0.36)),
                ((-r - 0.36, -r), (WELL + 0.36, e)),
            ]
        } else {
            &[((-WELL, WELL), (WELL, WELL + 0.36))]
        };
        for &(a, o) in edges {
            self.part(side, a, o, (top - 0.36, top), kind::WOOD, WALNUT);
        }
    }

    /// The front of a gallery along `spans` of `side`'s edge of the well, under the
    /// floor at `y`: a lipped cornice, a panelled frieze with a moulding under it, and
    /// a console where each beam under it comes to the edge.
    fn gallery_front(&mut self, side: Side, y: f32, spans: &[Span]) {
        let wood = |lib: &mut Self, a: Span, depth: f32, (y0, y1): Span, rgb: [f32; 3]| {
            lib.part(side, a, (WELL - depth, WELL), (y + y0, y + y1), kind::WOOD, rgb);
        };
        for &(a0, a1) in spans {
            wood(self, (a0, a1), 0.28, (-0.1, 0.03), WALNUT);
            // Consoles stop short of the frieze's back, which shows from the gallery
            // behind, so their backs don't share its plane.
            let consoles: Vec<f32> = [-4.5f32, 4.5].into_iter().filter(|&c| c > a0 && c < a1).collect();
            for &c in &consoles {
                let o = (WELL - 0.24, WELL - 0.01);
                self.part(side, (c - 0.15, c + 0.15), o, (y - 1.15, y - 0.1), kind::WOOD, WALNUT);
                wood(self, (c - 0.11, c + 0.11), 0.18, (-1.32, -1.15), WALNUT);
            }
            // Raised panels between the piers' faces (or the span's ends) and the
            // consoles.
            let mut cuts = vec![if a0 <= -WELL { -WELL + 0.35 } else { a0 }];
            for &c in &consoles {
                cuts.extend([c - 0.15, c + 0.15]);
            }
            cuts.push(if a1 >= WELL { WELL - 0.35 } else { a1 });
            for seg in cuts.chunks(2) {
                let n = ((seg[1] - seg[0]) / 1.45).round().max(1.0);
                let pitch = (seg[1] - seg[0]) / n;
                for i in 0..n as usize {
                    let p = seg[0] + i as f32 * pitch;
                    wood(self, (p + 0.12, p + pitch - 0.12), 0.13, (-0.78, -0.22), PANEL);
                }
            }
            wood(self, (a0, a1), 0.15, (-0.98, -0.9), WALNUT);
            wood(self, (a0, a1), 0.1, (-0.9, -0.1), FRIEZE);
        }
    }

    /// A lamp on a post standing in a gallery's railing at `foot`: over the handrail a
    /// turned column, a gilt font with a flame in a glass chimney, and a gilt crown
    /// round the chimney's mouth.
    fn lamp_standard(&mut self, foot: Vec3) {
        let column = [
            (0.0, 0.0),
            (0.06, 0.0),
            (0.06, 0.03),
            (0.044, 0.05),
            (0.04, 0.08),
            (0.056, 0.15),
            (0.05, 0.22),
            (0.03, 0.3),
            (0.034, 0.33),
            (0.05, 0.345),
            (0.05, 0.36),
            (0.0, 0.36),
        ];
        let font = [
            (0.0, 0.0),
            (0.032, 0.0),
            (0.036, 0.015),
            (0.07, 0.04),
            (0.08, 0.06),
            (0.074, 0.075),
            (0.052, 0.08),
            (0.0, 0.08),
        ];
        // Open at both ends.
        let chimney = [
            (0.05, 0.0),
            (0.06, 0.02),
            (0.075, 0.08),
            (0.072, 0.13),
            (0.05, 0.2),
            (0.042, 0.27),
        ];
        let crown = [
            (0.04, 0.0),
            (0.056, 0.0),
            (0.058, 0.035),
            (0.05, 0.05),
            (0.04, 0.035),
            (0.04, 0.0),
        ];
        let [column, font, chimney, crown] = [&column[..], &font, &chimney, &crown].map(profile);
        let column = self.shape(Mesh::lathe(&column, 24));
        let font = self.shape(Mesh::lathe(&font, 24));
        let chimney = self.shape(Mesh::lathe(&chimney, 24));
        let crown = self.shape(Mesh::lathe(&crown, 24));
        let flame = self.shape(flame()).sized(Vec3::new(0.035, 0.085, 0.035));
        let up = |h: f32| foot + Vec3::Y * h;
        let gilt = rgba(GILT, 1.0);
        let post = Vec3::new(0.11, 1.16, 0.11);
        self.decor(Inst::aabox(up(0.58), post, kind::WOOD, WALNUT));
        let id = Quat::IDENTITY;
        self.decor(column.at(up(1.16), id, kind::WOOD, rgba(WALNUT, 1.0)));
        self.decor(font.at(up(1.52), id, kind::METAL, gilt));
        self.decor(crown.at(up(1.85), id, kind::METAL, gilt));
        self.translucent(chimney.at(up(1.6), id, kind::CLEAR, [1.0, 0.9, 0.75, 0.08]));
        self.translucent(flame.at(up(1.615), id, kind::FIRE, [1.0, 0.6, 0.25, 4.0]));
    }

    /// The stone frame round the door from the atrium into department `dept`'s room
    /// on `side` of `level` (its door `d` across): pilasters either side, an
    /// entablature with the department's sign set in its frieze, and a gem of the
    /// department's colour at each end of the sign.
    fn portal(&mut self, level: usize, side: Side, d: f32, dept: usize) {
        let y = floor_y(level);
        let first = self.catalog.departments[dept].categories.start;
        let dcol = catalog::category_color(&self.catalog.categories[first]);
        let out = side.out3();
        let rot = Quat::from_mat3(&Mat3::from_cols(Vec3::Y.cross(-out), Vec3::Y, -out));
        let sign = side.at3(d, FACE - 0.175, y + 4.42);
        let board = Vec3::new(3.3, 0.5, 0.05);
        self.decor(Inst::new(sign, rot, board, kind::PLAQUE, rgba(dcol, 0.12), 0));
        let name = self.catalog.departments[dept].name.clone();
        self.inscribe(&name.to_uppercase(), DEPT_LETTERS, sign - out * 0.025, -out, 3.0);
        self.labels.push(Label {
            pos: sign - out * 0.04,
            normal: -out,
            text: name,
            em: DEPT_LETTERS.em,
            half: board.y * 0.5,
            kind: LabelKind::Department(dept),
            level,
        });
        // The gems, cut with a table and facets, each in a gilt collet; drawn each frame
        // (`door_gems`), brightening as the department nears done.
        let gem = [
            (0.0, 0.0),
            (0.5, 0.0),
            (0.5, 0.1),
            (0.36, 0.26),
            (0.22, 0.31),
            (0.0, 0.31),
        ];
        let gem = self.shape(Mesh::lathe(&profile(&gem), 8));
        let gem = gem.sized(Vec3::new(0.22, 0.07, 0.22));
        let collet = [
            (0.105, 0.0),
            (0.14, 0.0),
            (0.14, 0.03),
            (0.12, 0.05),
            (0.105, 0.04),
            (0.105, 0.0),
        ];
        let collet = self.shape(Mesh::lathe(&profile(&collet), 24));
        // A facet's edge level.
        let rot = out_of_wall(side) * Quat::from_rotation_y(PI / 8.0);
        for s in [-1.0f32, 1.0] {
            let at = side.at3(d + s * 2.05, FACE - 0.15, y + 4.42);
            self.decor(collet.at(at, rot, kind::METAL, rgba(GILT, 1.0)));
            self.door_gems
                .push((dept, gem.at(at, rot, kind::CRYSTAL, rgba(dcol, 0.8))));
        }
        // A stone `a` across from the door, `depth` off the wall, `ys` up from the floor.
        let stone = |lib: &mut Self, (a0, a1): Span, depth: f32, (y0, y1): Span, rgb: [f32; 3]| {
            let (a, o, ys) = ((d + a0, d + a1), (FACE - depth, FACE), (y + y0, y + y1));
            lib.part(side, a, o, ys, kind::STONE, rgb);
        };
        stone(self, (-PORTAL_HALF, PORTAL_HALF), 0.3, (4.82, 4.94), PALE);
        stone(self, (-2.6, 2.6), 0.24, (4.74, 4.82), PALE);
        stone(self, (-2.56, 2.56), 0.21, (3.98, 4.1), PALE);
        for s in [-1.0f32, 1.0] {
            let span = |x0: f32, x1: f32| ((s * x0).min(s * x1), (s * x0).max(s * x1));
            let (a0, a1) = span(1.92, 2.48);
            let a = (d + a0, d + a1);
            let plinth = piece(side, a, (FACE - 0.26, FACE), (y, y + 0.42), kind::STONE, DRESSED);
            self.against_wall(plinth);
            stone(self, span(1.92, 2.48), 0.25, (3.72, 3.98), PALE);
            stone(self, span(1.98, 2.42), 0.18, (0.42, 3.72), DRESSED);
        }
        stone(self, (-2.45, 2.45), 0.15, (4.1, 4.74), DRESSED);
    }

    /// Panelling on both sides of the flight on `side` rising from `y0` to `y1`: a rail
    /// along the slope under the stringer, a plinth, stiles between, and raised panels
    /// stepping up; and the same on the flight's high end, where it shows under the
    /// floor it climbs to.
    fn stair_panelling(&mut self, side: Side, y0: f32, y1: f32) {
        let r = STAIR_HALF_RUN;
        // Just under the stringer, `a` along the flight.
        let under = |a: f32| y0 + (y1 - y0) * (a + r) / (2.0 * r) - 0.22;
        let start = -r + 1.5;
        let bays = 9;
        let pitch = (r - start) / bays as f32;
        let mid = y0 + 2.0;
        for (stone, out) in [(WELL, -1.0f32), (WELL + STAIR_W, 1.0)] {
            // `o` from the stone's face out `depth`.
            let o = |depth: f32| ((stone + out * depth).min(stone), (stone + out * depth).max(stone));
            let oc = |depth: f32| stone + out * depth * 0.5;
            let wood = |lib: &mut Self, a: Span, depth: f32, y: Span, rgb: [f32; 3]| {
                lib.part(side, a, o(depth), y, kind::WOOD, rgb);
            };
            let along = |a: f32, dy: f32, depth: f32| side.at3(a, oc(depth), under(a) + dy);
            // Front to back: panels, the frame round them, then what they stand on.
            for i in 0..bays {
                let a = start + i as f32 * pitch;
                let top = under(a) - 0.25;
                let panel = (a + 0.16, a + pitch - 0.16);
                if top > mid + 0.4 {
                    wood(self, panel, 0.035, (y0 + 0.34, mid - 0.19), PANEL);
                    wood(self, panel, 0.035, (mid + 0.19, top - 0.11), PANEL);
                    wood(
                        self,
                        (a + 0.05, a + pitch - 0.05),
                        0.045,
                        (mid - 0.05, mid + 0.05),
                        WALNUT,
                    );
                } else if top > y0 + 0.7 {
                    wood(self, panel, 0.035, (y0 + 0.34, top - 0.11), PANEL);
                }
            }
            for i in 0..=bays {
                let a = start + i as f32 * pitch;
                // The last stands behind the gallery railing's end newel: set back so
                // the newel's cap is proud of it rather than flush.
                let depth = if i == bays { 0.04 } else { 0.045 };
                wood(self, (a - 0.05, a + 0.05), depth, (y0 + 0.2, under(a) - 0.1), WALNUT);
            }
            wood(self, (start - 0.05, r), 0.045, (y0, y0 + 0.2), WALNUT);
            self.beam(along(start, -0.07, 0.045), along(r, -0.07, 0.045), 0.045, 0.14, WALNUT);
            for i in 0..bays {
                let a = start + i as f32 * pitch;
                wood(self, (a, a + pitch), 0.025, (y0 + 0.2, under(a) - 0.25), FRIEZE);
            }
            self.beam(along(start, -0.45, 0.025), along(r, -0.45, 0.025), 0.025, 0.76, FRIEZE);
        }
        // The high end, under the floor above.
        let head = y1 - SLAB;
        let (o0, o1) = (WELL, WELL + STAIR_W);
        let end = |lib: &mut Self, o: Span, depth: f32, y: Span, rgb: [f32; 3]| {
            lib.part(side, (r, r + depth), o, y, kind::WOOD, rgb);
        };
        let panel = (o0 + 0.26, o1 - 0.26);
        end(self, panel, 0.035, (y0 + 0.34, mid - 0.19), PANEL);
        end(self, panel, 0.035, (mid + 0.19, head - 0.3), PANEL);
        end(self, (o0, o1), 0.045, (y0, y0 + 0.2), WALNUT);
        end(self, (o0, o1), 0.045, (head - 0.16, head), WALNUT);
        end(self, (o0 + 0.12, o1 - 0.12), 0.045, (mid - 0.05, mid + 0.05), WALNUT);
        for (s0, s1) in [(o0, o0 + 0.12), (o1 - 0.12, o1)] {
            end(self, (s0, s1), 0.045, (y0 + 0.2, head - 0.16), WALNUT);
        }
        end(self, (o0, o1), 0.025, (y0, head), FRIEZE);
    }

    /// The skylight over the well: leaded glass in an oak frame, iron glazing bars,
    /// ribs from its corners and middles to a gilt boss the chandelier hangs from, two
    /// panes clear to the night with the moon in one; and under it, the lantern's
    /// walls, rose windows in each and a pier at each corner.
    fn skylight(&mut self) {
        let lantern = WELL + 0.2;
        let inner = WELL - 0.45;
        for side in SIDES {
            let a = (-lantern, inner);
            self.part(side, a, (inner, lantern), (SKY - 0.5, SKY), kind::WOOD, WALNUT);
            for c in [-5.6f32, 0.0, 5.6] {
                self.rose_window(side, c, SKY - 1.3);
            }
        }
        let roof = floor_y(STOREYS - 1) + ROOM_H;
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let c = Vec3::new(sx * WELL, roof, sz * WELL);
            self.decor(Inst::minmax(
                c - Vec3::new(0.35, 0.0, 0.35),
                c + Vec3::new(0.35, SKY - 0.5 - roof, 0.35),
                kind::STONE,
                DRESSED,
            ));
        }
        // Ribs: across the middle both ways (the second in halves, meeting the first),
        // and from each corner in to the boss.
        let rib = |lib: &mut Self, mn: Vec3, mx: Vec3| lib.decor(Inst::minmax(mn, mx, kind::WOOD, WALNUT));
        rib(self, Vec3::new(-inner, SKY - 0.45, -0.16), Vec3::new(inner, SKY, 0.16));
        for s in [-1.0f32, 1.0] {
            let (z0, z1) = (s * 0.16, s * inner);
            rib(
                self,
                Vec3::new(-0.16, SKY - 0.45, z0.min(z1)),
                Vec3::new(0.16, SKY, z0.max(z1)),
            );
        }
        for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let dir = Vec3::new(sx, 0.0, sz).normalize();
            let (near, far) = (0.5, inner * 2f32.sqrt());
            let c = dir * ((near + far) * 0.5) + Vec3::Y * (SKY - 0.175);
            let rot = Quat::from_rotation_arc(Vec3::X, dir);
            let size = Vec3::new(far - near, 0.35, 0.2);
            self.decor(Inst::new(c, rot, size, kind::WOOD, rgba(WALNUT, 1.0), 0));
        }
        // Iron glazing bars every 1.8 m, the two ways at different depths.
        const IRON: [f32; 3] = [0.1, 0.1, 0.11];
        let bar = |lib: &mut Self, mn: Vec3, mx: Vec3| lib.decor(Inst::minmax(mn, mx, kind::METAL, IRON));
        for k in [-4, -3, -2, -1, 1, 2, 3, 4] {
            let v = k as f32 * 1.8;
            bar(
                self,
                Vec3::new(-inner, SKY - 0.12, v - 0.025),
                Vec3::new(inner, SKY, v + 0.025),
            );
            bar(
                self,
                Vec3::new(v - 0.025, SKY - 0.1, -inner),
                Vec3::new(v + 0.025, SKY, inner),
            );
        }
        // The boss: a gilt rose under the ribs' meeting, and a turned drop under it,
        // made from the drop's point up.
        let boss = [
            (0.0, 0.0),
            (0.03, 0.02),
            (0.06, 0.07),
            (0.075, 0.12),
            (0.06, 0.17),
            (0.03, 0.2),
            (0.03, 0.23),
            (0.07, 0.25),
            (0.07, 0.28),
            (0.04, 0.3),
            (0.05, 0.33),
            (0.18, 0.36),
            (0.32, 0.4),
            (0.43, 0.46),
            (0.5, 0.53),
            (0.53, 0.6),
            (0.53, 0.68),
            (0.0, 0.68),
        ];
        let boss = self.shape(Mesh::lathe(&profile(&boss), 32));
        let foot = Vec3::new(0.0, SKY - 1.13, 0.0);
        self.decor(boss.at(foot, Quat::IDENTITY, kind::METAL, rgba(GILT, 1.0)));
        // Two panes of clear glass, the night through them, the moon in one.
        self.decor(Inst::minmax(
            Vec3::new(-5.4, SKY - 0.008, -3.6),
            Vec3::new(-1.8, SKY - 0.002, -1.8),
            kind::EMISSIVE,
            [0.03, 0.045, 0.11],
        ));
        let moon = Vec3::new(-4.5, SKY - 0.012, -2.7);
        for (k, turn) in [0.0f32, 30.0, 60.0].into_iter().enumerate() {
            let at = moon - Vec3::Y * (k as f32 * 0.002);
            let rot = Quat::from_rotation_y(turn.to_radians());
            let size = Vec3::new(0.8, 0.002, 0.8);
            self.decor(Inst::new(at, rot, size, kind::EMISSIVE, [0.92, 0.94, 1.0, 2.6], 0));
        }
        for (dx, dz, w, turn) in [
            (0.12f32, -0.1f32, 0.22f32, 20.0f32),
            (-0.15, 0.08, 0.16, 50.0),
            (0.06, 0.2, 0.12, 10.0),
        ] {
            let at = moon + Vec3::new(dx, -0.008, dz);
            let rot = Quat::from_rotation_y(turn.to_radians());
            let size = Vec3::new(w, 0.002, w * 0.8);
            self.decor(Inst::new(at, rot, size, kind::EMISSIVE, [0.6, 0.64, 0.76, 2.6], 0));
        }
        for (x, z) in [
            (-3.2f32, -3.1f32),
            (-2.4, -2.2),
            (-2.9, -2.6),
            (-2.1, -3.3),
            (-5.1, -1.95),
            (-3.95, -3.35),
        ] {
            let star = Vec3::new(x, SKY - 0.012, z);
            self.decor(Inst::aabox(
                star,
                Vec3::new(0.03, 0.002, 0.03),
                kind::EMISSIVE,
                [1.0, 1.0, 1.0],
            ));
        }
        // The glass, turned so its leading runs both ways across its underside.
        self.decor(Inst::new(
            Vec3::new(0.0, SKY + 0.04, 0.0),
            Quat::from_rotation_x(-FRAC_PI_2),
            Vec3::new(2.0 * WELL, 2.0 * WELL, 0.08),
            kind::GLASS,
            [0.32, 0.42, 0.85, 1.0],
            0,
        ));
    }

    /// A round window of moonlit glass in the lantern wall on `side`, centred at `c`
    /// across and `y` up: a moulded ring of stone, and tracery spokes to a boss.
    fn rose_window(&mut self, side: Side, c: f32, y: f32) {
        let (ri, ro) = (0.6, 0.74);
        let at = side.at3(c, WELL, y);
        let stone = rgba(DRESSED, 1.0);
        // The ring's section, (radius, off the wall): chamfered inside, rolled outside.
        let ring = [
            (ri, 0.0),
            (ro, 0.0),
            (ro, 0.04),
            (ro - 0.03, 0.085),
            (ri + 0.05, 0.085),
            (ri, 0.05),
            (ri, 0.0),
        ];
        let ring = self.shape(Mesh::lathe(&profile(&ring), 64));
        self.decor(ring.at(at, out_of_wall(side), kind::STONE, rgba(PALE, 1.0)));
        let boss = [
            (0.0, 0.0),
            (0.09, 0.0),
            (0.09, 0.04),
            (0.065, 0.08),
            (0.03, 0.095),
            (0.0, 0.1),
        ];
        let boss = self.shape(Mesh::lathe(&profile(&boss), 24));
        self.decor(boss.at(at, out_of_wall(side), kind::STONE, rgba(PALE, 1.0)));
        for k in 0..4 {
            let (proud, angle) = (0.05 + 0.004 * k as f32, k as f32 * FRAC_PI_4);
            let look = (kind::STONE, stone);
            self.decor(in_wall(
                side,
                (c, y),
                (WELL - proud, WELL),
                (2.0 * ri + 0.04, 0.05),
                angle,
                look,
            ));
        }
        let glass = self.shape(Mesh::prism(&round(ri + 0.02, 48), 0.01));
        let at = side.at3(c, WELL - 0.007, y);
        self.decor(glass.at(at, wall_rot(side), kind::GLASS, [0.32, 0.42, 0.85, 1.1]));
    }

    /// Lanterns floating in the air of the well with nothing holding them up, each
    /// bobbing in its own time, two of them lighting the galleries round them: a globe
    /// of clear glass in a gilt cage with a flame floating in it, a domed cap with a
    /// ring, and a turned drop under it.
    fn floating_lanterns(&mut self) {
        const SPOTS: [(f32, f32, f32); 6] = [
            (0.4, 5.0, 10.6),
            (1.45, 4.4, 13.9),
            (2.5, 5.3, 11.5),
            (3.55, 4.6, 15.2),
            (4.6, 5.1, 12.3),
            (5.65, 4.3, 14.6),
        ];
        // Made about the lantern's middle, y up. The glass is whole but for its mouth,
        // under the cap, so the flame shows from below.
        let globe = [
            (0.0, -0.2),
            (0.07, -0.19),
            (0.14, -0.15),
            (0.18, -0.08),
            (0.19, 0.0),
            (0.175, 0.08),
            (0.12, 0.16),
        ];
        let drop = [
            (0.0, -0.36),
            (0.02, -0.34),
            (0.034, -0.3),
            (0.024, -0.26),
            (0.034, -0.23),
            (0.04, -0.21),
            (0.0, -0.195),
        ];
        let cap = [
            (0.0, 0.15),
            (0.135, 0.15),
            (0.15, 0.17),
            (0.12, 0.21),
            (0.07, 0.25),
            (0.045, 0.27),
            (0.05, 0.3),
            (0.035, 0.33),
            (0.0, 0.35),
        ];
        let [globe_p, drop, cap] = [&globe[..], &drop, &cap].map(profile);
        let glass = self.shape(Mesh::lathe(&globe_p, 32));
        let drop = self.shape(Mesh::lathe(&drop, 16));
        let cap = self.shape(Mesh::lathe(&cap, 24));
        // A ring at the top, 9 cm across (the torus is made 2.25 across).
        let ring = self.shape(Mesh::torus(0.25, 16, 6)).scaled(0.04);
        let flame = self.shape(flame()).sized(Vec3::new(0.07, 0.17, 0.07));
        // The cage: a collar round the foot of the glass, and four ribs from it up the
        // glass to the cap, just outside it.
        let mut cage = Mesh::default();
        let collar = Vec3::splat(0.17 / 2.25);
        cage.add(&Mesh::torus(0.25, 24, 6), Vec3::Y * -0.19, Quat::IDENTITY, collar);
        let rod = round(1.0, 8);
        for q in 0..4 {
            let a = q as f32 * FRAC_PI_2;
            let dir = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a));
            let path: Vec<Vec3> = (globe[1..].iter())
                .map(|&(r, y)| dir * (r + 0.006) + Vec3::Y * y)
                .collect();
            cage.add(
                &Mesh::sweep(&path, &[0.007], &rod, true),
                Vec3::ZERO,
                Quat::IDENTITY,
                Vec3::ONE,
            );
        }
        let cage = self.shape(cage);
        let gilt = rgba(GILT, 1.0);
        for (k, &(t, r, y)) in SPOTS.iter().enumerate() {
            let c = Vec3::new(libm::cosf(t) * r, y, libm::sinf(t) * r);
            let rot = Quat::from_rotation_y(t * 2.3 + k as f32);
            let bob = Motion::Bob {
                amp: 0.07 + 0.02 * (k % 3) as f32,
                period: 6.5 + 0.9 * k as f32,
                rock: 0.04,
            };
            let m = self.mover(c, bob, k as f32 * 2.1, None);
            for part in [drop, cap, cage] {
                self.moving(m, part.at(c, rot, kind::METAL, gilt));
            }
            let upright = rot * Quat::from_rotation_x(FRAC_PI_2);
            self.moving(m, ring.inst(c + Vec3::Y * 0.39, upright, kind::METAL, gilt));
            self.moving(m, flame.at(c - Vec3::Y * 0.13, rot, kind::FIRE, [1.0, 0.62, 0.26, 6.0]));
            self.moving(m, glass.at(c, rot, kind::CLEAR, [1.0, 0.86, 0.62, 0.1]));
            if k % 3 == 0 {
                self.light_riding(m, c, 7.0, [1.0, 0.7, 0.4], 0.9, LightKind::Static);
            }
        }
    }
}
