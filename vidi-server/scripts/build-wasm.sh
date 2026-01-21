#!/bin/bash
# Build WASM artifacts for vidi-server
#
# Prerequisites:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli
#   cargo install wasm-opt  # optional, for optimization

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SERVER_DIR="$PROJECT_ROOT/vidi-server"
WASM_OUT="$SERVER_DIR/wasm"

echo "Building vidi WASM..."
cd "$PROJECT_ROOT"

# Build for WASM target
cargo build --release --target wasm32-unknown-unknown

# Create output directory
mkdir -p "$WASM_OUT"

# Generate JS bindings with wasm-bindgen
echo "Generating JS bindings..."
wasm-bindgen target/wasm32-unknown-unknown/release/vidi.wasm \
  --target web \
  --out-dir "$WASM_OUT" \
  --no-typescript

# Optimize WASM if wasm-opt is available (optional, may fail with newer WASM features)
if command -v wasm-opt &> /dev/null; then
  echo "Optimizing WASM..."
  if wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int -o "$WASM_OUT/vidi_bg_opt.wasm" "$WASM_OUT/vidi_bg.wasm" 2>/dev/null; then
    mv "$WASM_OUT/vidi_bg_opt.wasm" "$WASM_OUT/vidi_bg.wasm"
    echo "wasm-opt optimization applied"
  else
    echo "wasm-opt failed (likely version mismatch), skipping optimization"
  fi
else
  echo "wasm-opt not found, skipping optimization"
fi

echo "WASM build complete!"
echo "Output: $WASM_OUT"
ls -la "$WASM_OUT"
