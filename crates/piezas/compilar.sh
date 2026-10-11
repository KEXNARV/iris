#!/bin/sh
# Compila las piezas de ejemplo a WASM y deja el .wasm junto a su crate (lo usan las pruebas del
# host, en src/nucleo/huesped.rs). Necesita el target wasm32-unknown-unknown.
set -eu
cd "$(dirname "$0")"
for d in */; do
  n=${d%/}
  (cd "$n" && cargo build --release --target wasm32-unknown-unknown)
  cp "$n/target/wasm32-unknown-unknown/release/$n.wasm" "$n/$n.wasm"
  echo "$n: $(wc -c < "$n/$n.wasm") bytes"
done
