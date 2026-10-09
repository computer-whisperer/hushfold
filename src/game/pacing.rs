//! The pacing bot: plays a library headless with the real mechanisms, to measure how
//! long it takes and when skills are bought.
//!
//! It walks at real speeds on the nav field (round furniture, up stairs) to the
//! pile that gives the most books per second of walking and gathers an armful. A
//! plain scoop takes the top of the pile whatever it is, and books it can't file
//! yet are set down where it stands; with Sifting Touch it seeks the group (the
//! coarsest it has) with the most books in reach and scoops only that; further
//! scoops take only piles it can see. When the Gathering Charm is ready and most
//! of what is left where it gathered belongs in other rooms, it sends that to the
//! hub; when Summon Series is, it hunts sets: it calls the strays of a series held
//! that they would complete (the biggest), else the series held with the most
//! loose volumes in reach.
//!
//! It files the armful nearest slot first, casting Glimpse and Shelving Rush as it
//! starts whenever they are ready. It knows every slot, so reading is charged as
//! time: finding each series' run by its letters and titles, and each category's
//! cases by their signs, less with Signposts and Kindred Glow, nothing while Sort
//! Sense or Glimpse shows the way (first guesses, to be calibrated against play).
//! With Homing Toss and Fine Fingers it tosses a series at its shelf, with the real
//! throw and landing, when it can see the spot and that saves a walk.
//!
//! It walks to post the familiars riding with it (couriers round the hub or at
//! heaps of books far from home, shelvers at the cases with books piling up near
//! them; one idle two minutes comes back by itself), running their real code over
//! each trip. It buys the cheapest skill it can afford, or with `HUSHFOLD_PATH`
//! (reading, hands, familiars, filing) favours that tech path, counting the skills
//! off it as three times dearer (with `HUSHFOLD_BASICS`, all but Deeper Arms,
//! Swift Slippers, Signposts and Kindred Glow, which every path then buys at face
//! value). Other tosses, and Echo (it knows where every book
//! lies), are upside it doesn't count.
//!
//! Run with `cargo test --release pacing -- --ignored --nocapture`.

use super::*;
use std::collections::HashMap;

/// Time step for familiars, flights and falling books over a trip.
const DT: f32 = 0.05;
/// Seconds per scoop (aiming, then holding to the full ring), and per filing click.
const SCOOP: f32 = 0.2 + TAP + SCOOP_GROW;
/// Seconds to aim at a book of another group and press F to seek it.
const SEEK: f32 = 0.5;
/// Seconds to find a series' run by reading, by Kindred Glow level, for a player who
/// has learned the layout: the letters on the shelf edges and the titles; the case
/// by its letters, then a faint glow on it; a glow across the room; the glow
/// everywhere.
const FIND_RUN: [f32; 4] = [4.0, 2.0, 0.8, 0.0];
/// Seconds to find a category's cases by looking for its sign, without Signposts
/// or a glow that reaches across the room.
const FIND_CASES: f32 = 3.0;
/// Seconds to cast a spell.
const CAST: f32 = 0.3;
/// With Homing Toss (and Fine Fingers), a series is tossed at its shelf rather than
/// walked to when that saves more than this walk (m), from no further than this.
const TOSS_OVER: f32 = 3.0;
const TOSS_REACH: f32 = 14.0;
/// Seconds to aim and throw a toss.
const TOSS: f32 = 1.2;
/// A preferred path makes the skills off it look this much dearer.
const OFF_PATH: u64 = 3;
/// What every path buys at face value with `HUSHFOLD_BASICS` set: carrying, walking,
/// and finding where books go.
const BASICS: [Upg; 4] = [Upg::Arms, Upg::Slippers, Upg::Signposts, Upg::Glow];

/// A preferred tech path (`HUSHFOLD_PATH`): the skills it favours.
fn path_skills(path: &str) -> &'static [Upg] {
    match path {
        "reading" => &[
            Upg::Signposts,
            Upg::Glow,
            Upg::SortSense,
            Upg::Wayfinding,
            Upg::Lens,
            Upg::Glimpse,
            Upg::Echo,
            Upg::Summon,
        ],
        "hands" => &[
            Upg::Sift,
            Upg::Broad,
            Upg::Fine,
            Upg::LongToss,
            Upg::Homing,
            Upg::Lodestone,
        ],
        "familiars" => &[Upg::Shelver, Upg::Courier, Upg::Wings, Upg::Gathering],
        "filing" => &[Upg::Arms, Upg::Slippers, Upg::Tidy, Upg::Bundle, Upg::Aura, Upg::Rush],
        _ => &[],
    }
}
const FILE_CLICK: f32 = 0.9;
/// Cells ranked by walking per book before the bot looks closer at what to sift.
const SHORTLIST: usize = 10;
/// How far the bot aims G (at the floor to post a familiar, or at one to take it
/// back): it walks to within that of the spot.
const AIM: f32 = 12.0;
/// Seconds per G.
const POST: f32 = 1.0;

/// What to seek: a group and which one of it (`None`: a plain scoop).
type Sift = Option<(Group, usize)>;

struct Bot {
    g: Game,
    audio: Audio,
    /// Seconds played.
    t: f32,
    /// Where the bot stands.
    pos: Vec3,
    /// Seconds spent on stairs, and books shelved by familiars.
    climbing: f32,
    by_familiars: u32,
    /// Where the time goes: [walking to piles, scooping, walking to slots, filing
    /// clicks, reading], seconds; trips and books gathered.
    spent: [f32; 5],
    trips: u32,
    gathered: u32,
    /// Scoops that took nothing (the books lay past a case from the aim).
    empty_scoops: u32,
    /// Familiars posted, and Gathering Charms cast.
    posts: u32,
    charms: u32,
    /// The group it seeks.
    seeking: Sift,
    /// Each spell's (by hotbar slot) time it can be cast again and its effect ends,
    /// on the bot's clock.
    spell_ready: [f32; SPELLS.len()],
    spell_until: [f32; SPELLS.len()],
    /// Books tossed at their shelves, and Summon Series cast.
    tossed: u32,
    summons: u32,
    /// The skills it favours (none: it buys the cheapest), and whether every path
    /// buys the basics at face value.
    path: &'static [Upg],
    basics: bool,
    /// What last moved each book the bot moved other than to its arms (to explain
    /// any left behind).
    moved_by: HashMap<u32, &'static str>,
}

impl Bot {
    fn walk_speed(&self) -> f32 {
        // Some hurrying.
        [4.0, 5.5, 7.0][self.g.level(Upg::Slippers) as usize] * 1.3
    }

    fn slot(u: Upg) -> usize {
        SPELLS.iter().position(|&x| x == u).unwrap()
    }

    /// Spell `u` is learned and has recharged, by the bot's clock.
    fn ready(&self, u: Upg) -> bool {
        self.g.level(u) > 0 && self.t >= self.spell_ready[Self::slot(u)]
    }

    /// Charge a cast of spell `u` and start its clocks.
    fn commit(&mut self, u: Upg) {
        let (lasts, recharge) = spell_times(u, self.g.level(u));
        self.t += CAST;
        self.spent[4] += CAST;
        let i = Self::slot(u);
        (self.spell_until[i], self.spell_ready[i]) = (self.t + lasts, self.t + recharge);
    }

    fn fileable(&self) -> Vec<bool> {
        let g = &self.g;
        (0..g.lib.books.len() as u32)
            .map(|b| g.dept_open(g.lib.dept_of(g.lib.category_of_book(b))))
            .collect()
    }

    /// Walk to `to` (stopping `short` of it); returns the walking distance, or `None`
    /// if it can't be reached.
    fn walk(&mut self, to: Vec3, short: f32) -> Option<f32> {
        let speed = self.walk_speed();
        let g = &mut self.g;
        g.nav.fill(&mut g.walk, self.pos);
        let d = g.nav.distance(&g.walk, self.pos, to);
        if !d.is_finite() {
            return None;
        }
        let floors = (self.g.lib.world.level_at(to) as f32 - self.g.lib.world.level_at(self.pos) as f32).abs();
        // A flight of stairs is ~11 m of walking.
        self.climbing += floors * 11.0 / speed;
        self.t += (d - short).max(0.0) / speed;
        self.spent[0] += (d - short).max(0.0) / speed;
        self.pos = to;
        Some(d)
    }

    /// Run familiars, books in flight and falling books for `secs`.
    fn advance(&mut self, secs: f32) {
        let before = self.g.total_shelved();
        let mut left = secs;
        while left > 0.0 {
            let dt = left.min(DT);
            self.g.time += dt;
            self.g.update_familiars(dt);
            self.g.update_flights(dt, &mut self.audio);
            self.g.update_streams(dt);
            let g = &mut self.g;
            let landings = g.loose.update(dt, &mut g.pos, &mut g.rot);
            g.home_tossed(&landings);
            left -= dt;
        }
        self.by_familiars += self.g.total_shelved() - before;
    }

    /// Where to gather next, and which group to seek (`None`: a plain scoop).
    fn choose(&mut self) -> Option<(usize, Vec3, Sift)> {
        let fileable = self.fileable();
        let (cap, speed, pos) = (self.g.capacity(), self.walk_speed(), self.pos);
        let g = &mut self.g;
        g.nav.fill(&mut g.walk, pos);
        let mut cells: Vec<(f32, usize, Vec3)> = (g.loose.piles())
            .filter_map(|(lv, c, books)| {
                let n = books.iter().filter(|&&b| fileable[b as usize]).count();
                (n > 0 && g.nav.open_at(c)).then(|| {
                    let walk = g.nav.distance(&g.walk, pos, c);
                    ((walk / speed + 1.0) / n.min(cap) as f32, lv, c)
                })
            })
            .filter(|c| c.0.is_finite())
            .collect();
        cells.sort_by(|a, b| a.0.total_cmp(&b.0));
        let sift = SIFT_RADIUS[g.level(Upg::Sift) as usize];
        if sift == 0.0 {
            return cells.first().map(|&(_, lv, c)| (lv, c, None));
        }
        let group = g.groups()[0];
        let mut best: Option<(f32, usize, Vec3, (Group, usize))> = None;
        for &(_, lv, c) in cells.iter().take(SHORTLIST) {
            let near = g.loose.near(lv, Vec2::new(c.x, c.z), sift, |b| fileable[b as usize]);
            let mut tally: HashMap<usize, usize> = HashMap::new();
            for b in near {
                *tally.entry(group.of(&g.lib, b)).or_default() += 1;
            }
            // Ties to the lowest key, so runs repeat.
            let Some((&key, &n)) = tally.iter().max_by_key(|&(&k, &n)| (n, std::cmp::Reverse(k))) else {
                continue;
            };
            let walk = g.nav.distance(&g.walk, pos, c);
            let score = (walk / speed + 1.0 + SCOOP) / n.min(cap) as f32;
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, lv, c, (group, key)));
            }
        }
        best.map(|(_, lv, c, k)| (lv, c, Some(k)))
    }

    /// Gather an armful at the pile in cell `c` on level `lv`, scooping again at
    /// whatever is in reach until the arms are full.
    fn gather(&mut self, lv: usize, c: Vec3, sift: Sift) -> bool {
        let fileable = self.fileable();
        let floor = self.g.lib.world.ground(Vec2::new(c.x, c.z), c.y + 0.1).unwrap_or(c.y);
        let at = self.g.nav.standable_near(Vec3::new(c.x, floor, c.z));
        let reach = self.g.gather_reach();
        if self.walk(at, reach).is_none() {
            return false;
        }
        if sift.is_some() && sift != self.seeking {
            self.seeking = sift;
            self.t += SEEK;
            self.spent[1] += SEEK;
        }
        let cap = self.g.capacity();
        let mut aside = Vec::new();
        let mut target = Some(c);
        while let Some(p) = target {
            let g = &mut self.g;
            let room = cap - g.held.len() - aside.len();
            let spot = Vec2::new(p.x, p.z);
            let books = match sift {
                Some((group, key)) => {
                    let lib = &g.lib;
                    let keep = |b: u32| fileable[b as usize] && group.of(lib, b) == key;
                    g.loose
                        .gather(lv, spot, SIFT_RADIUS[g.level(Upg::Sift) as usize], room, keep)
                }
                None => g
                    .loose
                    .gather(lv, spot, 0.35 + 0.05 * (room as f32).sqrt(), room, |_| true),
            };
            if books.is_empty() {
                self.empty_scoops += 1;
                break;
            }
            for b in books {
                if fileable[b as usize] {
                    g.state[b as usize] = BookState::Held;
                    g.held.push(b);
                } else {
                    aside.push(b);
                }
            }
            self.t += SCOOP;
            self.spent[1] += SCOOP;
            if g.held.len() + aside.len() >= cap {
                break;
            }
            // The richest pile in reach that it can see to aim at.
            let matches = |b: u32| fileable[b as usize] && sift.is_none_or(|(group, key)| group.of(&g.lib, b) == key);
            let eye = at + Vec3::Y * EYE;
            let mut piles: Vec<(usize, Vec3)> = (g.loose.piles())
                .filter(|&(l, q, _)| l == lv && q.distance(at) < reach)
                .filter_map(|(_, _, books)| {
                    let n = books.iter().filter(|&&b| matches(b)).count();
                    let top = books
                        .iter()
                        .map(|&b| g.loose.pose(b).0)
                        .max_by(|a, b| a.y.total_cmp(&b.y))?;
                    (n > 0).then_some((n, top))
                })
                .collect();
            piles.sort_by_key(|&(n, _)| std::cmp::Reverse(n));
            target = (piles.into_iter())
                .map(|(_, top)| top)
                .find(|&top| g.lib.world.line_of_sight(eye, top + Vec3::Y * 0.05));
        }
        // The Gathering Charm on what is left here, when most of it belongs in other
        // rooms (or is veiled).
        let charm = self.g.level(Upg::Gathering);
        if self.ready(Upg::Gathering) {
            let g = &mut self.g;
            let left = g.charmed(lv, c, charm);
            let here = g.lib.room_at(c);
            let away = (left.iter())
                .filter(|&&b| here != Some(g.lib.dept_of(g.lib.category_of_book(b))))
                .count();
            if away >= 20 && away * 2 >= left.len() && g.charm_heap(lv, c, charm) {
                for &b in &left {
                    self.moved_by.insert(b, "charmed");
                }
                self.commit(Upg::Gathering);
                self.charms += 1;
            }
        }
        // Summon Series, if ready and there is room, hunting sets: a series held that
        // the volumes in reach would complete (the biggest), else the series held
        // with the most loose volumes in reach (three or more).
        let summon = self.g.level(Upg::Summon);
        let room = cap.saturating_sub(self.g.held.len() + aside.len());
        if self.ready(Upg::Summon) && room > 0 {
            let g = &mut self.g;
            g.player.pos = self.pos;
            let mut series: Vec<u32> = g.held.iter().map(|&b| g.lib.books[b as usize].series).collect();
            series.sort();
            series.dedup();
            let held_of =
                |g: &Game, s: u32| g.held.iter().filter(|&&b| g.lib.books[b as usize].series == s).count() as u32;
            let best = (series.into_iter())
                .filter_map(|s| {
                    let n = g.summonable(s, summon).len();
                    let missing =
                        (g.lib.series[s as usize].volumes - g.series_shelved[s as usize] - held_of(g, s)) as usize;
                    let completes = missing > 0 && n >= missing && missing <= room;
                    if completes {
                        Some(((1, g.lib.series[s as usize].volumes as usize), s))
                    } else {
                        (n >= 3).then_some(((0, n), s))
                    }
                })
                .max_by_key(|&(score, s)| (score, std::cmp::Reverse(s)));
            if let Some((_, s)) = best {
                g.active = (g.held.iter())
                    .position(|&b| g.lib.books[b as usize].series == s)
                    .unwrap();
                if g.summon(summon, &mut self.audio) {
                    self.commit(Upg::Summon);
                    self.summons += 1;
                }
            }
        }
        // What can't be filed yet goes back down where the bot stands, within its
        // cell's clearance (dropped from past the middle of a wall, a book would
        // come out on its far side).
        if !aside.is_empty() {
            for (k, &b) in aside.iter().enumerate() {
                self.moved_by.insert(b, "set aside");
                let a = k as f32 * 2.4;
                let from = self.pos + Vec3::new(a.cos() * 0.15, 1.0, a.sin() * 0.15);
                self.g.loose.throw(b, from, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0.0);
            }
            self.t += 1.0;
        }
        true
    }

    /// File the armful, nearest slot first; a click files one book, or a whole series
    /// with Bundle, and the aura (or Shelving Rush) files at its rate where that is
    /// quicker. With Homing Toss and Fine Fingers it tosses a series at its shelf from
    /// where it stands, when it can see the spot and that saves a walk; what misses
    /// lies there to be gathered again.
    fn file(&mut self) {
        for u in [Upg::Glimpse, Upg::Rush] {
            if self.ready(u) {
                self.commit(u);
            }
        }
        let glimpse_until = self.spell_until[Self::slot(Upg::Glimpse)];
        let rush_until = self.spell_until[Self::slot(Upg::Rush)];
        let rush_r = spells::rush_reach(self.g.level(Upg::Rush));
        let homing = self.g.level(Upg::Homing) > 0 && self.g.level(Upg::Fine) > 0;
        let mut found_series = std::collections::HashSet::new();
        let mut found_cases = std::collections::HashSet::new();
        let g = &mut self.g;
        let speed = [4.0, 5.5, 7.0][g.level(Upg::Slippers) as usize] * 1.3;
        g.nav.fill(&mut g.walk, self.pos);
        let (aura_r, aura_rate) = AURA_LEVELS[g.level(Upg::Aura) as usize];
        let start = self.pos;
        let mut here = self.pos;
        let mut here_d = 0.0f32;
        // A walk between two slots: the straight line on one floor (no less than the
        // difference of their walks from where filing began), else that difference.
        let hop = |(p, pd): (Vec3, f32), (q, qd): (Vec3, f32)| {
            let flat = Vec2::new(q.x - p.x, q.z - p.z).length();
            if (q.y - p.y).abs() < 1.0 {
                flat.max((qd - pd).abs())
            } else {
                (qd - pd).abs().max(flat)
            }
        };
        // Each book's slot and walk from where filing began, worked out once.
        let mut walks: HashMap<u32, (Vec3, f32)> = HashMap::new();
        for &b in &g.held {
            let to = g.approach(b);
            let d = g.nav.distance(&g.walk, start, to);
            walks.insert(b, (to, if d.is_finite() { d } else { start.distance(to) }));
        }
        while !g.held.is_empty() {
            // Nearest from here, as Tidy Arms picks.
            let (i, to, d) = (g.held.iter().enumerate())
                .map(|(i, b)| (i, walks[b].0, walks[b].1))
                .min_by(|a, b| hop((here, here_d), (a.1, a.2)).total_cmp(&hop((here, here_d), (b.1, b.2))))
                .unwrap();
            let b = g.held[i];
            if (to.y - here.y).abs() >= 1.0 {
                self.climbing += (to.y - here.y).abs() / 6.0 * 11.0 / speed;
            }
            let hop = hop((here, here_d), (to, d));
            let rushing = self.t < rush_until;
            let reach = if rushing { aura_r.max(rush_r) } else { aura_r };
            let walk_m = (hop - reach.max(1.8)).max(0.0);
            // Reading where it goes, unless Sort Sense (in the book's room at I) or
            // Glimpse shows it: the run, and the cases the first time this trip.
            let s = g.lib.books[b as usize].series;
            let cat = g.lib.category_of_book(b);
            let sense = g.level(Upg::SortSense);
            let shown =
                sense >= 2 || (sense == 1 && g.lib.room_at(here) == Some(g.lib.dept_of(cat))) || self.t < glimpse_until;
            let glow = g.level(Upg::Glow) as usize;
            let mut read = 0.0;
            if !shown && found_series.insert(s) {
                read += FIND_RUN[glow];
                if g.level(Upg::Signposts) == 0 && glow < 2 && found_cases.insert(cat) {
                    read += FIND_CASES;
                }
            }
            self.t += read;
            self.spent[4] += read;
            // A toss at the floor in front of its run, from where the bot stands.
            if homing && walk_m > TOSS_OVER {
                let ground = |p: Vec3| g.lib.world.ground(Vec2::new(p.x, p.z), p.y + 0.1).unwrap_or(p.y);
                let stand = g.nav.standable_near(Vec3::new(here.x, ground(here), here.z));
                let info = &g.lib.books[b as usize];
                let front = info.slot_pos + g.lib.faces[info.face as usize].normal * 0.6;
                let target = Vec3::new(front.x, ground(front), front.z);
                let eye = stand + Vec3::Y * EYE;
                if target.distance(stand) < TOSS_REACH && g.lib.world.line_of_sight(eye, target + Vec3::Y * 0.3) {
                    // Seeking the series, so only it flies.
                    let (group, before) = (g.group, g.held.clone());
                    g.player.pos = stand;
                    (g.group, g.seek) = (Group::Series, Some(b));
                    if g.aim_toss(target) < 0.8 {
                        g.toss(false, &mut self.audio);
                    }
                    (g.group, g.seek) = (group, None);
                    if g.held.len() < before.len() {
                        for b in before.into_iter().filter(|b| !g.held.contains(b)) {
                            self.moved_by.insert(b, "tossed");
                            self.tossed += 1;
                        }
                        self.t += TOSS;
                        self.spent[3] += TOSS;
                        continue;
                    }
                }
            }
            (here, here_d) = (to, d);
            let batch: Vec<u32> = if g.level(Upg::Bundle) > 0 {
                (g.held.iter().copied())
                    .filter(|&x| g.lib.books[x as usize].series == s)
                    .collect()
            } else {
                vec![b]
            };
            let rate = if rushing { spells::RUSH_RATE } else { aura_rate };
            let file = if rate > 0.0 {
                FILE_CLICK.min(batch.len() as f32 / rate)
            } else {
                FILE_CLICK
            };
            self.t += walk_m / speed + file;
            self.spent[2] += walk_m / speed;
            self.spent[3] += file;
            g.held.retain(|x| !batch.contains(x));
            let mut thunks = 0;
            for x in batch {
                g.on_shelved(x, &mut self.audio, &mut thunks);
            }
        }
        self.pos = here;
    }

    /// Post familiars where there is the most work for them, walking there (to
    /// within aim), and move those that have sat idle a while, walking to take them
    /// back first: a player setting up their helpers on purpose.
    fn post_familiars(&mut self) {
        for i in 0..self.g.familiars.len() {
            if self.g.familiars[i].post.is_some() {
                continue;
            }
            let spot = match self.g.familiars[i].role {
                Role::Shelver => self.shelver_spot(i),
                Role::Courier => self.courier_spot(i),
            };
            if let Some(at) = spot
                && self.walk(at, AIM).is_some()
            {
                self.g.post_familiar(i, at);
                self.t += POST;
                self.posts += 1;
            }
        }
    }

    /// In front of the cases with the most open books lying within a few metres that
    /// belong on cases near them, away from other shelvers.
    fn shelver_spot(&mut self, me: usize) -> Option<Vec3> {
        let g = &mut self.g;
        let others: Vec<Vec3> = (g.familiars.iter().enumerate())
            .filter(|&(i, f)| i != me && f.role == Role::Shelver)
            .filter_map(|(_, f)| f.post.as_ref().map(|p| p.at))
            .collect();
        let mut best: Option<(usize, Vec3)> = None;
        for fi in 0..g.lib.faces.len() {
            let f = &g.lib.faces[fi];
            if f.case_idx != 1 || !g.dept_open(g.lib.dept_of(f.category)) {
                continue;
            }
            let spot = f.center + f.normal * 1.2;
            if others.iter().any(|o| o.distance(spot) < 6.0) {
                continue;
            }
            let lv = g.lib.world.level_at(spot);
            let lib = &g.lib;
            let near = |b: u32| {
                let a = lib.books[b as usize].slot_pos;
                let f = &lib.faces[lib.books[b as usize].face as usize];
                (a + f.normal * 0.75).distance(spot) < 6.0
            };
            let n = g.loose.near(lv, Vec2::new(spot.x, spot.z), 3.0, near).len();
            if n >= 3 && best.is_none_or(|(bn, _)| n > bn) {
                best = Some((n, spot));
            }
        }
        best.map(|(_, spot)| g.nav.standable_near(spot))
    }

    /// Round the hub or at a heap, wherever the most open books lie far from their
    /// rooms, away from other couriers.
    fn courier_spot(&mut self, me: usize) -> Option<Vec3> {
        let fileable = self.fileable();
        let g = &mut self.g;
        let others: Vec<Vec3> = (g.familiars.iter().enumerate())
            .filter(|&(i, f)| i != me && f.role == Role::Courier)
            .filter_map(|(_, f)| f.post.as_ref().map(|p| p.at))
            .collect();
        let mut best: Option<(usize, Vec3)> = None;
        for (_, c, books) in g.loose.piles() {
            if others.iter().any(|o| o.distance(c) < 5.0) || !g.nav.open_at(c) {
                continue;
            }
            let far = (books.iter())
                .filter(|&&b| fileable[b as usize])
                .filter(|&&b| g.lib.rooms[g.lib.dept_of(g.lib.category_of_book(b))].drop.distance(c) > 8.0)
                .count();
            if far >= 4 && best.is_none_or(|(bn, _)| far > bn) {
                best = Some((far, c));
            }
        }
        // Round the hub: the books within a courier's reach of a spot at its edge.
        let reach = familiar::WINGS[g.level(Upg::Wings) as usize].2;
        let hub = g.lib.hub;
        let lv = g.lib.world.level_at(hub);
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::TAU / 8.0;
            let c = g.nav.standable_near(hub + Vec3::new(a.cos(), 0.0, a.sin()) * 3.0);
            if others.iter().any(|o| o.distance(c) < 4.0) || !g.nav.open_at(c) {
                continue;
            }
            let lib = &g.lib;
            let far =
                |b: u32| fileable[b as usize] && lib.rooms[lib.dept_of(lib.category_of_book(b))].drop.distance(c) > 8.0;
            let n = g.loose.near(lv, Vec2::new(c.x, c.z), reach, far).len();
            if n >= 4 && best.is_none_or(|(bn, _)| n > bn) {
                best = Some((n, c));
            }
        }
        best.map(|(_, c)| g.nav.standable_near(c))
    }

    /// Buy the cheapest skill affordable (on a preferred path, those off it counted
    /// as dearer), and again until none is.
    fn spend(&mut self) {
        loop {
            let g = &self.g;
            // On a preferred path, the skills off it count as dearer.
            let favoured =
                |u: Upg| self.path.is_empty() || self.path.contains(&u) || (self.basics && BASICS.contains(&u));
            let weight = |u: Upg| if favoured(u) { 1 } else { OFF_PATH };
            let cheapest = (UPGRADES.iter().copied())
                .filter_map(|u| g.next_cost(u).map(|c| (c * weight(u), c, u)))
                .min_by_key(|x| x.0);
            match cheapest {
                Some((_, c, u)) if c <= self.g.lumens => {
                    self.g.buy(u, &mut self.audio);
                    println!(
                        "{:6.1} min  bought {:?} -> {}  (shelved {})",
                        self.t / 60.0,
                        u,
                        self.g.level(u),
                        self.g.total_shelved()
                    );
                }
                _ => break,
            }
        }
    }
}

/// Play `layout` to the end (or 30 hours); returns hours.
pub(super) fn simulate(layout: Layout) -> f32 {
    let mut g = Game::new(layout);
    g.persist = false;
    let pos = g.player.pos;
    let mut bot = Bot {
        g,
        audio: Audio::silent(),
        t: 0.0,
        pos,
        climbing: 0.0,
        by_familiars: 0,
        spent: [0.0; 5],
        trips: 0,
        gathered: 0,
        empty_scoops: 0,
        seeking: None,
        spell_ready: [0.0; SPELLS.len()],
        spell_until: [0.0; SPELLS.len()],
        tossed: 0,
        summons: 0,
        path: path_skills(&std::env::var("HUSHFOLD_PATH").unwrap_or_default()),
        basics: std::env::var("HUSHFOLD_BASICS").is_ok(),
        moved_by: HashMap::new(),
        posts: 0,
        charms: 0,
    };
    let mut next_report = 0.0;
    let mut next_earn = 0.0;
    let mut stuck = 0;
    while bot.g.total_shelved() < bot.g.total_books() && bot.t < 60.0 * 60.0 * 30.0 {
        bot.post_familiars();
        let trip = bot.t;
        let gathered = match bot.choose() {
            Some((lv, c, sift)) => bot.gather(lv, c, sift),
            None => false,
        };
        if gathered {
            bot.trips += 1;
            bot.gathered += bot.g.held.len() as u32;
            bot.file();
            stuck = 0;
        } else {
            // Nothing the bot can reach to file: let the familiars work a while.
            bot.t += 5.0;
            stuck += 1;
            if stuck > 200 {
                break;
            }
        }
        let secs = bot.t - trip;
        bot.advance(secs);
        bot.spend();
        // The lumens earned so far, every two minutes, for costing the tree.
        if bot.t >= next_earn {
            println!("earn {:.1} {}", bot.t / 60.0, bot.g.earned);
            next_earn += 120.0;
        }
        if bot.t >= next_report {
            let g = &bot.g;
            let depts: Vec<String> = (g.dept_shelved.iter().zip(&g.dept_total).enumerate())
                .filter(|&(d, _)| g.dept_open(d))
                .map(|(_, (&s, &n))| format!("{:.0}%", 100.0 * s as f32 / n as f32))
                .collect();
            println!(
                "{:6.1} min  shelved {:6} / {}  open departments {}",
                bot.t / 60.0,
                g.total_shelved(),
                g.total_books(),
                depts.join(" ")
            );
            // Milestones reached so far: series and categories complete.
            let series = (0..g.lib.series.len())
                .filter(|&s| g.series_shelved[s] == g.lib.series[s].volumes)
                .count();
            let cats = (0..g.lib.category_books.len())
                .filter(|&c| g.category_shelved[c] == g.lib.category_books[c])
                .count();
            println!(
                "            complete: {series} / {} series, {cats} / {} categories",
                g.lib.series.len(),
                g.lib.category_books.len()
            );
            let sp = bot.spent.map(|x| x / bot.trips.max(1) as f32);
            println!(
                "            per trip: {:.0} books, {:.0} s to the pile, {:.0} s gathering, {:.0} s to slots, {:.0} s filing, {:.0} s reading",
                bot.gathered as f32 / bot.trips.max(1) as f32,
                sp[0],
                sp[1],
                sp[2],
                sp[3],
                sp[4]
            );
            (bot.spent, bot.trips, bot.gathered) = ([0.0; 5], 0, 0);
            next_report += 15.0 * 60.0;
        }
    }
    let g = &bot.g;
    println!(
        "done at {:.2} h, shelved {}; familiars shelved {} ({:.0}%) and delivered {}, {} posted; {} charms, {} summons, {} books tossed home; {:.1}% of the time on stairs; {} empty scoops",
        bot.t / 3600.0,
        g.total_shelved(),
        bot.by_familiars,
        100.0 * bot.by_familiars as f32 / g.total_shelved().max(1) as f32,
        g.familiars.iter().map(|f| f.delivered).sum::<u32>(),
        bot.posts,
        bot.charms,
        bot.summons,
        bot.tossed,
        100.0 * bot.climbing / bot.t,
        bot.empty_scoops
    );
    // Any left behind, and why the bot couldn't get at them.
    let left: Vec<u32> = (0..g.lib.books.len() as u32)
        .filter(|&b| g.state[b as usize] != BookState::Shelved)
        .take(5)
        .collect();
    if !left.is_empty() {
        let start = Loose::new(&g.lib);
        for b in left {
            let p = g.pos[b as usize];
            println!(
                "left: book {b} {:?} at {p} (resting {}, flying {}, reachable {}); last moved by the bot: {}, starts at {}",
                g.state[b as usize],
                g.loose.is_resting(b),
                g.loose.is_flying(b),
                g.nav.distance(&g.walk, bot.pos, p).is_finite(),
                bot.moved_by.get(&b).unwrap_or(&"nothing"),
                start.pose(b).0
            );
        }
    }
    bot.t / 3600.0
}

#[test]
#[ignore]
fn pacing_hall() {
    simulate(Layout::Hall);
}

#[test]
#[ignore]
fn pacing_great_library() {
    simulate(Layout::Great);
}
