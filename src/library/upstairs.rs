//! The upper floors' heaps (generation 5): each department upstairs asks something of
//! its own besides finding and filing. Their heaps are relaid here from what each room
//! was given (`scatter_books`), with numbers of their own, so that nothing below the
//! first floor moves and a save from before loads while the first floor is veiled
//! (`RELAID`):
//! - the Map Room and the Chronicle Hall, which open together, are each heaped with
//!   the other's books;
//! - the Verse Gallery holds half the Music Room's books, stacked by its door, a floor
//!   below the Music Room;
//! - the Clockworks' stopped clocks hold some of its books in the air until the series
//!   lettered on their faces are filed;
//! - the Hearth Library's books lie about the other rooms upstairs where they were
//!   read: a few on each reading table, the rest in small piles between the heaps.
//!
//! The Arcanum's ward, the Weatherhouse's draught, the folios and the Bestiary's
//! restless books are rules of play (`game`), not where books lie.

use super::*;

const SEED: u64 = 0x5550_5354_4149_5235;
/// Of a department's own books in its room, the share that goes: to the other room of
/// the swap, to the Verse Gallery (the Music Room's), about the rooms upstairs (the
/// Hearth Library's).
const SWAPPED: f32 = 0.9;
const SONGS: f32 = 0.55;
const LEFT_ABOUT: f32 = 0.85;
/// The stopped clocks by tier: the first tier's keys lie in heaps; each later tier's
/// has a volume held by a clock of the tier before, so they start again in turn.
const TIERS: [usize; 3] = [3, 3, 2];
/// About how many books a stopped clock holds; how high its face floats and its
/// radius; the shell its books hang in round its face; how close two may hang; and
/// the lowest they hang (over a head).
const HELD: usize = 250;
const FACE_Y: f32 = 2.9;
const FACE_R: f32 = 0.42;
const SHELL: (f32, f32) = (0.8, 1.55);
const APART: f32 = 0.17;
const LOWEST: f32 = 1.95;
/// The Hearth Library's books left about: how many to a reading table, the fewest to a
/// pile on the floor, and how far such a pile keeps from a heap's middle.
const ON_TABLE: (usize, usize) = (6, 14);
const LEFT_PILE: usize = 15;
const CLEAR_OF_HEAPS: f32 = 2.3;

fn xz(p: Vec3) -> Vec2 {
    Vec2::new(p.x, p.z)
}

impl Library {
    /// Relay the heaps of the rooms from `RELAID` on, given what generation 4 gave each
    /// (`pools`, by room, as units of `series_units`). Their drops are replaced.
    pub(super) fn relay_upstairs(&mut self, mut pools: Vec<Vec<Vec<u32>>>) {
        let mut rng = StdRng::seed_from_u64(SEED);
        let relaid: HashSet<u32> = pools.iter().flatten().flatten().copied().collect();
        self.drops.retain(|d| !relaid.contains(&d.0));
        let (map, chron) = (dept::MAP_ROOM, dept::CHRONICLE_HALL);
        let to_chron = self.own_units(&mut rng, &mut pools[map], map, SWAPPED);
        let to_map = self.own_units(&mut rng, &mut pools[chron], chron, SWAPPED);
        pools[map].extend(to_map);
        pools[chron].extend(to_chron);
        let music = dept::MUSIC_ROOM;
        let mut songs = self.own_units(&mut rng, &mut pools[music], music, SONGS);
        let hearth = dept::HEARTH_LIBRARY;
        let left = self.own_units(&mut rng, &mut pools[hearth], hearth, LEFT_ABOUT);
        let clocks = self.stop_clocks(&mut rng, &mut pools[dept::CLOCKWORKS]);
        let mut heaps: Vec<Vec<Vec2>> = vec![Vec::new(); pools.len()];
        for r in RELAID..pools.len() {
            let mut units = std::mem::take(&mut pools[r]);
            units.shuffle(&mut rng);
            let lv = self.rooms[r].level;
            let mut spots = self.room_spots(&mut rng, r, 0.8, 0.7);
            // Clear of the stopped clocks' books, which fall where they hang.
            spots.retain(|p| clocks.iter().all(|c| c.distance(*p) > SHELL.1 + 1.2));
            if r == dept::VERSE_GALLERY {
                // The songs stacked nearest its door, as many of its heaps as their
                // share of its books.
                let door = xz(self.rooms[r].door);
                spots.sort_by(|a, b| a.distance(door).total_cmp(&b.distance(door)));
                let n_songs: usize = songs.iter().map(Vec::len).sum();
                let n_all = n_songs + units.iter().map(Vec::len).sum::<usize>();
                let k = (spots.len() * n_songs + n_all / 2) / n_all.max(1);
                self.spread(&mut rng, lv, &spots[..k], std::mem::take(&mut songs));
                self.spread(&mut rng, lv, &spots[k..], units);
            } else {
                self.spread(&mut rng, lv, &spots, units);
            }
            heaps[r] = spots;
        }
        self.leave_about(&mut rng, left, &heaps);
    }

    /// Take out of `pool` about `share` of its units of department `d`, each by chance.
    fn own_units(&self, rng: &mut StdRng, pool: &mut Vec<Vec<u32>>, d: usize, share: f32) -> Vec<Vec<u32>> {
        let (go, stay) = (std::mem::take(pool).into_iter())
            .partition(|u: &Vec<u32>| self.dept_of_book(u[0]) == d && rng.gen_range(0.0..1.0f32) < share);
        *pool = stay;
        go
    }

    /// Pile centres in room `r` (`pile_spots`), its walls' thickness in from them.
    fn room_spots(&self, rng: &mut StdRng, r: usize, radius: f32, gap: f32) -> Vec<Vec2> {
        let room = &self.rooms[r];
        let pad = Vec2::splat(0.2);
        self.pile_spots(rng, room.level, &[(room.min + pad, room.max - pad)], radius, gap)
    }

    /// The Clockworks' stopped clocks over open floor, spread out, each holding about
    /// `HELD` of the books its room was given (whole units, never a key's) in the air
    /// round its face, its key lettered on it. Returns where they stand.
    fn stop_clocks(&mut self, rng: &mut StdRng, pool: &mut Vec<Vec<u32>>) -> Vec<Vec2> {
        let r = dept::CLOCKWORKS;
        let n: usize = TIERS.iter().sum();
        let door = xz(self.rooms[r].door);
        // Each next the spot farthest from those chosen and from the door.
        let cand = self.room_spots(rng, r, SHELL.1, 0.4);
        let mut at: Vec<Vec2> = Vec::new();
        while at.len() < n {
            let far = |p: &Vec2| (at.iter()).map(|q| q.distance(*p)).fold(p.distance(door), f32::min);
            let Some(&p) = cand.iter().max_by(|a, b| far(a).total_cmp(&far(b))) else {
                break;
            };
            at.push(p);
        }
        assert_eq!(at.len(), n, "room for {n} stopped clocks");
        // The first tier nearest the door.
        at.sort_by(|a, b| a.distance(door).total_cmp(&b.distance(door)));

        // Keys: series of three to seven volumes whose titles fit a face; past the first
        // tier, one with a volume here to hang on a clock of the tier before.
        let here: HashSet<u32> = pool.iter().flatten().copied().collect();
        let mut cands: Vec<u32> = (0..self.series.len() as u32)
            .filter(|&s| {
                let ser = &self.series[s as usize];
                self.dept_of(ser.category) == r && (3..=7).contains(&ser.volumes) && ser.title.chars().count() <= 28
            })
            .collect();
        cands.shuffle(rng);
        let mut keys: Vec<u32> = Vec::new();
        let mut locks: Vec<(usize, u32)> = Vec::new();
        let mut cands = cands.into_iter();
        for (t, &count) in TIERS.iter().enumerate() {
            let before: usize = TIERS[..t].iter().sum();
            let mut k = 0;
            while k < count {
                let s = cands.next().expect("series enough to key the clocks");
                if t == 0 {
                    keys.push(s);
                    k += 1;
                    continue;
                }
                let Some(&lock) = self.series_books[s as usize].iter().find(|b| here.contains(b)) else {
                    continue;
                };
                // Held by a clock of the tier before, in turn.
                let by = before - TIERS[t - 1] + k % TIERS[t - 1];
                locks.push((by, lock));
                keys.push(s);
                k += 1;
            }
        }
        for &(_, lock) in &locks {
            for u in pool.iter_mut() {
                u.retain(|&b| b != lock);
            }
        }
        pool.retain(|u| !u.is_empty());
        let keyed: HashSet<u32> = keys.iter().copied().collect();
        let (free, kept): (Vec<Vec<u32>>, Vec<Vec<u32>>) = (std::mem::take(pool).into_iter())
            .partition(|u| self.dept_of_book(u[0]) == r && !keyed.contains(&self.books[u[0] as usize].series));
        *pool = kept;
        let mut held: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut free = free.into_iter();
        for h in held.iter_mut() {
            while h.len() < HELD
                && let Some(u) = free.next()
            {
                h.extend(u);
            }
        }
        pool.extend(free);
        for (by, lock) in locks {
            held[by].push(lock);
        }

        let floor = self.world.levels[self.rooms[r].level].y;
        for (i, (p, books)) in at.iter().zip(held).enumerate() {
            let c = Vec3::new(p.x, floor + FACE_Y, p.y);
            let to_door = door - *p;
            let n_face = Vec3::new(to_door.x, 0.0, to_door.y).normalize();
            let title = self.series[keys[i] as usize].title.clone();
            let hands = rooms::stopped_clock(self, c, n_face, FACE_R, &title);
            let mut hung: Vec<Vec3> = Vec::new();
            for &b in &books {
                let mut best = c;
                for _ in 0..24 {
                    let d = loop {
                        let v = Vec3::new(
                            rng.gen_range(-1.0..1.0),
                            rng.gen_range(-1.0..1.0),
                            rng.gen_range(-1.0..1.0),
                        );
                        let l = v.length();
                        if l > 0.1 && l <= 1.0 {
                            break v / l;
                        }
                    };
                    let mut q = c + d * rng.gen_range(SHELL.0..SHELL.1) * Vec3::new(1.0, 0.75, 1.0);
                    if q.y < floor + LOWEST {
                        q.y = 2.0 * c.y - q.y;
                    }
                    best = q;
                    if hung.iter().all(|h| h.distance(q) >= APART) {
                        break;
                    }
                }
                hung.push(best);
                let axis = loop {
                    let v = Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                    );
                    if v.length() > 0.1 {
                        break v.normalize();
                    }
                };
                let turn = Quat::from_axis_angle(axis, rng.gen_range(0.0..TAU));
                self.frozen.push((b, best, turn));
            }
            self.stopped.push(StoppedClock {
                face: c + n_face * 0.01,
                n: n_face,
                r: FACE_R,
                key: keys[i],
                books,
                hands,
                stopped_at: rng.gen_range(0.0..12.0),
            });
        }
        at
    }

    /// The Hearth Library's books (`units`) left about the other rooms upstairs: a few
    /// on each reading table, the rest in small piles on the floor clear of the heaps
    /// (`heaps`, each room's pile centres).
    fn leave_about(&mut self, rng: &mut StdRng, units: Vec<Vec<u32>>, heaps: &[Vec<Vec2>]) {
        let rooms = RELAID..dept::HEARTH_LIBRARY;
        let mut units = units.into_iter();
        // Reading tables: pieces standing table height off the floor, broad enough.
        let mut tables: Vec<(usize, Vec2)> = Vec::new();
        for r in rooms.clone() {
            let room = &self.rooms[r];
            let floor = self.world.levels[room.level].y;
            for s in self.world.solids_on(room.level) {
                let (mn, mx) = (xz(s.min), xz(s.max));
                let top = s.max.y - floor;
                let inside = mn.cmpge(room.min).all() && mx.cmple(room.max).all();
                let broad = (mx - mn).min_element() >= 0.6;
                if inside && broad && (top - TABLE_H).abs() < 0.06 && s.stops.has(Stops::BOOKS) {
                    tables.push((r, (mn + mx) * 0.5));
                }
            }
        }
        tables.shuffle(rng);
        for (r, c) in tables {
            let want = rng.gen_range(ON_TABLE.0..=ON_TABLE.1);
            let mut pile = Vec::new();
            while pile.len() < want
                && let Some(u) = units.next()
            {
                pile.extend(u);
            }
            if pile.is_empty() {
                break;
            }
            self.add_pile(rng, self.rooms[r].level, c, 0.15, pile);
        }
        // The floor: spots clear of the heaps, the books shared out by room as the spots
        // are.
        let mut spots: Vec<Vec<Vec2>> = vec![Vec::new(); self.rooms.len()];
        for r in rooms.clone() {
            spots[r] = (self.room_spots(rng, r, 0.5, 0.5).into_iter())
                .filter(|p| heaps[r].iter().all(|h| h.distance(*p) > CLEAR_OF_HEAPS))
                .collect();
        }
        let rest: Vec<Vec<u32>> = units.collect();
        let books: usize = rest.iter().map(Vec::len).sum();
        let all_spots: usize = spots.iter().map(Vec::len).sum();
        // No pile under `LEFT_PILE`: fewer spots if need be.
        let use_spots = all_spots.min(books / LEFT_PILE).max(1);
        let mut rest = rest.into_iter();
        let mut given = 0;
        let mut spots_given = 0;
        for r in rooms {
            let mut mine = std::mem::take(&mut spots[r]);
            mine.shuffle(rng);
            mine.truncate((mine.len() * use_spots).div_ceil(all_spots.max(1)));
            spots_given += mine.len();
            let share = books * spots_given / use_spots.max(1);
            let mut room_units = Vec::new();
            while given < share
                && let Some(u) = rest.next()
            {
                given += u.len();
                room_units.push(u);
            }
            if !mine.is_empty() {
                self.spread(rng, self.rooms[r].level, &mine, room_units);
            }
        }
        debug_assert!(rest.next().is_none(), "the Hearth Library's books all left somewhere");
    }
}
