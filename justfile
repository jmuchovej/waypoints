# Waypoints' Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
[private]
default:
    @just --list

mod app "src/justfile"
mod tauri "src-tauri/justfile"
mod waypoints "crates/waypoints-domain"
mod entities "crates/waypoints-entities"
mod migrations "crates/waypoints-migrations"

# Generate TypeScript types from Rust models
typegen:
    cargo run -p Waypoints --bin typegen
    cargo run -p waypoints-domain --bin typegen
    treefmt src/app/bindings.g.ts crates/waypoints-domain/types.g.ts

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
