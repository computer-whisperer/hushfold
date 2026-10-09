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

## Loose books
Every book not shelved or carried is loose (`src/loose.rs`), simulated with
semi-physics rather than rigid bodies (see SCALING.md for why):
- Each level has a 4 cm heightfield holding the floor (or a hole where it has
  none), stairs, furniture tops (furniture is a solid column from its floor) and
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
  edge falls too.
- Gathering takes only books with no wall, case or closed gate between them and
  the aim point.
- Generation picks drop points per pile; dropping them through the same code
  makes the starting piles.

## Verbs
- Click a pile: gather the topmost books within a radius of the aim point, up
  to arm capacity; whatever lay on them falls.
- Hold right mouse: charge a toss (the arc and landing ring show where it goes);
  release to throw every held book of the active book's category as a stream.
- F (Sifting Touch): draw books of the aimed book's category out of the pile
  around the aim point, from any depth.
- Click shelf: aiming anywhere on a series run places a held book of that series
  into its exact slot (forgiving aim; the skill is finding the run).
- Scroll/Q/E: cycle the active book (HUD shows its category/series/volume).
- Tab: Grimoire (upgrades). Shift: hurry.

## Progression (Lumens ✦)
1 per book, +volumes on series completion, +100 per category.
Upgrades: Deeper Arms I–IV, Swift Slippers I–II, Sort Sense (compass + exact-slot
pulse), Sifting Touch I–II (sift a category out of a pile; 0.9 / 1.8 m),
Series Bundle (place whole series at once), Tidy Arms (stack sorted by
slot, active = nearest target), Lodestone Grasp (long reach, hold to pull),
Shelving Aura I–III (books float to nearby slots on their own),
Paper Familiars ×5 (autonomous helpers), Familiar Wings I–II.

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

Pacing is tuned with ignored tests (`--ignored --nocapture`): `familiar_throughput`
runs the real familiar code headless to measure books/s per Wings level, and
`pacing_simulation` (the Hall) and `pacing_great_library` play an idealised bot
(the pile with the least walking per book, clicking until its arms are full,
nearest slot, cheapest upgrade, walking measured on the nav field; it never
tosses or sifts, and picks up only books whose shelves are open) using those rates. The late-game engine is Deeper Arms IV ×
Shelving Aura, so the aura's books-per-second rate is the main dial. History:
- 0.1.0 (aura 6/15/40 books/s, 11 m at III): ~1.0 h, the whole East Wing in ~16 min.
- 0.1.1: ~1.4 h, the East Wing the last ~30 min, last upgrade affordable at ~86% shelved.
- Loose piles: ~1.5 h, the East Wing still the last ~30 min, last upgrade at ~92%.
  Piles now spread, and the bot walks to the strays at their edges.
- Same game, honest walking (the bot used to walk through bookcases): ~2.0 h.
- The Great Library with the Hall's skills: ~16.6 h; every upgrade bought by
  2.8 h, every department open by ~7 h, then a flat ~6–7k books an hour. The
  skills run out in the first sixth; phase 3 rebuilds them for this.

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
