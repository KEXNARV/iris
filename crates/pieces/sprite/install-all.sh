#!/bin/sh
# Installs every built character as a buddy in Iris (~/.config/iris/buddies/<id>/: its recipe
# and its two pieces), so /buddy lists them. Doesn't touch the buddy you have on.
#   ./install-all.sh          (to remove one: rm -r ~/.config/iris/buddies/<id>)
set -eu
cd "$(dirname "$0")"
dest="${XDG_CONFIG_HOME:-$HOME/.config}/iris/buddies"
mkdir -p "$dest"
for d in out/*/; do
  id=$(basename "$d")
  [ "$id" = "_scenes" ] && continue
  [ -f "$d/recipe.toml" ] || continue
  mkdir -p "$dest/$id"
  cp "$d/recipe.toml" "$d/sprite.wasm" "$d/stage.wasm" "$dest/$id/"
  echo "$id"
done
