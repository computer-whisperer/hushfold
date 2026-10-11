//! When each part of the library was finished (every book that belongs in it home):
//! the sides of its bookcases, its categories, rooms and storeys, and the whole of
//! it; and the playing of what generation laid down to wait for those
//! (`library::celebrate`): movers are posed from it (`motion`), and sparks and sounds
//! come from here. It all follows from what is shelved, so saves don't store it:
//! what was finished before a load counts as finished long ago, its lasting change in
//! place and its moment past.

use super::*;

/// How long before a load what was already finished counts as finished: long enough
/// for any change to be over.
pub(super) const LONG_AGO: f64 = 1.0e5;
/// Lasting sparks come only within this of the eye, a moment's within MOMENT_SEEN,
/// and there are never more than MAX_PARTICLES in the air.
const LASTING_SEEN: f32 = 30.0;
const MOMENT_SEEN: f32 = 60.0;
const MAX_PARTICLES: usize = 3000;

/// When each part of the library was finished (on the game's clock), if it is.
pub(super) struct Finished {
    face: Vec<Option<f64>>,
    category: Vec<Option<f64>>,
    room: Vec<Option<f64>>,
    storey: Vec<Option<f64>>,
    library: Option<f64>,
    /// How many books belong on each bookcase side, and how many are home.
    face_books: Vec<u32>,
    face_shelved: Vec<u32>,
    /// Sparks owed but not yet sent (a fraction of one), by source.
    owed: Vec<f32>,
    /// The clock up to which waiting sounds have been played.
    cued_to: f64,
}

impl Finished {
    pub(super) fn new(lib: &Library) -> Finished {
        let mut face_books = vec![0; lib.faces.len()];
        for b in &lib.books {
            face_books[b.face as usize] += 1;
        }
        Finished {
            face: vec![None; lib.faces.len()],
            category: vec![None; lib.catalog.categories.len()],
            room: vec![None; lib.rooms.len()],
            storey: vec![None; lib.storeys()],
            library: None,
            face_shelved: vec![0; face_books.len()],
            face_books,
            owed: vec![0.0; lib.sparks.len()],
            cued_to: 0.0,
        }
    }

    /// When `f` was finished, if it is.
    pub(super) fn when(&self, f: Finish) -> Option<f64> {
        match f {
            Finish::Face(i) => self.face.get(i).copied().flatten(),
            Finish::Category(c) => self.category.get(c).copied().flatten(),
            Finish::Room(d) => self.room.get(d).copied().flatten(),
            Finish::Storey(l) => self.storey.get(l).copied().flatten(),
            Finish::Library => self.library,
        }
    }
}

/// What one book coming home finished.
#[derive(Clone, Copy, Default)]
pub(super) struct Filled {
    pub series: bool,
    pub category: bool,
    pub room: bool,
    pub storey: bool,
    pub library: bool,
}

impl Game {
    /// Seconds since `f` was finished, if it is.
    pub fn finished_since(&self, f: Finish) -> Option<f64> {
        self.finished.when(f).map(|t| self.clock - t)
    }

    /// Count book `b` home, at `at` on the clock (`LONG_AGO` before it, loading): in
    /// its series, category, department and bookcase side, and what that finishes.
    pub(super) fn count_shelved(&mut self, b: u32, at: f64) -> Filled {
        let info = &self.lib.books[b as usize];
        let (s, f) = (info.series as usize, info.face as usize);
        let cat = self.lib.series[s].category;
        let d = self.lib.dept_of(cat);
        self.series_shelved[s] += 1;
        self.category_shelved[cat] += 1;
        self.dept_shelved[d] += 1;
        let fin = &mut self.finished;
        fin.face_shelved[f] += 1;
        let mut filled = Filled {
            series: self.series_shelved[s] == self.lib.series[s].volumes,
            ..Filled::default()
        };
        if fin.face_shelved[f] == fin.face_books[f] {
            fin.face[f] = Some(at);
        }
        if self.category_shelved[cat] == self.lib.category_books[cat] {
            filled.category = true;
            fin.category[cat] = Some(at);
            // A side its books didn't reach is finished with them.
            for g in self.lib.faces_of(cat) {
                if fin.face_books[g] == 0 {
                    fin.face[g] = Some(at);
                }
            }
        }
        if self.dept_shelved[d] == self.dept_total[d] {
            filled.room = true;
            fin.room[d] = Some(at);
            let l = self.lib.rooms[d].level;
            let rooms = &self.lib.rooms;
            if (0..rooms.len()).all(|e| rooms[e].level != l || fin.room[e].is_some()) {
                filled.storey = true;
                fin.storey[l] = Some(at);
                if fin.storey.iter().all(Option::is_some) {
                    filled.library = true;
                    fin.library = Some(at);
                }
            }
        }
        filled
    }

    /// Play what waits for the parts of the library that are finished: the sounds
    /// whose time came since the last frame, and sparks.
    pub(super) fn celebrate(&mut self, dt: f32, audio: &mut Audio) {
        let (from, to) = (self.finished.cued_to, self.clock);
        self.finished.cued_to = to;
        for c in &self.lib.cues {
            if let Some(t) = self.finished.when(c.finish)
                && (from..to).contains(&(t + c.delay as f64))
            {
                audio.play_at(c.sound, c.volume, c.at);
            }
        }
        let eye = self.player.eye();
        for (i, sp) in self.lib.sparks.iter().enumerate() {
            let s = self.finished.when(sp.finish).map(|t| self.clock - t - sp.delay as f64);
            let seen = if sp.lasts.is_some() { MOMENT_SEEN } else { LASTING_SEEN };
            let owed = &mut self.finished.owed[i];
            let Some(s) = s else { continue };
            if s < 0.0 || sp.lasts.is_some_and(|l| s >= l as f64) || sp.at.distance(eye) > seen + sp.spread.length() {
                *owed = 0.0;
                continue;
            }
            *owed += sp.rate * dt;
            while *owed >= 1.0 {
                *owed -= 1.0;
                if self.particles.len() < MAX_PARTICLES {
                    self.particles.push(spark(sp, &mut self.rng));
                }
            }
        }
    }
}

/// Up to `v` either way along each axis, at random.
fn jitter(rng: &mut StdRng, v: Vec3) -> Vec3 {
    v * Vec3::new(
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
        rng.gen_range(-1.0..=1.0),
    )
}

/// One spark from `sp`.
fn spark(sp: &Sparks, rng: &mut StdRng) -> Particle {
    let (at, vel) = (sp.at + jitter(rng, sp.spread), sp.vel + jitter(rng, sp.jitter));
    let (rot, spin) = if sp.kind == kind::EMISSIVE {
        (Quat::IDENTITY, Vec3::ZERO)
    } else {
        let axis = jitter(rng, Vec3::ONE).normalize_or(Vec3::Y);
        (
            Quat::from_scaled_axis(jitter(rng, Vec3::splat(3.0))),
            axis * rng.gen_range(1.5..4.0),
        )
    };
    Particle {
        pos: at,
        vel,
        life: 0.0,
        max: sp.life * rng.gen_range(0.7..1.2),
        kind: sp.kind,
        rgba: sp.rgba,
        size: sp.size * rng.gen_range(0.7..1.3),
        rot,
        spin,
        lift: sp.lift,
        drag: sp.drag,
        wander: sp.wander,
        rise: if sp.lasts.is_some() {
            0.0
        } else {
            (sp.life * 0.2).min(0.6)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What was finished before a load is long past: its moment's sparks don't come,
    /// its lasting ones do. One finished in play has its moment.
    #[test]
    fn a_loaded_finish_has_no_moment() {
        let mut g = Game::new(Layout::Great);
        let mut audio = Audio::silent();
        // The Observatory and the Alchemical Wing.
        let (loaded, live) = (0, 1);
        for d in [loaded, live] {
            let at = g.lib.rooms[d].drop + Vec3::Y;
            g.lib.sparks.push(Sparks::burst(Finish::Room(d), at));
            g.lib.sparks.push(Sparks::motes(Finish::Room(d), at, Vec3::ONE));
        }
        g.finished = Finished::new(&g.lib);
        g.player.pos = g.lib.rooms[loaded].drop;
        let books: Vec<u32> = (0..g.lib.books.len() as u32)
            .filter(|&b| g.lib.dept_of_book(b) == loaded)
            .collect();
        for b in books {
            g.state[b as usize] = BookState::Shelved;
            g.count_shelved(b, g.clock - LONG_AGO);
        }
        assert!(g.finished_since(Finish::Room(loaded)).is_some_and(|s| s >= LONG_AGO));
        let run = |g: &mut Game, audio: &mut Audio| {
            g.particles.clear();
            for _ in 0..60 {
                g.clock += 1.0 / 60.0;
                g.celebrate(1.0 / 60.0, audio);
            }
            g.particles.iter().filter(|p| p.rise == 0.0).count()
        };
        assert_eq!(run(&mut g, &mut audio), 0, "a loaded finish's moment");
        assert!(!g.particles.is_empty(), "a loaded finish's lasting sparks");
        g.cheat(Cheat::Finish(Finish::Room(live)), &mut audio);
        g.player.pos = g.lib.rooms[live].drop;
        assert!(run(&mut g, &mut audio) > 30, "a finish in play has its moment");
    }
}
