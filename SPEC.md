# High-Performance 2D Canvas Specification

## Project Overview

A cross-platform 2D graph/canvas application (like GeoGebra or natto.dev) where objects can be manipulated, moved, and connected. Targets web and native desktop with a single codebase.

## Core Requirements

### Functional
- **Interactive Canvas**: Pan, zoom, select, and drag objects
- **Graph Objects**: Nodes (circles, rectangles), edges (lines, bezier curves)
- **Transformations**: Matrix-based pan/zoom with sub-pixel precision
- **Spatial Operations**: Collision detection, hit testing, spatial hashing
- **Physics Simulation**: Force-directed graph layout (optional feature)
- **Persistence**: Save/load graph state (JSON format)

### Non-Functional
- **Performance**: Maintain 60 FPS with 10,000+ nodes
- **Cross-Platform**: Single codebase runs on web and native desktop
- **Memory**: Low footprint (target <100MB for 10k nodes)
- **Startup Time**: <2 seconds to interactive

## Architecture Options

### Option A: Rust + egui + wgpu (Native-First)
- **Stack**: Rust, egui (UI), wgpu (rendering)
- **Build Targets**: Native binaries, WebAssembly + WebGPU/WebGL
- **Pros**: Maximum performance, zero Electron bloat, single codebase
- **Cons**: Steeper learning curve, custom UI components

### Option B: TypeScript + PixiJS + Tauri + Rust Wasm (Web-First)
- **Stack**: TypeScript/React, PixiJS (WebGL), Tauri (desktop wrapper), Rust Wasm (math core)
- **Build Targets**: Web, Desktop (via Tauri)
- **Pros**: Modern web UI, familiar tooling, Tauri is lightweight (5MB vs 150MB Electron)
- **Cons**: JS/Wasm bridge overhead, must optimize memory transfer

### Option C: C++ + Dear ImGui + Emscripten (Industry Standard)
- **Stack**: C++, Dear ImGui, imgui-node-editor
- **Build Targets**: Native desktop, WebAssembly + WebGL
- **Pros**: Battle-tested, massive ecosystem, game-engine quality
- **Cons**: Painful tooling (CMake), programmer-centric UI

## Implementation Status

### Option A: Rust + egui ✅ COMPLETE
- Location: `/workspaces/canvas-rust-egui/`
- Binary: `target/release/canvas-rust-egui` (16MB)
- Features: Pan, zoom, drag nodes, bezier edges, force-directed layout
- Metrics: Frame time, FPS, node/edge count displayed in UI

### Option B: TypeScript + PixiJS + Tauri ✅ COMPLETE
- Location: `/workspaces/canvas-pixi/`
- Web build: `dist/` (626KB JS bundle)
- Features: Pan, zoom, drag nodes, bezier edges, force-directed layout
- Metrics: Frame time, FPS, node/edge count displayed in UI
- Tauri: Configured but not built (requires OS-specific tooling)

## Benchmark Framework

### Test Scenarios
1. **Static Render**: 10,000 nodes, 15,000 bezier edges
2. **Dynamic Pan/Zoom**: 600 frames of programmatic camera movement
3. **Physics Test**: 5,000 nodes with force-directed simulation (10s)
4. **Interaction**: Drag highly-connected node, measure edge recalc

### Metrics (Both Implementations)
- **Frame Time**: Target <16.6ms (60 FPS)
- **Average Frame Time**: Rolling average of last 60 frames
- **Min/Max Frame Time**: Performance variance
- **FPS**: Calculated from frame time
- **Node Count**: Current number of nodes
- **Edge Count**: Current number of edges

### Running Benchmarks

**Option A (Rust):**
```bash
cd /workspaces/canvas-rust-egui
cargo run --release
# Click buttons to generate 1K/10K/50K nodes
# Click "Force-Directed" to run physics simulation
# Metrics displayed in left panel
```

**Option B (Web):**
```bash
cd /workspaces/canvas-pixi
npm run dev
# Open http://localhost:1420
# Click buttons to generate 1K/10K/50K nodes
# Click "Force-Directed" to run physics simulation
# Metrics displayed in left panel
```

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Rendering | PixiJS | Fastest 2D WebGL renderer |
| Desktop | Tauri | 5MB vs 150MB Electron |
| Wasm | Rust | Safe, fast, good tooling |
| UI Framework | React + Vite | Familiar, modern |
| State Management | Zustand | Lightweight, fast |

## File Structure

```
/workspaces/
├── SPEC.md
├── canvas-rust-egui/           # Option A: Rust + egui
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs            # Canvas app with metrics
│   └── target/release/
│       └── canvas-rust-egui   # 16MB binary
└── canvas-pixi/               # Option B: TypeScript + PixiJS + Tauri
    ├── package.json
    ├── src/
    │   ├── App.tsx            # Main app with controls
    │   ├── Canvas.tsx        # PixiJS canvas
    │   ├── store.ts          # Zustand state
    │   └── index.css
    ├── src-tauri/            # Tauri config
    └── dist/                 # Built web assets
```
