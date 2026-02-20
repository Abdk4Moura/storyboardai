#!/bin/bash
set -e

# Change to project directory
cd /workspaces/canvas-rust-egui

# Rebuild Wasm
echo "Building Frontend (WASM)..."
trunk build --release

# Rebuild Server
echo "Building Backend (Server)..."
cargo build --release --bin server

# Restart server
echo "Restarting server on port 8033..."
# Kill any existing process on port 8033
PID=$(lsof -t -i:8033)
if [ -n "$PID" ]; then
  kill $PID
fi

# Start new server
./target/release/server > server.log 2>&1 &
echo "Server started with PID $!"
