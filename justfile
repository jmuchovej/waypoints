# Waypoints' Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
[private]
default:
    @just --list

mod app "src/justfile"
mod tauri "src-tauri/justfile"
mod domain "crates/domain/justfile"
mod store "crates/store/justfile"
mod editor "crates/editor/justfile"

# Generate TypeScript types from Rust models
typegen:
    cargo run -p Waypoints --bin typegen
    cargo run -p domain --bin typegen
    cargo run -p editor --bin typegen
    treefmt src/app/bindings.g.ts crates/domain/types.g.ts crates/editor/types.g.ts

clean:
    cargo clean
    @just app::clean
    @just tauri::clean
    devenv gc

# Spawn [desktop] app development
dev:
    @just tauri::dev

# Build alpha flavor
alpha:
    @just tauri::alpha

# Build beta flavor
beta:
    @just tauri::beta

# Build production release
release:
    @just tauri::release

# Testing
test:
    flutter test

# Setup commands for new developers
setup:
    @just app::setup
    @just tauri::setup
