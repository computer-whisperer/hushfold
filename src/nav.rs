//! Walking distances around the furniture: a coarse grid over the hall and a
//! distance field from the player, so "nearest" means nearest on foot rather
//! than through the back of a bookcase.

use crate::library::{HALL_X, HALL_Z, Obstacle, VEIL_X};
use glam::{Vec2, Vec3};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const CELL: f32 = 0.5;
/// Clearance the grid keeps from furniture (the player's radius).
const CLEARANCE: f32 = 0.3;
/// Clearance for straight-segment tests; smaller, so a player standing flush
/// against a case never counts as inside it.
const SEG_CLEARANCE: f32 = 0.15;
/// Grid step costs: orthogonal and diagonal, in tenths of a cell.
const STEP: u32 = 10;
const DIAG: u32 = 14;

/// How to walk from the player to a target.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    /// The farthest point along the route that is in a straight, clear line from
    /// the player (the target itself when `direct`). Floor height.
    pub waypoint: Vec3,
    /// Walking distance to the target, in metres.
    pub dist: f32,
    pub direct: bool,
}

pub struct Nav {
    w: usize,
    h: usize,
    /// Furniture footprints (min, max) in xz.
    boxes: Vec<(Vec2, Vec2)>,
    /// Cells blocked by furniture (the veil is applied per query).
    blocked: Vec<bool>,
    east_open: bool,
    /// Distance from the source cell in tenths of a cell; u32::MAX = unreached.
    dist: Vec<u32>,
    prev: Vec<u32>,
    source: Option<(usize, bool)>,
}

impl Nav {
    pub fn new(obstacles: &[Obstacle]) -> Self {
        let w = (2.0 * HALL_X / CELL).ceil() as usize;
        let h = (2.0 * HALL_Z / CELL).ceil() as usize;
        let boxes: Vec<(Vec2, Vec2)> = obstacles
            .iter()
            .map(|o| (Vec2::new(o.min.x, o.min.z), Vec2::new(o.max.x, o.max.z)))
            .collect();
        let mut nav = Nav {
            w,
            h,
            boxes,
            blocked: vec![false; w * h],
            east_open: false,
            dist: vec![u32::MAX; w * h],
            prev: vec![u32::MAX; w * h],
            source: None,
        };
        for i in 0..w * h {
            let c = nav.center(i);
            let wall = c.x.abs() > HALL_X - CLEARANCE || c.y.abs() > HALL_Z - CLEARANCE;
            let furniture = nav.boxes.iter().any(|&(mn, mx)| {
                c.x > mn.x - CLEARANCE && c.x < mx.x + CLEARANCE && c.y > mn.y - CLEARANCE && c.y < mx.y + CLEARANCE
            });
            nav.blocked[i] = wall || furniture;
        }
        nav
    }

    fn center(&self, i: usize) -> Vec2 {
        let (x, z) = (i % self.w, i / self.w);
        Vec2::new(-HALL_X + (x as f32 + 0.5) * CELL, -HALL_Z + (z as f32 + 0.5) * CELL)
    }

    fn cell(&self, p: Vec2) -> usize {
        let x = (((p.x + HALL_X) / CELL) as usize).min(self.w - 1);
        let z = (((p.y + HALL_Z) / CELL) as usize).min(self.h - 1);
        z * self.w + x
    }

    fn veiled(&self, x: f32) -> bool {
        !self.east_open && x > VEIL_X - CLEARANCE
    }

    fn free(&self, i: usize) -> bool {
        !self.blocked[i] && !self.veiled(self.center(i).x)
    }

    /// True if a straight walk from `a` to `b` touches no furniture.
    pub fn clear(&self, a: Vec2, b: Vec2) -> bool {
        if !self.east_open && a.x.max(b.x) > VEIL_X {
            return false;
        }
        let d = b - a;
        let inv = Vec2::new(1.0 / d.x, 1.0 / d.y);
        !self.boxes.iter().any(|&(mn, mx)| {
            let t1 = (mn - Vec2::splat(SEG_CLEARANCE) - a) * inv;
            let t2 = (mx + Vec2::splat(SEG_CLEARANCE) - a) * inv;
            let tmin = t1.min(t2).max_element().max(0.0);
            let tmax = t1.max(t2).min_element().min(1.0);
            tmax >= tmin
        })
    }

    /// Recompute the distance field if the player has changed cell or the veil has lifted.
    pub fn update(&mut self, from: Vec3, east_open: bool) {
        let src = self.cell(Vec2::new(from.x, from.z));
        if self.source == Some((src, east_open)) {
            return;
        }
        self.source = Some((src, east_open));
        self.east_open = east_open;
        self.dist.fill(u32::MAX);
        self.prev.fill(u32::MAX);
        let mut heap = BinaryHeap::new();
        // The player's own cell may overlap furniture clearance; start there anyway.
        self.dist[src] = 0;
        heap.push(Reverse((0u32, src as u32)));
        let (w, h) = (self.w as i32, self.h as i32);
        while let Some(Reverse((d, i))) = heap.pop() {
            let i = i as usize;
            if d > self.dist[i] {
                continue;
            }
            let (x, z) = ((i % self.w) as i32, (i / self.w) as i32);
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= w || nz >= h {
                    continue;
                }
                let n = (nz * w + nx) as usize;
                if !self.free(n) {
                    continue;
                }
                let cost = if dx != 0 && dz != 0 {
                    // No cutting corners past furniture.
                    if !self.free((z * w + nx) as usize) || !self.free((nz * w + x) as usize) {
                        continue;
                    }
                    DIAG
                } else {
                    STEP
                };
                let nd = d + cost;
                if nd < self.dist[n] {
                    self.dist[n] = nd;
                    self.prev[n] = i as u32;
                    heap.push(Reverse((nd, n as u32)));
                }
            }
        }
    }

    /// The reached cell nearest `p` (its own cell, or a neighbour if that one is blocked).
    fn reached_near(&self, p: Vec2) -> Option<usize> {
        let c = self.cell(p);
        if self.dist[c] != u32::MAX {
            return Some(c);
        }
        let (x, z) = ((c % self.w) as i32, (c / self.w) as i32);
        let mut best: Option<(f32, usize)> = None;
        for dz in -2..=2 {
            for dx in -2..=2 {
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= self.w as i32 || nz >= self.h as i32 {
                    continue;
                }
                let n = (nz * self.w as i32 + nx) as usize;
                if self.dist[n] == u32::MAX {
                    continue;
                }
                let d = self.dist[n] as f32 * CELL / STEP as f32 + self.center(n).distance(p);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, n));
                }
            }
        }
        best.map(|(_, n)| n)
    }

    /// Walking distance from `from` (the field's source) to `to`; infinite if unreachable.
    pub fn distance(&self, from: Vec3, to: Vec3) -> f32 {
        let (a, b) = (Vec2::new(from.x, from.z), Vec2::new(to.x, to.z));
        if self.clear(a, b) {
            return a.distance(b);
        }
        match self.reached_near(b) {
            Some(c) => self.dist[c] as f32 * CELL / STEP as f32 + self.center(c).distance(b),
            None => f32::INFINITY,
        }
    }

    /// The route from `from` (the field's source) to `to`.
    pub fn route(&self, from: Vec3, to: Vec3) -> Option<Route> {
        let (a, b) = (Vec2::new(from.x, from.z), Vec2::new(to.x, to.z));
        if self.clear(a, b) {
            return Some(Route {
                waypoint: Vec3::new(b.x, 0.0, b.y),
                dist: a.distance(b),
                direct: true,
            });
        }
        let end = self.reached_near(b)?;
        let dist = self.dist[end] as f32 * CELL / STEP as f32 + self.center(end).distance(b);
        // Walk back toward the player; the first cell in plain view is the farthest
        // visible point along the route.
        let mut i = end;
        let mut waypoint = self.center(end);
        while self.prev[i] != u32::MAX {
            let c = self.center(i);
            if self.clear(a, c) {
                waypoint = c;
                break;
            }
            i = self.prev[i] as usize;
            waypoint = self.center(i);
        }
        Some(Route {
            waypoint: Vec3::new(waypoint.x, 0.0, waypoint.y),
            dist,
            direct: false,
        })
    }
}

