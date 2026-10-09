# Scaling up: the Great Library

The plan for growing Hushfold from one hall (~11.7k books, ~1.5 h) to a library of
many rooms and floors (~100–150k books, 8–10 h) without the late game turning into
watching helpers work. Agreed 2026-10-08. DESIGN.md describes what exists; this
file describes where it is going and records the decisions that steer it.

## Decisions

1. **Books are loose objects, simulated with custom semi-physics** (not a rigid-body
   engine). Thrown books fly ballistically with spin and bounce off furniture; on
   landing they settle onto a heightfield of whatever is below and slide downhill
   while the slope exceeds an angle of repose, so piles form and spill naturally.
   Chose because: 100k+ thin boxes mostly at rest, saves must reproduce exact poses,
   and thin-box stacks jitter in rigid-body solvers; the settle code already existed
   in the heap generator. Rejected rapier because the realism it adds (books knocking
   each other over) costs determinism, save size and frame time at this count.
2. **The small library stays, on the same systems.** One codebase, two layouts; the
   hall keeps its ~1.5 h pace and simply has no stairs or lifts. Rejected freezing
   0.1.x because two diverging games would split every fix.
3. **Familiars become roles with stations** (couriers, sorters, shelvers), a light
   logistics layer, rather than free-roaming filers.
4. **Target size ~100–150k books for 8–10 h.**

## The organising idea

At scale, the cost of filing is moving books long distances and working out where
they go, not the click. A big library is a hierarchy (department ≈ floor or wing →
room → category → series → slot), and the efficient way to sort a mountain of books
is a bucket sort: coarse passes that move many books far and imprecisely, then
finer passes that move fewer books a short way precisely. Every skill acts at one
level of that hierarchy and is weak alone; the player's job is planning the passes.

The hierarchy must be readable by eye at each level: department = colour family
(first pass by eye), category = pattern/trim within it plus room signs, series and
slot = the title (Sort Sense helps).

Skill principles: no skill spans more than one level; upgrades broaden (more
stations, finer filters, longer throws) rather than multiply; ~50 small purchases
over 8 h; the player's own Toss and Sift stay the fastest coarse tools.

## Loose-book model (phase 1)

- `src/loose.rs` owns the rest pose of every loose book, a 0.5 m bucket grid of
  resting books, and a 4 cm heightfield: `base` (floor and the tops of furniture,
  which are solid columns) and `top` (base plus resting books).
- A resting book is a footprint with a planar underside: bottom-centre height,
  yaw axis, and slopes along its two horizontal axes. Supports are read on exactly
  the heightfield cells a book's own top is written to (cell centres inside it).
- Tall furniture standing edge to edge (a row of bookcases) is merged into one
  solid for the physics, so nothing is pushed from one case into the next.
- **Drop** (generation, landing, re-drop): sample support under the footprint;
  while the slope exceeds the angle of repose, step downhill; then fit the plane
  (slopes clamped) and raise it until nothing sinks below support.
- **Removal** (gather, sift, familiars): everything above the removed books (on
  any cell both cover, transitively) is taken out, the heightfield rebuilt under
  it from what remains, and the set re-dropped, each book after those below it.
  A book whose whole underside still fits keeps its exact pose; the rest fall
  (animated, from wherever they are drawn) and may slide.
- **Flight**: gravity, spin, bounce off walls, ceiling, the closed veil and
  furniture; furniture taller than 0.9 m sheds books off its top toward open floor
  (tables hold them); a book lands when it reaches the heightfield, and never skids
  through the closed veil.
- **Verbs**: left click gathers the topmost books within a radius of the aim point;
  hold right mouse to charge a toss (an arc shows where it lands) and release to
  throw every held book of the active book's category; F sifts (an upgrade): pulls
  books of the aimed book's category from anywhere in the pile within a radius.
- **Save**: rest poses of books that left their generated pose; on load everything
  is re-checked for support (0.1.x saves had no poses, so their piles re-settle).

## Phases

0. Kill-tests: toss/settle looks natural with thousands of books (passed in
   phase 1: generated piles, gathers, sifts and 30-book tosses screenshot well);
   150k books render within budget (passed 2026-10-09 on an RX 6800: ~290k extra
   book and ghost boxes in view cost a few ms of GPU, ~1.08M cost ~15–19 ms, i.e.
   ~10 ms per million; the cost at scale is the per-frame rebuild and upload, which
   persistent chunk buffers remove).
1. **Loose books, Toss and Sift in the current hall.** Familiars and the pacing bot
   ported to piles. Built; awaiting playtest of the feel. The pacing bot (no toss
   or sift) went from ~1.4 h to ~1.5 h because piles now spread.
2. Multi-floor, multi-room generator; player gravity and stairs; nav as a grid per
   floor linked at stairs and lifts; persistent per-book GPU buffers with dirty
   updates (today every book and ghost is rebuilt and uploaded each frame); room
   culling and per-room light lists (the shader loops every light per pixel).
   Catalogue gains departments (16 categories → ~100 in ~10 departments).
   Design below; built in steps: world model (Hall only, generating exactly as
   before), the Great Library and choosing it, rendering, a pacing baseline.
   **Built; awaiting playtest.** Baseline (the Hall's skills, an idealised bot
   that never tosses or sifts but picks up only fileable books): ~16.6 h, every
   upgrade bought by 2.8 h, then flat. The same bot now takes the Hall in ~2.0 h
   (it walks round furniture now; it used to walk through it, hence ~1.5 h).
3. Skill tree rebuilt around the hierarchy levels: reading (pile tinting by
   department, coarse sense, room-scoped Sort Sense), coarse moves (Toss range and
   landing, Sift filters, carts, restorable lifts and chutes), staging (labelled
   tables), filing (hands, Bundle, Tidy Arms, a weaker aura), familiar roles.
   Ideas parked for it: a toss that lands on its own series run files itself.
4. The Great Library: content and balance. The pacing bot measures mechanism
   throughput with the real code; playtests judge whether the logistics are fun.

## Phase 2 design

Agreed as the phase 2 plan 2026-10-09 ("continue with phase 2"); details below are
the implementation's own choices, revisable at the phase boundary.

### World model
- `Library` is layout-agnostic; `library/hall.rs` and `library/great.rs` build it
  from shared pieces (bookcase units, category filling, sconces, piles). The Hall
  must generate bit-identically (test `hall_generates_as_before`): saves name books
  by index and rebuild unmoved piles from generation.
- **Levels**: a floor elevation and roof height per level. Floors are slabs
  (rectangles); anywhere a level has no slab is a hole (the atrium well, stair
  wells). Stairs are ramps from one level's floor to the next.
- **Solids** have real vertical extent and stand on a level's floor (walls,
  railings, bookcases, tables). Slabs block sight and aim rays too.
- **Departments** generalise wings, **gates** the veil. A gate is a thin solid
  (across the Hall, or filling a doorway) that blocks walking, throwing,
  gathering, aiming and routes until the departments opened before it are
  shelved to a fraction; it is drawn as the veil and dissolves. The Hall: two
  departments (West, East Wing), one gate at 60%.
- Chose a solid for the gate because every system already respects solids; the
  veil's special cases (x < VEIL_X in physics, nav, aim and gathering) go away.
  Gathering and landing skids now never cross a tall solid, which also stops a
  sift reaching through a bookcase.

### Loose books across levels
- One heightfield and bucket grid per level over its bounds. The heightfield is
  stored in 32×32-cell tiles: a tile's base is flat floor, a hole (−∞) or per-cell
  (furniture, ramps); its top is allocated only once a book lies on it. Chose
  tiles over dense arrays because the Great Library's ~7.5k m² over three levels
  would be ~9M cells (~73 MB dense) of which piles touch a small part.
- A resting book belongs to one level (saved with it). A flight carries its level:
  it rises into the level above through a hole, bounces off a slab's underside
  elsewhere, and falls into the level below through a hole. A railing (taller
  than the shed height) sheds books toward open floor, and over the well that
  is down. A book that slides off an edge falls.

### Player and nav
- Gravity: the player stands on the highest slab or ramp within a step (0.45 m)
  of their feet; a higher rise blocks like a wall; railings guard edges. Solids
  collide only where they overlap the body vertically.
- Nav: a 0.5 m grid per level, ramps linking the cells at their two ends.
  Dijkstra from the player over all levels, stopping once every requested target
  is settled.

### Rendering
- Chunks: resting and shelved books and ghosts live in persistent per-chunk
  buffers (8 × 6 × 8 m boxes of space, not per level and 4 m tile: boxes need no
  notion of level), rebuilt only when something in them changes. Loose bumps a
  per-book draw version; Game compares state and version per book each frame
  (~0.1 ms for 118k). Static geometry stays one buffer (a few thousand boxes).
  Moving things (flying, falling, held, carried, familiars, particles) stay in
  the per-frame buffer.
- Culling: chunks outside the frustum are skipped; walls and floors are drawn
  first so they hide rooms of books behind them. No occlusion culling (the GPU
  has room).
- Lights: **the 64 lights nearest the eye** (by distance minus reach), after the
  player's candle and familiars. Chose because every room lit correctly in
  screenshots with no shader change; the planned per-cell light grid (storage
  buffers, a list per 4 m cell) is rejected for now and stays the fallback if
  lights visibly pop at the edge of the 64.
- Measured 2026-10-09, same session, alternating builds (unpaced, RX 6900 XT):
  per-frame rebuild 128 fps in the Great Library (CPU 2.5 ms), chunks 185 fps
  (CPU 0.75 ms); the Hall 315 → 360 fps. GPU time is read from timestamp
  queries (F3); the old wait-for-the-GPU timer swung with the GPU's clocks.

### The Great Library (first cut)
- Three levels 6 m apart. A 28 m square atrium whose 18 m well is open from the
  ground floor to a skylight; galleries 5 m wide ring it on the upper levels,
  railed at the well. Four flights of stairs in the gallery strips (west and
  east: ground to first; north and south: first to second), so the well stays a
  clear drop zone for tossing.
- Twelve department rooms, one off each side of the atrium on each level, 26 m
  wide and 18 m deep, entered by a gated door: four rows of five double-sided
  cases running away from the door, a category on each side of a row (eight per
  room, five cases each), ~10k books a room, ~120k in all.
- Catalogue: twelve departments of eight categories. Colour family tells the
  department; within one, the spine pattern tells the category (eight patterns,
  one per category). The Hall keeps its per-category hues and random patterns.
- Rooms open in order (ground floor first, then up) as the departments already
  open are shelved to a fraction. A book of a department starts only where it is
  reachable by the time its department opens: its own room, the atrium, or a
  room that opens earlier, so early rooms are mixed and later rooms purer, and
  the atrium holds a great mixed heap.
- Familiars still fly straight (through walls); they are rebuilt as roles with
  stations in phase 3.
- Choosing: starting over asks which library; each has its own save.
