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
mod cartograph "crates/cartograph/justfile"
mod website "website/justfile"

# Generate TypeScript types from Rust models
typegen:
    @just domain::typegen
    @just editor::typegen
    @just cartograph::typegen

clean:
    cargo clean
    @just app::clean
    @just tauri::clean
    devenv gc

test *ARGS:
    @just app::test {{ ARGS }}

# Setup commands for new developers
setup:
    @just app::setup
    @just tauri::setup
