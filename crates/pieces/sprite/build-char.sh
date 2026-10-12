#!/bin/sh
# Builds a character: src/chars/<id>.rs → out/<id>/ (recipe.toml, sprite.wasm, stage.wasm) and
# out/<id>.iris. Its NAME, SCENE and DESCRIPTION come from the consts in its file. The scene's
# piece is built once per scene (out/_scenes/<scene>.wasm). CARGO_TARGET_DIR is respected (to
# build several at once without waiting on each other).
#   ./build-char.sh pikachu
set -eu
cd "$(dirname "$0")"
ID=$1
f=src/chars/$ID.rs
[ -f "$f" ] || { echo "no existe $f" >&2; exit 1; }
meta() { python3 -c "import re,sys;m=re.search(r'pub const $1: &str = \"((?:[^\"\\\\\\\\]|\\\\\\\\.)*)\";', open(sys.argv[1]).read());print(m.group(1) if m else '')" "$f"; }
NAME=$(meta NAME); SCENE=$(meta SCENE); DESCRIPTION=$(meta DESCRIPTION)
[ -n "$NAME" ] && [ -n "$SCENE" ] || { echo "$f: faltan NAME o SCENE" >&2; exit 1; }
target=${CARGO_TARGET_DIR:-target}
out=out/$ID
mkdir -p "$out" out/_scenes
IRIS_CHARACTER="$ID" cargo build -q --release --target wasm32-unknown-unknown
cp "$target/wasm32-unknown-unknown/release/sprite.wasm" "$out/sprite.wasm"
if [ ! -f "out/_scenes/$SCENE.wasm" ]; then
  (cd ../stage && IRIS_SCENE="$SCENE" CARGO_TARGET_DIR=target/"$SCENE" cargo build -q --release --target wasm32-unknown-unknown && cp target/"$SCENE"/wasm32-unknown-unknown/release/stage.wasm ../sprite/out/_scenes/"$SCENE".wasm.tmp && mv ../sprite/out/_scenes/"$SCENE".wasm.tmp ../sprite/out/_scenes/"$SCENE".wasm)
fi
cp "out/_scenes/$SCENE.wasm" "$out/stage.wasm"
python3 - "$out/recipe.toml" "$ID" "$NAME" "$SCENE" "$DESCRIPTION" <<'PY'
import sys
p, id_, name, scene, desc = sys.argv[1:]
t = open('recipe.template.toml').read()
t = t.replace('@ID@', id_).replace('@NAME@', name).replace('@SCENE@', scene).replace('@DESCRIPTION@', desc.replace('"', '\\"'))
open(p, 'w').write(t)
PY
(cd "$out" && rm -f "../$ID.iris" && zip -q "../$ID.iris" recipe.toml sprite.wasm stage.wasm)
echo "$out"
