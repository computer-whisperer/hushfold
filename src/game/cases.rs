//! Every bookcase side's moment when its last book goes home (`Finish::Face`): a ripple
//! runs out across it from that book, each book in turn easing out of its slot,
//! tipping forward a little and settling back as the ring passes, catching the light
//! at the crest; each shelf's label brightens as the ring reaches it. Afterwards the
//! side is as it was but for its labels, which glow a little warmer.
//!
//! Shelved books are drawn from the scene's chunks, rebuilt only when something in
//! them changes. While a side ripples, its books are held out of their chunks and
//! drawn here each frame, its chunks rebuilt as it begins and as it ends. A side
//! finished far from the eye keeps still.

use super::*;
use glam::Vec4;

/// When the ripple sets out after the last book goes home, how fast its ring runs out
/// across the side, and how long a book takes to come out and settle back as it
/// passes: how far out its foot comes, and how far it tips forward (radians).
const DELAY: f32 = 0.1;
const SPEED: f32 = 1.6;
const BUMP: f32 = 0.8;
const OUT: f32 = 0.012;
const TIP: f32 = 0.05;
/// How far the ring can have to run (corner to corner of a side), so when it is over.
const REACH: f32 = 3.6;
const MOMENT: f32 = DELAY + REACH / SPEED + BUMP;
/// How long a label takes to brighten and settle as the ring passes it.
const FLARE: f32 = 1.4;
/// The label's glow at the top of its flare, and for good after it, in its colour.
const FLARE_GLOW: f32 = 0.9;
const LIT: [f32; 4] = [1.0, 0.72, 0.34, 0.42];
/// The glint running across a side: how many sides at once have one (those nearest
/// the eye), how far out of the shelves it runs, how far it reaches, its colour and
/// its brightness.
const GLINTS: usize = 2;
const GLINT_OUT: f32 = 0.35;
const GLINT_REACH: f32 = 2.2;
const GLINT: [f32; 3] = [1.0, 0.74, 0.42];
const GLINT_GLOW: f32 = 0.85;
/// The middle of a side, up from its foot, and how far round that it reaches.
const MIDDLE: f32 = 1.4;
const SIDE_REACH: f32 = 1.9;
/// A side ripples if it is finished within this of the eye; its lit labels are drawn
/// to `LABELS_SEEN`, fading out over its last quarter into the case's own.
const RIPPLE_SEEN: f32 = 30.0;
const LABELS_SEEN: f32 = 14.0;

/// Each side's books, the one that came home last on each, and their ripples.
pub(super) struct Cases {
    /// Side `f`'s books are `books[first[f]..first[f + 1]]`.
    first: Vec<u32>,
    books: Vec<u32>,
    last: Vec<Option<u32>>,
    ripple: Vec<Ripple>,
    /// The sides rippling now.
    running: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum Ripple {
    /// Not finished yet.
    Waiting,
    /// Its books are drawn here, not from their chunks.
    Running,
    /// Over, or passed over (finished far off, or before a load).
    Over,
}

impl Cases {
    pub(super) fn new(lib: &Library) -> Cases {
        let faces = lib.faces.len();
        let mut first = vec![0u32; faces + 1];
        for b in &lib.books {
            first[b.face as usize + 1] += 1;
        }
        for f in 0..faces {
            first[f + 1] += first[f];
        }
        let mut next = first.clone();
        let mut books = vec![0; lib.books.len()];
        for (i, b) in lib.books.iter().enumerate() {
            let at = &mut next[b.face as usize];
            books[*at as usize] = i as u32;
            *at += 1;
        }
        Cases {
            first,
            books,
            last: vec![None; faces],
            ripple: vec![Ripple::Waiting; faces],
            running: Vec::new(),
        }
    }

    /// Book `b` has come home: the last on its side so far.
    pub(super) fn came_home(&mut self, lib: &Library, b: u32) {
        self.last[lib.books[b as usize].face as usize] = Some(b);
    }

    /// Whether book `b` is drawn here, rippling, rather than from its chunk.
    pub(super) fn holds(&self, lib: &Library, b: u32) -> bool {
        self.ripple[lib.books[b as usize].face as usize] == Ripple::Running
    }

    fn books(&self, f: usize) -> &[u32] {
        &self.books[self.first[f] as usize..self.first[f + 1] as usize]
    }
}

impl Game {
    /// Begin the ripples of sides finished near the eye and end those that are over,
    /// marking their chunks to be rebuilt without their books or with them again.
    pub(super) fn ripples(&mut self, scene: &mut Scene) {
        let eye = self.player.eye();
        let mut changed = Vec::new();
        for f in 0..self.lib.faces.len() {
            if self.cases.ripple[f] != Ripple::Waiting {
                continue;
            }
            let Some(since) = self.finished_since(Finish::Face(f)) else {
                continue;
            };
            let near = middle(&self.lib.faces[f]).distance(eye) < RIPPLE_SEEN;
            self.cases.ripple[f] = if since < MOMENT as f64 && near && !self.cases.books(f).is_empty() {
                self.cases.running.push(f);
                changed.push(f);
                Ripple::Running
            } else {
                Ripple::Over
            };
        }
        for i in (0..self.cases.running.len()).rev() {
            let f = self.cases.running[i];
            if self.finished_since(Finish::Face(f)).is_none_or(|s| s >= MOMENT as f64) {
                self.cases.running.swap_remove(i);
                self.cases.ripple[f] = Ripple::Over;
                changed.push(f);
            }
        }
        for f in changed {
            for &b in self.cases.books(f) {
                scene.redraw_at(self.lib.books[b as usize].slot_pos);
            }
        }
    }

    /// The rippling sides' books in view, and the finished sides' lit labels, into
    /// `insts`.
    pub(super) fn draw_cases(&self, planes: &[Vec4; 6], insts: &mut Vec<Inst>) {
        let eye = self.player.eye();
        for &f in &self.cases.running {
            let mid = middle(&self.lib.faces[f]);
            if gfx::ball_visible(planes, mid, SIDE_REACH) {
                self.ripple(f, self.ripple_time(f), insts);
            }
        }
        for f in 0..self.lib.faces.len() {
            if self.cases.ripple[f] == Ripple::Waiting {
                continue;
            }
            let mid = middle(&self.lib.faces[f]);
            let far = mid.distance(eye) - SIDE_REACH;
            let fade = (1.0 - (far / LABELS_SEEN - 0.75) * 4.0).min(1.0);
            if fade <= 0.0 || !gfx::ball_visible(planes, mid, SIDE_REACH) {
                continue;
            }
            let (from, t) = (self.ripple_from(f), self.ripple_time(f));
            for row in 0..ROWS {
                let mut label = self.lib.label_cover(f, row);
                let u = (t - Vec3::from(label.pos).distance(from) / SPEED) / FLARE;
                if u <= 0.0 {
                    continue;
                }
                let k = sound::smoothstep(u) * fade;
                let flare = bump(u) * bump(u) * FLARE_GLOW * fade;
                let c = label.color;
                label.color = [
                    c[0] + (LIT[0] - c[0]) * k,
                    c[1] + (LIT[1] - c[1]) * k,
                    c[2] + (LIT[2] - c[2]) * k,
                    c[3] + (LIT[3] - c[3]) * k + flare,
                ];
                insts.push(label);
            }
        }
    }

    /// The glints running across the rippling sides nearest the eye: each a soft warm
    /// light riding the ring out from where it set out toward the side's furthest
    /// book, a little out from the shelves, as (where, reach, colour, brightness).
    pub(super) fn case_lights(&self) -> Vec<(Vec3, f32, [f32; 3], f32)> {
        let eye = self.player.eye();
        let mut near: Vec<(f32, usize)> = (self.cases.running.iter())
            .map(|&f| (middle(&self.lib.faces[f]).distance(eye), f))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        near.truncate(GLINTS);
        let mut lights = Vec::with_capacity(near.len());
        for (_, f) in near {
            let (from, t) = (self.ripple_from(f), self.ripple_time(f));
            let far = (self.cases.books(f).iter())
                .map(|&b| self.lib.books[b as usize].slot_pos)
                .max_by(|a, b| a.distance(from).total_cmp(&b.distance(from)))
                .unwrap_or(from);
            let run = far.distance(from).max(0.5);
            let s = t * SPEED / run;
            if !(0.0..1.0).contains(&s) {
                continue;
            }
            let at = from.lerp(far, s) + self.lib.faces[f].normal * GLINT_OUT;
            let fade = sound::smoothstep(s * 6.0) * sound::smoothstep((1.0 - s) * 3.0);
            lights.push((at, GLINT_REACH, GLINT, GLINT_GLOW * fade));
        }
        lights
    }

    /// Where side `f`'s ripple sets out from: the book that finished it, or its middle
    /// (a side its category's books didn't reach).
    fn ripple_from(&self, f: usize) -> Vec3 {
        match self.cases.last[f] {
            Some(b) => self.lib.books[b as usize].slot_pos,
            None => middle(&self.lib.faces[f]),
        }
    }

    /// Seconds since side `f`'s ripple set out (long ago if it was finished before a
    /// load).
    fn ripple_time(&self, f: usize) -> f32 {
        self.finished_since(Finish::Face(f)).map_or(0.0, |s| s as f32 - DELAY)
    }

    /// Side `f`'s books `t` seconds into its ripple: each at rest in its slot, or as
    /// the ring passes it, out a little, tipped forward about its spine's foot and lit
    /// at the crest.
    fn ripple(&self, f: usize, t: f32, insts: &mut Vec<Inst>) {
        let (from, face) = (self.ripple_from(f), &self.lib.faces[f]);
        for &b in self.cases.books(f) {
            let info = &self.lib.books[b as usize];
            let lag = (hash(b, 1) - 0.5) * 0.12;
            let u = (t - lag - info.slot_pos.distance(from) / SPEED) / BUMP;
            if u <= 0.0 || u >= 1.0 {
                insts.push(self.book_inst(b as usize, info.slot_pos, info.slot_rot, false));
                continue;
            }
            let k = bump(u) * (0.75 + 0.5 * hash(b, 2));
            let foot = info.slot_pos + face.normal * (info.size.z * 0.5) - Vec3::Y * (info.size.y * 0.5);
            let tip = Quat::from_axis_angle(face.right, TIP * k);
            let at = foot + tip * (info.slot_pos - foot) + face.normal * (OUT * k);
            let mut i = self.book_inst(b as usize, at, tip * info.slot_rot, false);
            let glint = k * k * k;
            let [r, g, bl, s] = i.color;
            i.color = [
                r * (1.0 + 1.3 * glint) + 0.02 * glint,
                g * (1.0 + 1.0 * glint) + 0.014 * glint,
                bl * (1.0 + 0.7 * glint) + 0.006 * glint,
                s,
            ];
            insts.push(i);
        }
    }
}

/// The middle of a side, at about the height of its middle shelf.
fn middle(face: &Face) -> Vec3 {
    face.center + Vec3::Y * MIDDLE
}

/// Out and back over `u` from 0 to 1, easing at both ends and at the top.
fn bump(u: f32) -> f32 {
    let s = libm::sinf(PI * u.clamp(0.0, 1.0));
    s * s
}

/// A number in 0..1 picked by book `b` and `k`, the same each time.
fn hash(b: u32, k: u32) -> f32 {
    let mut x = b.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Books drawn from the chunks, and books lying still in them.
    fn chunked(g: &Game) -> (usize, usize) {
        let s = &g.scene;
        let drawn = s.chunks.iter().map(|c| c.opaque.len()).sum();
        (drawn, (0..s.chunks.len()).map(|c| s.books(c).len()).sum())
    }

    /// A side finished in play ripples: its books leave their chunks and are drawn each
    /// frame, never far from their slots, and once the moment is over they are back in
    /// their chunks just as they were. One finished before a load stays still.
    #[test]
    fn a_finished_side_ripples_and_is_as_it_was() {
        let mut g = Game::new(Layout::Hall);
        let mut audio = Audio::silent();
        let f = 0;
        let (c, n) = (g.lib.faces[f].center, g.lib.faces[f].normal);
        g.look_at(c + n * 2.5, c + Vec3::Y * 1.4);
        g.build_instances(1.6);
        // What a chunk draws: where and how.
        let look = |i: &Inst| (i.pos, i.rot, i.scale, i.color, i.param);
        let chunks = |g: &Game| -> Vec<_> { g.scene.chunks.iter().flat_map(|c| c.opaque.iter().map(look)).collect() };
        let before = chunks(&g);
        g.cheat(Cheat::Finish(Finish::Face(f)), &mut audio);
        let books = g.cases.books(f).to_vec();
        let mut moved = 0;
        for k in 0..((MOMENT + 0.5) * 30.0) as usize {
            g.clock += 1.0 / 30.0;
            g.build_instances(1.6);
            let (drawn, still) = chunked(&g);
            if (k as f32) < MOMENT * 30.0 - 1.0 {
                assert_eq!(drawn + books.len(), still, "held out at {k}");
                let mut drawn = Vec::new();
                g.draw_cases(&gfx::frustum(g.view_proj(1.6)), &mut drawn);
                let rippling: Vec<&Inst> = drawn.iter().filter(|i| i.kind == kind::BOOK).collect();
                assert_eq!(rippling.len(), books.len());
                for (i, &b) in rippling.iter().zip(&books) {
                    let off = Vec3::from(i.pos).distance(g.lib.books[b as usize].slot_pos);
                    assert!(off < OUT * 1.25 + TIP * 1.25 * 0.3, "book {b} {off} m out");
                    moved += (off > 1e-4) as usize;
                }
            } else if (k as f32) > MOMENT * 30.0 + 1.0 {
                assert_eq!(drawn, still, "put back at {k}");
            }
        }
        assert!(moved > books.len(), "{moved} rippled");
        let after = chunks(&g);
        for &b in &books {
            let info = &g.lib.books[b as usize];
            let i = look(&g.book_inst(b as usize, info.slot_pos, info.slot_rot, false));
            assert!(after.contains(&i) && !before.contains(&i), "book {b} in its slot");
        }

        let mut g = Game::new(Layout::Hall);
        for &b in &books {
            g.state[b as usize] = BookState::Shelved;
            g.count_shelved(b, g.clock - LONG_AGO);
        }
        g.look_at(c + n * 2.5, c + Vec3::Y * 1.4);
        g.build_instances(1.6);
        let (drawn, still) = chunked(&g);
        assert_eq!(drawn, still, "a side finished before loading is still");
        assert!(g.cases.running.is_empty());
    }
}
