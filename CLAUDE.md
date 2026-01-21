# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build (native)
cargo build

# Build release
cargo build --release

# Build for WASM
cargo build --release --target wasm32-unknown-unknown

# Run an example
cargo run --example simple

# Check without building
cargo check

# Run clippy
cargo clippy

# Format code
cargo fmt
```

## Architecture

Vidi is a data visualization library built on Bevy (game engine). It provides a declarative API for creating interactive 2D/3D plots and dashboards.

### Core Modules

- **`src/core.rs`** - Data model definitions: `Plot`, `Graph2D`, `Graph3D`, `Layer2D/3D`, `Style`, `Color`, `Scale`, `Dashboard`. These are serializable structs representing visualization data.

- **`src/dash.rs`** - Builder API: `dash()` entry point returns `DashBuilder`. Chain methods like `.add_2d()`, `.add_3d()`, `.add_distribution()` to compose plots. Call `.show()` to render.

- **`src/runtime.rs`** - Bevy application bootstrap: `run_dashboard()` creates the Bevy `App` with plugins and runs the render loop. Has separate WASM implementation.

- **`src/render/`** - Bevy ECS rendering implementation:
  - `mod.rs` - `DashRenderPlugin` registers systems
  - `components.rs` - ECS components: `PlotId`, `PlotTile`, `TileView`, `TileRect`, `TileCamera`
  - `resources.rs` - ECS resources: `DashboardRes`, `TileRegistry`, `UnitMeshes`
  - `systems.rs` - Core systems: tile sync, layout, camera management, input handling, dirty-tile redraw
  - `draw.rs` - Actual geometry drawing functions

### Data Flow

1. User builds a `Dashboard` via `DashBuilder` API
2. `run_dashboard()` inserts dashboard as `DashboardRes` resource
3. `sync_plots_to_tiles` system creates `PlotTile` entities for each plot
4. `update_tile_layout` computes grid layout and viewport rects
5. `sync_tile_cameras` creates per-tile cameras with viewports
6. `draw_dirty_tiles` renders changed plots using Bevy primitives

### Key Patterns

- Dirty-tile rendering: Only redraws tiles marked in `TileRegistry.dirty` queue
- Per-tile cameras with viewports for grid layout
- Builder pattern for API ergonomics
- Dual target support: native + wasm32 (with `#[cfg(target_arch = "wasm32")]`)

### Bevy Version

Uses Bevy 0.17.3 with Rust edition 2024.
