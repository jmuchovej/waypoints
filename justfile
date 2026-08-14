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

[doc("Verifies that devenv-managed files and xtask-generated workflows don't differ from committed files.")]
[script]
check-generated:
    # Entering the devenv shell is what regenerates devenv's outputs, so the
    # baseline cannot be a pre-run snapshot of the tree — asking xtask for the
    # file list would already have rewritten it. Git holds the baseline anyway:
    # the question is whether what is *committed* matches what devenv produces.
    files="$(devenv shell cargo xtask devenv-files --copied)"
    if [ -z "$files" ]; then
      echo "devenv reported no copied files." >&2
      exit 1
    fi

    rc=0
    git --no-pager diff HEAD --exit-code -- $files || rc=1
    # xtask can answer without writing, so it does.
    devenv shell cargo xtask workflows --check || rc=1

    if [ "$rc" -ne 0 ]; then
      echo "Generated files are stale. Any devenv output diffed above has already" >&2
      echo "been regenerated; for workflows, run \`cargo xtask workflows\`." >&2
      echo "Review and commit the result." >&2
      exit 1
    fi
    echo "$(echo $files) and .github/workflows are in sync with their sources."
