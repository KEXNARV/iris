# JARVIS voice (optional)

JARVIS answers out loud with [Kokoro](https://github.com/thewh1teagle/kokoro-onnx), voice
`em_alex`. Without this installed everything works the same, it just doesn't speak (on the first
attempt it shows «sin voz» in the chat).

## Install

```sh
python3 -m venv ~/.local/share/jarvis/py
~/.local/share/jarvis/py/bin/pip install kokoro-onnx soundfile

mkdir -p ~/.local/share/jarvis/kokoro && cd ~/.local/share/jarvis/kokoro
for f in kokoro-v1.0.onnx voices-v1.0.bin; do
  curl -LO "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/$f"
done
```

It needs `paplay` (PipeWire or PulseAudio). The `scripts/speak.py` script ships inside JARVIS
and copies itself to `~/.local/share/jarvis/hablar.py`.

## Usage

- It answers out loud when you talk to it (hold space); if you type, silently.
- `/voice` changes the mode: `auto`, `always` or `never`. Also `JARVIS_HABLA=always|never`.
- Esc or space shuts it up.
- `JARVIS_VOZ` picks another Kokoro voice (`em_santa`, `ef_dora`) and `JARVIS_VOZ_VELOCIDAD` the
  speed (1.0 normal).

## Logitech keyboard (optional)

If `ghubd` is running (ghub-linux, the keyboard's own controller), JARVIS sends it its state over
`$XDG_RUNTIME_DIR/ghub-linux.sock` and the keyboard follows along (listening, thinking, error,
ready…). Without `ghubd`, the messages are just dropped.

## Core as an image

In foot the core is drawn with Sixel, with finer dots than braille. `JARVIS_SIXEL=0` goes back
to braille; `JARVIS_DOT` changes the dot spacing in pixels (4 by default).

## Floating JARVIS (Hyprland)

`jarvis --flotante` is a window for quick commands: when it appears it listens on its own, runs,
answers out loud in one sentence and hides. With the Omarchy config in Lua:

```lua
-- hyprland.lua
o.window({ class = "jarvis-flotante" }, {
  float = true, center = true, size = { 1100, 650 }, workspace = "special:jarvis silent",
})
-- bindings.lua
o.bind("SUPER + ALT + C", "JARVIS flotante", hl.dsp.workspace.toggle_special("jarvis"))
-- autostart.lua (foot directly: omarchy-launch-tui doesn't pass it --app-id)
o.exec_on_start("cd ~ && setsid foot --app-id=jarvis-flotante jarvis --flotante --dangerously-skip-permissions")
```

## Keys

- Ctrl+T: the whole conversation (in Cine). Ctrl+O: open or close the long answer.
- Ctrl+G: what each tool did, with its full input and output.
- ← → Ctrl+← → Home End Ctrl+A/E/W/K/U to edit; Shift+Enter, new line; ↑ ↓, what you sent.
- Ctrl+V pastes an image; in foot it shows as a thumbnail in the chat.
