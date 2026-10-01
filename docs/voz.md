# Voz de JARVIS (opcional)

JARVIS contesta en voz alta con [Kokoro](https://github.com/thewh1teagle/kokoro-onnx), voz
`em_alex`. Sin esto instalado todo funciona igual, solo que no habla (al primer intento avisa
«sin voz» en el chat).

## Instalar

```sh
python3 -m venv ~/.local/share/jarvis/py
~/.local/share/jarvis/py/bin/pip install kokoro-onnx soundfile

mkdir -p ~/.local/share/jarvis/kokoro && cd ~/.local/share/jarvis/kokoro
for f in kokoro-v1.0.onnx voices-v1.0.bin; do
  curl -LO "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/$f"
done
```

Hace falta `paplay` (PipeWire o PulseAudio). El script `scripts/hablar.py` viaja dentro de
JARVIS y se copia solo a `~/.local/share/jarvis/hablar.py`.

## Uso

- Contesta hablando cuando le hablas (mantén espacio); si escribes, en silencio.
- `/voz` cambia el modo: `auto`, `siempre` o `nunca`. También `JARVIS_HABLA=siempre|nunca`.
- Esc o espacio lo callan.
- `JARVIS_VOZ` elige otra voz de Kokoro (`em_santa`, `ef_dora`) y `JARVIS_VOZ_VELOCIDAD` la
  velocidad (1.0 normal).

## Teclado Logitech (opcional)

Si corre `ghubd` (ghub-linux, el control propio del teclado), JARVIS le manda su estado por
`$XDG_RUNTIME_DIR/ghub-linux.sock` y el teclado lo acompaña (escuchando, pensando, error,
listo…). Sin `ghubd`, los mensajes se pierden sin más.

## Núcleo en imagen

En foot el núcleo se dibuja con Sixel, con puntos más finos que el braille. `JARVIS_SIXEL=0`
vuelve al braille; `JARVIS_DOT` cambia la separación de los puntos en píxeles (4 por defecto).
