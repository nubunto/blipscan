#!/usr/bin/env bash
# Build for web and assemble a servable dist/ directory.
#
# Cargo runs emcc inside target/.../deps and only promotes the .js/.wasm
# up a level — the --preload-file .data side-car is left behind in deps/.
# solarl.js fetches "solarl.data" relative to itself, so the served copy
# 404s unless all four files sit in one directory. This colocates them.
set -euo pipefail

PROFILE="${1:-release}"
case "$PROFILE" in
release) cargo build --release --target wasm32-unknown-emscripten ;;
debug) cargo build --target wasm32-unknown-emscripten ;;
*)
  echo "usage: $0 [release|debug]" >&2
  exit 1
  ;;
esac

OUT="target/wasm32-unknown-emscripten/$PROFILE"
DIST="$OUT/dist"
mkdir -p "$DIST"
cp "$OUT/solarl.js" "$OUT/solarl.wasm" "$OUT/deps/solarl.data" "$DIST/"
cat web/index.html | BINARY=solarl envsubst >"$DIST/index.html"

echo "dist ready: $DIST"
echo "serve with: (cd $DIST && python3 -m http.server 8080)"
