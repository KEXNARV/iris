# Channels

The numbers the parts of a buddy share, by name (see `src/core/recipe/bus.rs` and the store
plan, `docs/store-phase-2-plan.md`). A recipe wires a part's input to any of them with
`inputs = { spin = "music" }`; without wiring, an input reads the channel of its own name.

**These names are a public API.** Recipes in the wild use them: once published, a name isn't
renamed or removed, only added. All values are `f64`; "on" is > 0.5; "none" is NaN.

## The script

Whatever a recipe's `[script]` defines: each channel eases toward its value for the current
state (`builtin:script`). The built-in recipes use these, and any recipe can add its own:

| Channel     | What it drives in the built-in parts                          |
|-------------|---------------------------------------------------------------|
| `blob_r`    | the body's radius at rest                                     |
| `amp`, `wob`, `chaos` | how much the body's outline ripples, how fast, how wildly |
| `arc_speed`, `arcs`, `arc_len` | the arcs: spin speed, how visible, how long  |
| `ripple`    | the voice waves (speaking)                                    |
| `orbit`     | the ideas orbiting (thinking)                                 |
| `sweep`     | the radar (running) and the scale's sweep                     |
| `fill`, `vu`, `cardinal`, `tests` | the scale's modes                       |
| `dashed`    | the dotted outline (offline)                                  |
| `scan`      | the reading line                                              |
| `stitch`    | the stitches (editing)                                        |
| `packets`   | the packages (web)                                            |
| `bud`       | the sprout (delegating)                                       |
| `zzz`       | the zees and slow breathing (sleeping)                        |
| `pupil`, `dilate` | how visible the pupil is, how open                      |
| `bob`, `droop` | bobbing (asking) and sagging (no voice)                    |
| `motes`     | how visible the motes are                                     |
| `boot`      | booting (1 while it boots)                                    |
| `lens`      | the magnifier (searching)                                     |
| `plan`      | the task marks (planning)                                     |
| `git`       | the git graph                                                 |
| `press`     | the presses (compacting)                                      |

## Published by the built-in parts

| Part                | Channels                                                               |
|---------------------|------------------------------------------------------------------------|
| `builtin:script`    | `col.r`, `col.g`, `col.b`: the buddy's color now (0–255)               |
| `builtin:spring`    | `spring.sq` (squash: + wider, − taller), `spring.v` (any part pushes by adding to it) |
| `builtin:flashes`   | `red`, `shake`, `deflate`, `nope`, `copy`, `pass`, `swallow`: flashes that events leave, fading to 0 |
| `builtin:rhythm`    | `phase` (the body's wobble), `arc_phase` (the spin), `level` (voice, 0–1), `music` (0–1), `groove` (how much it dances, 0–1) |
| `builtin:kids`      | `kids.alive` (on while there are subagents), `heard.t`, `heard.a` (when and from what angle a kid last sent something; none if never) |
| `builtin:gestures`  | `gesture` (0 none, 1 stretch, 2 looks at the clock, 3 looks at the chat, 4 yawn), `yawn` (0–1) |
| `builtin:gaze`      | `gaze.x`, `gaze.y` (where it looks, world units), `gaze.lim` (how far it can look) |
| `builtin:presence`  | `body.x`, `body.y`, `body.radius`, `body.base` (where the body is; base = radius at rest), `ink.body`, `ink.ring` (the inks for the body and its surroundings: red on errors), `boot.t` (boot progress 0–1), `breath`, `beat`, `body.voice` (radius added by breathing, the beat and the voice), `body.sx`, `body.sy` (stretch), `blink` (on while blinking), `low` (on in a small panel or a kid: fine details off) |
| `builtin:magnifier` | `magnifier.x`, `magnifier.y`, `magnifier.r`                            |
| `builtin:eye`       | `pupil.x`, `pupil.y`, `pupil.r`, `pupil.blink`, `pupil.ink`            |
| `builtin:reading`   | `reading.y`, `reading.thickness`, `reading.active`                     |
| `builtin:liquid`    | `liquid.ctx`, `liquid.cap`, `liquid.swell`, `liquid.t`, `liquid.ink`   |
| `builtin:sprout`    | `sprout.x`, `sprout.y`, `sprout.r` (the bud's tip; none without bud)   |

The body parts (`builtin:blob`, `builtin:baymax-face`) read the inner layers above (pupil,
magnifier, reading, liquid, sprout) and draw them inside the body.

## Order

Parts run in the order of the recipe's `[[piece]]` list. A part reads what the parts before it
wrote in this frame, and what the parts after it wrote in the previous one. While drawing,
each part first lays out (in list order, on a copy of the bus for that grid) and then they draw
in `[draw] order`.
