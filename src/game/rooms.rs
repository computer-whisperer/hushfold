//! What some of the Great Library's rooms ask of play besides finding and filing
//! (where their books lie is `library::upstairs`'s):
//! - the Arcanum's ward quiets magic inside it, giving it back a step at a time as the
//!   room is put in order: familiars at a quarter shelved, spells and Homing Toss at
//!   half, the Shelving Aura at three quarters. Three strands of light across its
//!   doorway go out one by one as it does;
//! - the Clockworks' stopped clocks start again when the series on their faces are
//!   filed whole, and let fall the books they held;
//! - a draught blows through the Weatherhouse, gusting, the way its vane's arrow points
//!   from: thrown books drift with it and familiars fly slower against it;
//! - the Bestiary's restless books, once it opens, scurry from anyone walking near (stand
//!   still to take them).
//!
//! Each room says what it asks the first time you step into it in a session.

use super::*;

/// The Arcanum's ward lifts a step at each of these shares of its books shelved.
const WARD: [f32; 3] = [0.25, 0.5, 0.75];

/// What the Arcanum's ward quiets, in the order it gives them back.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Magic {
    /// Posting familiars, and their taking or filing books inside.
    Familiars,
    /// Casting, charms and summons reaching inside, Shelving Rush and Homing Toss.
    Spells,
    /// The Shelving Aura.
    Aura,
}

/// The Weatherhouse's draught: its push on a thrown book (m/s²) at its mean, and how
/// far gusts swell and slacken it; familiars flying straight into it keep this share
/// of their speed.
const DRAUGHT: f32 = 2.6;
const GUSTS: f32 = 0.6;
const HEADWIND: f32 = 0.65;

/// Restless books: a loose one within `STARTLE` of a player walking at least
/// `STARTLE_SPEED` hops `HOP` metres away from them, rests a moment, and after
/// `HOPS` hops settles for the session. A hop rises `HOP_UP`.
const STARTLE: f32 = 2.2;
pub(super) const STARTLE_SPEED: f32 = 0.6;
const HOP: (f32, f32) = (1.5, 2.5);
const HOP_UP: f32 = 0.45;
const HOP_REST: f32 = 1.5;
const HOPS: u8 = 3;
/// The turns off straight away a hop tries, in order, and how high off its floor it
/// may land (on books, not on furniture).
const HOP_TURNS: [f32; 5] = [0.0, 0.6, -0.6, 1.2, -1.2];
const HOP_LANDS: f32 = 1.2;

impl Game {
    // ---------------------------------------------------------------- the ward

    /// The share of department `d`'s books shelved.
    fn share_shelved(&self, d: usize) -> f32 {
        self.dept_shelved[d] as f32 / self.dept_total[d].max(1) as f32
    }

    /// The strands of the Arcanum's ward still holding (three to none; none outside
    /// the Great Library).
    pub fn ward_strands(&self) -> usize {
        if self.lib.layout != Layout::Great {
            return 0;
        }
        let share = self.share_shelved(dept::ARCANUM);
        WARD.iter().filter(|&&w| share < w).count()
    }

    /// Whether the Arcanum's ward quiets `magic` at `p` (anywhere in the room). Asked
    /// of many books at a time (familiars looking for work), so it is a box test.
    pub fn quieted(&self, magic: Magic, p: Vec3) -> bool {
        let lifted = WARD.len() - self.ward_strands();
        (magic as usize) >= lifted && self.inside(dept::ARCANUM, p)
    }

    /// Whether `p` is in the Great Library's room `d`: a box test (its rectangle, from
    /// its floor to its roof), cheaper than `Library::room_at`.
    fn inside(&self, d: usize, p: Vec3) -> bool {
        let room = &self.lib.rooms[d];
        let level = &self.lib.world.levels[room.level];
        let q = Vec2::new(p.x, p.z);
        (level.y - 0.3..level.roof).contains(&p.y) && q.cmpge(room.min).all() && q.cmplt(room.max).all()
    }

    /// Just outside the Arcanum's doorway, on the gallery: where couriers leave its
    /// books while the ward keeps them out. Worked out once (`threshold`).
    pub(super) fn find_threshold(lib: &Library, nav: &Nav) -> Vec3 {
        if lib.layout != Layout::Great {
            return Vec3::ZERO;
        }
        let door = lib.rooms[dept::ARCANUM].door;
        let out = Vec3::new(door.x, 0.0, door.z).normalize();
        nav.standable_near(door - out * 2.3)
    }

    /// The ward's strands across the Arcanum's doorway, while it is open and they hold.
    fn draw_ward(&self, out: &mut Vec<Inst>) {
        let strands = self.ward_strands();
        if strands == 0 || !self.dept_open(dept::ARCANUM) {
            return;
        }
        let door = self.lib.rooms[dept::ARCANUM].door;
        let outward = Vec3::new(door.x, 0.0, door.z).normalize();
        let across = Vec3::Y.cross(outward);
        let rot = Quat::from_mat3(&glam::Mat3::from_cols(across, Vec3::Y, across.cross(Vec3::Y)));
        let at = door - outward * 0.5;
        for k in 0..strands {
            let pulse = 0.5 + 0.5 * libm::sinf(self.time * 1.7 + k as f32 * 2.1);
            out.push(Inst::new(
                at + Vec3::Y * (0.9 + 0.9 * k as f32),
                rot,
                Vec3::new(2.9, 0.022, 0.022),
                kind::EMISSIVE,
                [0.62, 0.36, 1.0, 1.6 + 1.6 * pulse],
                0,
            ));
        }
    }

    // ---------------------------------------------------------------- stopped clocks

    /// Start again every stopped clock in the Clockworks whose key series is filed
    /// whole: the books it held fall from where they hung. Returns those started.
    pub(super) fn start_clocks(&mut self) -> Vec<usize> {
        let mut started = Vec::new();
        for k in 0..self.lib.stopped.len() {
            let key = self.lib.stopped[k].key as usize;
            if self.started[k].is_some() || self.series_shelved[key] < self.lib.series[key].volumes {
                continue;
            }
            self.started[k] = Some(self.clock);
            started.push(k);
            let face = self.lib.stopped[k].face;
            for i in 0..self.lib.stopped[k].books.len() {
                let b = self.lib.stopped[k].books[i];
                if self.state[b as usize] != BookState::Loose || !self.loose.is_frozen(b) {
                    continue;
                }
                let mut j = || self.rng.gen_range(-1.0..1.0f32);
                let (push, spin) = (Vec3::new(j(), j(), j()), Vec3::new(j(), j(), j()));
                let out = (self.loose.pose(b).0 - face).normalize_or_zero();
                self.loose.thaw(b, out * 0.6 + push * 0.3, spin * 4.0);
            }
        }
        started
    }

    /// The stopped clocks' hands, still or keeping time since they started again, and
    /// a haze of stillness round each while it holds its books.
    fn draw_clocks(&self, out: &mut Vec<Inst>, trans: &mut Vec<Inst>) {
        for (k, c) in self.lib.stopped.iter().enumerate() {
            let since = self.started[k].map_or(0.0, |t| (self.clock - t) as f32);
            let hours = c.stopped_at + since / 3600.0;
            for (hand, turns) in c.hands.iter().zip([hours / 12.0, hours]) {
                let turn = Quat::from_axis_angle(c.n, -(turns - libm::floorf(turns)) * std::f32::consts::TAU);
                let pose = motion::Pose {
                    turn,
                    shift: c.face - turn * c.face,
                };
                out.push(pose.inst(hand));
            }
            let haze = 1.0 - (since / 2.0).min(1.0);
            if haze > 0.0 {
                trans.push(Inst::new(
                    c.face,
                    Quat::IDENTITY,
                    Vec3::new(3.4, 2.5, 3.4),
                    kind::MIST,
                    [0.55, 0.7, 1.0, 0.3 * haze],
                    1,
                ));
            }
        }
    }

    // ---------------------------------------------------------------- the draught

    /// The Weatherhouse's draught now (Great Library only): which way it blows on the
    /// level, and how hard (m/s²), gusting. Its vane's arrow points into it.
    pub(super) fn draught(&self) -> Option<Vec2> {
        let (m, rest) = self.lib.vane?;
        let arrow = motion::pose_of(&self.lib.movers, m, self.clock).turn * rest;
        let from = Vec2::new(arrow.x, arrow.z).normalize_or_zero();
        let t = self.time;
        let gust = 1.0 + GUSTS * (0.6 * libm::sinf(t * 0.73) + 0.4 * libm::sinf(t * 1.9 + 1.3));
        Some(-from * DRAUGHT * gust)
    }

    /// Hand the loose books the draught, where it blows.
    pub(super) fn blow(&mut self) {
        let wind = self.draught().map(|push| {
            let room = &self.lib.rooms[dept::WEATHERHOUSE];
            (room.level, room.min, room.max, push)
        });
        self.loose.set_wind(wind);
    }

    /// The share of its speed a familiar flying along `dir` at `p` keeps against the
    /// draught.
    pub(super) fn headwind(&self, p: Vec3, dir: Vec3) -> f32 {
        if self.lib.vane.is_none() || !self.inside(dept::WEATHERHOUSE, p) {
            return 1.0;
        }
        match self.draught() {
            Some(push) => {
                let into = -Vec2::new(dir.x, dir.z)
                    .normalize_or_zero()
                    .dot(push.normalize_or_zero());
                1.0 - (1.0 - HEADWIND) * into.max(0.0)
            }
            _ => 1.0,
        }
    }

    // ---------------------------------------------------------------- restless books

    /// Restless books near `at` (on its level) hop away from it, as from someone
    /// walking there: those not resting from a hop, and not yet settled.
    pub(super) fn startle(&mut self, at: Vec3) {
        if self.lib.layout != Layout::Great || !self.dept_open(dept::BESTIARY) {
            return;
        }
        let lv = self.lib.world.level_at(at);
        let lib = &self.lib;
        let near = self
            .loose
            .near(lv, Vec2::new(at.x, at.z), STARTLE, |b| lib.is_restless(b));
        let mut hopped = false;
        for b in near {
            let (hops, rest) = self.wary.get(&b).copied().unwrap_or((0, 0.0));
            if hops >= HOPS || self.time < rest || self.state[b as usize] != BookState::Loose {
                continue;
            }
            self.wary.insert(b, (hops + 1, self.time + HOP_REST));
            let (from, turn) = self.loose.pose(b);
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            let (jit, spin) = (Vec2::new(j(), j()) * 0.5, Vec3::new(j(), j(), j()) * 3.0);
            let away = (Vec2::new(from.x - at.x, from.z - at.z).normalize_or(Vec2::X) + jit).normalize_or(Vec2::X);
            let dist = self.rng.gen_range(HOP.0..HOP.1);
            let up = libm::sqrtf(2.0 * crate::loose::GRAVITY * HOP_UP);
            let flight = 2.0 * up / crate::loose::GRAVITY;
            // Away, or as near away as lands on open floor of its own level (never on
            // furniture or a wall's top); with nowhere to go, it is cornered and stays.
            let start = from + Vec3::Y * 0.05;
            let floor = self.lib.world.levels[lv].y;
            let mut arc = Vec::new();
            let hop = HOP_TURNS.iter().find_map(|&off| {
                let dir = Vec2::from_angle(off).rotate(away);
                let vel = Vec3::new(dir.x, 0.0, dir.y) * (dist / flight) + Vec3::Y * up;
                let land = self.loose.predict(start, vel, &mut arc)?;
                let open = self.lib.world.level_at(land) == lv
                    && land.y - floor < HOP_LANDS
                    && self.nav.open_at(Vec3::new(land.x, floor, land.z));
                open.then_some(vel)
            });
            let Some(vel) = hop else { continue };
            self.loose.take(&[b]);
            self.loose.throw(b, start, vel, turn, spin, 0.0);
            hopped = true;
        }
        if hopped && !self.told_restless {
            self.told_restless = true;
            self.hint = Some((
                "A restless book! They scurry from anyone walking near — stand still to take them.".into(),
                5.0,
            ));
        }
    }

    // ---------------------------------------------------------------- first steps in

    /// What room `r` asks of play, said the first time you step in this session (while
    /// it still asks it).
    fn room_word(&self, r: usize) -> Option<String> {
        if self.lib.layout != Layout::Great {
            return None;
        }
        let word = match r {
            dept::MAP_ROOM => {
                "The cartographers and the chroniclers swapped their stacks: most heaps here are the Chronicle Hall's."
            }
            dept::CHRONICLE_HALL => {
                "The chroniclers and the cartographers swapped their stacks: most heaps here are the Map Room's."
            }
            dept::VERSE_GALLERY => "The heaps by the door are songs, waiting for the Music Room a floor up.",
            dept::ARCANUM if self.ward_strands() > 0 => {
                "The Arcanum's ward quiets magic here. It weakens as the room fills: familiars back at a quarter, spells at half, the Aura at three quarters."
            }
            dept::CLOCKWORKS if self.started.iter().any(Option::is_none) => {
                "Time stopped here. Each clock starts again when the series on its face is filed whole — aim at a clock to read it."
            }
            dept::WEATHERHOUSE => {
                "A draught blows through the Weatherhouse: what you toss drifts with it. The vane points where it comes from."
            }
            dept::HEARTH_LIBRARY => {
                "Its readers left the Hearth Library's books wherever they were reading, in the rooms upstairs."
            }
            _ => return None,
        };
        Some(word.into())
    }

    /// Say what the room just stepped into asks, the first time this session.
    pub(super) fn step_in(&mut self) {
        let room = self.lib.room_at(self.player.pos);
        if room == self.in_room {
            return;
        }
        self.in_room = room;
        let Some(r) = room else { return };
        if self.told_rooms & (1 << r) != 0 {
            return;
        }
        if let Some(word) = self.room_word(r) {
            self.told_rooms |= 1 << r;
            self.hint = Some((word, 7.0));
        }
    }

    /// The rooms' own things to draw each frame: the ward's strands, the clocks.
    pub(super) fn draw_rooms(&self, out: &mut Vec<Inst>, trans: &mut Vec<Inst>) {
        self.draw_ward(out);
        self.draw_clocks(out, trans);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn great() -> (Game, Audio) {
        (Game::new(Layout::Great), Audio::silent())
    }

    /// Shelve `n` loose books of department `d` (opening what that opens).
    fn shelve(g: &mut Game, d: usize, n: usize, audio: &mut Audio) {
        let books: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.dept_of_book(b) == d && g.loose.is_resting(b))
            .take(n)
            .collect();
        g.loose.take(&books);
        for b in books {
            g.credit_shelved(b);
        }
        g.check_gates(audio);
    }

    #[test]
    fn folios_take_three_places() {
        let (mut g, mut audio) = great();
        let n = g.lib.books.len() as u32;
        let swapped = |b: u32| matches!(g.lib.dept_of_book(b), dept::MAP_ROOM | dept::CHRONICLE_HALL);
        let folios = (0..n).filter(|&b| g.lib.is_folio(b)).count();
        let of = (0..n).filter(|&b| swapped(b)).count();
        assert!((0..n).all(|b| !g.lib.is_folio(b) || swapped(b)));
        let share = folios as f32 / of as f32;
        assert!((0.05..0.2).contains(&share), "{share:.2} of their books are folios");
        let folio = (0..n).find(|&b| g.lib.is_folio(b) && g.loose.is_resting(b)).unwrap();
        g.loose.take(&[folio]);
        g.take_into_arms(&[folio], &mut audio);
        assert_eq!(g.carried(), FOLIO);
        assert_eq!(g.room_in_arms(), g.capacity() - FOLIO);
    }

    #[test]
    fn the_ward_lifts_in_steps() {
        let (mut g, mut audio) = great();
        g.open_all_gates();
        let inside = g.lib.rooms[dept::ARCANUM].drop;
        let outside = g.threshold;
        // Couriers leave its books on the gallery by its door.
        let door = g.lib.rooms[dept::ARCANUM].door;
        assert_eq!(g.lib.world.level_at(outside), g.lib.world.level_at(door), "{outside}");
        assert!(
            (outside.y - door.y).abs() < 0.05 && outside.distance(door) < 3.5,
            "{outside}"
        );
        assert!(g.nav.open_at(outside));
        assert_eq!(g.ward_strands(), 3);
        for m in [Magic::Familiars, Magic::Spells, Magic::Aura] {
            assert!(g.quieted(m, inside), "{m:?}");
            assert!(!g.quieted(m, outside), "{m:?} outside");
        }
        let total = g.dept_total[dept::ARCANUM] as usize;
        shelve(&mut g, dept::ARCANUM, total / 4 + 1, &mut audio);
        assert!(!g.quieted(Magic::Familiars, inside) && g.quieted(Magic::Spells, inside));
        shelve(&mut g, dept::ARCANUM, total / 4 + 1, &mut audio);
        assert!(!g.quieted(Magic::Spells, inside) && g.quieted(Magic::Aura, inside));
        shelve(&mut g, dept::ARCANUM, total / 4 + 1, &mut audio);
        assert_eq!(g.ward_strands(), 0);
        assert!(!g.quieted(Magic::Aura, inside));
    }

    #[test]
    fn a_stopped_clock_starts_when_its_key_is_filed() {
        let (mut g, mut audio) = great();
        g.open_all_gates();
        let (key, held) = (g.lib.stopped[0].key, g.lib.stopped[0].books.clone());
        assert!(held.iter().all(|&b| g.loose.is_frozen(b)));
        let vols: Vec<u32> = g.lib.series_books[key as usize].clone();
        assert!(
            vols.iter().all(|&b| !g.loose.is_frozen(b)),
            "a first clock's key lies in heaps"
        );
        let mut thunks = 0;
        for &b in &vols {
            g.loose.take(&[b]);
            g.on_shelved(b, &mut audio, &mut thunks);
        }
        assert!(g.started[0].is_some());
        assert!(held.iter().all(|&b| !g.loose.is_frozen(b)));
        for _ in 0..300 {
            let landings = g.loose.update(1.0 / 60.0, &mut g.pos, &mut g.rot);
            assert!(landings.len() <= held.len());
        }
        assert!(
            held.iter().all(|&b| g.loose.is_resting(b)),
            "what it held lies on the floor"
        );
        let floor = g.lib.world.levels[g.lib.rooms[dept::CLOCKWORKS].level].y;
        assert!(held.iter().all(|&b| g.loose.pose(b).0.y < floor + 1.5));
    }

    /// Every clock's key can be filed: each later clock's has a volume held by a clock
    /// of the tier before, never by itself or a later one.
    #[test]
    fn the_clocks_start_in_turn() {
        let (g, _) = great();
        let n = g.lib.stopped.len();
        assert_eq!(n, 8);
        let mut started = vec![false; n];
        loop {
            let held_by_stopped = |b: u32| (0..n).any(|k| !started[k] && g.lib.stopped[k].books.contains(&b));
            let next = (0..n).find(|&k| {
                !started[k]
                    && !g.lib.series_books[g.lib.stopped[k].key as usize]
                        .iter()
                        .any(|&b| held_by_stopped(b))
            });
            match next {
                Some(k) => started[k] = true,
                None => break,
            }
        }
        assert!(started.iter().all(|&s| s), "{started:?}");
    }

    #[test]
    fn the_draught_bends_throws_in_the_weatherhouse() {
        let (mut g, _) = great();
        g.open_all_gates();
        g.blow();
        let push = g.draught().unwrap();
        assert!(push.length() > 0.5);
        let room = &g.lib.rooms[dept::WEATHERHOUSE];
        let floor = g.lib.world.levels[room.level].y;
        let mid = (room.min + room.max) * 0.5;
        let across = Vec3::new(-push.y, 0.0, push.x).normalize();
        let from = Vec3::new(mid.x, floor + 1.4, mid.y) - across * 3.0;
        let mut arc = Vec::new();
        let land = g.loose.predict(from, across * 6.0 + Vec3::Y * 2.0, &mut arc).unwrap();
        let drift = Vec2::new(land.x - from.x, land.z - from.z).dot(push.normalize());
        assert!(drift > 0.3, "drifted {drift:.2} m downwind");
        // Outside it, none.
        let atrium = Vec3::new(0.0, 1.4, 5.0);
        let land = g
            .loose
            .predict(atrium, Vec3::X * 6.0 + Vec3::Y * 2.0, &mut arc)
            .unwrap();
        assert!((land.z - 5.0).abs() < 0.01);
    }

    #[test]
    fn restless_books_scurry_from_a_walker() {
        let (mut g, _) = great();
        g.open_all_gates();
        let b = (0..g.lib.books.len() as u32)
            .find(|&b| g.lib.is_restless(b) && g.loose.is_resting(b))
            .unwrap();
        let at = g.loose.pose(b).0;
        g.startle(at + Vec3::X * 1.0);
        assert!(!g.loose.is_resting(b), "it hops");
        for _ in 0..120 {
            g.loose.update(1.0 / 60.0, &mut g.pos, &mut g.rot);
        }
        let now = g.loose.pose(b).0;
        let moved = Vec2::new(now.x - at.x, now.z - at.z).length();
        assert!(moved > 0.8, "moved {moved:.2} m");
        // A book that is not restless stays.
        let calm = (0..g.lib.books.len() as u32)
            .find(|&c| !g.lib.is_restless(c) && g.loose.is_resting(c) && g.loose.pose(c).0.distance(now) < 3.0);
        if let Some(c) = calm {
            g.startle(g.loose.pose(c).0);
            assert!(g.loose.is_resting(c));
        }
    }
}
