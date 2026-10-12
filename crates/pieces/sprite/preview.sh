#!/bin/sh
# Builds a character and draws its preview sheet: out/<id>.png, eight moments (waiting,
# thinking, speaking, reading, done, error, running, asleep). With PREVIEW_SET=2, the other
# signs (searching, editing, testing, compacting, no voice, delegating, planning, git) in
# out/<id>-2.png. Prints the sheet's path, and any part that broke (it must not).
#   ./preview.sh pikachu
set -eu
cd "$(dirname "$0")"
dir=$(./build-char.sh "$1" | tail -1)
id=$1
tmp=$(mktemp -d)
(cd ../../.. && PREVIEW_DIR="$(realpath crates/pieces/sprite/$dir)" PREVIEW_OUT="$tmp/s" cargo test -q --release preview -- --ignored --nocapture 2>&1 | grep "ROTA" || true)
sheet="out/$id.png"
[ "${PREVIEW_SET:-1}" = "2" ] && sheet="out/$id-2.png"
montage -label '%t' "$tmp"/s-*.ppm -tile 4x2 -geometry 300x300+3+3 -background '#222' -fill white "$sheet"
rm -rf "$tmp"
realpath "$sheet"
