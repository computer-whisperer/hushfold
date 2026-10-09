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
   Design below (Phase 3 design). **Built and playtested** (2026-10-09); the
   bot: the Great Library ~10.6 h, the Hall ~1.6 h. The follow-up (hands,
   earned reading, spells) is below, "After the first playtest".
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
  wells). Stairs rise from one level's floor to the next: bodies (player, nav)
  walk them as a smooth slope, books rest on their treads (`Stairs::tread`),
  so a thrown book lies on a step rather than sunk into it.
- **Solids** have real vertical extent; those in the way of bodies or books stand
  on a floor (walls, railings, bookcases, tables), and one may rise through the
  floor above (a stair's guard at its top). Slabs block sight and aim rays too.
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
  (furniture, stairs); its top is allocated only once a book lies on it. Chose
  tiles over dense arrays because the Great Library's ~7.5k m² over three levels
  would be ~9M cells (~73 MB dense) of which piles touch a small part.
- A resting book belongs to one level (saved with it). A flying book carries its level:
  it rises into the level above through a hole, bounces off a slab's underside
  elsewhere, and falls into the level below through a hole. A railing (taller
  than the shed height) sheds books toward open floor, and over the well that
  is down. A book that slides off an edge falls.

### Player and nav
- Gravity: the player stands on the highest slab or stair within a step (0.45 m)
  of their feet; a higher rise blocks like a wall; railings guard edges. Solids
  collide only where they overlap the body vertically.
- Nav: a 0.5 m grid per level, stairs linking the cells at their two ends.
  Dijkstra from the player over all levels, run in full whenever the player
  changes cell (no early exit: the whole field serves Tidy Arms and Sort Sense).

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

## World model rebuild (between phases 2 and 3)

Agreed 2026-10-09 ("let's proceed with the refactor"), after the Great Library's
first polish pass. Further polish waits until gameplay has moved on, so it is not
redone when a building is re-laid out.

Why: solids were described twice (boxes drawn, boxes collided) and floors, stairs
and gates were interpreted separately by player movement, the nav grid, book
physics and aiming. Most of phase 2's world bugs came from that: the invisible
wall under the upper stairs (movement misread them), walking into the side of the
stairs (drawn, not collided), books sunk into the steps (physics saw a slope, the
screen showed steps), and sight lines passing through stair stone.

- **One world** (`src/world.rs`, owned by the Library): levels, floor slabs,
  flights of stairs, solids, gates and which are open, raised roofs. It alone says
  what the library is to bodies, books and sight; nothing else interprets a slab,
  a flight or a gate.
  - Bodies (the player, and the nav grid planning for them) stand on floors and
    flight slopes, are stopped by solids that stop bodies where they overlap the
    body above a step, by floor edges and stair stone rising into the body, and by
    closed gates. The body's measures (step, height, radius) are defined there once.
  - Sight and aim: solids that stop sight, floors, stair stone, closed gates.
  - Books: `Loose` builds its heightfields from the world (floors, treads, solids
    that stop books, closed gates) and keeps them current as gates open.
- **Each solid says what it stops** (`Stops`: bodies, books, sight). Railings and
  chairs stop bodies and books but are seen past; a bookcase has a sight-only box
  above it (no aiming over its top), and the stone over a doorway blocks sight.
  Replaces `blocks_ray` and `sight_top`. A solid belongs to every level whose
  space it reaches (a stair's guard rising past the floor above stands on that
  floor too), so the nav grid and books on each level see what bodies there meet.
  One that stops books must stand on a floor (checked): books know a level as a
  heightfield, so nothing can stop them from above; the stone over a doorway
  stops sight only, and a very high toss can pass over a closed veil.
- Gate state lives in the world (`open_gate`); `Loose` and `Nav` keep their own
  account of it, and `Game::open_gate` tells all three.
- **Each piece is declared once, by role**: `decor` (drawn only: small, or out of
  reach), `solid` (drawn, and its box collides), `collide` (one box standing for a
  piece the same helper draws in parts, like a bookcase; or an unseen guard, like a
  railing's). A flight of stairs is declared once and its steps are drawn from its
  own profile, the one books rest on.
- **Pieces apart from layouts**: reusable pieces (bookcase, reading table,
  chandelier, sconce, railing, newel, pier, door frame, panelling) live in
  `library/kit.rs`; `hall.rs` and `great.rs` only place things. Chose this so work
  on pieces survives re-laying out a building.
- Guards: `no_coplanar_faces` (z-fighting), `books_rest_on_the_treads`, sight
  blocked by stairs, and the Hall fingerprint (the Hall's collision must come out
  identical so its saves hold; the Great Library's collision is kept too, so its
  saves hold as well).
- Deferred: a spatial index for solids (queries are linear scans; it goes inside
  the world when a profile asks); raised floors (daises, landings) as floors with
  their own height; familiars moving through the world (phase 3).

## Phase 3 design

Started 2026-10-09 ("continue with Phase 3"), from the sketch agreed with the plan
(phases, item 3). What follows is the implementation's own reading of it,
revisable at the phase boundary after a playtest.

### The tree, by level of the hierarchy

One set of skills for both libraries; each library lists the ones that fit it, at
its own costs (`tree(layout)`), so the Hall keeps a short game and the Great
Library about 45 purchases spread over the whole of it. A skill means the same in
both. The Grimoire groups them by what they act on.

- **Reading** (where books go)
  - *Sort Sense I–II* (slot): I shows the active book's slot and the way to it
    while you stand in the room it belongs to (the Hall's rooms are its wings);
    II, anywhere.
  - *Wayfinding* (department; Great only): when Sort Sense doesn't reach, the
    compass points the walk to the active book's room instead.
  - *Seeker's Lens* (both libraries): V lights every loose book of the active
    (or aimed) book's group, department, category or series, and dims the rest.
    Chose this over the sketch's tint by department colour because twelve hue
    families can't all be told apart, and the highlight follows R to whatever
    level you are working at. Done in the shader from ids each book carries, so
    toggling it rebuilds nothing.
- **Moving** (coarse)
  - *Sorting by*: toss throws, and sift draws, every book of the active (or
    aimed) book's *group*, which R cycles: category (from the start), department
    (*Broad Strokes*, Great only), series (*Fine Fingers*). Chose one selector
    for both verbs because the group is the level of the hierarchy you are
    working at, and it reads on the HUD.
  - *Sifting Touch I–IV*: radius 0.9 / 1.8 / 2.7 / 3.6 m.
  - *Long Toss I–II*: harder throws (12 → 16 → 20 m/s) that scatter less.
  - *Lodestone Grasp*: as before (long reach, hold to keep gathering).
- **Filing** (precise): *Deeper Arms I–IV*, *Swift Slippers I–II*, *Tidy Arms*,
  *Series Bundle* as before; *Shelving Aura I–III* at 0.1.1's gentler numbers
  (2/4/8 books/s, 3/4.5/6 m). The sketch said "a weaker aura"; the bot says the
  Great Library's late game is bound by filing clicks without it (~1 book/s), so
  it stays the engine, gated by cost to the middle of the game.
- **Familiars**: bought one at a time by role, plus *Familiar Wings I–II*
  (faster, carry more, reach further from their post).

### Familiars: roles at posts

A familiar rides on your shoulder until you post it: G at a spot on the floor
posts the familiar riding longest; G at a posted familiar (or its perch) takes it
back. A posted familiar works only round its post, so where you put it is the
decision; none does the whole job.

- **Shelver** (slot): files loose books lying within ~3 m of its post whose
  shelves are a short walk from it (~8 m). Post one at a category's cases and
  bring that category to it.
- **Courier** (department): takes loose books lying within ~3 m of its post and
  carries them to where they're wanted: the post of a shelver that can file
  them, if there is one, or else the doorway of their department's room. It
  leaves books that are already there, and books of veiled departments.
- Shelver posts are the staging points the sketch's labelled tables were for:
  couriers bring a category to its shelver. Chose this over labelled tables
  because it needs no new verb or furniture, and the post already says where.
- **Moving through the world**: each post keeps a walking-distance field from it
  over the nav grid (`nav::Field`; the player's own field is one too), redone
  when a gate opens. A familiar's trip follows the field's path at flying height,
  straightened where the line is clear to sight, so it goes through doors and up
  the stairs, or across the well over a railing, never through a wall or floor.

### Deferred, with the reason

- *Carts*: Arms IV and Toss already carry in bulk; push physics is a new system.
- *Lifts and chutes*: the well already carries books down, and the bot spends
  ~2% of its time on stairs, so a lift would save little; a playtest may still
  find carrying up the stairs a chore.
- *Sorters* (a familiar that splits a pile by destination): couriers and the
  Broad Strokes sift cover the bucket pass; revisit if heaps feel like a chore.

### Pacing

The bot is reworked to measure the new mechanisms honestly: a plain gather takes
the top of the pile whatever it is (veiled books are tossed aside where it
stands); it sifts by the group that gives the most books per second of walking
once it can; it posts couriers at the biggest heaps and shelvers at the cases of
the categories piling up near them, and runs the real familiar code over each
trip. It still never tosses toward a destination, so tossing is upside it
doesn't count. Targets: the Great Library in 8–10 h with the last purchase late
in the game; the Hall in 1.5–2 h.

Measured 2026-10-09 (`game/pacing.rs`): the Great Library ~10.6 h, 40 purchases
spread to ~9.4 h, every department open by ~5.5 h, peak ~22k books an hour around
hour six; the Hall ~1.6 h. A first measure said ~9.0 h; review found the bot
moved its familiars for free (~880 times) and gathered piles it couldn't see
(~12% of the books), and with both charged honestly it is ~10.6 h. Familiars
shelve ~6% of the Great Library and carry ~8% to their rooms; walking to post
them costs about what they save, and they take about half the lumens, so as
costed they don't pay for themselves (open question). Findings on the way:
- With today's skills the late game was bound by filing clicks (~0.8 s a book),
  not walking or gathering; the aura's cost, not its strength, was the dial.
- The bot's filing route matters as much as any skill: visiting slots in rings
  of distance from where filing began zigzagged across rooms (~25 s a trip
  walking); nearest-from-here, as Tidy Arms picks, takes ~3 s. Between slots it
  charges the straight line, right once the aura files through a case and
  optimistic before it, so the first hour or two are flattered.
- Costs: the first cut summed to more lumens than the library pays (~246k);
  now ~196k.

## After the first playtest: hands, reading, spells

Agreed 2026-10-09 after the first playtest of phase 3. Overbuild skills and spells
now, pare down once play shows which don't earn their place.

What the playtest found:
- **Taking books was unclear.** A click took the top of the pile round the
  crosshair, up to a full armful; there was no way to take one book. Sift took the
  *aimed* book's group while the Lens lit the *held* book's, so aiming a hair off
  sifted the neighbour's category; R, V and F were three keys for one idea.
- **Reading was free.** From the first minute every empty slot of every carried
  series glowed across the floor and carried categories' signs showed through the
  stacks, so nobody needed the shelf letters or the alphabet.
- **Throwing is fun.** Keep building on the toss.
- **Couriers work at the hub**, posted in the atrium carrying books out to their
  rooms; what they lack is a supply. (Reversed: I had proposed removing them.)
- **The late game buys numbers.** In the bot's Great Library run 12 of 16 kinds of
  ability arrive in the first 2 h and the last at 6.6 h; the last 4 h buy only
  familiars and levels. Familiars take 60% of the lumens (118k of 196k).

### Step 1: hands and seeking

- **Tap** left on a loose book takes that book. **Hold** left: a ring grows on
  the pile round the aim point (it follows the aim while held), and releasing
  takes the top of the pile inside it, up to what the arms hold; the ring's
  largest size is the old click's (grows with Deeper Arms). Left hold gathers,
  right hold throws. Chose this over Ctrl-click for one book because the ring is
  seen, and it mirrors the toss.
- **Seeking** (needs Sifting Touch): F at a loose book seeks its group at R's
  level; F at a book of the group sought, or at nothing, stops; F at nothing with
  empty seeking seeks the held book's group. While seeking, the ring takes only
  the sought group from any depth (Sifting Touch's levels set its largest size),
  the Seeker's Lens (once learned) lights the sought books and dims the rest,
  toss throws held books of the sought group, and the HUD names what is sought.
  V goes. Chose one explicit filter because the Lens and Sift disagreed.
- **Lodestone Grasp** is reach only (holding is the scoop now).
- Before Sifting Touch, taking books one at a time is sorting by hand: reading
  spines and picking one category's books out.

### Step 2: reading, earned

- Free: signs in view, shelf-edge letter ranges, the crosshair tip on a run (its
  title, and "file 2" in gold when carried), faint outlines of empty slots.
- **Signposts** (category): carried categories' signs show through the stacks.
- **Kindred Glow I–III** (series): carried series' runs glow; I faint within ~3 m
  (the case in front of you), II brighter within ~12 m, III the full glow and
  glint everywhere (what was free).
- **Sort Sense** and **Wayfinding** as before, dearer, after Glow II.
- **Glimpse** (the first spell): for 10 s the slots of every carried book light
  up; recharge 45 s. Spell first, passive (Sort Sense) later.
- The bot gets a search time per series it files, by reading level, to be
  calibrated against play. A first guess (8 s a series with nothing, 3 s at Glow
  I, 1 s at II, and 6 s a category without Signposts) made the Hall 2.6 h and the
  Great Library 14.7 h; for a player who has learned the layout (4 / 2 / 0.8 / 0
  s a series, 3 s a category, nothing while Sort Sense or Glimpse shows it), with
  Signposts and Glow I priced below Deeper Arms so they come in the first minutes:
  the Hall ~2.3 h (Signposts at 1.7 min, Glow I at 2.9), the Great Library
  ~13.8 h, reading about half of each trip until Glow III (~5.5 h). Re-costing
  (step 4) brings these back to target.

### Step 3: spells and the hub

- **Spells** sit on a hotbar (keys 1–5), each with its own recharge dial; bought
  in the Grimoire's Spells group like any skill, with levels.
- **Gathering Charm**: cast at a heap; its books stream to the hub (the atrium
  floor under the well; the Hall's reading room), flying the walking route at
  head height like the cranes, out of doors and over the railing down the well.
  Levels: a wider heap, a shorter recharge (first guess ~2 min). While seeking,
  only the sought group goes.
- **Couriers at the hub**: the charm feeds them; the bot posts them there.
- **Familiars that last**: a shelver serves a whole category's cases from one
  post; a familiar idle a while flies back to the player's shoulder by itself.

### Step 4: the rest of the bank, and paths

| Spell | Does | Recharge (first guess) | Overlaps |
|---|---|---|---|
| Echo | a ring sweeps out; loose books of the sought group flare through walls within ~25 m for 20 s | 60 s | Seeker's Lens (view only) |
| Summon Series | loose books of the active book's series fly to the arms: from the room, the floor, anywhere by level | 90 s | — |
| Shelving Rush | for 10 s carried books within ~6 m of their slots file themselves, no rate limit | 60 s | Shelving Aura |

- **Homing Toss** (Moving, passive): a tossed book landing within ~2 m of its
  slot slips into it. The bot learns this one use of the toss.
- Rules: every spell asks where, what or when (no press-when-ready chores);
  recharges are a few trips long so nobody waits; overlapping abilities compete
  for the same limit (arms, walking), so owning both helps less than double.
- **Paths**: the bot gets a preferred path (reading and spells; hands and
  throwing; familiars; filing) and runs each; every path should finish in 8–10 h
  and none much faster. If spells stack too well, the fallback is one shared pool
  of magic instead of a recharge per spell.
- Then re-cost the tree so a new kind of ability arrives every ~30–45 min in the
  Great Library and every ~8 min in the Hall.

### Built, and measured (2026-10-09)

Steps 1–4 are built as above, with these differences:
- The Gathering Charm is offered only in the Great Library: every book in the
  Hall starts in its own wing, so there is nothing for it to fix (in the Great
  Library ~14% of pile books start in another room and ~21% in the atrium).
  Dropped from a fixed height over the hub the heap grew a 6.7 m tower; books now
  drop from above the heap's top, thrown out over a disc (440 books: a mound
  ~0.8 m high, ~1.7 m across).
- A familiar idle two minutes folds up and reappears on the shoulder (no flight
  back, which would need a route to a moving player). The shelver's reach was
  left as it was (8–12 m on foot from its post), not widened to a whole category.
- Fewer familiars: 5 shelvers and 4 couriers in the Great Library, 3 and 2 in
  the Hall.

**Re-costing.** Costs are fitted to purchase schedules (`tuning/*.schedule`: the
order the cheapest-first bot should buy in and the minute of each) with
`tuning/retune.py`, a few rounds of running the bot and scaling each cost by
target over actual. Lessons on the way:
- Costing straight from an earning curve overshoots both ways: cheaper skills
  arrive sooner, which raises earning, which brings everything sooner still (the
  Hall went 1.89 h → 1.04 h in one round). Scaling cost by target/actual
  settles in two or three rounds.
- The schedule decides the length as much as the costs: with the engines
  (Kindred Glow III, Deeper Arms IV, the aura) early, the Hall ends in ~1.2 h
  whatever the later costs. The schedules spread them to the end, and put what
  the bot doesn't use (Echo, the Lens, Long Toss, Lodestone, Fine Fingers, Wings)
  earlier.
- In the Great Library the first ~4 h shelve ~10% of the books (reading and a
  click a book until Glow III and the aura); moving Glow III to ~2 h and the
  aura to ~1.7 h brought it from 10.4 h to 9.6 h.

Result (cheapest-first): the Hall **~1.7 h**, a new kind of skill every ~5 min
to ~80 min, the last purchase at ~96 min; the Great Library **~9.7 h**, a new
kind every ~11–25 min to ~5.5 h, the last purchase at ~8.3 h. The trees cost
about what the libraries pay by their last purchase.

**Paths** (the bot favouring one tech path, the skills off it counted three
times dearer):

| Path | Hall | Great Library |
|---|---|---|
| none (cheapest first) | 1.72 h | 9.66 h |
| filing | 1.59 h | 8.52 h |
| reading | 1.77 h | 11.47 h |
| familiars | 1.79 h | ~13 h (stopped at 10 h, 67% shelved) |
| hands | 2.20 h | 13.14 h |

With Deeper Arms, Swift Slippers, Signposts and Kindred Glow at face value for
every path (`HUSHFOLD_BASICS`): the Hall 1.37 h (filing), 1.64 h (familiars), 1.82
h (reading), 1.85 h (hands); the Great Library 8.17 h (filing), 11.30 h (reading),
11.56 h (hands), 11.74 h (familiars). The imbalance is not the basics. In the bot's model the time goes to
filing (and, early, reading), so the filing skills are the engine and no other
path has one: gathering is a few seconds a trip, tossing home is a series at a
time, familiars shelve ~6–10%.

**Found and fixed on the way:** a book that ended a flight step inside a solid
came out by the nearer face, so at long frames (a hitch; the bot's 0.05 s steps)
a hard throw could pass the middle of an outer wall and come to rest on its top,
outside the building, out of reach. Steps now never move a book more than 5 cm.

### Next (agreed 2026-10-09): sets, the armful, familiars

From a look at the reward structure (series bonuses are half of all lumens, but
series complete only when their last stray turns up, so ~all completions land in
the last hour) and the first playtest of the opening:
- **Some series lie together** (a mix, not all): each series of two or more
  volumes draws how much of it starts in one pile: ~25% whole, ~40% a core of
  40–80% with the rest scattered one by one, ~35% scattered entirely. A whole set
  goes to its own room (85%, else the atrium); other units by the old shares
  (65% own room, 20% atrium, 15% earlier rooms). Measured: 26% of series start in
  one heap, 37% entirely in their own room, the median widest gap between a
  series' volumes 26 m (was 33 m). The Great Library's mix of veiled books in
  early areas stays as it is. The Hall keeps its released generation for now:
  saves keep only moved books and rebuild untouched piles from generation, so
  changing it would rearrange the piles of 0.1.x saves (open question).
- **Set tools sized to the scatter, and earlier.** Summon Series I reaches the
  room you are in (II the floor, III anywhere); Fine Fingers (seeking a series)
  comes within the first hour or so in the Great Library.
- **The bot hunts sets**: Summon Series goes first to a series it can complete.
- **Familiars buffed** ahead of measuring: they shelved ~6–10% and the familiars
  path was the slowest.
- **The toss throws the whole armful**, or while seeking only what is sought (it
  threw the active book's category, often one book of a mixed early armful).
- Then refit the costs and measure the paths again.
