# Waypoints' Development Tasks
# Run `just` to see all available commands

set script-interpreter := ["bash", "-euo", "pipefail"]

# Default task - show all available commands
[private]
default:
    @just --list

# App (front-end) scoped recipes
mod app "waypoints/justfile"

# App (back-end) scoped recipes
mod tauri "waypoints/src-tauri/justfile"

# Recipes for `cartograph`
mod cartograph "crates/cartograph/justfile"

# Recipes for `domain`
mod domain "crates/domain/justfile"

# Recipes for `store`
mod store "crates/store/justfile"

# Recipes for `server`
mod server "crates/server/justfile"

# Recipes for the `website`
mod website "website/justfile"

# Icon generation for every app flavor
mod icons "tooling/icongen/justfile"

# Generate TypeScript types from Rust models
typegen:
    @just domain::typegen
    @just cartograph::typegen

# Validate TypeScript types on the app & website
typecheck:
    just app::typecheck
    just website::typecheck

# Clean-up dependency installations
clean:
    cargo clean
    @just app::clean
    @just tauri::clean
    devenv gc

# Run the full test-suite
test *ARGS:
    @just app::test {{ ARGS }}

# Setup commands for new developers
setup:
    @just app::setup
    @just tauri::setup

[doc("Verifies that devenv-managed files and xtask-generated workflows don't differ from committed files.")]
[script]
check-generated:
    # devenv's secretspec integration gates evaluation behind a stated reason.
    # This recipe reads generated file lists and regenerates workflows; it never
    # reads a secret value.
    export SECRETSPEC_REASON="just check-generated compares devenv- and xtask-generated files against the committed tree; no secret values are read"

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
