# Hushfold — design notes

A first-person "tidy the magical library" game. Rust, wgpu 29 + winit 0.30 + egui 0.34.
Performance is a design pillar: the whole book population (~11.7k) is one instanced
draw, re-uploaded each frame; nothing scales per-object on the CPU beyond a copy.

## Fantasy
A moonlit, dust-mote library. Two wings of freestanding double-sided bookcases
around a central reading room with a chandelier. Heaps of books everywhere.
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

## Verbs
- Click heap: grab the top of the heap up to arm capacity (heaps are stacks,
  so removal never leaves floating books).
- Click shelf: aiming anywhere on a series run places a held book of that series
  into its exact slot (forgiving aim; the skill is finding the run).
- Scroll/Q/E: cycle the active book (HUD shows its category/series/volume).
- Tab: Grimoire (upgrades). Shift: hurry.

## Progression (Lumens ✦)
1 per book, +volumes on series completion, +100 per category.
Upgrades: Deeper Arms I–IV, Swift Slippers I–II, Sort Sense (compass + exact-slot
pulse), Series Bundle (place whole series at once), Tidy Arms (stack sorted by
slot, active = nearest target), Lodestone Grasp (long reach, hold to pull),
Shelving Aura I–III (books float to nearby slots on their own),
Paper Familiars ×5 (autonomous helpers), Familiar Wings I–II.

"Nearest" means nearest on foot: `nav.rs` keeps a Dijkstra distance field from
the player over a 0.5 m grid (furniture inflated by the player's radius),
recomputed when the player changes cell (~0.2 ms). A slot's cost is the walk to
the aisle in front of it, so the far side of a double-sided case costs the walk
round the end of the row. Tidy Arms switches the active book only when another
is 0.6 m nearer, and leaves a book you picked by cycling active until it is
filed or you gather again. Sort Sense's marker sits on the slot when the walk is a straight
line, and otherwise on the farthest visible point along the route.
East wing is veiled until 60% of the West wing is sorted.

## Rendering
Box-only geometry, per-instance `kind` selects procedural material in WGSL
(wood, book, floor, stone, rug, emissive, plaque). Forward lighting with up to
64 point lights, hemisphere ambient, fog. 4x MSAA HDR -> bloom -> ACES tonemap
-> egui. Held books are drawn in a second pass with cleared depth.

## Persistence
Deterministic world from a seed; save = seed + shelved/held/in-flight ids +
lumens + upgrades, JSON in $XDG_DATA_HOME/hushfold/save.json (%APPDATA% on
Windows), autosaved. Settings live beside it in settings.json.
