# Waypoints' Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
[private]
default:
    @just --list

mod app "waypoints/justfile"
mod tauri "waypoints/src-tauri/justfile"
mod domain "crates/domain/justfile"
mod store "crates/store/justfile"
mod editor "crates/editor/justfile"

# Generate TypeScript types from Rust models
typegen:
    cargo run -p Waypoints --bin typegen
    @just domain::typegen
    @just editor::typegen
    treefmt src/app/bindings.g.ts
    bun run scripts/patch-bindings.ts
    treefmt src/app/bindings.g.ts

clean:
    cargo clean
    @just app::clean
    @just tauri::clean
    devenv gc

# Spawn [desktop] app development
dev:
    @just tauri::dev

# Build a preview release
beta:
    @just tauri::preview

# Build a stable release
release:
    @just tauri::stable

# Setup commands for new developers
setup:
    @just app::setup
    @just tauri::setup
