//! The pacing bot: plays a library headless with the real mechanisms, to measure how
//! long it takes and when skills are bought.
//!
//! It walks at real speeds on the nav field (round furniture, up stairs) to the
//! pile that gives the most books per second of walking and gathers an armful. A
//! plain click takes the top of the pile whatever it is, and books it can't file
//! yet are set down where it stands; with Sifting Touch it sifts the group (the
//! coarsest it has) with the most books in reach. It files the armful nearest slot
//! first, posts its familiars (couriers at heaps of books far from home, shelvers
//! at the cases with books piling up near them) and runs their real code over each
//! trip, and buys the cheapest skill it can afford. It never tosses toward a
//! destination and reads nothing (it knows every slot), so tossing and the reading
//! skills are upside it doesn't count.
//!
//! Run with `cargo test --release pacing -- --ignored --nocapture`.

use super::*;
use std::collections::HashMap;

/// Time step for familiars, flights and falling books over a trip.
const DT: f32 = 0.05;
/// Seconds per gathering or sifting click, and per filing click.
const CLICK: f32 = 0.4;
const FILE_CLICK: f32 = 0.9;
/// Cells ranked by walking per book before the bot looks closer at what to sift.
const SHORTLIST: usize = 10;
/// Seconds a familiar may sit idle at its post before the bot moves it.
const RESTLESS: f32 = 30.0;

/// What to sift: a group and which one of it (`None`: a plain click).
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
    /// Where the time goes: [walking to piles, gathering clicks, walking to slots,
    /// filing clicks], seconds; trips and books gathered.
    spent: [f32; 4],
    trips: u32,
    gathered: u32,
    /// Clicks that took nothing (the books lay past a case from the click).
    empty_clicks: u32,
}

impl Bot {
    fn walk_speed(&self) -> f32 {
        // Some hurrying.
        [4.0, 5.5, 7.0][self.g.level(Upg::Slippers) as usize] * 1.3
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
            let g = &mut self.g;
            let _ = g.loose.update(dt, &mut g.pos, &mut g.rot);
            left -= dt;
        }
        self.by_familiars += self.g.total_shelved() - before;
    }

    /// Where to gather next, and which group to sift (`None`: a plain click).
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
            let score = (walk / speed + 1.0 + CLICK) / n.min(cap) as f32;
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, lv, c, (group, key)));
            }
        }
        best.map(|(_, lv, c, k)| (lv, c, Some(k)))
    }

    /// Gather an armful at the pile in cell `c` on level `lv`, clicking again at
    /// whatever is in reach until the arms are full.
    fn gather(&mut self, lv: usize, c: Vec3, sift: Sift) -> bool {
        let fileable = self.fileable();
        let floor = self.g.lib.world.ground(Vec2::new(c.x, c.z), c.y + 0.1).unwrap_or(c.y);
        let at = self.g.nav.standable_near(Vec3::new(c.x, floor, c.z));
        let reach = self.g.gather_reach();
        if self.walk(at, reach).is_none() {
            return false;
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
                self.empty_clicks += 1;
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
            self.t += CLICK;
            self.spent[1] += CLICK;
            if g.held.len() + aside.len() >= cap {
                break;
            }
            let matches = |b: u32| fileable[b as usize] && sift.is_none_or(|(group, key)| group.of(&g.lib, b) == key);
            target = (g.loose)
                .best_pile(|l, q, books| {
                    let n = books.iter().filter(|&&b| matches(b)).count();
                    (l == lv && q.distance(at) < reach && n > 0).then(|| -(n as f32))
                })
                .map(|(_, top)| top);
        }
        // What can't be filed yet goes back down where the bot stands.
        if !aside.is_empty() {
            for (k, &b) in aside.iter().enumerate() {
                let a = k as f32 * 2.4;
                let from = self.pos + Vec3::new(a.cos(), 1.0, a.sin()) * 0.6;
                self.g.loose.throw(b, from, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0.0);
            }
            self.t += 1.0;
        }
        true
    }

    /// File the armful, nearest slot first; a click files one book, or a whole series
    /// with Bundle, and the aura files at its rate where that is quicker.
    fn file(&mut self) {
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
            let walk_m = (hop - aura_r.max(1.8)).max(0.0);
            (here, here_d) = (to, d);
            let s = g.lib.books[b as usize].series;
            let batch: Vec<u32> = if g.level(Upg::Bundle) > 0 {
                (g.held.iter().copied())
                    .filter(|&x| g.lib.books[x as usize].series == s)
                    .collect()
            } else {
                vec![b]
            };
            let file = if aura_rate > 0.0 {
                FILE_CLICK.min(batch.len() as f32 / aura_rate)
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

    /// Post familiars that are riding, or have sat idle a while, where there is work
    /// for them.
    fn post_familiars(&mut self) {
        for i in 0..self.g.familiars.len() {
            let f = &self.g.familiars[i];
            if f.post.is_some() && f.idle < RESTLESS {
                continue;
            }
            let spot = match f.role {
                Role::Shelver => self.shelver_spot(i),
                Role::Courier => self.courier_spot(i),
            };
            match spot {
                Some(at) => self.g.post_familiar(i, at),
                None => self.g.familiars[i].idle = 0.0,
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

    /// At the heap with the most open books far from their rooms, away from other
    /// couriers.
    fn courier_spot(&mut self, me: usize) -> Option<Vec3> {
        let fileable = self.fileable();
        let g = &self.g;
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
        best.map(|(_, c)| g.nav.standable_near(c))
    }

    /// Buy the cheapest skill affordable, and again until none is.
    fn spend(&mut self) {
        loop {
            let g = &self.g;
            let cheapest = (UPGRADES.iter().copied())
                .filter_map(|u| g.next_cost(u).map(|c| (c, u)))
                .min_by_key(|x| x.0);
            match cheapest {
                Some((c, u)) if c <= self.g.lumens => {
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
        spent: [0.0; 4],
        trips: 0,
        gathered: 0,
        empty_clicks: 0,
    };
    let mut next_report = 0.0;
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
            let sp = bot.spent.map(|x| x / bot.trips.max(1) as f32);
            println!(
                "            per trip: {:.0} books, {:.0} s to the pile, {:.0} s gathering, {:.0} s to slots, {:.0} s filing",
                bot.gathered as f32 / bot.trips.max(1) as f32,
                sp[0],
                sp[1],
                sp[2],
                sp[3]
            );
            (bot.spent, bot.trips, bot.gathered) = ([0.0; 4], 0, 0);
            next_report += 15.0 * 60.0;
        }
    }
    let g = &bot.g;
    println!(
        "done at {:.2} h, shelved {}; familiars shelved {} ({:.0}%); {:.1}% of the time on stairs; {} empty clicks",
        bot.t / 3600.0,
        g.total_shelved(),
        bot.by_familiars,
        100.0 * bot.by_familiars as f32 / g.total_shelved().max(1) as f32,
        100.0 * bot.climbing / bot.t,
        bot.empty_clicks
    );
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
