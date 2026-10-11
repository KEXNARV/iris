# Store, phase 2: everything is a piece

2026-10-11 · Kevin Narvaez · status: **proposal, waiting for approval**

Kevin's rule for the store: **as atomic and customizable as possible; a user can do literally
anything.** This plan turns that into a model, a contract and an order of work, and ends with
Kevin building a buddy, publishing it and installing it from the store exactly as any user
would.

It builds on the store plan of 2026-10-10 (the Claude doc "Iris · Plan técnico de la tienda de
piezas") and on phase 1, already done: the WASM host (`src/core/host.rs`), the `iris-piece`
crate and the motes running in WASM pixel-for-pixel like the native ones.

## 1. The model

Today a buddy is Rust code: `Original<C>` holds 24 pieces in fixed fields, decides in `paint`
which one goes first and what each one receives, and `script.rs` says how much of each piece
shows in each state. Baymax is the same thing with another body. Nothing can be changed without
recompiling Iris.

The model takes that apart into four atoms:

| Atom         | What it is                                                         | Format          |
|--------------|--------------------------------------------------------------------|-----------------|
| **Piece**    | one thing that reacts and/or draws: a body, an eye, the arcs, an antenna, a mood, a sound that plays on errors | code (WASM, or native for the built-in ones) |
| **Channel**  | a named number that changes over time (`breath`, `gaze.x`, `arcs`, `body.radius`…). Iris publishes some; pieces read them and publish their own | — |
| **Recipe**   | a buddy: which pieces, in what order, with what settings, wired to which channels, and its **script** (the value of each channel in each state) | data (TOML) |
| **Asset**    | palettes, sounds, sprites that recipes and pieces use               | data            |

**A buddy is a recipe.** The original and Baymax become two recipes made only of built-in
pieces. Anyone can take one, swap its eye for another, take its arcs out, add three antennas,
make it sleep with different zees or make the arcs spin when music plays, **without writing
code**, and publish the result. Whoever wants something new writes a piece.

**Nothing is special.** The mood, the gaze, blinking, the spring that stretches it, the kids of
the subagents: all of them are pieces publishing channels, so all of them can be replaced. The
only things that stay in Iris are the clock, the grid and the sandbox.

### Channels

Channels are how atoms talk without knowing each other. All are `f64`, named with dots, and a
piece reads them as inputs and writes them as outputs.

- **From Iris** (read-only, every frame): `t`, `dt`, `state` (0–20) and `state.since`, `voice`,
  `music`, `context` (how full the context is), `calm`, `light` (light mode), `kids` (how many),
  `boot`.
- **From the script**: whatever the recipe defines per state (`arcs`, `ripple`, `zzz`, `lens`…,
  or new names), eased from one state to the next as `script.rs` does today.
- **From pieces**: whatever they declare as outputs. The body publishes `body.x`, `body.y`,
  `body.radius`; the gaze publishes `gaze.x`, `gaze.y`; the mood publishes `breath`, `blink`.
- **Events** don't go through channels: they arrive as calls (§2).

Order rule: a piece reads the outputs of the pieces before it in this frame, and of the ones
after it from the previous frame. There are no cycles to resolve and the result is the same on
every machine.

The names Iris and the built-in pieces publish are a **public API**, written down in
`docs/channels.md`. Renaming one breaks recipes, so they're frozen once published.

### A recipe

```toml
[recipe]
name = "kirby"
version = "1.0.0"
description = "Kirby, round and pink"

# The value of each channel in each state; the ones not listed take `default`.
[script.default]
arcs = 0.0
zzz = 0.0
ripple = 0.0
[script.thinking]
arcs = 1.0
[script.sleeping]
zzz = 1.0
[script.speaking]
ripple = 1.0

# Pieces, in drawing order (the first one is drawn underneath).
[[piece]]
use = "builtin:mood"              # breathing, blinking, the shake on errors
[[piece]]
use = "builtin:gaze"
[[piece]]
use = "builtin:arcs"
settings = { count = 3, length = 0.4 }
inputs = { show = "arcs", spin = "music" }   # wired to channels: spins with the music
[[piece]]
use = "kexnarv/kirby-body@1"      # from the store
settings = { color = "#F7A1C4", cheeks = true }
[[piece]]
use = "builtin:eye"
inputs = { x = "gaze.x", y = "gaze.y" }
[[piece]]
use = "builtin:zees"
inputs = { show = "zzz" }
[[piece]]
use = "builtin:kids"
settings = { recipe = "self" }    # each subagent is a little kirby

[palette]
use = "kexnarv/pastel@1"          # or inline colors
```

`inputs` wires a piece's inputs to channels; without it, each input reads the channel of the
same name. `settings` are the values the piece declares as adjustable (number, color, on/off,
one of a list), with their limits. Since the recipe is data, Iris can show every setting as a
control and change it live.

## 2. The contract (ABI 2)

One contract for every piece, whatever it does. ABI 1 (per-space `p[16]`) stays loadable for
the particle pieces already written; everything new is ABI 2.

**What a piece declares**, in its package's `iris.toml`, so Iris can wire it without running it:

```toml
[piece]
name = "kirby-body"
inputs = ["breath", "blink", "alarm", "voice"]
outputs = ["body.x", "body.y", "body.radius"]
events = ["error", "done", "kid.born"]      # the ones it wants; the rest don't wake it up

[settings]
color = { kind = "color", default = "#F7A1C4" }
cheeks = { kind = "bool", default = true }
size = { kind = "number", default = 1.0, min = 0.5, max = 2.0 }
```

**What the module exports:**

| Export                         | When                     | What it does                                         |
|--------------------------------|--------------------------|------------------------------------------------------|
| `iris_abi() -> u32`            | on load                  | returns 2                                            |
| `iris_io() -> u32`             | on load                  | where Iris writes inputs and settings and reads outputs, in the declared order |
| `iris_new()`                   | when the recipe is built | a new instance, with its settings already written    |
| `iris_step()`                  | every frame              | advances; may write its outputs                      |
| `iris_event(kind, a, b)`       | when something happens   | the 8 events, a state change (`a` from, `b` to), a subagent born/pulse/finish (`a` id, `b` ok) |
| `iris_paint() -> u64`          | every visible frame      | the draw ops, as in ABI 1                            |

**Text.** Besides numbers, a piece can read what's happening in words: the user's message,
what Claude answers, each tool call (its name and argument: the file being read, the command
being run), the model and the folder. They come through `iris.text(kind)` and as events
(`text.user`, `text.claude`, `tool.start`, `tool.end`). Since no piece can reach the network or
files, nothing it reads leaves the machine; even so, a piece that reads text says so in its
manifest (`permissions = ["text"]`) and `/store` shows it before installing («esta pieza lee
la conversación»), so nothing does it behind the user's back.

**What Iris offers (imports `iris.*`):** `rand()` with the buddy's seed; `text(kind)` (above); `play(sound)` to play
a sound from the package (rate-limited); `sprite(id, x, y, scale)` to stamp a picture from the
package onto the grid. Nothing else: no files, network or clock. (`text` adds `iris.text` to the imports a piece may use; without the permission, loading it fails.)

A piece that only publishes channels (a mood, a gaze that follows the music) doesn't export
`iris_paint`. A piece that only plays sounds doesn't draw or publish. That's what makes them
atomic: each one does one thing.

The built-in pieces get the same interface (native Rust behind the same declaration), so a
recipe doesn't know or care whether a piece is native or WASM.

## 3. What "literally anything" means, and where it stops

| A user can                                                         | How                              |
|--------------------------------------------------------------------|----------------------------------|
| change any setting of any piece, live                              | `/buddy edit`, no code         |
| take out, add, reorder or duplicate pieces                         | the recipe, no code              |
| change how the buddy reacts to each state                          | the script, no code              |
| wire anything to anything (arcs to the music, size to the context) | `inputs`, no code                |
| colors and sounds                                                  | assets, no code                  |
| draw something that doesn't exist yet, or a new behavior           | a piece, in Rust (any language that compiles to WASM works) |
| replace the mood, the gaze or the kids themselves                  | a piece                          |

Where it stops, on purpose:

- **The sandbox.** A piece sees everything Iris sees (§2, "Text"), but it can only answer by
  drawing on the grid and playing its own sounds: no files, no network, no clock. Whatever it
  reads can't leave the machine, and that's what makes it safe to install anything from the
  store.
- **The budget.** All the pieces of a buddy together get 2 ms per frame (fuel) and 20,000 draw
  ops; the motes in WASM take 10.6 µs per frame today, so that's room for well over a hundred
  pieces. A piece that goes over is turned off with the notice in §9.
- **The core.** Pieces draw inside the core. The rest of the screen (screen styles, `/theme`)
  needs a different contract and comes later.

## 4. Editing a buddy inside Iris

Freedom with no tools gets you lost, so the editor is part of the plan, not an extra:

- `/buddy` lists the installed buddies; `/buddy edit` opens the current one as a list of its
  pieces, with the core live on the right.
- On a piece: its settings as controls (←/→ for numbers, enter for on/off, a list for options),
  `x` takes it out, `d` duplicates it, shift+↑/↓ moves it, `+` adds an installed piece.
- `s` opens the script: a table of channels × states.
- Every change shows instantly; «save» writes the recipe in
  `~/.config/iris/buddies/<name>.toml`. Editing a built-in or store buddy saves a copy
  («kirby (mío)»): the original is never touched, so updates keep working.

## 5. Every file editable with any tool

A buddy is a folder of ordinary files, so a user can open any of them with whatever they like
(Photoshop, Aseprite, GIMP, Krita, Audacity, VS Code, Notepad) and Iris picks up the change the
moment the file is saved:

```
~/.config/iris/buddies/kirby/
├── recipe.toml        the recipe (any text editor)
├── body.png           pictures: PNG, GIF, WebP, also animated
├── body-sleeping.png  one picture per state, if they want
├── blink.gif
└── hello.ogg          sounds: OGG, WAV, MP3
```

- **A buddy made of pictures, no code.** The built-in `image` piece draws a picture onto the
  core: the transparent parts stay empty, the brightness decides each dot's tone, and the colors
  come from the picture or from the palette (a setting). It takes one picture per state
  (`frames.sleeping = "body-sleeping.png"`), an animated GIF, or a sprite sheet, and its inputs
  can move, scale and tint it (`y = "breath"`, `scale = "voice"`). Drawing Kirby in Photoshop and
  saving is enough to see him in the core.
- **Live reload.** Iris watches the folder of the buddy in use. Saving a picture, a sound or
  the recipe updates the core in the next frame; a file that's broken or half-saved keeps the
  last good version and says what's wrong («body.png no se pudo leer: ¿está a medio guardar?»).
- **`/buddy edit` and the files are the same thing.** The editor writes `recipe.toml`; editing
  `recipe.toml` by hand updates the editor. `o` in the editor opens the folder in the file
  manager.
- **Packing is zipping the folder.** `/buddy publish` packs that folder as is.

One honest limit: the core is a grid of dots (2×4 per terminal cell in braille, finer in
Sixel), so a detailed picture is shown as dots at that resolution. The template tells the
author the grid size to draw at.

## 6. The kit

| Command                       | What it does                                                    |
|-------------------------------|-----------------------------------------------------------------|
| `iris piece new antena`     | creates a Rust project with `Cargo.toml`, `iris.toml` and a working piece. Asks what it is (a body, an accessory, a mood, a sound…) and picks a template. If the `wasm32-unknown-unknown` target is missing, offers to install it |
| `iris piece try`           | builds and opens the core full screen with the piece inside a recipe (the original's by default; `--recipe` for another). Reloads on every build. ←/→ go through the 21 states, 1–8 fire the events, `k` adds a kid, `v` simulates voice, `e` opens the editor to tune its settings |
| `iris piece pack`       | validates and writes `antena-1.0.0.iris`                        |
| `iris piece publish`         | logs in if needed and uploads it                                |
| `iris piece status`           | in review, published or rejected (with the reason)              |
| `/buddy publish`             | the same for a recipe made in the editor: no project, no code   |

`iris-piece` goes to crates.io (the name is free, checked today), so a project doesn't need
the Iris repo.

## 7. Packages

A `.iris` is a zip with `iris.toml` and, depending on what it carries:

```
antena-1.0.0.iris        a piece          iris.toml (+ [piece], [settings]), piece.wasm, sounds/, sprites/
kirby-1.0.0.iris         a buddy (recipe) iris.toml (+ [recipe], [script], [[piece]])
pastel-1.0.0.iris        a palette        iris.toml (+ [palette])
campanas-1.0.0.iris      a sound pack     iris.toml (+ [sounds]), sounds/*.ogg
```

A recipe depends on pieces by name and major version (`kexnarv/kirby-body@1`). Installing a
recipe installs what it uses; a piece that's missing or failing is skipped with a notice and
the rest of the buddy keeps going.

Limits: 5 MB per package, 2 MB per `.wasm`, 256×256 sprites, 10 s per sound.

## 8. Publishing, review and signing

Same as the 2026-10-10 plan, adjusted:

- **Accounts: GitHub or an Iris account**, whichever the author prefers; downloading needs
  neither. Both end in the same Iris session token, saved in `~/.config/iris/store-token`.
  - *GitHub*, with the device flow: `iris login` shows a code, the author types it at
    `github.com/login/device`, done. Kevin creates the GitHub OAuth App.
  - *Iris account*: email and password (`iris login --email`, or on the web). Passwords hashed
    with argon2; the email is verified with a link, and a forgotten password is reset the same
    way. That needs a service to send email (Resend, Postmark or an SMTP account): Kevin picks
    one and creates its key.
  - Google comes later; the session doesn't care how you logged in.
- **Upload** (`POST /api/pieces`): the server validates before anything reaches Kevin: a sane
  zip, a complete manifest, the `.wasm` with `wasmparser` (its ABI, the declared exports, no
  imports outside `iris.*`), and runs it in the same host through the 21 states and the events
  within the limits. A recipe is checked by loading it with its pieces. Errors go back to the
  author in plain words.
- **Review on a web panel only for Kevin**, at `iris.knarvaez.com/admin`:
  - Only Kevin's account gets in, with a passkey (fingerprint or face, no password to steal).
  - The queue shows each package **running live** in the same core as the landing, with its
    manifest, the validation results, the permissions it asks for and what changed since its
    last version. Buttons to approve or reject with a reason the author will see.
  - **The signing key never touches the server.** The first time, the panel creates an ed25519
    key inside Kevin's browser (WebCrypto, marked non-exportable, kept in that browser) and
    offers to save an encrypted backup file. Approving signs the new index right there in the
    browser and uploads only the signature. Whoever gets into the VPS can take the store down,
    but can't sign anything Iris would install.
  - The price: approving works from the browsers where the key is (his PC, his phone if he adds
    it from the backup). If the key is lost with no backup, a new one ships in the next Iris
    version and installs keep working from then on.
  - Iris only installs what's in an index with a valid signature, with the right `sha256`.
- **Server:** migration `0003` adds `sessions`, `accounts` (email, password hash, verified, or
  GitHub id), `reports`, a `kind` column (`piece`, `recipe`, `palette`,
  `sounds`) instead of `space`, and `dependencies` for recipes. Packages in
  `/var/lib/iris-server/packages/<sha256>.iris`.

## 9. In Iris: `/store`

- One tab per kind: **Buddies**, **Piezas**, **Paletas**, **Sonidos**. Moving through the list,
  the core **wears it** for real: a buddy shows itself, a piece shows on the current buddy.
- Enter installs (with its dependencies) and asks «¿ponerlo ahora?». A piece goes into the
  current buddy as a copy («tu buddy (con antena)»), so it can be undone.
- `/store remove` and `/store update`, as in the 2026-10-10 plan.
- **Reporting:** every package has «reportar» in `/store` and on the web (it's someone else's
  character, it's offensive, it's broken). Reports land in Kevin's panel; taking a package
  down puts it in the index's revoked list, so installed copies turn off with a notice and the
  original comes back.
- **When a piece fails**, the notice is plain and never silent (Kevin, 2026-10-10): «la antena
  dejó de responder y la quité de tu buddy; lo demás sigue igual». The third time it stays off
  until a new version comes out, and says so.

## 10. The walk (the test that closes the phase)

On a separate Linux user, so nothing from Kevin's setup helps:

1. Install Iris and Rust.
2. `/buddy edit` on the original: take the arcs out, make the zees bigger, wire the scale to
   the music. Save as «mi buddy». **No code so far.**
3. Open its folder, draw a body in Photoshop (or any image program), save: it's in the core.
   **Still no code.**
4. `iris piece new kirby-eyes` → `iris piece try`: write eyes that react to errors, put them in
   the recipe from the editor, and publish both (`iris piece publish`, `/buddy publish`).
5. As Kevin, at `iris.knarvaez.com/admin`: see both running, approve them.
6. Back on the other user: `/store kirby` → enter → it's in the core, with the picture body,
   the store eyes and everything else built-in.

## 11. Order

The hardest part comes first, because everything else stands on it: turning the original into
a recipe of pieces without it changing a single dot.

| Phase | What                                                                     | Gate                                            |
|-------|--------------------------------------------------------------------------|-------------------------------------------------|
| 2a    | channels, the recipe format, and the 24 built-in pieces behind the new interface; the original and Baymax as recipes | the existing golden tests (`core::tests::golden`) pass unchanged: same dots in every state |
| 2b    | ABI 2 in the host and in `iris-piece`; a WASM piece inside a recipe       | the built-in eye ported to WASM draws the same as the native one |
| 2c    | `/buddy edit`, the `image` piece, the buddy folder with live reload    | steps 2 and 3 of the walk, with no code        |
| 2d    | the kit (`new`, `try`, `pack`), `iris-piece` on crates.io      | from nothing to a piece moving in under five minutes |
| 2e    | accounts (GitHub and Iris), upload, validation (iris-server)              | a broken package is rejected with a clear message |
| 2f    | the host in the browser (`core.wasm` running recipes and WASM pieces), the admin panel, signing in the browser, reports | a package changed after approval is rejected   |
| 2g    | `/store`, dependencies, failure notices                                 | **the walk, end to end**                       |

Assets (`play`, `sprite`, palettes and sound packs) come in 2b–2d as the templates need them.

## Later

Screen styles (`/theme`), the public catalog at `iris.knarvaez.com/store` (the browser host
from 2f makes it cheap), Google accounts, Windows tests of the kit.

## Decided (2026-10-11)

1. **The model:** a buddy is a recipe of pieces (data), every part of the built-in buddies is a
   replaceable piece, and pieces talk through named channels. Every file of a buddy can be
   edited with any program and Iris reloads it live.
2. **The order:** start by rebuilding the original as a recipe (2a).
3. **What pieces see:** everything, text included, declared with `permissions = ["text"]` and
   shown before installing.
4. **Command names in English:** `iris piece new/try/pack/publish/status`, `/store`,
   `/buddy edit`, `iris login`.

5. **Accounts:** GitHub or an Iris account (email and password), both in this phase.
6. **Review:** a web panel only for Kevin, with a passkey; the signing key lives in his browser
   and never on the server.
7. **Owned characters:** allowed. Whoever owns one can report it and Kevin takes it down.

## What Kevin has to do

- Create the GitHub OAuth App (phase 2e).
- Pick an email service and create its key (phase 2e).
- On the first visit to the panel, register his passkey and save the key backup (phase 2f).
