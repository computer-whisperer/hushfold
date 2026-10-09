//! The Hall: one storey, two wings of freestanding bookcases either side of a reading
//! room, the East Wing veiled until the West is mostly sorted. Its generation must
//! not change (saves name books by index; see `hall_generates_as_before`).

use super::*;

pub const HALL_X: f32 = 24.0;
pub const HALL_Z: f32 = 18.0;
pub const HALL_H: f32 = 8.0;
pub const VEIL_X: f32 = 11.6;
const CASES_PER_CATEGORY: usize = 3;
const WINDOWS_X: [f32; 8] = [-20.0, -15.5, -7.0, -2.3, 2.3, 7.0, 15.5, 20.0];

pub(super) fn build(lib: &mut Library, rng: &mut StdRng) {
    lib.levels.push(Level {
        y: 0.0,
        roof: HALL_H,
        min: Vec2::new(-HALL_X, -HALL_Z),
        max: Vec2::new(HALL_X, HALL_Z),
    });
    lib.slabs.push(Slab {
        level: 0,
        min: Vec2::new(-HALL_X, -HALL_Z),
        max: Vec2::new(HALL_X, HALL_Z),
    });
    lib.spawn = (Vec3::new(0.0, 0.0, 6.5), 0.0);
    lib.build_hall();
    lib.build_stacks(rng);
    lib.build_center();
    lib.build_piles(rng);
    // The veil hangs between the east columns until 60% of the West Wing is home.
    lib.gates.push(Gate {
        level: 0,
        min: Vec3::new(VEIL_X - 0.05, 0.0, -HALL_Z),
        max: Vec3::new(VEIL_X + 0.05, HALL_H, HALL_Z),
        opens: 1,
        counts: vec![0],
        need: 0.6,
    });
    // Moonbeams through the north windows.
    let beam_dir = Vec3::new(0.0, -0.62, 1.0).normalize();
    for x in WINDOWS_X {
        let start = Vec3::new(x, 4.4, -HALL_Z + 0.1);
        let len = 9.0;
        let rot = Quat::from_rotation_arc(Vec3::Y, beam_dir);
        lib.static_trans.push(Inst::new(
            start + beam_dir * len * 0.5,
            rot,
            Vec3::new(1.6, len, 3.4),
            kind::SHAFT,
            [0.35, 0.45, 0.9, 0.05],
            0,
        ));
    }
}

impl Library {
    fn build_hall(&mut self) {
        let (hx, hz, hh) = (HALL_X, HALL_Z, HALL_H);
        let floor_col = [0.30, 0.16, 0.075];
        self.push(Inst::minmax(
            Vec3::new(-hx, -0.2, -hz),
            Vec3::new(hx, 0.0, hz),
            kind::FLOOR,
            floor_col,
        ));
        // Walls.
        let t = 0.5;
        self.push(Inst::minmax(
            Vec3::new(-hx - t, 0.0, -hz - t),
            Vec3::new(hx + t, hh, -hz),
            kind::STONE,
            STONE,
        ));
        self.push(Inst::minmax(
            Vec3::new(-hx - t, 0.0, hz),
            Vec3::new(hx + t, hh, hz + t),
            kind::STONE,
            STONE,
        ));
        self.push(Inst::minmax(
            Vec3::new(-hx - t, 0.0, -hz),
            Vec3::new(-hx, hh, hz),
            kind::STONE,
            STONE,
        ));
        self.push(Inst::minmax(
            Vec3::new(hx, 0.0, -hz),
            Vec3::new(hx + t, hh, hz),
            kind::STONE,
            STONE,
        ));
        // Ceiling and beams.
        self.push(Inst::minmax(
            Vec3::new(-hx - t, hh, -hz - t),
            Vec3::new(hx + t, hh + 0.3, hz + t),
            kind::WOOD,
            DARK_WOOD,
        ));
        let mut x = -hx + 2.0;
        while x < hx {
            self.push(Inst::minmax(
                Vec3::new(x - 0.18, hh - 0.45, -hz),
                Vec3::new(x + 0.18, hh, hz),
                kind::WOOD,
                [0.11, 0.055, 0.025],
            ));
            x += 4.0;
        }
        // Wainscot panelling with a rail.
        let wh = 1.15;
        let wc = [0.13, 0.065, 0.03];
        self.push(Inst::minmax(
            Vec3::new(-hx, 0.0, -hz),
            Vec3::new(hx, wh, -hz + 0.06),
            kind::WOOD,
            wc,
        ));
        self.push(Inst::minmax(
            Vec3::new(-hx, 0.0, hz - 0.06),
            Vec3::new(hx, wh, hz),
            kind::WOOD,
            wc,
        ));
        self.push(Inst::minmax(
            Vec3::new(-hx, 0.0, -hz),
            Vec3::new(-hx + 0.06, wh, hz),
            kind::WOOD,
            wc,
        ));
        self.push(Inst::minmax(
            Vec3::new(hx - 0.06, 0.0, -hz),
            Vec3::new(hx, wh, hz),
            kind::WOOD,
            wc,
        ));
        for (a, b) in [
            (Vec3::new(-hx, wh, -hz), Vec3::new(hx, wh + 0.06, -hz + 0.1)),
            (Vec3::new(-hx, wh, hz - 0.1), Vec3::new(hx, wh + 0.06, hz)),
            (Vec3::new(-hx, wh, -hz), Vec3::new(-hx + 0.1, wh + 0.06, hz)),
            (Vec3::new(hx - 0.1, wh, -hz), Vec3::new(hx, wh + 0.06, hz)),
        ] {
            self.push(Inst::minmax(a, b, kind::WOOD, WALNUT));
        }

        // Columns flanking the reading room (the veil hangs between the east ones).
        for sx in [-VEIL_X, VEIL_X] {
            for z in [-16.0, -8.0, 8.0, 16.0] {
                let c = Vec3::new(sx, 0.0, z);
                self.push(Inst::aabox(
                    c + Vec3::Y * hh * 0.5,
                    Vec3::new(0.7, hh, 0.7),
                    kind::STONE,
                    [0.42, 0.39, 0.35],
                ));
                self.push(Inst::aabox(
                    c + Vec3::Y * 0.2,
                    Vec3::new(0.95, 0.4, 0.95),
                    kind::STONE,
                    [0.3, 0.28, 0.25],
                ));
                self.push(Inst::aabox(
                    c + Vec3::Y * (hh - 0.6),
                    Vec3::new(0.95, 0.3, 0.95),
                    kind::STONE,
                    [0.3, 0.28, 0.25],
                ));
                self.obstacle(c - Vec3::new(0.48, 0.0, 0.48), c + Vec3::new(0.48, hh, 0.48), true);
            }
        }

        // Moonlit windows on the north wall.
        for x in WINDOWS_X {
            let c = Vec3::new(x, 4.4, -hz + 0.02);
            let glass = Inst::new(
                c,
                Quat::IDENTITY,
                Vec3::new(1.5, 4.2, 0.08),
                kind::GLASS,
                [0.32, 0.42, 0.85, 1.2],
                0,
            );
            self.push(glass);
            // Frame and sill.
            self.push(Inst::aabox(
                c + Vec3::new(0.0, -2.2, 0.12),
                Vec3::new(1.9, 0.14, 0.3),
                kind::STONE,
                [0.45, 0.42, 0.38],
            ));
            self.push(Inst::aabox(
                c + Vec3::new(0.0, 2.2, 0.08),
                Vec3::new(1.9, 0.2, 0.2),
                kind::STONE,
                [0.45, 0.42, 0.38],
            ));
            for s in [-1.0, 1.0] {
                self.push(Inst::aabox(
                    c + Vec3::new(s * 0.85, 0.0, 0.08),
                    Vec3::new(0.2, 4.6, 0.2),
                    kind::STONE,
                    [0.45, 0.42, 0.38],
                ));
            }
            self.light(
                Vec3::new(x, 3.2, -hz + 1.5),
                8.0,
                [0.45, 0.55, 1.0],
                0.9,
                LightKind::Static,
            );
        }

        // Sconces.
        for z in [-14.0, -6.0, 6.0, 14.0] {
            self.sconce(Vec3::new(-hx + 0.15, 2.6, z), Vec3::X);
            self.sconce(Vec3::new(hx - 0.15, 2.6, z), -Vec3::X);
        }
        for x in [-17.0, 17.0] {
            self.sconce(Vec3::new(x, 2.6, hz - 0.15), -Vec3::Z);
        }
    }

    fn build_stacks(&mut self, rng: &mut StdRng) {
        let mut titles = HashSet::new();
        let rows_z = [-12.0f32, -4.0, 4.0, 12.0];
        for wing in 0..2 {
            let xs: [f32; 3] = if wing == 0 {
                [-19.0, -19.0 + UNIT_PITCH, -19.0 + 2.0 * UNIT_PITCH]
            } else {
                [19.0 - 2.0 * UNIT_PITCH, 19.0 - UNIT_PITCH, 19.0]
            };
            // Unit geometry.
            for &rz in &rows_z {
                for &cx in &xs {
                    self.build_unit(Vec3::new(cx, 0.0, rz));
                }
            }
            let mut cat = self.catalog.departments[wing].categories.start;
            for &rz in &rows_z {
                for nz in [-1.0f32, 1.0] {
                    let normal = Vec3::new(0.0, 0.0, nz);
                    let right = Vec3::Y.cross(normal);
                    let mut order: Vec<f32> = xs.to_vec();
                    order.sort_by(|a, b| (a * right.x).partial_cmp(&(b * right.x)).unwrap());
                    let cases: Vec<Vec3> = order
                        .iter()
                        .map(|&cx| Vec3::new(cx, 0.0, rz + nz * UNIT_DEPTH * 0.5))
                        .collect();
                    debug_assert_eq!(cases.len(), CASES_PER_CATEGORY);
                    self.add_category(rng, cat, &cases, normal, &mut titles);
                    cat += 1;
                }
            }
        }
    }

    fn build_center(&mut self) {
        // Rug.
        let second = [22u32, 30, 70];
        let mut rug = Inst::aabox(
            Vec3::new(0.0, 0.012, 0.0),
            Vec3::new(11.0, 0.024, 8.0),
            kind::RUG,
            [0.28, 0.035, 0.03],
        );
        rug.param = second[0] | (second[1] << 8) | (second[2] << 16);
        self.push(rug);

        // Reading tables with chairs and candles.
        for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
            let c = Vec3::new(tx, 0.0, tz);
            self.push(Inst::aabox(
                c + Vec3::Y * (TABLE_H - 0.03),
                Vec3::new(2.4, 0.06, 1.2),
                kind::WOOD,
                WALNUT,
            ));
            for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                self.push(Inst::aabox(
                    c + Vec3::new(sx * 1.08, (TABLE_H - 0.06) * 0.5, sz * 0.48),
                    Vec3::new(0.09, TABLE_H - 0.06, 0.09),
                    kind::WOOD,
                    DARK_WOOD,
                ));
            }
            self.obstacle(c - Vec3::new(1.25, 0.0, 0.65), c + Vec3::new(1.25, TABLE_H, 0.65), true);
            for sz in [-1.0f32, 1.0] {
                for sx in [-0.6f32, 0.6] {
                    let ch = c + Vec3::new(sx, 0.0, sz * 1.0);
                    self.push(Inst::aabox(
                        ch + Vec3::Y * 0.45,
                        Vec3::new(0.48, 0.05, 0.46),
                        kind::WOOD,
                        [0.2, 0.09, 0.04],
                    ));
                    self.push(Inst::aabox(
                        ch + Vec3::new(0.0, 0.75, sz * 0.21),
                        Vec3::new(0.48, 0.6, 0.04),
                        kind::WOOD,
                        [0.2, 0.09, 0.04],
                    ));
                    for (lx, lz) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
                        self.push(Inst::aabox(
                            ch + Vec3::new(lx, 0.22, lz),
                            Vec3::new(0.04, 0.44, 0.04),
                            kind::WOOD,
                            DARK_WOOD,
                        ));
                    }
                    self.obstacle(ch - Vec3::new(0.26, 0.0, 0.26), ch + Vec3::new(0.26, 1.05, 0.26), false);
                }
            }
            let candle = c + Vec3::new(0.7, TABLE_H, 0.0);
            self.push(Inst::aabox(
                candle + Vec3::Y * 0.02,
                Vec3::new(0.14, 0.04, 0.14),
                kind::WOOD,
                BRASS,
            ));
            self.push(Inst::aabox(
                candle + Vec3::Y * 0.11,
                Vec3::new(0.05, 0.16, 0.05),
                kind::PLAIN,
                [0.8, 0.75, 0.6],
            ));
            let mut flame = Inst::aabox(
                candle + Vec3::Y * 0.22,
                Vec3::new(0.025, 0.05, 0.025),
                kind::EMISSIVE,
                [1.0, 0.55, 0.18],
            );
            flame.color[3] = 14.0;
            flame.param = 1;
            self.push(flame);
            self.light(candle + Vec3::Y * 0.35, 4.5, [1.0, 0.6, 0.28], 0.7, LightKind::Flicker);
        }

        // Chandelier.
        let ch = Vec3::new(0.0, 5.6, 0.0);
        for k in 0..12 {
            let a = k as f32 / 12.0 * TAU;
            let p = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            self.push(Inst::new(
                p,
                Quat::from_rotation_y(-a),
                Vec3::new(0.08, 0.06, 0.72),
                kind::WOOD,
                [BRASS[0], BRASS[1], BRASS[2], 1.0],
                0,
            ));
            if k % 2 == 0 {
                self.push(Inst::aabox(
                    p + Vec3::Y * 0.1,
                    Vec3::new(0.05, 0.14, 0.05),
                    kind::PLAIN,
                    [0.8, 0.75, 0.6],
                ));
                let mut flame = Inst::aabox(
                    p + Vec3::Y * 0.21,
                    Vec3::new(0.03, 0.06, 0.03),
                    kind::EMISSIVE,
                    [1.0, 0.55, 0.18],
                );
                flame.color[3] = 14.0;
                flame.param = 1;
                self.push(flame);
            }
        }
        for k in 0..3 {
            let a = k as f32 / 3.0 * TAU;
            let top = Vec3::new(0.0, HALL_H, 0.0);
            let bottom = ch + Vec3::new(a.cos(), 0.0, a.sin()) * 1.3;
            let d = top - bottom;
            let rot = Quat::from_rotation_arc(Vec3::Y, d.normalize());
            self.push(Inst::new(
                (top + bottom) * 0.5,
                rot,
                Vec3::new(0.025, d.length(), 0.025),
                kind::WOOD,
                [0.1, 0.08, 0.05, 1.0],
                0,
            ));
        }
        self.light(ch + Vec3::Y * 0.2, 20.0, [1.0, 0.68, 0.38], 4.0, LightKind::Flicker);

        // Fireplace on the south wall.
        let fz = HALL_Z;
        let fc = Vec3::new(0.0, 0.0, fz);
        let st = [0.4, 0.37, 0.33];
        self.push(Inst::minmax(
            fc + Vec3::new(-2.2, 0.0, -0.9),
            fc + Vec3::new(-1.4, 2.6, 0.0),
            kind::STONE,
            st,
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(1.4, 0.0, -0.9),
            fc + Vec3::new(2.2, 2.6, 0.0),
            kind::STONE,
            st,
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(-2.4, 2.0, -1.0),
            fc + Vec3::new(2.4, 2.7, 0.0),
            kind::STONE,
            st,
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(-2.5, 2.7, -1.1),
            fc + Vec3::new(2.5, 2.85, 0.0),
            kind::WOOD,
            WALNUT,
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(-1.6, 2.85, -0.6),
            fc + Vec3::new(1.6, HALL_H, 0.0),
            kind::STONE,
            [0.33, 0.3, 0.27],
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(-2.4, 0.0, -1.6),
            fc + Vec3::new(2.4, 0.08, -0.9),
            kind::STONE,
            [0.25, 0.23, 0.21],
        ));
        self.push(Inst::minmax(
            fc + Vec3::new(-1.4, 0.0, -0.5),
            fc + Vec3::new(1.4, 2.0, -0.4),
            kind::PLAIN,
            [0.02, 0.015, 0.012],
        ));
        for (i, (x, ang)) in [(-0.3f32, 0.3f32), (0.25, -0.25), (0.0, 0.05)].iter().enumerate() {
            let p = fc + Vec3::new(*x, 0.12 + i as f32 * 0.07, -0.75);
            self.push(Inst::new(
                p,
                Quat::from_rotation_y(*ang),
                Vec3::new(0.9, 0.14, 0.14),
                kind::WOOD,
                [0.08, 0.04, 0.02, 1.0],
                0,
            ));
        }
        for k in 0..7 {
            let x = -0.45 + k as f32 * 0.15;
            let h = 0.25 + 0.2 * ((k as f32 * 1.7).sin() * 0.5 + 0.5);
            let mut flame = Inst::aabox(
                fc + Vec3::new(x, 0.25 + h * 0.5, -0.75),
                Vec3::new(0.09, h, 0.09),
                kind::EMISSIVE,
                [1.0, 0.42, 0.1],
            );
            flame.color[3] = 8.0;
            flame.param = 1;
            self.push(flame);
        }
        self.obstacle(fc + Vec3::new(-2.5, 0.0, -1.7), fc + Vec3::new(2.5, 3.0, 0.0), true);
        // The chimney breast, up to the ceiling.
        self.obstacle(fc + Vec3::new(-1.6, 0.0, -0.6), fc + Vec3::new(1.6, HALL_H, 0.0), true);
        self.light(
            fc + Vec3::new(0.0, 0.8, -1.6),
            11.0,
            [1.0, 0.45, 0.15],
            3.0,
            LightKind::Flicker,
        );
    }

    /// Scatter the books in piles: a few on the reading tables, the rest on the floor
    /// of their own wing. Only drop points are chosen here; `Loose` settles them.
    fn build_piles(&mut self, rng: &mut StdRng) {
        for wing in 0..2 {
            let mut pool: Vec<u32> = (0..self.books.len() as u32)
                .filter(|&b| self.dept_of(self.category_of_book(b)) == wing)
                .collect();
            pool.shuffle(rng);
            if wing == 0 {
                for (tx, tz) in [(-4.5f32, -2.6f32), (4.5, -2.6), (-4.5, 2.6), (4.5, 2.6)] {
                    let n = rng.gen_range(14..28).min(pool.len());
                    let books: Vec<u32> = pool.drain(..n).collect();
                    self.add_pile(rng, 0, Vec2::new(tx - 0.35, tz), 0.32, books);
                }
            }
            let (x0, x1) = if wing == 0 {
                (-HALL_X + 0.8, VEIL_X - 0.8)
            } else {
                (VEIL_X + 0.8, HALL_X - 0.8)
            };
            // Choose spots first, then share the wing's books evenly between them.
            let spot_r = 1.0;
            let mut spots: Vec<Vec2> = Vec::new();
            for _ in 0..20000 {
                let p = Vec2::new(
                    rng.gen_range(x0 + spot_r..x1 - spot_r),
                    rng.gen_range(-HALL_Z + 0.8 + spot_r..HALL_Z - 0.8 - spot_r),
                );
                if self.pile_spot_ok(p, spot_r, &spots) {
                    spots.push(p);
                }
            }
            let count = spots.len();
            for (i, p) in spots.into_iter().enumerate() {
                let left = count - i;
                let n = if left == 1 {
                    pool.len()
                } else {
                    ((pool.len() as f32 / left as f32) * rng.gen_range(0.75..1.25)) as usize
                };
                let n = n.min(pool.len());
                if n == 0 {
                    continue;
                }
                let radius = 0.4 + 0.06 * (n as f32).sqrt();
                let books: Vec<u32> = pool.drain(..n).collect();
                self.add_pile(rng, 0, p, radius, books);
            }
        }
    }

    fn pile_spot_ok(&self, p: Vec2, r: f32, spots: &[Vec2]) -> bool {
        for o in &self.obstacles {
            let q = Vec2::new(p.x.clamp(o.min.x, o.max.x), p.y.clamp(o.min.z, o.max.z));
            if q.distance(p) < r + 1.1 {
                return false;
            }
        }
        if p.x.abs() < 6.0 && p.y.abs() < 8.5 {
            return false; // keep the reading room's middle (and the spawn point) walkable
        }
        if p.y > HALL_Z - 3.0 && p.x.abs() < 3.5 {
            return false; // hearth
        }
        if (p.x - VEIL_X).abs() < r + 1.0 {
            return false;
        }
        !spots.iter().any(|q| q.distance(p) < r * 2.0 + 1.1)
    }
}
