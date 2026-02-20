#!/bin/bash
set -e

# Change to project directory
cd /workspaces/canvas-rust-egui

# Rebuild Wasm
echo "Building Rust Wasm..."
cargo +nightly-2026-02-20 build --release --target wasm32-unknown-unknown

# Generate bindings
echo "Generating wasm-bindgen bindings..."
wasm-bindgen --out-dir dist --target web target/wasm32-unknown-unknown/release/canvas-rust-egui.wasm

# Restart server
echo "Restarting server on port 8034..."
# Kill any existing process on port 8034
PID=$(lsof -t -i:8034)
if [ -n "$PID" ]; then
  kill $PID
fi

# Start new server
python3 -m http.server 8034 &
echo "Server started with PID $!"
