# Hushfold Library

A cozy first-person "tidy the magical library" game in Rust: ~11,700 books in
~2,300 series lie in heaps across a moonlit hall. Gather armfuls, find each
book's category, series and volume slot, and file it. Lumens earned from filing
buy abilities that make the work progressively smoother, up to paper-crane
familiars that shelve on their own.

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
| Left click | gather from a heap · file onto a series run |
| Scroll, Q / E | cycle the active book |
| Tab | the Grimoire (upgrades) |
| M | music on/off |
| F3 | performance overlay |
| F11 | fullscreen (remembered) |
| F12 | screenshot to `~/Pictures` (`%USERPROFILE%\Pictures` on Windows) |
| Esc | pause: settings (look speed, invert, field of view, volumes) and save-and-quit |

Progress autosaves every 20 s and on pause or quit. The save lives in
`$XDG_DATA_HOME/hushfold/save.json` on Linux and `%APPDATA%\hushfold\save.json` on
Windows (delete it to start over); settings sit beside it in `settings.json`.

## How filing works

- Each category owns three bookcases on one side of an aisle; the sign and
  lantern above the middle case name it. Signs of categories you carry show
  through the stacks, marked "other side" when their shelves face away from you. Lanterns brighten with progress and
  burn in the category's colour once it is complete.
- Within a category, series are alphabetical (ignoring leading articles), top
  shelf to bottom, left to right. Shelf edges carry their letter ranges.
- Empty slots show as faint outlines in the series' colour; outlines of series
  you are carrying glow, with a glint sweeping through them. Click anywhere on a
  series' run to file a volume you carry; it flies to its exact slot.
- The East Wing is veiled until 60% of the West Wing is home.

## Upgrades

Deeper Arms (6 → 96 books), Swift Slippers, Sort Sense (exact-slot pulse and
compass), Tidy Arms (self-sorting armful, nearest home first), Series Bundle
(file a whole series at once), Lodestone Grasp (long reach, hold to gather),
Paper Familiar ×5, Shelving Aura I–III (books drift home as you pass),
Familiar Wings.

## Technical notes

- wgpu 29 + winit 0.30 + egui 0.34. Everything is a unit box: one instanced
  draw for all books and one for the static hall, with procedural materials
  (wood grain, spines with patterns and volume numerals, paper edges, planks,
  stone, rug) selected per instance in `src/shader.wgsl`.
- Up to 64 point lights in a forward pass, 4× MSAA HDR, a 6-level bloom chain
  and ACES tonemapping. Held books are drawn in a second pass with cleared
  depth so they never clip into shelves.
- All book transforms are re-uploaded every frame (~19k instances, ~1.2 MB).
  CPU frame time is ~0.25 ms; GPU ~1–2 ms at 1864×2088 on an RX 6900 XT.
  `HUSHFOLD_UNPACED=1` renders without compositor pacing for benchmarking.
- All audio is synthesized at startup (`src/audio.rs`): book thunks, page
  rustles, pentatonic chimes, fire crackle and a generative music-box loop.
- The world is deterministic from a seed, so a save is just ids and counters.

## Development

- `cargo test --release`: generation invariants, save round-trip, audio sanity.
- Scripted runs for screenshots and smoke tests:
  `HUSHFOLD_UNPACED=1 HUSHFOLD_SCRIPT=script.txt HUSHFOLD_SHOT_DIR=out cargo run --release`.
  Scripted runs use a fixed 60 Hz step and never touch the real save. Commands
  (one per line): `pose x y z yaw pitch`, `wait frames`, `walk fwd frames`,
  `click`, `cycle n`, `faceheap`, `faceslot`, `facefam i`, `fill category fraction`,
  `take category n`, `unveil`, `grant lumens`, `buy <Upgrade>`, `menu on|off`,
  `intro on|off`, `stats on|off`, `shot name`, `print`, `printactive`, `quit`.
  `fill` and `take` pull books out of the middle of heaps, so use them for shelf
  shots rather than heap shots.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
