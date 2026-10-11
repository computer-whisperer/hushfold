//! Spells: bought in the Grimoire like any skill, cast from a hotbar (keys 1–5),
//! each with its own recharge. Glimpse lights every carried series for a while; the
//! Gathering Charm sends a heap streaming to the hub along the walking routes; Echo
//! marks the heaps holding a kind of book; Summon Series calls a series' loose
//! volumes to the arms; Shelving Rush files carried books near their slots as fast
//! as you pass.

use super::*;
use std::rc::Rc;

/// The spells, in hotbar order: key 1 casts the first.
pub const SPELLS: [Upg; 5] = [Upg::Glimpse, Upg::Gathering, Upg::Echo, Upg::Summon, Upg::Rush];

/// A spell at a level: (seconds its effect lasts, seconds until it can be cast again).
pub fn spell_times(u: Upg, level: u32) -> (f32, f32) {
    match (u, level) {
        (Upg::Glimpse, 1) => (10.0, 45.0),
        (Upg::Glimpse, _) => (15.0, 30.0),
        (Upg::Gathering, l) => (0.0, GATHERING[l.clamp(1, 3) as usize - 1].2),
        (Upg::Echo, 1) => (20.0, 60.0),
        (Upg::Echo, _) => (30.0, 45.0),
        (Upg::Summon, 1) => (0.0, 90.0),
        (Upg::Summon, 2) => (0.0, 75.0),
        (Upg::Summon, _) => (0.0, 60.0),
        (Upg::Rush, 1) => (10.0, 60.0),
        (Upg::Rush, _) => (15.0, 45.0),
        _ => (0.0, 0.0),
    }
}

/// How far from its slot a carried book flies home while Shelving Rush lasts, by
/// level, and how many books a second it sends.
pub(super) fn rush_reach(level: u32) -> f32 {
    if level >= 2 { 8.0 } else { 6.0 }
}
pub(super) const RUSH_RATE: f32 = 30.0;

/// How far Echo rings out, by level.
fn echo_reach(level: u32) -> f32 {
    if level >= 2 { 50.0 } else { 25.0 }
}
/// How fast Echo's ring sweeps out (m/s).
pub const ECHO_SPEED: f32 = 30.0;

/// Gathering Charm by level: (how far round the aim it takes books, most books, recharge
/// in seconds).
const GATHERING: [(f32, usize, f32); 3] = [(2.5, 150, 120.0), (3.5, 300, 90.0), (5.0, 600, 60.0)];
/// How far off a heap can be charmed.
const CHARM_REACH: f32 = 14.0;
/// Books this near the hub (on its floor) are already there.
pub(super) const HUB_R: f32 = 4.0;
/// Charmed books fly at this height over the floor, and this fast (m/s).
const STREAM_H: f32 = 1.9;
const STREAM_SPEED: f32 = 9.0;
/// Seconds a charmed book takes to rise from the heap to the stream, and over which
/// a heap's books set off.
const RISE: f32 = 0.6;
const SET_OFF: f32 = 2.5;
/// How far a charmed book flies beside the stream's line, at most.
const RIBBON: f32 = 0.25;
/// Height over the hub the stream ends at, where books drop onto the heap.
const DROP_H: f32 = 2.2;

/// A book a charm carries: it rises from where it lay to the start of the way, flies
/// along it beside the line, and drops at its end.
pub(super) struct Stream {
    book: u32,
    /// The way, in order, from over the heap to over the hub.
    way: Rc<Vec<Vec3>>,
    /// Seconds since it set off (negative while it waits its turn).
    t: f32,
    /// Where it lay.
    from: (Vec3, Quat),
    /// Beside the line by this much.
    off: Vec3,
    spin: Vec3,
}

impl Game {
    /// Cast the spell in hotbar slot `slot`, if it is learned and has recharged.
    pub(super) fn cast(&mut self, slot: usize, audio: &mut Audio) {
        let Some(&u) = SPELLS.get(slot) else { return };
        let (lvl, name) = (self.level(u), upg_def(u).name);
        if lvl == 0 {
            self.hint = Some((format!("{name} is learned in the Grimoire (Tab)."), 2.0));
            return;
        }
        if self.quieted(Magic::Spells, self.player.pos) {
            audio.play(Sfx::Nope, 0.3);
            self.hint = Some((
                "The Arcanum's ward quiets spells here until the room is half in order.".into(),
                2.5,
            ));
            return;
        }
        let wait = self.spell_wait[slot];
        if wait > 0.0 {
            audio.play(Sfx::Nope, 0.3);
            self.hint = Some((format!("{name} returns in {:.0} s.", wait.ceil()), 1.5));
            return;
        }
        let cast = match u {
            Upg::Gathering => self.gather_to_hub(lvl),
            Upg::Echo => self.echo(lvl),
            Upg::Summon => self.summon(lvl, audio),
            _ => true,
        };
        if !cast {
            audio.play(Sfx::Nope, 0.3);
            return;
        }
        let (lasts, recharge) = spell_times(u, lvl);
        self.spell_left[slot] = lasts;
        self.spell_wait[slot] = recharge;
        audio.play_pitched(Sfx::Spell, 0.5, slot);
    }

    /// Seconds since Echo rang out, while its marks last.
    pub fn echo_age(&self) -> Option<f32> {
        let slot = SPELLS.iter().position(|&u| u == Upg::Echo)?;
        let lasts = spell_times(Upg::Echo, self.level(Upg::Echo)).0;
        (self.spell_left[slot] > 0.0).then(|| lasts - self.spell_left[slot])
    }

    /// Spell `u`'s effect is running.
    pub fn spell_on(&self, u: Upg) -> bool {
        SPELLS
            .iter()
            .position(|&x| x == u)
            .is_some_and(|i| self.spell_left[i] > 0.0)
    }

    /// Echo: mark the heaps within reach holding books of the kind sought, or else of
    /// the active book's kind; nearby marks merge. False, with a hint, if there is no
    /// kind to listen for.
    fn echo(&mut self, lvl: u32) -> bool {
        let kind = self
            .sought()
            .or_else(|| self.active_book().map(|b| (self.group, self.group_of(b))));
        let Some((group, key)) = kind else {
            self.hint = Some(("Seek a kind first (F at a book or a shelf), or hold one.".into(), 2.0));
            return false;
        };
        let (eye, reach) = (self.player.eye(), echo_reach(lvl));
        let mut marks: Vec<(Vec3, u32)> = Vec::new();
        for (_, c, books) in self.loose.piles() {
            if c.distance(eye) > reach {
                continue;
            }
            let kin: Vec<Vec3> = (books.iter())
                .filter(|&&b| group.of(&self.lib, b) == key)
                .map(|&b| self.loose.pose(b).0)
                .collect();
            let Some(top) = kin.iter().copied().max_by(|a, b| a.y.total_cmp(&b.y)) else {
                continue;
            };
            let n = kin.len() as u32;
            match marks.iter_mut().find(|(p, _)| p.distance(top) < 3.0) {
                Some(m) => {
                    m.1 += n;
                    m.0.y = m.0.y.max(top.y + 0.3);
                }
                None => marks.push((top + Vec3::Y * 0.3, n)),
            }
        }
        if marks.is_empty() {
            self.hint = Some((format!("No books of that kind lie within {reach:.0} m."), 2.5));
        }
        self.echo = marks;
        self.echo_from = self.player.pos;
        self.echo_kind = Some((group, key));
        true
    }

    /// Summon Series: loose volumes of the series sought (sorted by series), else of
    /// the active book's, in reach (nearby, the floor, anywhere by level), nearest
    /// first, dissolve where they lie and fly to the arms from beside you. Seeking
    /// calls a series with no volume to hand, as when its last lie out of reach.
    /// False, with a hint, if none can come.
    pub(super) fn summon(&mut self, lvl: u32, audio: &mut Audio) -> bool {
        let sought = match self.sought() {
            Some((Group::Series, s)) => Some(s as u32),
            _ => None,
        };
        let held = self.active_book().map(|b| self.lib.books[b as usize].series);
        let Some(series) = sought.or(held) else {
            let seek = if self.level(Upg::Fine) > 0 {
                "sort by series (R) and seek it (F)"
            } else {
                "seek it by series (Fine Fingers)"
            };
            self.hint = Some((
                format!("Hold a volume of the series, or {seek}, to summon the rest."),
                3.0,
            ));
            return false;
        };
        let room = self.room_in_arms();
        if room == 0 {
            self.hint = Some(("Your arms are full — file some books first.".into(), 2.0));
            return false;
        }
        let mut books = self.summonable(series, lvl);
        if books.is_empty() {
            self.hint = Some(("No loose volumes of that series lie in reach.".into(), 2.0));
            return false;
        }
        let mut used = 0;
        books.retain(|&b| {
            used += self.weight(b);
            used <= room
        });
        if books.is_empty() {
            self.hint = Some((
                format!("A folio takes {FOLIO} places in your arms — file some books first."),
                2.0,
            ));
            return false;
        }
        let color = self.lib.series[series as usize].color;
        let eye = self.player.eye();
        for &b in &books {
            self.burst(self.pos[b as usize], 6, color, 0.6);
        }
        self.loose.take(&books);
        for &b in &books {
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            self.pos[b as usize] = eye + self.player.forward() * 0.8 + Vec3::new(j(), j() * 0.4, j()) * 0.6;
            self.burst(self.pos[b as usize], 3, color, 0.4);
        }
        self.take_into_arms(&books, audio);
        true
    }

    /// The loose volumes of `series` Summon Series of level `lvl` reaches from where
    /// the player stands, nearest first: at I those in the room you are in (in the
    /// atrium, on its floor of the atrium), at II on your floor, at III anywhere.
    pub(super) fn summonable(&self, series: u32, lvl: u32) -> Vec<u32> {
        let here = self.player.pos;
        let (floor, room) = (self.lib.world.level_at(here), self.lib.room_at(here));
        let mut books: Vec<u32> = (self.lib.series_books[series as usize].iter().copied())
            .filter(|&b| self.loose.is_resting(b) && !self.quieted(Magic::Spells, self.loose.pose(b).0))
            .filter(|&b| match lvl {
                1 => {
                    self.lib.room_at(self.loose.pose(b).0) == room
                        && (room.is_some() || self.loose.level_of(b) == floor)
                }
                2 => self.loose.level_of(b) == floor,
                _ => true,
            })
            .collect();
        books.sort_by(|&a, &b| {
            let d = |x: u32| self.loose.pose(x).0.distance_squared(here);
            d(a).total_cmp(&d(b))
        });
        books
    }

    /// Recharges and effects run down.
    pub(super) fn update_spells(&mut self, dt: f32) {
        for t in self.spell_wait.iter_mut().chain(&mut self.spell_left) {
            *t = (*t - dt).max(0.0);
        }
    }

    /// The Gathering Charm at the heap aimed at (within a charm's reach): its books
    /// round the aim (only the group sought, while seeking), short of those already
    /// at the hub, stream there. False, with a hint, if there is nothing to send.
    fn gather_to_hub(&mut self, lvl: u32) -> bool {
        let (o, d) = (self.player.eye(), self.player.forward());
        let block = self.lib.world.ray(o, d, CHARM_REACH, Stops::SIGHT);
        let Some((book, t)) = self.loose.pick(o, d, CHARM_REACH.min(block + 0.3)) else {
            self.hint = Some(("Aim at a heap to send it to the hub.".into(), 2.0));
            return false;
        };
        self.charm_heap(self.loose.level_of(book), o + d * t, lvl)
    }

    /// The books a charm of level `lvl` at `at` on level `lv` would send: round it
    /// (only the group sought, while seeking), short of those already at the hub,
    /// nearest first, as many as it moves.
    pub(super) fn charmed(&mut self, lv: usize, at: Vec3, lvl: u32) -> Vec<u32> {
        let (radius, most, _) = GATHERING[lvl as usize - 1];
        let hub = self.lib.hub;
        let hub_lv = self.lib.world.level_at(hub);
        let (lib, sought) = (&self.lib, self.sought());
        let keep = |b: u32| sought.is_none_or(|(group, key)| group.of(lib, b) == key);
        let mut books = self.loose.near(lv, Vec2::new(at.x, at.z), radius, keep);
        books.retain(|&b| {
            let p = self.loose.pose(b).0;
            let home = lv == hub_lv && self.at_hub(p);
            !home && !self.quieted(Magic::Spells, p)
        });
        books.sort_by(|&a, &b| {
            let d = |x: u32| self.loose.pose(x).0.distance_squared(at);
            d(a).total_cmp(&d(b))
        });
        books.truncate(most);
        books
    }

    /// Within the hub's reach, on its floor's height.
    fn at_hub(&self, p: Vec3) -> bool {
        let hub = self.lib.hub;
        (p.y - hub.y).abs() < 1.5 && Vec2::new(p.x - hub.x, p.z - hub.z).length() < HUB_R
    }

    /// Send the heap round `at` on level `lv` to the hub, as a charm of level `lvl`
    /// does; false, with a hint, if nothing goes.
    pub(super) fn charm_heap(&mut self, lv: usize, at: Vec3, lvl: u32) -> bool {
        let hub = self.lib.hub;
        let books = self.charmed(lv, at, lvl);
        if books.is_empty() {
            let msg = if self.at_hub(at) {
                "Those books are at the hub already."
            } else if self.quieted(Magic::Spells, at) {
                "The Arcanum's ward holds those books until the room is half in order."
            } else {
                "Nothing there to send."
            };
            self.hint = Some((msg.into(), 2.0));
            return false;
        }
        let floor = self.lib.world.ground(Vec2::new(at.x, at.z), at.y + 0.3).unwrap_or(at.y);
        let start = Vec3::new(at.x, floor + STREAM_H, at.z);
        self.nav.fill(&mut self.hub_field, hub);
        let Some(mut way) = self.plan(&self.hub_field, start, hub + Vec3::Y * DROP_H) else {
            self.hint = Some(("No way from here to the hub.".into(), 2.0));
            return false;
        };
        way.push(start);
        way.reverse();
        let way = Rc::new(way);
        self.loose.take(&books);
        let n = books.len();
        for (k, &b) in books.iter().enumerate() {
            let mut j = || self.rng.gen_range(-1.0..1.0f32);
            let off = Vec3::new(j(), j() * 0.6, j()) * RIBBON;
            let spin = Vec3::new(j(), j(), j()) * 2.5;
            self.state[b as usize] = BookState::Streaming;
            self.streams.push(Stream {
                book: b,
                way: way.clone(),
                t: -(k as f32 / n as f32) * SET_OFF,
                from: (self.pos[b as usize], self.rot[b as usize]),
                off,
                spin,
            });
        }
        self.dirty_save = true;
        true
    }

    /// Move charmed books along their ways; those at the end drop onto the hub's heap.
    pub(super) fn update_streams(&mut self, dt: f32) {
        let mut k = 0;
        while k < self.streams.len() {
            let s = &mut self.streams[k];
            s.t += dt;
            let b = s.book as usize;
            if s.t < 0.0 {
                k += 1;
                continue;
            }
            let way = &s.way;
            let rot = s.from.1 * Quat::from_scaled_axis(s.spin * s.t);
            if s.t < RISE {
                let e = s.t / RISE;
                let e = e * e * (3.0 - 2.0 * e);
                self.pos[b] = s.from.0.lerp(way[0] + s.off, e);
                self.rot[b] = rot;
                k += 1;
                continue;
            }
            // Along the way: `left` metres from the start of segment `seg`.
            let mut left = (s.t - RISE) * STREAM_SPEED;
            let mut seg = 0;
            while seg + 1 < way.len() && left > way[seg].distance(way[seg + 1]) {
                left -= way[seg].distance(way[seg + 1]);
                seg += 1;
            }
            if seg + 1 >= way.len() {
                // Over the hub: drop from above the heap's top, thrown out over a disc
                // so the heap spreads into a mound rather than a tower.
                let s = self.streams.swap_remove(k);
                let hub = self.lib.hub;
                let p = self.pos[b];
                let top = self.loose.height_at(self.lib.world.level_at(hub), Vec2::new(p.x, p.z));
                let from = Vec3::new(p.x, p.y.max(top + 0.6), p.z);
                let mut j = || self.rng.gen_range(-1.0..1.0f32);
                let a = j() * PI;
                let out = Vec3::new(libm::cosf(a), 0.0, libm::sinf(a)) * 2.2 * (j() * 0.5 + 0.5).sqrt();
                self.state[b] = BookState::Loose;
                self.loose.throw(s.book, from, out, self.rot[b], s.spin, 0.0);
                continue;
            }
            let (a, c) = (way[seg], way[seg + 1]);
            let p = a + (c - a).normalize_or_zero() * left;
            // Closer to the line near the end, so the heap gathers.
            let to_end = way.len() - 2 - seg;
            let squeeze = if to_end == 0 {
                (c.distance(p) / 2.0).min(1.0)
            } else {
                1.0
            };
            self.pos[b] = p + s.off * squeeze;
            self.rot[b] = rot;
            k += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SLAB;

    /// A charm on a heap on the top floor of the Great Library sends it along the
    /// walking routes to the hub, out over a railing and down the well, never further
    /// into a wall, a case or a floor than a book flies beside the line; the books
    /// end on the hub's heap.
    #[test]
    fn a_charm_sends_a_heap_to_the_hub() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        g.open_all_gates();
        let (_, c, _) = (g.loose.piles())
            .filter(|&(lv, _, _)| lv == 2)
            .max_by_key(|(_, _, books)| books.len())
            .unwrap();
        assert!(g.charm_heap(2, c, 3));
        let sent: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.state[b as usize] == BookState::Streaming)
            .collect();
        assert!(sent.len() > 30, "{} books sent", sent.len());
        let solids: Vec<(Vec3, Vec3)> = (g.lib.world.solids().iter())
            .filter(|s| s.stops.has(Stops::SIGHT) || s.stops.has(Stops::BODIES))
            .map(|s| (s.min, s.max))
            .chain(g.lib.world.slabs.iter().map(|s| {
                let y = g.lib.world.levels[s.level].y;
                (Vec3::new(s.min.x, y - SLAB, s.min.y), Vec3::new(s.max.x, y, s.max.y))
            }))
            .collect();
        let depth = |p: Vec3| {
            (solids.iter())
                .map(|&(mn, mx)| (p - mn).min(mx - p).min_element().max(0.0))
                .fold(0.0f32, f32::max)
        };
        let mut deepest = (0.0f32, Vec3::ZERO);
        for _ in 0..60 * 40 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
            for s in &g.streams {
                let p = g.pos[s.book as usize];
                if s.t > RISE && depth(p) > deepest.0 {
                    deepest = (depth(p), p);
                }
            }
            if g.streams.is_empty() && sent.iter().all(|&b| g.loose.is_resting(b)) {
                break;
            }
        }
        assert!(g.streams.is_empty(), "{} books still on their way", g.streams.len());
        assert!(
            deepest.0 <= RIBBON + 0.05,
            "a book {:.2} m inside something at {}",
            deepest.0,
            deepest.1
        );
        let hub = g.lib.hub;
        for &b in &sent {
            let p = g.loose.pose(b).0;
            assert!(g.loose.is_resting(b) && g.loose.level_of(b) == 0, "book {b} at {p}");
            assert!(
                Vec2::new(p.x - hub.x, p.z - hub.z).length() < HUB_R + 2.0,
                "book {b} at {p}"
            );
        }
        assert_eq!(g.loose.heightfield_errors(), 0);
    }

    /// Hold book `b` (taken from its pile).
    fn hold(g: &mut Game, b: u32) {
        g.loose.take(&[b]);
        g.state[b as usize] = BookState::Held;
        g.held.push(b);
    }

    /// The Shelving Aura sends home, from inside an Alchemical Wing alcove, books whose
    /// slots are in its corner, half behind the end of the back case.
    #[test]
    fn the_aura_reaches_an_alcoves_corner() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        g.open_all_gates();
        g.upg[upg_index(Upg::Aura)] = 3;
        let books = crate::game::familiar::tests::corner_books(&g);
        let face = &g.lib.faces[g.lib.books[books[0] as usize].face as usize];
        g.player.pos = Vec3::new(face.center.x, 0.0, face.center.z) + face.normal * 1.5;
        for &b in &books {
            hold(&mut g, b);
        }
        for _ in 0..60 * 10 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, false);
        }
        let filed = (books.iter())
            .filter(|&&b| g.state[b as usize] == BookState::Shelved)
            .count();
        assert_eq!(filed, books.len(), "{filed} of {} filed", books.len());
    }

    /// Summon Series brings the loose volumes of the active book's series in reach
    /// to the arms; at its last level from anywhere.
    #[test]
    fn summon_calls_a_series_home_to_the_arms() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = Audio::silent();
        g.upg[upg_index(Upg::Arms)] = 4;
        let loose_of = |g: &Game, s: u32| -> Vec<u32> {
            (0..g.lib.books.len() as u32)
                .filter(|&b| g.lib.books[b as usize].series == s && g.loose.is_resting(b))
                .collect()
        };
        let series = (0..g.lib.series.len() as u32)
            .find(|&s| loose_of(&g, s).len() >= 4)
            .unwrap();
        let books = loose_of(&g, series);
        hold(&mut g, books[0]);
        assert!(g.summon(3, &mut audio));
        assert_eq!(g.held.len(), books.len(), "every loose volume came");
        assert!(loose_of(&g, series).is_empty());
        assert!(!g.summon(3, &mut audio), "none left to call");
    }

    /// Seeking a series, sorted by series, Summon Series calls it with none of it in
    /// hand: F at the shelf where it goes, then the spell.
    #[test]
    fn summon_calls_a_series_sought() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = Audio::silent();
        g.upg[upg_index(Upg::Arms)] = 4;
        g.upg[upg_index(Upg::Sift)] = 1;
        let loose_of = |g: &Game, s: u32| -> Vec<u32> {
            (0..g.lib.books.len() as u32)
                .filter(|&b| g.lib.books[b as usize].series == s && g.loose.is_resting(b))
                .collect()
        };
        let series = (0..g.lib.series.len() as u32)
            .find(|&s| loose_of(&g, s).len() >= 4)
            .unwrap();
        let books = loose_of(&g, series);
        assert!(!g.summon(3, &mut audio), "nothing held, nothing sought");
        let first = &g.lib.books[books[0] as usize];
        let (face, row) = (first.face, first.row);
        let run = (g.lib.runs[face as usize][row as usize].iter())
            .find(|r| r.series == series)
            .copied();
        g.aim = Aim::Shelf {
            face,
            row,
            run,
            dist: 1.0,
        };
        g.group = Group::Category;
        g.press_seek(&mut audio);
        assert!(!g.summon(3, &mut audio), "seeking its category isn't seeking it");
        g.group = Group::Series;
        assert_eq!(g.sought(), Some((Group::Series, series as usize)));
        assert!(g.summon(3, &mut audio));
        assert_eq!(g.held.len(), books.len(), "every loose volume came");
        assert!(loose_of(&g, series).is_empty());
    }

    /// Shelving Rush files the carried books near their slots, all at once rather
    /// than at the aura's pace, and only while it lasts; only from in front of their
    /// cases, never through the back.
    #[test]
    fn rush_files_what_is_near() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = Audio::silent();
        g.upg[upg_index(Upg::Arms)] = 4;
        g.upg[upg_index(Upg::Rush)] = 1;
        // Ten books of one category, and one shelved on the back of its middle case,
        // standing in front of that case.
        let cat = 0;
        let face = (g.lib.faces.iter())
            .find(|f| f.category == cat && f.case_idx == 1)
            .unwrap();
        let (front, normal) = (face.center, face.normal);
        let back = (0..g.lib.faces.len() as u32)
            .find(|&f| {
                let other = &g.lib.faces[f as usize];
                other.normal.dot(normal) < -0.99
                    && other.center.distance(front - normal * crate::library::UNIT_DEPTH) < 0.1
            })
            .unwrap();
        let behind = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.books[b as usize].face == back && g.loose.is_resting(b))
            .min_by(|&a, &b| {
                let off = |x: u32| g.lib.books[x as usize].slot_pos.distance(front);
                off(a).total_cmp(&off(b))
            })
            .unwrap();
        let books: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.category_of_book(b) == cat && g.loose.is_resting(b) && b != behind)
            .take(10)
            .chain([behind])
            .collect();
        for &b in &books {
            hold(&mut g, b);
        }
        g.player.pos = Vec3::new(front.x, 0.0, front.z) + normal * 1.5;
        let eye = g.player.eye();
        let in_reach = |b: u32| g.lib.books[b as usize].slot_pos.distance(eye) < rush_reach(1);
        assert!(in_reach(behind) && !g.lib.reaches_front(behind, eye));
        let near: Vec<u32> = (books.iter().copied())
            .filter(|&b| in_reach(b) && g.lib.reaches_front(b, eye))
            .collect();
        assert!(near.len() >= 3, "{} near", near.len());
        g.update(0.5, &Input::default(), &mut audio, true);
        assert_eq!(g.held.len(), books.len(), "nothing files without the aura or the rush");
        let cast = Input {
            cast: Some(SPELLS.iter().position(|&u| u == Upg::Rush).unwrap()),
            ..Input::default()
        };
        g.update(1.0 / 60.0, &cast, &mut audio, true);
        for _ in 0..30 {
            g.update(1.0 / 60.0, &Input::default(), &mut audio, true);
        }
        assert!(near.iter().all(|b| !g.held.contains(b)), "within half a second");
        // Not the one whose slot is on the far side of the case.
        assert!(g.held.contains(&behind));
        assert_eq!(g.held.len(), books.len() - near.len());
    }

    /// Echo marks the heaps holding the kind sought within its reach, and counts them.
    #[test]
    fn echo_marks_the_heaps_of_a_kind() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        g.upg[upg_index(Upg::Sift)] = 1;
        let aimed = (0..g.lib.books.len() as u32)
            .find(|&b| g.loose.is_resting(b) && g.loose.level_of(b) == 0)
            .unwrap();
        g.aim = Aim::Loose {
            book: aimed,
            point: g.loose.pose(aimed).0,
            dist: 2.0,
        };
        g.press_seek(&mut audio);
        assert!(g.echo(1));
        let (group, key) = g.sought().unwrap();
        let eye = g.player.eye();
        let within: u32 = (0..g.lib.books.len() as u32)
            .filter(|&b| {
                g.loose.is_resting(b)
                    && group.of(&g.lib, b) == key
                    && g.loose.pose(b).0.distance(eye) < echo_reach(1) - 1.0
            })
            .count() as u32;
        let marked: u32 = g.echo.iter().map(|m| m.1).sum();
        assert!(
            !g.echo.is_empty() && marked >= within,
            "{marked} marked, {within} within reach"
        );
        assert!(g.echo.iter().all(|m| m.0.distance(eye) < echo_reach(1) + 2.0));
    }
}
