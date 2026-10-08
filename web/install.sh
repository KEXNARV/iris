#!/bin/sh
# Instalador de Iris para Arch, Ubuntu y sus derivados:
#   curl -fsSL https://iris.knarvaez.com/install.sh | sh
# Opciones (después de `sh -s --`): --sin-voz (no baja el modelo de voz, ~550 MB) y --desinstalar.
# Lo mismo por variables: IRIS_SIN_VOZ=1, IRIS_DESTINO=/otra/carpeta.
set -eu

REPO="KEXNARV/iris"
BASE="https://github.com/$REPO/releases/latest/download"
PAQUETE="iris-linux-x86_64.tar.gz"
DESTINO="${IRIS_DESTINO:-$HOME/.local/bin}"
DATOS="${XDG_DATA_HOME:-$HOME/.local/share}/iris"
MODELO="ggml-large-v3-turbo-q5_0.bin"
MODELO_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$MODELO"
VAD="ggml-silero-v5.1.2.bin"
VAD_URL="https://huggingface.co/ggml-org/whisper-vad/resolve/main/$VAD"

dice() { printf '\033[36m›\033[0m %s\n' "$*"; }
ojo() { printf '\033[33m!\033[0m %s\n' "$*"; }
falla() { printf '\033[31m✗\033[0m %s\n' "$*" >&2; exit 1; }

SIN_VOZ="${IRIS_SIN_VOZ:-}"
for a in "$@"; do
  case "$a" in
    --sin-voz) SIN_VOZ=1 ;;
    --desinstalar)
      rm -f "$DESTINO/iris"
      dice "Iris quitado de $DESTINO."
      dice "Tus ajustes y modelos siguen en ~/.config/iris y $DATOS; bórralos a mano si no los quieres."
      exit 0 ;;
    *) falla "no conozco la opción $a (hay --sin-voz y --desinstalar)" ;;
  esac
done

[ "$(uname -s)" = Linux ] || falla "este instalador es para Linux. En Windows: irm https://iris.knarvaez.com/install.ps1 | iex"
[ "$(uname -m)" = x86_64 ] || falla "por ahora Iris solo está compilado para x86_64 (este equipo es $(uname -m))"
command -v curl >/dev/null || falla "hace falta curl"

# sudo solo si no somos root; con curl | sh lee la contraseña de la terminal.
SUDO=""
[ "$(id -u)" -eq 0 ] || SUDO="sudo"

dice "Revisando lo que Iris necesita del sistema…"
if command -v pacman >/dev/null; then
  # pacman -T dice qué falta sin tocar nada.
  FALTAN=$(pacman -T vulkan-icd-loader alsa-lib wl-clipboard ffmpeg || true)
  if [ -n "$FALTAN" ]; then
    dice "Instalando: $FALTAN"
    $SUDO pacman -S --needed --noconfirm $FALTAN
  fi
elif command -v apt-get >/dev/null; then
  # Ubuntu 24.04 renombró libasound2 a libasound2t64.
  ASOUND=libasound2t64
  apt-cache show "$ASOUND" >/dev/null 2>&1 || ASOUND=libasound2
  FALTAN=""
  for p in libvulkan1 mesa-vulkan-drivers "$ASOUND" wl-clipboard ffmpeg; do
    dpkg -s "$p" >/dev/null 2>&1 || FALTAN="$FALTAN $p"
  done
  if [ -n "$FALTAN" ]; then
    dice "Instalando:$FALTAN"
    $SUDO apt-get update -qq
    $SUDO apt-get install -y -qq $FALTAN
  fi
else
  ojo "No reconozco el gestor de paquetes: asegúrate de tener libvulkan, alsa-lib y wl-clipboard."
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
dice "Bajando la última versión de Iris…"
curl -fsSL --retry 3 -o "$TMP/$PAQUETE" "$BASE/$PAQUETE" || falla "no pude bajar $BASE/$PAQUETE"
curl -fsSL --retry 3 -o "$TMP/$PAQUETE.sha256" "$BASE/$PAQUETE.sha256" || falla "no pude bajar la suma de verificación"
(cd "$TMP" && sha256sum -c "$PAQUETE.sha256" >/dev/null) || falla "el archivo bajado no coincide con su sha256; no instalo nada"
tar -xzf "$TMP/$PAQUETE" -C "$TMP"
mkdir -p "$DESTINO"
install -m 755 "$TMP/iris/iris" "$DESTINO/iris"
dice "Iris quedó en $DESTINO/iris"

if [ -z "$SIN_VOZ" ]; then
  mkdir -p "$DATOS/models"
  if [ ! -s "$DATOS/models/$MODELO" ]; then
    dice "Bajando el modelo de voz (Whisper, ~550 MB). Se salta con --sin-voz."
    curl -fL --retry 3 --progress-bar -o "$DATOS/models/$MODELO.part" "$MODELO_URL" \
      && mv "$DATOS/models/$MODELO.part" "$DATOS/models/$MODELO" \
      || ojo "no pude bajar el modelo de voz; Iris funciona igual, sin dictado"
  fi
  if [ ! -s "$DATOS/models/$VAD" ]; then
    curl -fsSL --retry 3 -o "$DATOS/models/$VAD" "$VAD_URL" || ojo "no pude bajar el detector de voz (opcional)"
  fi
fi

if ! command -v claude >/dev/null; then
  dice "Iris va encima de Claude Code, que no está: lo instalo con su instalador oficial."
  curl -fsSL https://claude.ai/install.sh | bash || ojo "no pude instalar Claude Code; míralo en https://docs.claude.com/claude-code"
fi

case ":$PATH:" in
  *":$DESTINO:"*) ;;
  *) ojo "$DESTINO no está en tu PATH. Añade a tu shell: export PATH=\"$DESTINO:\$PATH\"" ;;
esac

echo
dice "Listo. Abre una terminal y escribe: iris"
dice "Si nunca usaste Claude Code, corre primero: claude   (para iniciar sesión)"
