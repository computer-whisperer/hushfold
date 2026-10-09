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
        }
        self.dirty_save = true;
    }

    fn cheat_shelve(&mut self, frac: f32, audio: &mut Audio) {
        loop {
            let mut pools: Vec<Vec<u32>> = vec![Vec::new(); self.dept_total.len()];
            for b in 0..self.lib.books.len() as u32 {
                if self.state[b as usize] == BookState::Loose && self.loose.is_resting(b) {
                    pools[self.lib.dept_of(self.lib.category_of_book(b))].push(b);
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
            let open = |g: &Self| g.lib.world.gates_open().iter().filter(|&&o| o).count();
            let before = open(self);
            self.check_gates(audio);
            if open(self) == before {
                break;
            }
        }
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
                (at, into.x.atan2(-into.y))
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
