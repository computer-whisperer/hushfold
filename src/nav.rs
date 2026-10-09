//! Walking distances around the furniture and between floors: a coarse grid per
//! level, linked where stairs meet a floor, and a distance field from the player, so
//! "nearest" means nearest on foot rather than through the back of a bookcase or
//! the floor above.

use crate::world::{BODY_R, STEP_UP, Stops, World, in_body};
use glam::{Vec2, Vec3};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

const CELL: f32 = 0.5;
/// Clearance the grid keeps from furniture (a body's radius).
const CLEARANCE: f32 = BODY_R;
/// Clearance for straight-segment tests; smaller, so a player standing flush
/// against a case never counts as inside it.
const SEG_CLEARANCE: f32 = 0.15;
/// Grid step costs: orthogonal and diagonal, in tenths of a cell.
const STEP: u32 = 10;
const DIAG: u32 = 14;
/// How far round the player (in cells) the field is seeded with straight-line costs.
const SEED_CELLS: i32 = 3;
const UNREACHED: u32 = u32::MAX;

/// How to walk from the player to a target.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    /// The farthest point along the route that is in a straight, clear line from
    /// the player (the target itself when `direct`), on the floor.
    pub waypoint: Vec3,
    /// Walking distance to the target, in metres.
    pub dist: f32,
    pub direct: bool,
}

/// One level's grid.
struct Layer {
    min: Vec2,
    w: usize,
    h: usize,
    /// Height of the floor or stair at each cell's centre (NaN: none). Neighbours
    /// more than `STEP_UP` apart are a ledge (or the floor above), not a walk.
    surf: Vec<f32>,
    /// Furniture footprints standing on this level (min, max) in xz.
    boxes: Vec<(Vec2, Vec2)>,
    /// Cells too close to furniture, walls or edges to stand in.
    blocked: Vec<bool>,
    /// Cells with a piece of furniture over their centre (rooms, not corridors).
    solid: Vec<bool>,
}

impl Layer {
    fn center(&self, i: usize) -> Vec2 {
        let (x, z) = (i % self.w, i / self.w);
        Vec2::new(
            self.min.x + (x as f32 + 0.5) * CELL,
            self.min.y + (z as f32 + 0.5) * CELL,
        )
    }

    fn cell(&self, p: Vec2) -> usize {
        let x = (((p.x - self.min.x) / CELL) as usize).min(self.w - 1);
        let z = (((p.y - self.min.y) / CELL) as usize).min(self.h - 1);
        z * self.w + x
    }
}

pub struct Nav {
    layers: Vec<Layer>,
    /// Node index of each layer's first cell.
    first: Vec<usize>,
    /// Where stairs meet a floor: node → (node, cost).
    links: HashMap<u32, Vec<(u32, u32)>>,
    /// Every gate: (level, min, max) in xz, and whether it is closed.
    gates: Vec<(usize, Vec2, Vec2)>,
    closed: Vec<bool>,
    /// Nodes next to a closed gate.
    gated: Vec<bool>,
    /// Nodes reachable from the start without passing a closed gate.
    region: Vec<bool>,
    start: Vec3,
    /// Distance from the source node in tenths of a cell; UNREACHED otherwise.
    dist: Vec<u32>,
    prev: Vec<u32>,
    source: Option<usize>,
}

impl Nav {
    /// The grid for `world` (its gates as they stand), routes measured from `start`
    /// until the first `update`.
    pub fn new(world: &World, start: Vec3) -> Self {
        let mut layers = Vec::new();
        let mut first = Vec::new();
        let mut nodes = 0;
        for (l, lv) in world.levels.iter().enumerate() {
            let size = lv.max - lv.min;
            let (w, h) = ((size.x / CELL).ceil() as usize, (size.y / CELL).ceil() as usize);
            let mut layer = Layer {
                min: lv.min,
                w,
                h,
                surf: vec![f32::NAN; w * h],
                boxes: Vec::new(),
                blocked: vec![false; w * h],
                solid: vec![false; w * h],
            };
            for i in 0..w * h {
                if let Some(y) = world.surfaces(layer.center(i), Some(l)).max_by(f32::total_cmp) {
                    layer.surf[i] = y;
                }
            }
            let solids: Vec<_> = (world.solids_on(l)).filter(|o| o.stops.has(Stops::BODIES)).collect();
            layer.boxes = solids
                .iter()
                .map(|o| (Vec2::new(o.min.x, o.min.z), Vec2::new(o.max.x, o.max.z)))
                .collect();
            for i in 0..w * h {
                let c = layer.center(i);
                let s = layer.surf[i];
                let wall = c.x < lv.min.x + CLEARANCE
                    || c.x > lv.max.x - CLEARANCE
                    || c.y < lv.min.y + CLEARANCE
                    || c.y > lv.max.y - CLEARANCE;
                // Furniture the body would walk into from this cell's floor.
                let near = |pad: f32| {
                    solids.iter().filter(|o| in_body(o.min, o.max, s)).any(|o| {
                        c.x > o.min.x - pad && c.x < o.max.x + pad && c.y > o.min.z - pad && c.y < o.max.z + pad
                    })
                };
                layer.solid[i] = s.is_nan() || near(0.0);
                layer.blocked[i] = s.is_nan() || wall || near(CLEARANCE);
            }
            first.push(nodes);
            nodes += w * h;
            layers.push(layer);
        }
        // Stairs: the top of a flight on one level steps onto the floor of the next.
        let mut links: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
        for l in 0..layers.len().saturating_sub(1) {
            let (lo, up) = (&layers[l], &layers[l + 1]);
            for i in 0..lo.w * lo.h {
                let s = lo.surf[i];
                if s.is_nan() || lo.blocked[i] {
                    continue;
                }
                let c = lo.center(i);
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        let q = c + Vec2::new(dx as f32, dz as f32) * CELL;
                        let j = up.cell(q);
                        if up.center(j).distance(q) > 1e-3 || up.blocked[j] || (up.surf[j] - s).abs() > STEP_UP {
                            continue;
                        }
                        let cost = if dx != 0 && dz != 0 { DIAG } else { STEP };
                        let (a, b) = ((first[l] + i) as u32, (first[l + 1] + j) as u32);
                        links.entry(a).or_default().push((b, cost));
                        links.entry(b).or_default().push((a, cost));
                    }
                }
            }
        }
        let gates = (world.gates().iter())
            .map(|g| (g.level, Vec2::new(g.min.x, g.min.z), Vec2::new(g.max.x, g.max.z)))
            .collect::<Vec<_>>();
        let mut nav = Nav {
            closed: world.gates_open().iter().map(|&open| !open).collect(),
            gates,
            gated: vec![false; nodes],
            region: vec![false; nodes],
            start,
            layers,
            first,
            links,
            dist: vec![UNREACHED; nodes],
            prev: vec![UNREACHED; nodes],
            source: None,
        };
        nav.refresh_gates();
        nav
    }

    /// Open or close gate `i`.
    pub fn set_gate(&mut self, i: usize, closed: bool) {
        if self.closed[i] != closed {
            self.closed[i] = closed;
            self.refresh_gates();
        }
    }

    fn refresh_gates(&mut self) {
        self.source = None;
        self.gated.fill(false);
        for (g, &(l, mn, mx)) in self.gates.iter().enumerate() {
            if !self.closed[g] {
                continue;
            }
            let layer = &self.layers[l];
            for i in 0..layer.w * layer.h {
                let c = layer.center(i);
                if c.cmpgt(mn - CLEARANCE).all() && c.cmplt(mx + CLEARANCE).all() {
                    self.gated[self.first[l] + i] = true;
                }
            }
        }
        // What can be walked to from the start, ignoring furniture.
        self.region.fill(false);
        let Some(s) = self.locate(self.start) else { return };
        let mut stack = vec![s];
        self.region[s] = true;
        while let Some(n) = stack.pop() {
            for (m, _) in self.neighbours(n, |nav, m| !nav.layer_of(m).0.solid[nav.layer_of(m).1]) {
                if !self.region[m] {
                    self.region[m] = true;
                    stack.push(m);
                }
            }
        }
    }

    fn layer_of(&self, node: usize) -> (&Layer, usize) {
        let l = self.first.partition_point(|&f| f <= node) - 1;
        (&self.layers[l], node - self.first[l])
    }

    fn level_of_node(&self, node: usize) -> usize {
        self.first.partition_point(|&f| f <= node) - 1
    }

    fn free(&self, node: usize) -> bool {
        let (layer, i) = self.layer_of(node);
        !layer.blocked[i] && !self.gated[node]
    }

    /// Walkable neighbours of `node` (passing `ok`, never through a closed gate), with
    /// step costs: the 8 around it on its level (no cutting corners, no ledges), and
    /// stairs to the next level.
    fn neighbours(&self, node: usize, ok: impl Fn(&Self, usize) -> bool) -> Vec<(usize, u32)> {
        let l = self.level_of_node(node);
        let layer = &self.layers[l];
        let i = node - self.first[l];
        let (w, h) = (layer.w as i32, layer.h as i32);
        let (x, z) = ((i % layer.w) as i32, (i / layer.w) as i32);
        let s = layer.surf[i];
        let pass = |m: usize| ok(self, m) && !self.gated[m];
        let mut out = Vec::with_capacity(8);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, nz) = (x + dx, z + dz);
            if nx < 0 || nz < 0 || nx >= w || nz >= h {
                continue;
            }
            let j = (nz * w + nx) as usize;
            let m = self.first[l] + j;
            if !pass(m) || (layer.surf[j] - s).abs() > STEP_UP {
                continue;
            }
            let cost = if dx != 0 && dz != 0 {
                // No cutting corners past furniture.
                let (a, b) = ((z * w + nx) as usize, (nz * w + x) as usize);
                if !pass(self.first[l] + a) || !pass(self.first[l] + b) {
                    continue;
                }
                DIAG
            } else {
                STEP
            };
            out.push((m, cost));
        }
        if let Some(ls) = self.links.get(&(node as u32)) {
            for &(m, cost) in ls {
                if pass(m as usize) {
                    out.push((m as usize, cost));
                }
            }
        }
        out
    }

    /// The node `p` stands in: the highest floor or stair under it within reach.
    fn locate(&self, p: Vec3) -> Option<usize> {
        let at = Vec2::new(p.x, p.z);
        (0..self.layers.len())
            .filter_map(|l| {
                let layer = &self.layers[l];
                let i = layer.cell(at);
                let s = layer.surf[i];
                (!s.is_nan() && s <= p.y + 0.5).then_some((s, self.first[l] + i))
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, n)| n)
    }

    fn node_pos(&self, node: usize) -> Vec3 {
        let (layer, i) = self.layer_of(node);
        let c = layer.center(i);
        Vec3::new(c.x, layer.surf[i], c.y)
    }

    /// True if a straight walk from `a` to `b` stays on one floor (no ledges) and
    /// touches no furniture or closed gate.
    pub fn clear(&self, a: Vec3, b: Vec3) -> bool {
        let (Some(na), Some(nb)) = (self.locate(a), self.locate(b)) else {
            return false;
        };
        let l = self.level_of_node(na);
        if self.level_of_node(nb) != l {
            return false;
        }
        let layer = &self.layers[l];
        let (a2, b2) = (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z));
        let d = b2 - a2;
        let inv = Vec2::new(1.0 / d.x, 1.0 / d.y);
        let hits = |mn: Vec2, mx: Vec2, pad: f32| {
            let t1 = (mn - Vec2::splat(pad) - a2) * inv;
            let t2 = (mx + Vec2::splat(pad) - a2) * inv;
            let tmin = t1.min(t2).max_element().max(0.0);
            let tmax = t1.max(t2).min_element().min(1.0);
            tmax >= tmin
        };
        let gate = (self.gates.iter().zip(&self.closed)).any(|(&(gl, mn, mx), &c)| c && gl == l && hits(mn, mx, 0.0));
        if gate || layer.boxes.iter().any(|&(mn, mx)| hits(mn, mx, SEG_CLEARANCE)) {
            return false;
        }
        // No ledges or holes along the way.
        let steps = (d.length() / (CELL * 0.5)).ceil().max(1.0) as usize;
        let mut last = layer.surf[layer.cell(a2)];
        for k in 1..=steps {
            let s = layer.surf[layer.cell(a2 + d * (k as f32 / steps as f32))];
            if s.is_nan() || (s - last).abs() > STEP_UP {
                return false;
            }
            last = s;
        }
        true
    }

    /// True if `p` lies where the player can walk to (or reach from the floor
    /// nearby) without passing a closed gate.
    pub fn open_at(&self, p: Vec3) -> bool {
        let Some(n) = self.locate(p) else { return false };
        let (layer, i) = self.layer_of(n);
        let (x, z) = ((i % layer.w) as i32, (i / layer.w) as i32);
        (-3..=3).any(|dz| {
            (-3..=3).any(|dx| {
                let (nx, nz) = (x + dx, z + dz);
                nx >= 0
                    && nz >= 0
                    && nx < layer.w as i32
                    && nz < layer.h as i32
                    && self.region[n - i + (nz as usize * layer.w + nx as usize)]
            })
        })
    }

    /// The middle of the free cell nearest `p` on its floor (where a player would
    /// stand to reach something on a table, say), or `p` if none is near.
    #[cfg(test)]
    pub fn standable_near(&self, p: Vec3) -> Vec3 {
        let Some(n) = self.locate(p) else { return p };
        let (layer, i) = self.layer_of(n);
        let (x, z) = ((i % layer.w) as i32, (i / layer.w) as i32);
        let at = Vec2::new(p.x, p.z);
        let mut best: Option<(f32, usize)> = None;
        for dz in -6..=6 {
            for dx in -6..=6 {
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= layer.w as i32 || nz >= layer.h as i32 {
                    continue;
                }
                let m = n - i + nz as usize * layer.w + nx as usize;
                let d = layer.center(m - (n - i)).distance(at);
                if self.free(m) && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, m));
                }
            }
        }
        best.map_or(p, |(_, m)| self.node_pos(m))
    }

    /// Recompute the distance field if the player has changed cell (or a gate has
    /// changed).
    pub fn update(&mut self, from: Vec3) {
        let Some(src) = self.locate(from) else { return };
        if self.source == Some(src) {
            return;
        }
        self.source = Some(src);
        self.dist.fill(UNREACHED);
        self.prev.fill(UNREACHED);
        let mut heap = BinaryHeap::new();
        // Seed every free cell nearby that the player can walk to in a straight line, at
        // its true distance. The player's own cell may lie inside furniture clearance
        // (between two chairs, say) with no free neighbours; the seeds get past that.
        self.dist[src] = 0;
        heap.push(Reverse((0u32, src as u32)));
        let l = self.level_of_node(src);
        let layer = &self.layers[l];
        let i = src - self.first[l];
        let (w, h) = (layer.w as i32, layer.h as i32);
        let (sx, sz) = ((i % layer.w) as i32, (i / layer.w) as i32);
        let p = Vec2::new(from.x, from.z);
        for dz in -SEED_CELLS..=SEED_CELLS {
            for dx in -SEED_CELLS..=SEED_CELLS {
                let (nx, nz) = (sx + dx, sz + dz);
                if nx < 0 || nz < 0 || nx >= w || nz >= h {
                    continue;
                }
                let n = self.first[l] + (nz * w + nx) as usize;
                if n == src || !self.free(n) || !self.clear(from, self.node_pos(n)) {
                    continue;
                }
                let c = self.layers[l].center(n - self.first[l]);
                let d = (p.distance(c) / CELL * STEP as f32).round() as u32;
                self.dist[n] = d;
                heap.push(Reverse((d, n as u32)));
            }
        }
        while let Some(Reverse((d, n))) = heap.pop() {
            let n = n as usize;
            if d > self.dist[n] {
                continue;
            }
            for (m, cost) in self.neighbours(n, |nav, m| nav.free(m)) {
                let nd = d + cost;
                if nd < self.dist[m] {
                    self.dist[m] = nd;
                    self.prev[m] = n as u32;
                    heap.push(Reverse((nd, m as u32)));
                }
            }
        }
    }

    /// The reached node nearest `p` (its own cell, or a neighbour if that one is blocked).
    fn reached_near(&self, p: Vec3) -> Option<usize> {
        let n = self.locate(p)?;
        if self.dist[n] != UNREACHED {
            return Some(n);
        }
        let (layer, i) = self.layer_of(n);
        let base = n - i;
        let at = Vec2::new(p.x, p.z);
        let (x, z) = ((i % layer.w) as i32, (i / layer.w) as i32);
        let mut best: Option<(f32, usize)> = None;
        for dz in -2..=2 {
            for dx in -2..=2 {
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= layer.w as i32 || nz >= layer.h as i32 {
                    continue;
                }
                let j = nz as usize * layer.w + nx as usize;
                if self.dist[base + j] == UNREACHED {
                    continue;
                }
                let d = self.dist[base + j] as f32 * CELL / STEP as f32 + layer.center(j).distance(at);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, base + j));
                }
            }
        }
        best.map(|(_, n)| n)
    }

    fn field_distance(&self, node: usize, to: Vec3) -> f32 {
        let c = self.node_pos(node);
        self.dist[node] as f32 * CELL / STEP as f32 + Vec2::new(c.x, c.z).distance(Vec2::new(to.x, to.z))
    }

    /// Walking distance from `from` (the field's source) to `to`; infinite if unreachable.
    pub fn distance(&self, from: Vec3, to: Vec3) -> f32 {
        if self.clear(from, to) {
            return Vec2::new(from.x, from.z).distance(Vec2::new(to.x, to.z));
        }
        match self.reached_near(to) {
            Some(n) => self.field_distance(n, to),
            None => f32::INFINITY,
        }
    }

    /// The route from `from` (the field's source) to `to`.
    pub fn route(&self, from: Vec3, to: Vec3) -> Option<Route> {
        if self.clear(from, to) {
            return Some(Route {
                waypoint: to,
                dist: Vec2::new(from.x, from.z).distance(Vec2::new(to.x, to.z)),
                direct: true,
            });
        }
        let end = self.reached_near(to)?;
        let dist = self.field_distance(end, to);
        // Walk back toward the player; the first node in plain view is the farthest
        // visible point along the route.
        let mut n = end;
        let mut waypoint = self.node_pos(end);
        while self.prev[n] != UNREACHED {
            let c = self.node_pos(n);
            if self.clear(from, c) {
                waypoint = c;
                break;
            }
            n = self.prev[n] as usize;
            waypoint = self.node_pos(n);
        }
        Some(Route {
            waypoint,
            dist,
            direct: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;

    /// Tight spots a player can stand in (between two chairs, in a hall corner, beside
    /// the hearth) used to leave the field stuck in the player's own cell.
    #[test]
    fn tight_spots_still_reach_the_hall() {
        let lib = Library::generate(crate::library::Layout::Hall, crate::game::SEED);
        let mut nav = Nav::new(&lib.world, lib.spawn.0);
        nav.set_gate(0, false);
        let open_floor = Vec3::new(0.0, 0.0, 5.0);
        let spots = [
            (-4.5, -1.6),
            (4.5, 1.6),
            (-4.5, 3.6),
            (23.6, 17.6),
            (-23.6, -17.6),
            (2.9, 17.5),
            (-2.9, 17.5),
        ];
        for (x, z) in spots {
            let p = Vec3::new(x, 0.0, z);
            // Only spots the player's collision actually allows.
            let ok = lib
                .world
                .solids()
                .iter()
                .filter(|o| o.stops.has(Stops::BODIES))
                .all(|o| {
                    let q = Vec2::new(x.clamp(o.min.x, o.max.x), z.clamp(o.min.z, o.max.z));
                    q.distance(Vec2::new(x, z)) >= 0.3 - 1e-3
                });
            assert!(ok, "test spot ({x}, {z}) is not standable");
            nav.update(p);
            let d = nav.distance(p, open_floor);
            assert!(d.is_finite() && d < 40.0, "from ({x}, {z}): {d}");
            let r = nav.route(p, Vec3::new(-16.0, 0.0, -9.0)).expect("route");
            assert!(r.dist.is_finite());
        }
    }

    /// The closed veil walls off the East Wing; open, it is a walk away.
    #[test]
    fn the_veil_closes_the_east_wing() {
        let lib = Library::generate(crate::library::Layout::Hall, crate::game::SEED);
        let mut nav = Nav::new(&lib.world, lib.spawn.0);
        let (west, east) = (Vec3::new(5.0, 0.0, 6.0), Vec3::new(16.0, 0.0, 6.0));
        assert!(nav.open_at(west) && !nav.open_at(east));
        nav.update(west);
        assert!(!nav.distance(west, east).is_finite());
        nav.set_gate(0, false);
        assert!(nav.open_at(east));
        nav.update(west);
        let d = nav.distance(west, east);
        assert!((11.0..14.0).contains(&d), "{d}");
    }
}
