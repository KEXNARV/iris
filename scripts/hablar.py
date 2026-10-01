"""La voz de JARVIS: Kokoro (em_alex) en un proceso aparte que JARVIS deja abierto.

Recibe por stdin una orden JSON por línea:
    {"op": "say", "text": "…"}   decir una frase (se encola)
    {"op": "stop"}               callarse ya y olvidar lo que faltaba
Responde por stdout, una línea por evento:
    ready                        el modelo cargó
    start                        empezó a sonar
    level 0.123                  volumen (RMS) de lo que suena, ~30 por segundo
    idle                         terminó todo lo que tenía que decir

Mientras suena una frase se genera la siguiente, así no hay silencios entre frases.
JARVIS escribe este archivo en ~/.local/share/jarvis/hablar.py al arrancar.
"""

import json
import os
import queue
import subprocess
import sys
import threading
import time

import numpy as np
from kokoro_onnx import Kokoro

BASE = os.path.expanduser("~/.local/share/jarvis/kokoro")
VOICE = os.environ.get("JARVIS_VOZ", "em_alex")
SPEED = float(os.environ.get("JARVIS_VOZ_VELOCIDAD", "1.0"))

out_lock = threading.Lock()


def out(msg):
    with out_lock:
        sys.stdout.write(msg + "\n")
        sys.stdout.flush()


kokoro = Kokoro(os.path.join(BASE, "kokoro-v1.0.onnx"), os.path.join(BASE, "voices-v1.0.bin"))
texts = queue.Queue()
audios = queue.Queue()
gen = 0  # sube con cada «stop»: lo encolado antes queda descartado
playing = None
state = threading.Lock()
# Frases encoladas que todavía no terminaron de sonar (o se descartaron): «idle» solo con 0,
# no en el hueco entre una frase y la siguiente mientras se genera.
pending = 0


def done_one():
    global pending
    with state:
        pending = max(0, pending - 1)


def synth():
    while True:
        g, text = texts.get()
        if g != gen:
            continue
        try:
            audio, sr = kokoro.create(text, voice=VOICE, speed=SPEED, lang="es")
        except Exception as e:  # una frase rara no tiene que tumbar la voz
            print(f"hablar: {e}", file=sys.stderr, flush=True)
            done_one()
            continue
        if g == gen:
            audios.put((g, np.clip(audio, -1, 1).astype(np.float32), sr))


def play():
    global playing
    started = False
    while True:
        try:
            g, audio, sr = audios.get(timeout=0.15)
        except queue.Empty:
            if started and pending == 0:
                started = False
                out("idle")
            continue
        if g != gen:
            continue
        if not started:
            started = True
            out("start")
        pcm = (audio * 32767).astype(np.int16).tobytes()
        p = subprocess.Popen(
            ["paplay", "--raw", "--format=s16le", f"--rate={sr}", "--channels=1"],
            stdin=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        with state:
            playing = p
        threading.Thread(target=lambda: (p.stdin.write(pcm), p.stdin.close()), daemon=True).start()
        t0 = time.time()
        while p.poll() is None:
            if g != gen:
                p.kill()
                break
            i = int((time.time() - t0) * sr)
            win = audio[max(0, i - 800) : i + 800]
            rms = float(np.sqrt(np.mean(win * win))) if len(win) else 0.0
            out(f"level {rms:.3f}")
            time.sleep(0.033)
        out("level 0")
        with state:
            playing = None
        done_one()


threading.Thread(target=synth, daemon=True).start()
threading.Thread(target=play, daemon=True).start()
out("ready")

for line in sys.stdin:
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    if msg.get("op") == "say" and msg.get("text", "").strip():
        with state:
            pending += 1
        texts.put((gen, msg["text"]))
    elif msg.get("op") == "stop":
        gen += 1
        with state:
            pending = 0
        with state:
            if playing is not None:
                playing.kill()
        for q in (texts, audios):
            while not q.empty():
                try:
                    q.get_nowait()
                except queue.Empty:
                    break
        out("level 0")
        out("idle")
