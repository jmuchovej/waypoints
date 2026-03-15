# Waypoints' Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
[private]
default:
    @just --list

mod app "src/justfile"
mod tauri "src-tauri/justfile"
mod cartograph "crates/cartograph"
mod waypoints "crates/waypoints-domain"
mod entities "crates/waypoints-entities"
mod migrations "crates/waypoints-migrations"

# Generate TypeScript types from Rust models
typegen:
    cargo build -p Waypoints
    treefmt src/app/bindings.g.ts

clean:
    cargo clean
    @just app::clean
    @just tauri::clean
    devenv gc

# Spawn [desktop] app development
dev:
    @just tauri::dev

# Testing
test:
    flutter test

# Setup commands for new developers
setup:
    @just app::setup
    @just tauri::setup
