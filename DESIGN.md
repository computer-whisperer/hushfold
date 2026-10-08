# Hushfold — design notes

A first-person "tidy the magical library" game. Rust, wgpu 29 + winit 0.30 + egui 0.34.
Performance is a design pillar: the whole book population (~10k) is one instanced
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
- Empty slots render as faint ghosts in the series colour; ghosts of series you
  are holding glow brighter.

## Verbs
- Click heap: grab the top of the heap up to arm capacity (heaps are stacks,
  so removal never leaves floating books).
- Click shelf: aiming anywhere on a series run places a held book of that series
  into its exact slot (forgiving aim; the skill is finding the run).
- Scroll/Q/E: cycle the active book (HUD shows its category/series/volume).
- Tab: Grimoire (upgrades). Shift: hurry.

## Progression (Lumens ✦)
1 per book, +volumes on series completion, +100 per category.
Upgrades: Deeper Arms I–IV, Swift Slippers, Sort Sense (compass + exact-slot pulse),
Series Bundle (place whole series at once), Tidy Arms (stack sorted by slot,
active = nearest target), Lodestone Grasp (long reach, hold to pull),
Shelving Aura I/II (books float to nearby slots on their own),
Paper Familiars (autonomous helpers), Familiar Wings.
East wing is veiled until 60% of the West wing is sorted.

## Rendering
Box-only geometry, per-instance `kind` selects procedural material in WGSL
(wood, book, floor, stone, rug, emissive, plaque). Forward lighting with up to
64 point lights, hemisphere ambient, fog. 4x MSAA HDR -> bloom -> ACES tonemap
-> egui. Held books are drawn in a second pass with cleared depth.

## Persistence
Deterministic world from a seed; save = seed + shelved/held ids + lumens +
upgrades, JSON in $XDG_DATA_HOME/hushfold/save.json, autosaved.
