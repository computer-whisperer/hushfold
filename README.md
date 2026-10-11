# Hushfold Library

A cozy first-person "tidy the magical library" game in Rust: books lie in piles
across a moonlit library. Gather armfuls, find each book's category, series and
volume slot, and file it. Lumens earned from filing buy abilities that make the
work progressively smoother, up to folded-paper familiars that carry and shelve
books wherever you post them.

Two libraries, each with its own save (switch on the pause screen):
- **The Hall**: one storey, ~11,700 books in ~2,300 series, two wings.
- **The Great Library** (in development): three storeys round a skylit atrium,
  twelve department rooms opened one by one, each laid out and furnished in its
  own way (an observatory under a painted sky, an alchemist's workshop, a
  glasshouse herbarium, a hearth room…), ~118,000 books, about nine hours.

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
| Space | jump |
| Mouse | look |
| Left click | tap a book to take it · hold on a pile to scoop what is inside the ring · click a series run to file |
| Right mouse (hold) | charge a toss, with an arc showing where it lands; release to throw every book you carry of the group you seek, or else of the active book's group |
| X (hold) | the same for your whole armful (a tap drops it just ahead) |
| F | seek the aimed book's kind: scoops take only those books, from any depth (Sifting Touch); F again stops |
| R | the level you seek and toss at: category, or with upgrades department or series |
| 1–5 | cast a spell you have learned (the hotbar shows each one's recharge) |
| G | post a familiar riding with you at the floor you aim at, or take a posted one back |
| Scroll, Q / E | cycle the active book |
| Tab | the Grimoire (upgrades) |
| M | music on/off (remembered) |
| F3 | performance overlay |
| F9 | cheats, for trying late-game situations: lumens, every skill, lifting veils, shelving the library to a fraction, going to any room |
| F11 | fullscreen (remembered) |
| F12 | screenshot to `~/Pictures` (`%USERPROFILE%\Pictures` on Windows); a note says where it went |
| Esc | pause: which library, settings (look speed, invert, field of view, head bob, volumes), save-and-quit, and start over |

Progress autosaves every 20 s and on pause or quit. Saves live in
`$XDG_DATA_HOME/hushfold/` on Linux and `%APPDATA%\hushfold\` on Windows:
`save.json` for the Hall, `great.json` for the Great Library. **Start over** on the
pause screen replaces the current library's save with a fresh one (settings, in
`settings.json` beside them, and the other library are kept).

## How filing works

- Each category owns three bookcases on one side of an aisle (in the Great
  Library four to six, laid out as its room has them: rows, wall cases, alcoves);
  the sign and lantern above the middle case name it. Where a category's cases
  stand in two parts (either side of a nave, a door or a window), each part has a
  sign of its own, giving the part's letters under the name, and the Observatory's
  ranges say at their nave ends what is down either side. With Signposts, signs of
  categories you carry show through the stacks, marked "other side" when their shelves face
  away from you; one in plain view is only marked, or named above its board when
  too far off to read. Lanterns brighten with progress and burn in the category's colour once it is
  complete.
- Within a category, series are alphabetical (ignoring leading articles), top
  shelf to bottom, left to right. Shelf edges carry their letter ranges.
- Empty slots show as faint outlines in the series' colour; with Kindred Glow,
  outlines of series you are carrying glow (at its last level with a glint
  sweeping through them). While you carry books, a quiet line below the crosshair
  says where the active book goes: its category and the start of its shelf's
  letter range ("Astronomy · Al"; its room too, when you are outside it), to read
  against the shelf you look at, whose tip gives its range. The arms card (bottom
  left) gives the same with the department. Click anywhere on a series'
  run to file a volume you carry; it flies to its exact slot.
- The Hall's East Wing is veiled until 40% of the West Wing is home. In the
  Great Library the second room is veiled until a quarter of the Observatory's
  books are home, and each room after it until half the books of the rooms opened
  before it are home (the Map Room and the Chronicle Hall open together); a
  department's books start in its own room, the atrium or rooms that open earlier.
  Colour tells a book's department there, and the spine pattern its category.
- Upstairs, and in the Bestiary, rooms ask something of their own: two open
  together, each heaped with the other's books, some of them heavy folios; one
  holds another's songs; a ward quiets magic until its room is put in order;
  stopped clocks hold books in the air until the series on their faces are filed;
  a draught bends what you toss; one room's books were left all over; and some
  books won't sit still. Each room says what it asks when you first step in.
- Books are loose: they lie where they fall, piles slump when you pull books
  out from underneath, and tossed books tumble, bounce off the furniture and
  land in a scatter. Toss a category's books toward its shelves to pre-sort a
  pile, then gather the heap you made.
- Wading through heaps slows you, more the deeper they are (knee-deep, to about
  half), and you can't jump far out of them; a chest-high heap can't be walked
  into. Clearing the heaps in your way frees your feet.

## Upgrades

The Grimoire groups them by what they help with; each library offers the ones
that fit it. A skill you've learned can be turned down to a lower level, or off,
from its entry ("In use"), and back up, freely; learning its next level puts it
all back to use.

- **Reading**: Signposts (signs of the categories you carry show through the
  stacks), Kindred Glow (the runs of series you carry glow: on the case in front
  of you, across the room, everywhere), Sort Sense (the exact slot pulses and a
  compass points the way: in the book's room, then anywhere), Wayfinding (the
  way to the book's room), the Seeker's Lens (what you seek glows). Before them,
  read the signs, the letter ranges on the shelf edges and the titles.
- **Moving**: Sifting Touch (seek a kind of book and scoop only it, wider with
  each level), Broad Strokes and Fine Fingers (seek by department, by series),
  Long Toss, Homing Toss (a book you toss that lands in front of its own shelf,
  near its slot, slips into it), Lodestone Grasp (long reach).
- **Filing**: Deeper Arms (6 → 96 books, 192 in the Great Library), Swift
  Slippers, Tidy Arms (self-sorting armful, nearest home first), Series Bundle
  (file a whole series at once), Shelving Aura (books drift home as you pass in
  front of their shelves).
- **Familiars**: folded-paper birds that ride with you until you post them. A
  shelver (a crane) files the books lying near its post whose shelves are close
  by; a courier (a swift) carries books lying near its post to a shelver that can
  file them, or to their room. Familiar Wings makes them faster and stronger.
- **Spells**, each on its own recharge: Glimpse (for a few seconds every series
  you carry glows, and a marker shows each through the walls), the Gathering
  Charm (Great Library: a heap you aim at streams to the great heap in the
  atrium, where couriers can carry it on to the rooms), Echo (the heaps holding
  the kind of book you seek are marked through the walls), Summon Series (loose
  volumes of the active book's series fly to your arms), Shelving Rush (for a
  few seconds, books you carry near their slots fly home as fast as you pass, from
  in front of their shelves).
- A familiar with nothing to do for two minutes comes back to your shoulder by
  itself.

## Technical notes

- wgpu 29 + winit 0.30 + egui 0.34. Everything is instanced: unit boxes, and
  meshes fitted to the box (turned, extruded or swept, made by the library as it
  is generated: gears, dials, rings, jars), with procedural materials (wood grain,
  spines with patterns and volume numerals, paper edges, planks, stone, marble,
  brass, glass, flames, mist, fretwork, a night sky) selected per instance in
  `src/shader.wgsl`. Gears turn, pendulums swing, clocks keep time, lanterns and
  candles float, banners and drapes stir and ladders slide, posed each frame
  (`src/library/motion.rs`); each department's room has its own light and fog.
- Up to 64 point lights in a forward pass (the nearest), 4× MSAA HDR, a 6-level
  bloom chain and ACES tonemapping. Held books are drawn in a second pass with
  cleared depth so they never clip into shelves.
- Books lying still and the outlines of empty slots live in chunks of space on
  the GPU, re-uploaded only when something in a chunk changes; chunks out of
  view are skipped.
  CPU frame time is ~0.15 ms in the Hall and ~0.75 ms in the Great Library
  (~360 and ~185 fps unpaced at 1864×2088 on an RX 6900 XT, measured before
  the visual pass, which added 8–19% GPU time and no CPU time).
  `HUSHFOLD_UNPACED=1` renders without compositor pacing for benchmarking.
- Signs and shelf labels are lettered in the world (`src/text.rs`): glyph quads
  from a signed-distance atlas of the faces built into the game (Cinzel for the
  library's own lettering), lit like the boards they're on. The same atlas letters
  the books, each series in a typeface of its binding's day, from Garamond and
  Caslon on calf to Victorian faces on cloth and textura or fraktur on vellum. The
  HUD is set in Libre Caslon Text.
- All audio is synthesized, nothing loaded (`src/audio.rs`, `src/audio/`): the
  effects' clips on a thread at startup, the rest as it plays. Effects that
  happen somewhere (books filed, books landing, fires, familiars' wings) are
  heard from there; each place has its own ambience, crossfaded as you walk
  (`src/game/sound.rs` tells the mixer where you are); the music is composed
  afresh piece by piece, with quiet between pieces, and chimes are played in its
  key. `HUSHFOLD_SOUND_DIR=dir cargo test --release export_sounds -- --ignored`
  writes every sound to WAV files.
- Loose books use semi-physics (`src/loose.rs`): ballistic flight with spin,
  bounces off furniture, and a 4 cm heightfield per floor that landing books
  settle onto, sliding downhill while the support is steeper than they rest on.
- The world is deterministic from a seed, so a save is ids, counters and the
  poses of books that have moved from where they started.

## Development

- `cargo test --release`: generation invariants, pile physics (nothing floats or
  sinks after books are pulled out; tosses land in the open), save round-trip,
  audio sanity.
- Pacing: `cargo test --release pacing -- --ignored --nocapture` plays each
  library headless with the pacing bot (`HUSHFOLD_PATH=reading|hands|familiars|filing`
  makes it favour one tech path; `HUSHFOLD_BASICS=1` lets every path buy carrying,
  walking and the reading basics at face value). Skill costs are fitted to the purchase schedules
  in `tuning/` (the order and minute each purchase should come) with
  `tuning/retune.py`; see its header.
- Scripted runs for screenshots and smoke tests:
  `HUSHFOLD_UNPACED=1 HUSHFOLD_SCRIPT=script.txt HUSHFOLD_SHOT_DIR=out cargo run --release`.
  Scripted runs use a fixed 60 Hz step and never touch the real save; they start
  in the Hall unless `HUSHFOLD_LIBRARY=great`. Commands
  (one per line): `pose x y z yaw pitch`, `wait frames`, `walk fwd frames`,
  `click` (a tap), `press` / `release` (hold and release the left button),
  `cycle n`, `charge [all]` / `throw` (hold and release the toss; `all`: the armful toss, X), `seek`, `regroup`,
  `post`, `cheats on|off`, `cheat lumens n|skills|veils|shelve fraction|goto [department]`,
  `facepile`, `faceslot`, `facefam i`, `lookat x y z dist`, `fill category fraction`,
  `take category n`, `unveil`, `library hall|great` (a fresh one), `grant lumens`,
  `buy <Upgrade>`, `menu on|off`,
  `intro on|off|confirm`, `newgame`, `stats on|off`, `shot name`, `print` (includes where a charging
  toss would land), `printactive`, `quit`. `fill` and `take` pull books from
  anywhere in the piles, which then settle.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

The fonts built into the game are each under the SIL Open Font License 1.1; each
family's licence and where it came from are in `assets/fonts/<Family>/` (and in
`fonts/` in the release archives). Most are subsets made by
`assets/fonts/subset.py`; IM Fell English, UnifrakturMaguntia and Uncial Antiqua
reserve their names and are built in unmodified.

- Book titles: EB Garamond, Cardo, IM Fell English, Libre Caslon Text, Ibarra Real
  Nova, Castoro Titling, Baskervville, Bodoni Moda, Old Standard TT, Coustard,
  Goudy Bookletter 1911, Sorts Mill Goudy, Texturina, UnifrakturMaguntia, Uncial
  Antiqua.
- Signs and shelf labels: Cinzel.
- The HUD: Libre Caslon Text, and Noto Sans Symbols 2 for its symbols.
