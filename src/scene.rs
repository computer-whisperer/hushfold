//! Where books are drawn from: books lying still (in piles or on shelves) and the
//! ghosts of empty slots are kept in chunks of space, each rebuilt and re-uploaded
//! only when something in it changes. Books on the move are drawn per frame.

use crate::game::BookState;
use crate::gfx::Inst;
use glam::Vec3;
use std::collections::HashMap;

/// Chunk size: across, up, across.
const SIZE: Vec3 = Vec3::new(8.0, 6.0, 8.0);
const NONE: u32 = u32::MAX;

pub struct Chunk {
    /// Bounds of everything drawn in it (books straddle its edges a little).
    pub min: Vec3,
    pub max: Vec3,
    /// Books lying still in it, and books whose slots are in it.
    books: Vec<u32>,
    pub ghosts: Vec<u32>,
    pub opaque: Vec<Inst>,
    pub trans: Vec<Inst>,
    /// Its instances need rebuilding; they need uploading.
    pub rebuild: bool,
    pub upload: bool,
}

/// What a book's place in a chunk was decided from.
#[derive(Clone, Copy, PartialEq)]
struct Drawn {
    state: BookState,
    version: u32,
    chunk: u32,
}

#[derive(Default)]
pub struct Scene {
    pub chunks: Vec<Chunk>,
    index: HashMap<(i32, i32, i32), u32>,
    drawn: Vec<Drawn>,
    /// The chunks holding each series' slots.
    series_chunks: Vec<Vec<u32>>,
    /// Series whose ghosts are drawn highlighted: (level, hovered).
    lit: HashMap<u32, (u32, bool)>,
    /// The book whose slot pulses (Sort Sense), and where its slot is.
    active: Option<(u32, Vec3)>,
}

impl Scene {
    /// Chunks for every slot; books are placed by `update`.
    pub fn new(slots: impl Iterator<Item = (u32, Vec3)>, series_of: impl Fn(u32) -> u32, series: usize) -> Self {
        let mut s = Scene {
            series_chunks: vec![Vec::new(); series],
            ..Default::default()
        };
        for (b, at) in slots {
            let c = s.chunk_at(at);
            s.chunks[c as usize].ghosts.push(b);
            let sc = &mut s.series_chunks[series_of(b) as usize];
            if !sc.contains(&c) {
                sc.push(c);
            }
            if s.drawn.len() <= b as usize {
                s.drawn.resize(
                    b as usize + 1,
                    Drawn {
                        state: BookState::Held,
                        version: u32::MAX,
                        chunk: NONE,
                    },
                );
            }
        }
        s
    }

    fn chunk_at(&mut self, p: Vec3) -> u32 {
        let k = (p / SIZE).floor();
        let key = (k.x as i32, k.y as i32, k.z as i32);
        if let Some(&c) = self.index.get(&key) {
            return c;
        }
        let c = self.chunks.len() as u32;
        let lo = k * SIZE;
        self.chunks.push(Chunk {
            min: lo - Vec3::splat(0.6),
            max: lo + SIZE + Vec3::splat(0.6),
            books: Vec::new(),
            ghosts: Vec::new(),
            opaque: Vec::new(),
            trans: Vec::new(),
            rebuild: true,
            upload: true,
        });
        self.index.insert(key, c);
        c
    }

    /// Note a book's state and Loose version; if they differ from what its chunk was
    /// built with, move it to the chunk it lies in now (`at`, if it lies still) and mark
    /// what changed. Returns whether its state changed (its ghost may need redrawing).
    pub fn place(&mut self, b: u32, state: BookState, version: u32, at: Option<Vec3>) -> bool {
        let d = self.drawn[b as usize];
        if d.state == state && d.version == version {
            return false;
        }
        let chunk = at.map_or(NONE, |p| self.chunk_at(p));
        if d.chunk != NONE {
            let c = &mut self.chunks[d.chunk as usize];
            if let Some(i) = c.books.iter().position(|&x| x == b) {
                c.books.swap_remove(i);
            }
            c.rebuild = true;
        }
        if chunk != NONE {
            let c = &mut self.chunks[chunk as usize];
            c.books.push(b);
            c.rebuild = true;
        }
        self.drawn[b as usize] = Drawn { state, version, chunk };
        d.state != state
    }

    /// Redraw the ghost of book `b`'s slot.
    pub fn ghost_changed(&mut self, slot: Vec3) {
        let c = self.chunk_at(slot);
        self.chunks[c as usize].rebuild = true;
    }

    /// Set which series' ghosts are highlighted; chunks whose highlights change are
    /// marked.
    pub fn highlight(&mut self, lit: HashMap<u32, (u32, bool)>) {
        if lit == self.lit {
            return;
        }
        let changed: Vec<u32> = (lit.iter())
            .filter(|(s, v)| self.lit.get(s) != Some(v))
            .map(|(&s, _)| s)
            .chain(self.lit.keys().filter(|s| !lit.contains_key(s)).copied())
            .collect();
        for s in changed {
            for &c in &self.series_chunks[s as usize] {
                self.chunks[c as usize].rebuild = true;
            }
        }
        self.lit = lit;
    }

    /// Set the book whose slot pulses.
    pub fn set_active(&mut self, active: Option<(u32, Vec3)>) {
        if active.map(|a| a.0) == self.active.map(|a| a.0) {
            return;
        }
        for (_, slot) in [self.active, active].into_iter().flatten() {
            self.ghost_changed(slot);
        }
        self.active = active;
    }

    pub fn active(&self) -> Option<u32> {
        self.active.map(|a| a.0)
    }

    /// How series `s`'s ghosts are drawn: (level, hovered).
    pub fn lit(&self, s: u32) -> (u32, bool) {
        self.lit.get(&s).copied().unwrap_or((0, false))
    }

    pub fn redraw_all(&mut self) {
        for c in &mut self.chunks {
            c.rebuild = true;
        }
    }

    /// The books lying still in chunk `c`.
    pub fn books(&self, c: usize) -> &[u32] {
        &self.chunks[c].books
    }
}
