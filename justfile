# Waypoints' Development Tasks
# Run `just` to see all available commands

# Default task - show all available commands
[private]
default:
    @just --list

mod app "waypoints/justfile"
mod tauri "waypoints/src-tauri/justfile"
mod cartograph "crates/cartograph/justfile"
mod domain "crates/domain/justfile"
mod store "crates/store/justfile"
mod server "crates/server/justfile"
mod website "website/justfile"

# Generate TypeScript types from Rust models
typegen:
    @just domain::typegen
    @just cartograph::typegen

typecheck:
    just app::typecheck
    just website::typecheck

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
