//! Cheats for trying late-game situations (F9): lumens, every skill, lifted veils,
//! a library already partly shelved, and a quick way round it. They change the
//! library's save like play does; starting over undoes them.

use super::*;

/// One cheat, as the cheat window asks for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cheat {
    /// Add this many lumens.
    Lumens(u64),
    /// Learn every skill to its last level (new familiars ride with you).
    AllSkills,
    /// Lift every veil until the library is next loaded (on loading, veils follow
    /// from what is shelved).
    LiftVeils,
    /// Shelve books from the piles until every open department is this far home,
    /// then the departments whose veils that lifts, and so on.
    Shelve(f32),
    /// Go to where a new game starts (`None`), or to a department's doorway: just
    /// inside it, or in front of its veil while that is closed.
    GoTo(Option<usize>),
    /// Shelve every book of a part of the library at once, from the piles, the air,
    /// the clocks' hold and the arms (not those already flying or carried), so it is
    /// finished and celebrates.
    Finish(Finish),
}

impl Game {
    pub fn cheat(&mut self, c: Cheat, audio: &mut Audio) {
        match c {
            Cheat::Lumens(n) => self.lumens += n,
            Cheat::AllSkills => {
                for u in UPGRADES {
                    while let Some(cost) = self.next_cost(u) {
                        self.lumens += cost;
                        self.buy(u, audio);
                    }
                }
                self.toasts.clear();
                self.hint = None;
                self.toast("Every skill learned".into(), [0.95, 0.8, 0.45], false);
            }
            Cheat::LiftVeils => self.open_all_gates(),
            Cheat::Shelve(frac) => self.cheat_shelve(frac, audio),
            Cheat::GoTo(room) => self.cheat_go_to(room),
            Cheat::Finish(f) => self.cheat_finish(f, audio),
        }
        self.dirty_save = true;
    }

    fn cheat_shelve(&mut self, frac: f32, audio: &mut Audio) {
        loop {
            let mut pools: Vec<Vec<u32>> = vec![Vec::new(); self.dept_total.len()];
            for b in 0..self.lib.books.len() as u32 {
                if self.state[b as usize] == BookState::Loose && self.loose.is_resting(b) {
                    pools[self.lib.dept_of_book(b)].push(b);
                }
            }
            let mut take = Vec::new();
            for (d, mut pool) in pools.into_iter().enumerate() {
                if !self.dept_open(d) {
                    continue;
                }
                let want = (self.dept_total[d] as f32 * frac).ceil() as u32;
                // Picked at random, so piles thin out rather than vanish one by one.
                pool.shuffle(&mut self.rng);
                take.extend(
                    pool.into_iter()
                        .take(want.saturating_sub(self.dept_shelved[d]) as usize),
                );
            }
            if take.is_empty() {
                break;
            }
            self.loose.take(&take);
            for &b in &take {
                self.credit_shelved(b);
            }
            self.start_clocks();
            let open = |g: &Self| g.lib.world.gates_open().iter().filter(|&&o| o).count();
            let before = open(self);
            self.check_gates(audio);
            if open(self) == before {
                break;
            }
        }
    }

    fn cheat_finish(&mut self, f: Finish, audio: &mut Audio) {
        let lib = &self.lib;
        let part = |b: u32| match f {
            Finish::Face(i) => lib.books[b as usize].face as usize == i,
            Finish::Category(c) => lib.category_of_book(b) == c,
            Finish::Room(d) => lib.dept_of_book(b) == d,
            Finish::Storey(l) => lib.rooms[lib.dept_of_book(b)].level == l,
            Finish::Library => true,
        };
        let books: Vec<u32> = (0..lib.books.len() as u32)
            .filter(|&b| part(b) && matches!(self.state[b as usize], BookState::Loose | BookState::Held))
            .collect();
        let resting: Vec<u32> = books.iter().copied().filter(|&b| self.loose.is_resting(b)).collect();
        self.loose.take(&resting);
        for &b in &books {
            if self.state[b as usize] == BookState::Loose && !self.loose.is_resting(b) {
                self.loose.forget(b);
                self.tossed[b as usize] = false;
            }
            self.held.retain(|&h| h != b);
            self.credit_shelved(b);
        }
        self.active = self.active.min(self.held.len().saturating_sub(1));
        self.start_clocks();
        self.check_gates(audio);
    }

    fn cheat_go_to(&mut self, room: Option<usize>) {
        let (at, yaw) = match room {
            None => self.lib.spawn,
            Some(d) => {
                let r = &self.lib.rooms[d];
                let mid = (r.min + r.max) * 0.5;
                let into = (Vec2::new(mid.x - r.door.x, mid.y - r.door.z)).normalize_or_zero();
                let step = if self.dept_open(d) { 1.0 } else { -1.2 };
                let at = r.door + Vec3::new(into.x, 0.0, into.y) * step;
                (at, libm::atan2f(into.x, -into.y))
            }
        };
        self.player.pos = at;
        self.player.vel = Vec3::ZERO;
        self.player.yaw = yaw;
        self.player.pitch = -0.05;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shelving to a fraction fills every open department that far, lifting veils
    /// and filling the departments behind them, and pays lumens as play would.
    #[test]
    fn shelving_opens_the_library() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        g.cheat(Cheat::Shelve(0.6), &mut audio);
        assert!(g.lib.world.gates_open().iter().all(|&o| o), "every veil lifts at 60%");
        for d in 0..g.dept_total.len() {
            let frac = g.dept_shelved[d] as f32 / g.dept_total[d] as f32;
            assert!((0.6..0.61).contains(&frac), "department {d} at {frac}");
        }
        assert!(g.lumens >= g.total_shelved() as u64);
        assert_eq!(g.loose.heightfield_errors(), 0);
    }

    /// Finishing a room shelves all its books, from wherever they are, and finishes it
    /// and every side of its cases; its storey waits for the rest.
    #[test]
    fn finishing_a_room_finishes_its_cases() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        g.cheat(Cheat::Finish(Finish::Room(dept::CLOCKWORKS)), &mut audio);
        let d = dept::CLOCKWORKS;
        assert_eq!(g.dept_shelved[d], g.dept_total[d]);
        assert!(g.finished_since(Finish::Room(d)).is_some());
        let cats = g.lib.catalog.departments[d].categories.clone();
        for f in (0..g.lib.faces.len()).filter(|&f| cats.contains(&g.lib.faces[f].category)) {
            assert!(g.finished_since(Finish::Face(f)).is_some(), "face {f}");
        }
        assert!(g.finished_since(Finish::Storey(2)).is_none());
        assert!(g.finished_since(Finish::Room(0)).is_none());
        assert_eq!(g.loose.heightfield_errors(), 0);
        g.cheat(Cheat::Finish(Finish::Library), &mut audio);
        assert!(g.finished_since(Finish::Library).is_some());
        assert_eq!(g.total_shelved(), g.total_books());
    }

    /// Going to a veiled room leaves you in front of its veil, not shut in behind it.
    #[test]
    fn going_to_a_veiled_room_stays_outside() {
        for layout in Layout::ALL {
            let mut g = Game::new(layout);
            let last = g.lib.rooms.len() - 1;
            assert!(!g.dept_open(last));
            g.cheat_go_to(Some(last));
            assert_ne!(g.lib.room_at(g.player.pos), Some(last), "{layout:?}");
            g.cheat(Cheat::LiftVeils, &mut Audio::silent());
            g.cheat_go_to(Some(last));
            assert_eq!(g.lib.room_at(g.player.pos), Some(last), "{layout:?}");
        }
    }
}
