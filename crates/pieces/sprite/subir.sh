#!/bin/sh
# Uploads characters to the store with your Iris session (iris login): its scene if the store
# doesn't have it yet, its body and its buddy, in that order. They stay in review: approve the
# buddy in /admin and its pieces go with it.
#   ./subir.sh pikachu slime
set -eu
cd "$(dirname "$0")/store"
token=$(cat "${XDG_CONFIG_HOME:-$HOME/.config}/iris/store-token")
api=${IRIS_STORE_URL:-https://iris.knarvaez.com}
up() {
  printf '  %s: ' "$(basename "$1" .iris)"
  curl -sS -X POST "$api/api/pieces" -H "Authorization: Bearer $token" -H "Content-Type: application/zip" --data-binary @"$1"
  echo
}
for id in "$@"; do
  [ -d "$id" ] || { echo "$id: no está empaquetado (./pack-store.sh $id)"; continue; }
  echo "$id"
  scene=$(sed -n 's/^Escenario: \([a-z-]*\) .*/\1/p' "$id/1-escena.txt")
  if [ "$(curl -s -o /dev/null -w '%{http_code}' "$api/api/pieces/$scene")" != 200 ]; then
    up "$scene.iris"
  else
    echo "  $scene: ya está en la tienda"
  fi
  up "$id/2-$id-body.iris"
  up "$id/3-$id.iris"
done
