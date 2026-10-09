# Hushfold Library

A cozy first-person "tidy the magical library" game in Rust: books lie in piles
across a moonlit library. Gather armfuls, find each book's category, series and
volume slot, and file it. Lumens earned from filing buy abilities that make the
work progressively smoother, up to paper-crane familiars that carry and shelve
books wherever you post them.

Two libraries, each with its own save (switch on the pause screen):
- **The Hall**: one storey, ~11,700 books in ~2,300 series, two wings.
- **The Great Library** (in development): three storeys round a skylit atrium,
  twelve department rooms opened one by one, ~118,000 books, about nine hours.

Prebuilt Linux and Windows binaries are on the
[releases page](https://github.com/computer-whisperer/hushfold/releases), or build it yourself:

```
cargo run --release
```

On Linux, building needs the ALSA headers (`libasound2-dev` on Debian/Ubuntu,
`alsa-lib` on Arch).

## Controls

| | |
|---|---|
| WASD / Shift | walk / hurry |
| Mouse | look |
| Left click | gather the top of a pile · file onto a series run |
| Right mouse (hold) | charge a toss, with an arc showing where it lands; release to throw every book you carry of the active book's group |
| F | sift: draw only books of the aimed book's group out of a pile (an upgrade) |
| R | what toss and sift take: the active book's category, or with upgrades its department or series |
| V | the Seeker's Lens: loose books of the active book's group glow (an upgrade) |
| G | post a familiar riding with you at the floor you aim at, or take a posted one back |
| Scroll, Q / E | cycle the active book |
| Tab | the Grimoire (upgrades) |
| M | music on/off (remembered) |
| F3 | performance overlay |
| F9 | cheats, for trying late-game situations: lumens, every skill, lifting veils, shelving the library to a fraction, going to any room |
| F11 | fullscreen (remembered) |
| F12 | screenshot to `~/Pictures` (`%USERPROFILE%\Pictures` on Windows) |
| Esc | pause: which library, settings (look speed, invert, field of view, volumes), save-and-quit, and start over |

Progress autosaves every 20 s and on pause or quit. Saves live in
`$XDG_DATA_HOME/hushfold/` on Linux and `%APPDATA%\hushfold\` on Windows:
`save.json` for the Hall, `great.json` for the Great Library. **Start over** on the
pause screen replaces the current library's save with a fresh one (settings, in
`settings.json` beside them, and the other library are kept).

## How filing works

- Each category owns three bookcases (five in the Great Library) on one side of
  an aisle; the sign and lantern above the middle case name it. Signs of categories you carry show
  through the stacks, marked "other side" when their shelves face away from you. Lanterns brighten with progress and
  burn in the category's colour once it is complete.
- Within a category, series are alphabetical (ignoring leading articles), top
  shelf to bottom, left to right. Shelf edges carry their letter ranges.
- Empty slots show as faint outlines in the series' colour; outlines of series
  you are carrying glow, with a glint sweeping through them. Click anywhere on a
  series' run to file a volume you carry; it flies to its exact slot.
- The Hall's East Wing is veiled until 60% of the West Wing is home. In the
  Great Library each room is veiled until half the books of the rooms opened
  before it are home; a department's books start in its own room, the atrium or
  rooms that open earlier. Colour tells a book's department there, and the
  spine pattern its category.
- Books are loose: they lie where they fall, piles slump when you pull books
  out from underneath, and tossed books tumble, bounce off the furniture and
  land in a scatter. Toss a category's books toward its shelves to pre-sort a
  pile, then gather the heap you made.

## Upgrades

The Grimoire groups them by what they help with; each library offers the ones
that fit it.

- **Reading**: Sort Sense (the exact slot pulses and a compass points the way:
  in the book's room, then anywhere), Wayfinding (the way to the book's room),
  the Seeker's Lens.
- **Moving**: Sifting Touch (sift a group out of a pile, reaching wider with
  each level), Broad Strokes and Fine Fingers (sort by department, by series),
  Long Toss, Lodestone Grasp (long reach, hold to gather).
- **Filing**: Deeper Arms (6 → 96 books), Swift Slippers, Tidy Arms
  (self-sorting armful, nearest home first), Series Bundle (file a whole series
  at once), Shelving Aura (books drift home as you pass).
- **Familiars**: paper cranes that ride with you until you post them. A shelver
  files the books lying near its post whose shelves are close by; a courier
  carries books lying near its post to a shelver that can file them, or to
  their room. Familiar Wings makes them faster and stronger.

## Technical notes

- wgpu 29 + winit 0.30 + egui 0.34. Everything is a unit box, drawn instanced
  with procedural materials
  (wood grain, spines with patterns and volume numerals, paper edges, planks,
  stone, rug) selected per instance in `src/shader.wgsl`.
- Up to 64 point lights in a forward pass (the nearest), 4× MSAA HDR, a 6-level
  bloom chain and ACES tonemapping. Held books are drawn in a second pass with
  cleared depth so they never clip into shelves.
- Books lying still and the outlines of empty slots live in chunks of space on
  the GPU, re-uploaded only when something in a chunk changes; chunks out of
  view are skipped.
  CPU frame time is ~0.15 ms in the Hall and ~0.75 ms in the Great Library
  (~360 and ~185 fps unpaced at 1864×2088 on an RX 6900 XT).
  `HUSHFOLD_UNPACED=1` renders without compositor pacing for benchmarking.
- All audio is synthesized at startup (`src/audio.rs`): book thunks, page
  rustles, pentatonic chimes, fire crackle and a generative music-box loop.
- Loose books use semi-physics (`src/loose.rs`): ballistic flight with spin,
  bounces off furniture, and a 4 cm heightfield per floor that landing books
  settle onto, sliding downhill while the support is steeper than they rest on.
- The world is deterministic from a seed, so a save is ids, counters and the
  poses of books that have moved from where they started.

## Development

- `cargo test --release`: generation invariants, pile physics (nothing floats or
  sinks after books are pulled out; tosses land in the open), save round-trip,
  audio sanity.
- Scripted runs for screenshots and smoke tests:
  `HUSHFOLD_UNPACED=1 HUSHFOLD_SCRIPT=script.txt HUSHFOLD_SHOT_DIR=out cargo run --release`.
  Scripted runs use a fixed 60 Hz step and never touch the real save; they start
  in the Hall unless `HUSHFOLD_LIBRARY=great`. Commands
  (one per line): `pose x y z yaw pitch`, `wait frames`, `walk fwd frames`,
  `click`, `cycle n`, `charge` / `throw` (hold and release the toss), `sift`,
  `facepile`, `faceslot`, `facefam i`, `lookat x y z dist`, `fill category fraction`,
  `take category n`, `unveil`, `library hall|great` (a fresh one), `grant lumens`,
  `buy <Upgrade>`, `menu on|off`,
  `intro on|off|confirm`, `newgame`, `stats on|off`, `shot name`, `print` (includes where a charging
  toss would land), `printactive`, `quit`. `fill` and `take` pull books from
  anywhere in the piles, which then settle.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
