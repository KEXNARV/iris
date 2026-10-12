#!/bin/sh
# Builds a character and draws its preview sheet: out/<id>.png, eight moments (waiting,
# thinking, speaking, reading, done, error, running, asleep). Prints the sheet's path, and any
# part that broke (it must not).
#   ./preview.sh pikachu
set -eu
cd "$(dirname "$0")"
dir=$(./build-char.sh "$1" | tail -1)
id=$1
tmp=$(mktemp -d)
(cd ../../.. && PREVIEW_DIR="$(realpath crates/pieces/sprite/$dir)" PREVIEW_OUT="$tmp/s" cargo test -q --release preview -- --ignored --nocapture 2>&1 | grep "ROTA" || true)
montage -label '%t' "$tmp"/s-*.ppm -tile 4x2 -geometry 300x300+3+3 -background '#222' -fill white "out/$id.png"
rm -rf "$tmp"
realpath "out/$id.png"
