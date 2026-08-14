# Waypoints' Development Tasks
# Run `just` to see all available commands

set script-interpreter := ["bash", "-euo", "pipefail"]

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

[doc("Verifies that generated `files.\"...\"` and `tasks.\"mise:lock\"` outputs don't differ from committed files.")]
[script]
check-generated:
    before="$(mktemp -d)"
    trap 'rm -rf "$before"' EXIT
    cp treefmt.toml mise.toml mise.lock "$before/"
    # Regenerates configs
    devenv shell true >/dev/null
    rc=0
    for f in treefmt.toml mise.toml mise.lock; do
      diff -u "$before/$f" "$f" || rc=1
    done
    if [ "$rc" -ne 0 ]; then
      echo "Generated files were stale and have been regenerated above." >&2
      echo "Review and commit the result." >&2
      exit 1
    fi
    echo "treefmt.toml, mise.toml and mise.lock are in sync with devenv."
