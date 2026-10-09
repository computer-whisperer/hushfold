# Hushfold — design notes

A first-person "tidy the magical library" game. Rust, wgpu 29 + winit 0.30 + egui 0.34.
Performance is a design pillar: books lying still are kept in chunks of space on
the GPU and re-uploaded only when something in a chunk changes; per frame the CPU
only compares each book's state with what its chunk was built from.

## Fantasy
A moonlit, dust-mote library. Two wings of freestanding double-sided bookcases
around a central reading room with a chandelier. Piles of books everywhere.
Each category's lantern lights up (in its colour) when the category is complete,
so the hall gets visibly brighter and more colourful as you sort.

## Book model
- 16 categories (colour family, vocabulary), 8 per wing.
- Series: 1–12 volumes, shared colour/pattern/trim/height. Volume number on spine.
- Every book has exactly one slot. Category -> 3 cases on one face-row of a wing.
  Series alphabetical (ignoring leading articles), top shelf to bottom,
  left to right; a series never splits across shelves; shelves are justified.
- Empty slots render as faint ghosts in the series colour. Ghosts of series you
  are holding glow in a *signal colour*: the series hue scaled to full value and
  lifted toward white until it reaches a fixed luminance, so dark browns and
  blacks read as clearly as golds against the wood. A glint sweeps through them.
  The active book's slot (Sort Sense) pulses as a filled block; the active book
  in hand carries a bright rim.

## The world
`src/world.rs` is what the library is to everything that moves through it or
looks across it: levels, floor slabs (anywhere a level has none is a hole),
flights of stairs, solids and gates. Each solid says what it stops (bodies, books,
sight): railings and chairs stop walking and throwing but are seen past, and a
bookcase has a sight-only box above it so no one aims over its top. Bodies walk
floors and stair slopes and are stopped by solids, floor edges, stair stone and
closed gates;
sight by solids that stop it, floors, stair stone and closed gates; books build
their heightfields from the same world. Generation (`src/library/`) declares each
piece once: `decor` (drawn only), `solid` (drawn, and in the way), or parts drawn
with one `collide` box standing for them. Pieces live in `kit.rs`; `hall.rs` and
`great.rs` only place them. A test fails if two visible faces share a plane
(z-fighting).

## Loose books
Every book not shelved or carried is loose (`src/loose.rs`), simulated with
semi-physics rather than rigid bodies (see SCALING.md for why):
- Each level has a 4 cm heightfield holding the floor (or a hole where it has
  none), stair treads, furniture tops (furniture is a solid column from its floor) and
  every resting book's top, stored in 32×32-cell tiles allocated where books lie;
  a 0.5 m bucket grid indexes resting books for picking and gathering.
- A resting book is a footprint with a planar underside. Dropping one fits it
  to the cells under it and, while the support is steeper than the angle of
  repose (0.6), steps it downhill, so piles spread into natural mounds.
- Removing books lifts out everything above them (on any heightfield cell both
  cover), rebuilds the heightfield there and re-drops the lot, each after the
  books below it. A book whose whole underside still fits keeps its exact pose.
- Thrown books fly with gravity and spin, bounce off walls, ceilings, furniture
  and closed gates; furniture taller than 0.9 m sheds books off its top (tables
  hold them). They land on the heightfield and skid a little, never through a
  wall, case or gate. A flight rises into the level above through a hole in its
  floor and falls through holes to the level below; a book that slides off an
  edge falls too. A book that ends a flight step inside a solid comes out by the
  nearer face, so no step moves a book more than 5 cm (more steps at speed): at
  long frames (a hitch, the pacing bot's 0.05 s) a hard throw used to pass the
  middle of an outer wall and land on its top, outside the building.
- Gathering takes only books with no wall, case or closed gate between them and
  the aim point.
- Generation picks drop points per pile; dropping them through the same code
  makes the starting piles.

## Verbs
- Tap a loose book: take that book, from wherever it lies in its pile; whatever
  lay on it falls. Before Sifting Touch this is sorting by hand.
- Hold on a pile: a ring grows round the aim point (following it across piles)
  for 0.4 s after a 0.18 s tap; release to take the topmost books inside it, up
  to arm capacity. Its largest size grows with the room in the arms.
- Hold right mouse: charge a toss (the arc and landing ring show where it goes);
  release to throw the whole armful as a stream, or while seeking only the books
  of the group sought (none held: nothing, with a hint). A quick click throws at
  the least strength, dropping the armful just ahead. Left hold gathers, right
  hold throws. (It used to throw the active book's group, which in a mixed early
  armful was often one book; the playtest wanted the armful.)
- F (Sifting Touch): *seek* the aimed book's group (F at a book of the group
  sought, or at nothing, stops; at nothing while not seeking, seeks the active
  book's group). While seeking, the scoop takes only the group sought, from any
  depth, and grows to Sifting Touch's reach; the HUD names what is sought. Chose
  one explicit filter after the first playtest: the lens lit the held book's
  group while sift took the aimed one's, so aiming a hair off sifted the
  neighbour's.
- R: the level toss and seeking work at, the *group*: category, or with skills
  department (Broad Strokes) or series (Fine Fingers).
- Seeker's Lens: while seeking, loose books of the group sought glow and the
  rest fade; done in the shader from ids each book carries, so it rebuilds
  nothing.
- Click shelf: aiming anywhere on a series run places a held book of that series
  into its exact slot (forgiving aim; the skill is finding the run).
- Scroll/Q/E: cycle the active book (HUD shows its category/series/volume).
- Tab: Grimoire (upgrades). Shift: hurry.
- F9: cheats for trying late-game situations (`game/cheats.rs`): lumens, every
  skill, lifting veils (until the next load, since veil states follow from what
  is shelved), shelving the library to a fraction (open departments first, then
  those whose veils that lifts; books are credited as if filed, without the
  sparkle), and going to any room's doorway (outside its veil while veiled).

## Progression (Lumens ✦)
1 per book, +volumes on series completion, +100 per category.
Skills are grouped in the Grimoire by what they work on; each library offers
those that fit it, at its own costs (`upg_costs`), and a skill does the same in
both. Reading: Signposts (signs of carried categories show through the stacks),
Kindred Glow I–III (runs of carried series glow: within ~3 m, ~12 m, everywhere
with a glint; done in the shader from a reach and strength in the globals), Sort
Sense I–II (compass and exact-slot pulse: in the book's room, then anywhere),
Wayfinding (the compass points to the book's room), the Seeker's Lens. With none
of these, a player reads: the signs in view, the letter ranges on shelf edges,
the title the crosshair tip gives a run. Saves from before reading was earned
keep the guide signs and the full glow they had free. Moving: Sifting Touch I–IV (seeking; the scoop's reach 0.9–3.6 m), Broad
Strokes and Fine Fingers (seek and toss by department, by series), Long Toss
I–II (harder, tighter throws), Homing Toss I–II (a tossed book landing in front
of its own shelf within 2 / 3.5 m of its slot, along the floor, slips into it;
`Game::aim_toss` sets pitch and charge to land on a point, for the bot and
tests), Lodestone Grasp (long reach). Filing: Deeper Arms I–IV,
Swift Slippers I–II, Tidy Arms (stack sorted by slot, active = nearest target),
Series Bundle (place whole series at once), Shelving Aura I–III (books float to
nearby slots on their own). Familiars: shelvers and couriers, bought one at a time,
and Familiar Wings I–II (faster, carry more, reach further). Spells, cast with the
number keys from a hotbar that shows each one's recharge: Glimpse I–II (for 10–15
s every carried series glows everywhere and a marker shows each run through the
walls; recharge 45–30 s), the Gathering Charm I–III (Great Library only: cast at
a heap up to 14 m off, its books within 2.5 / 3.5 / 5 m, up to 150 / 300 / 600 and
only the group sought while seeking, stream to the hub, the great heap on the
atrium floor; recharge 120 / 90 / 60 s; in the Hall every book starts in its own
wing, so there is nothing for it to fix), Echo I–II (a ring sweeps out 30 m/s;
heaps within 25 / 50 m holding the kind sought, or the active book's kind, are
marked through the walls with how many lie there, marks within 3 m merged; 20 /
30 s, recharge 60 / 45 s), Summon Series I–III (the active book's series' loose
volumes within 15 m, on the floor, anywhere, nearest first, dissolve where they lie
and fly to the arms; recharge 90 / 75 / 60 s), Shelving Rush I–II (for 10 / 15 s
carried books within 6 / 8 m of their slots fly home at 30 a second; recharge 60
/ 45 s). Spells live in `game/spells.rs`.
Charmed books rise from the heap, fly a way planned over a walking field from the
hub the way a crane's is, a little beside its line (a ribbon, 0.25 m), and drop
from above the heap's top thrown out over a disc, so the hub's heap grows as a
mound; a charm of 440 books makes one ~0.8 m high and ~1.7 m across (dropped from
a fixed height they built a 6.7 m tower). Each department has a room (the Hall's are its wings) with
a doorway, which Sort Sense I and Wayfinding use.

Familiars (`game/familiar.rs`) are paper cranes that ride on the player's
shoulder until posted: G at a spot on the floor posts the one riding longest, G
at a posted one (its crane or perch) takes it back. A shelver files loose books
lying within a few metres of its post whose slots are a short walk from it; a
courier carries books lying near its post to the post of a shelver that can file
them, or else to their room's drop spot, and leaves books of veiled departments.
Each post keeps a walking-distance field from it (`nav::Field`); a trip follows
the field's way back toward the post until it meets the way out to the target,
at flying height, straightened wherever the line is clear to sight, so cranes
use doorways and stairs, or cross the well over a railing, and never pass
through a wall or floor (checked frame by frame in the familiar tests). A posted
familiar that finds nothing to do for two minutes folds up and comes back to the
shoulder (a burst where it was, a hint saying so), so a post that runs dry costs
no walk back to it.

"Nearest" means nearest on foot: `nav.rs` keeps a Dijkstra distance field from
the player over a 0.5 m grid per level (furniture inflated by the player's
radius; neighbouring cells more than a step apart in height are not linked;
the top of a flight of stairs links to the floor above),
recomputed when the player changes cell (~0.2 ms). A slot's cost is the walk to
the aisle in front of it, so the far side of a double-sided case costs the walk
round the end of the row. Tidy Arms switches the active book only when another
is 0.6 m nearer, and leaves a book you picked by cycling active until it is
filed or you gather again. Sort Sense's marker sits on the slot when the walk is a straight
line, and otherwise on the farthest visible point along the route.
Books belong to departments (the Hall's are its two wings). A gate (drawn as the
veil) closes the way into a department until the departments it counts are
shelved to a fraction: the East Wing opens at 60% of the West. A closed gate is
a solid to walking, throwing, gathering, aiming and routes; gate states follow
from what is shelved, so saves don't store them.

Pacing is tuned with the pacing bot (`game/pacing.rs`; `cargo test --release
pacing -- --ignored --nocapture`), which plays with the real mechanisms: it walks
on the nav field to the pile with the most books per second of walking; a plain
click takes the top of the pile whatever it is (books it can't file yet go back
down where it stands); with Sifting Touch it seeks the coarsest group it has with
the most books in reach (an F when the group changes) and scoops only it, and
scoops again only at piles it can see; with the Gathering Charm ready, it sends
what is left where it gathered to the hub when most of it belongs in other rooms;
it reads (time per series and category by reading skill, nothing while Sort Sense
or Glimpse shows the way, Glimpse cast whenever ready); it files nearest slot
first, as Tidy Arms picks (a click per book, or per series with Bundle, the aura
at its rate); it walks to post the familiars riding with it (couriers round the
hub or at heaps of books far from home, shelvers at cases with books piling up
near them; idle ones come back by themselves) and runs their real code over each
trip; and it buys the cheapest skill it can afford, or favours one tech path
(`HUSHFOLD_PATH`). Costs are fitted to the purchase schedules in `tuning/`
(`tuning/retune.py`). Between slots it charges the straight line (no less than the
difference of their walks from where filing began): right once the aura files
through a case, optimistic before it. The late-game engine is Deeper Arms IV ×
Shelving Aura, so the aura's books-per-second rate is the main dial. History:
- 0.1.0 (aura 6/15/40 books/s, 11 m at III): ~1.0 h, the whole East Wing in ~16 min.
- 0.1.1: ~1.4 h, the East Wing the last ~30 min, last upgrade affordable at ~86% shelved.
- Loose piles: ~1.5 h, the East Wing still the last ~30 min, last upgrade at ~92%.
  Piles now spread, and the bot walks to the strays at their edges.
- Same game, honest walking (the bot used to walk through bookcases): ~2.0 h.
- The Great Library with the Hall's skills: ~16.6 h; every upgrade bought by
  2.8 h, every department open by ~7 h, then a flat ~6–7k books an hour. The
  skills run out in the first sixth; phase 3 rebuilds them for this.
- Phase 3 (skills by level, familiars at posts, the bot above): the Hall ~1.6 h,
  the Great Library ~10.6 h, purchases spread to ~9.4 h, every department open
  by ~5.5 h. Familiars shelve ~6% of the Great Library and carry ~8% to their
  rooms, about what walking to post them costs, while taking about half the
  lumens; the bot spends ~4% of its time on stairs.
- After the first playtest (tap and scoop, seeking, earned reading, spells,
  Homing Toss; costs fitted to purchase schedules in `tuning/`): the Hall ~1.7 h,
  the Great Library ~9.7 h, a new kind of skill every ~5 min and every ~11–25
  min respectively until near the end. Favouring one tech path, the Great
  Library takes 8.5 h (filing) to 13.1 h (hands): no path but filing has an
  engine of its own yet.

## Rendering
Box-only geometry, per-instance `kind` selects procedural material in WGSL
(wood, book, floor, stone, rug, emissive, plaque). Forward lighting with up to
64 point lights (the player's candle, familiars, then the library's lights
nearest the eye), hemisphere ambient, fog. 4x MSAA HDR -> bloom -> ACES tonemap
-> egui. Held books are drawn in a second pass with cleared depth.

`scene.rs` keeps books lying still (in piles or shelved) and the ghosts of empty
slots in 8 × 6 × 8 m chunks with their own GPU buffers. Each frame every book's
state and Loose's draw version (bumped when a book comes to rest, leaves its rest,
or starts or ends a fall) are compared with what its chunk was built from; only
changed chunks are rebuilt and uploaded. Books on the move (flying, falling,
filing, carried, flying to the hand) are drawn per frame. Chunks outside the view
frustum are skipped; the rest are drawn after the walls and floors, nearest
first. F3 shows GPU time from timestamp queries where the adapter has them.

## Persistence
Deterministic world from a seed; save = seed + shelved/held/in-flight ids +
lumens + upgrade levels by name + the poses of loose books that moved from where
they were generated (books in the air are saved where they are and fall on
load), JSON in $XDG_DATA_HOME/hushfold/save.json (%APPDATA% on Windows),
autosaved. 0.1.x saves load: their piles keep the generated shape and fall into
the gaps left by books already taken. Settings live beside it in settings.json.
