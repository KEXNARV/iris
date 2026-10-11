# Plan: install Iris with one command, and the piece store

2026-10-08. What Kevin asked for: someone types one command in the terminal (Windows, Arch,
Ubuntu) and gets Iris, plus a store on his VPS where anyone can upload pieces and buddies (WASM,
sandboxed) that are only published once he approves them.

## What stands in the way today

| Today | Why it gets in the way | Fix |
|---|---|---|
| The binary is called `jarvis` | It would ship under the old name | Rename to `iris` (binary, `~/.config/iris` folders, commands) before publishing, migrating `~/.config/jarvis` |
| It only runs on Linux with Wayland | `wl-copy`/`wl-paste`, `notify-send`, UNIX sockets (surco, ghubd), `hyprctl` (floating), `cava` | Clipboard via `arboard`, notifications via `notify-rust`; surco, ghubd, cava and the floating window stay as Linux extras (`cfg(unix)`), which simply aren't there on Windows |
| It updates by compiling the repo | Requires Rust, git and the Vulkan SDK on every machine | Download the prebuilt binary of the latest release |
| Iris is an interface over `claude` | Without Claude Code installed and logged in it does nothing | The installer installs Claude Code with its official installer if missing; the first launch asks for `claude` to log in |
| Voice is heavy | Whisper large-v3-turbo q5 is ~550 MB; the speaking voice (kokoro) is Python | The model is downloaded the first time voice is used, with a progress bar; kokoro stays optional (`--con-voz`) |
| Pieces are Rust compiled in | Nothing can be loaded while Iris is running | WASM host (phase 3) |

## Phases

**0. Portable.** Rename to Iris, the fixes in the table, and make it build and start on
Windows (Windows Terminal understands Sixel since 1.22; otherwise, braille). Tested in a
Windows VM.

**1. Binaries.** GitHub Actions on pushing a `v*` tag: Linux x86_64 built on Ubuntu 22.04
(glibc 2.35, so it runs on Ubuntu 22.04+ and on Arch) and Windows x86_64 (MSVC, Vulkan SDK
for whisper). Uploaded to the Release with their sha256.

**2. Installer.**
- Linux: `curl -fsSL https://<domain>/install.sh | sh`. Detects Arch or Ubuntu, installs what's
  missing (ffmpeg and the optional bits), puts `iris` in `~/.local/bin`, checks the sha256.
- Windows: `irm https://<domain>/install.ps1 | iex`. Puts `iris.exe` in
  `%LOCALAPPDATA%\Iris` and adds it to PATH.
- Both: install Claude Code if it isn't there, and `iris --desinstalar` undoes everything.
- Tested in clean Arch and Ubuntu containers and in the Windows VM.

**3. WASM host in Iris.** Each piece or buddy is a WASM module that exports the same as the
`Piece` and `Buddy` traits do today (`step`, `event`/`change`/`notice`, `paint`, `palette`).
Iris passes it the context (state, voice, music, gaze) and the grid; the module has no files,
network or clock of its own, with a cap on memory and on fuel per frame (if it goes over, it
gets unloaded and the original buddy comes back). A buddy can use the pieces Iris ships
(particles, subagent kids, waves) and paint sprites bundled in its package. Engine: measure
`wasmtime` (fast, heavier binary) against `wasmi` (interpreter, small) with a real piece. Proof
that it works: the particles ported to WASM look the same as the native ones.

**4. Package and kit.** A `.iris` (zip): `iris.toml` (name, type, author, version, ABI
version, license), the `.wasm`, `sprites/` and a preview. An `iris-piece` crate with the types
and a template, and `iris pieza probar <carpeta>` to see it live while writing it.

**5. The store on the VPS.** An API (Rust with axum, SQLite and the packages on disk) behind
Caddy with HTTPS:
- upload without an account → it goes into review and can't be downloaded;
- Kevin's panel to approve or reject (with the animated preview);
- list, search and download what's approved, with its sha256;
- per-IP upload limits and a maximum size per package.
The same domain serves `install.sh`, `install.ps1` and a page to browse the store.

**Tune everything (Kevin's request, 2026-10-08).** Not just buddies: every part that can be
changed is a *space* with its own command, and the store sells pieces for any of them:
`/buddy` (who lives in the core), `/particles` (the motes), `/arcs` (the rings), `/colors`
(palettes), `/theme` (whole screen styles), `/sounds`. The package manifest says which space it
goes into. The drawing ones (buddy, particles, arcs) are WASM on the grid; palettes and sounds
are data; screen styles need their own ABI (boxes, text, the core in a rectangle) and come
later.

**6. `/tienda` in Iris.** Search, see the preview, install, update and remove. What's
installed shows up in `/buddy` and in the pieces just like the built-in ones.

## What's missing to get started

- The VPS: IP or domain, SSH user, what it has installed, and the store's domain.
- Whether characters owned by someone else are accepted (Kirby belongs to Nintendo) or only
  originals.
- A Windows VM to test on (or a Windows machine I can get into).
