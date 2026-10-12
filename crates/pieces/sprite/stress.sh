#!/bin/sh
# Every state, event and subagents, at several window sizes: "STRESS ok", or what broke.
#   ./stress.sh pikachu
set -eu
cd "$(dirname "$0")"
dir=$(./build-char.sh "$1" | tail -1)
cd ../../.. && PREVIEW_DIR="$(realpath crates/pieces/sprite/$dir)" cargo test -q --release stress -- --ignored --nocapture 2>&1 | grep "STRESS"
