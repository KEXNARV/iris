#!/bin/sh
# Builds the example pieces to WASM and leaves the .wasm next to its crate (used by the host's
# tests, in src/core/host.rs). Needs the wasm32-unknown-unknown target.
set -eu
cd "$(dirname "$0")"
for d in */; do
  n=${d%/}
  (cd "$n" && cargo build --release --target wasm32-unknown-unknown)
  cp "$n/target/wasm32-unknown-unknown/release/$n.wasm" "$n/$n.wasm"
  echo "$n: $(wc -c < "$n/$n.wasm") bytes"
done
