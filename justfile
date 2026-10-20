# Waypoints' Development Tasks
# Run `just` to see all available commands

set script-interpreter := ["bash", "-euo", "pipefail"]

# Default task - show all available commands
[private]
default:
    @just --list

# App (front-end) scoped recipes
mod app "src/justfile"

# App (back-end) scoped recipes
mod tauri "src-tauri/justfile"

# Gradle-side maintenance for the Android project
mod android "src-tauri/gen/android/justfile"

# Recipes for `cartograph`
mod cartograph "crates/cartograph/justfile"

# Recipes for `domain`
mod domain "crates/domain/justfile"

# Recipes for `store`
mod store "crates/store/justfile"

# Recipes for `xtask` dev tooling
mod xtask "tooling/xtask/justfile"

# Recipes for the `website`
mod website "website/justfile"

# Icon generation for every app flavor
mod icons "tooling/icongen/justfile"

# Station deployment: manifest validation and a kind cluster
mod deploy "deploy/justfile"

# Generate TypeScript types from Rust models
typegen:
    cargo xtask typegen

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

[doc("Refreshes every mise lockfile.")]
[script]
mise-lock:
    # Deliberately not run on shell entry. mise reports success after a
    # rate-limited run that wrote only part of the lockfile, so this belongs at
    # a moment someone is reading the output.
    export SECRETSPEC_REASON="just mise-lock refreshes the mise lockfiles, which needs an authenticated GitHub client"
    devenv shell -- secretspec run --reason "mise resolves lockfile entries through the GitHub API, which is rate-limited to sixty requests an hour unauthenticated" -- mise-lock

[doc("Refreshes the hashes nix cannot derive from the tree, after a lockfile change.")]
nix-pins pin="":
    nix-pins {{ pin }}

[doc("Reports what those hashes would become, writing nothing.")]
nix-pins-check pin="":
    nix-pins {{ pin }} --dry-run

[doc("Checks every mise lockfile against its config. Offline; no token needed.")]
[script]
mise-lock-check:
    export SECRETSPEC_REASON="just mise-lock-check compares the mise lockfiles against their configs; no secret values are read"
    devenv shell -- mise-lock --check

[doc("Verifies that devenv-managed files, the TypeScript bindings, and xtask-generated workflows don't differ from committed files.")]
[script]
check-generated:
    # devenv's secretspec integration gates evaluation behind a stated reason.
    # This recipe reads generated file lists and regenerates bindings and
    # workflows; it never reads a secret value.
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

    # The TypeScript bindings are regenerated in place the same way, and
    # diffed against the same baseline.
    bindings="crates/domain/typescript/types.g.ts crates/domain/typescript/schema.g.ts crates/cartograph/typescript/types.g.ts"
    devenv shell cargo xtask typegen
    files="$files $bindings"

    rc=0
    # Colocated checkouts have a .git where HEAD is jj's @-; added jj
    # workspaces have no .git, but `jj diff -r @` asks the same question
    # against the same baseline (the working-copy commit's parent).
    if git rev-parse --git-dir >/dev/null 2>&1; then
      git --no-pager diff HEAD --exit-code -- $files || rc=1
    else
      stale="$(jj diff -r @ --name-only -- $files)"
      if [ -n "$stale" ]; then
        jj diff -r @ --git -- $files
        rc=1
      fi
    fi
    devenv shell cargo xtask workflows --check || rc=1

    if [ "$rc" -ne 0 ]; then
      echo "Generated files are stale. Any devenv output or binding diffed above has" >&2
      echo "already been regenerated; for workflows, run \`cargo xtask workflows\`." >&2
      echo "Review and commit the result." >&2
      exit 1
    fi
    echo "$(echo $files) and .github/workflows are in sync with their sources."

[doc("Dry-runs Renovate against the working tree. MODE is extract, lookup, or full.")]
[script]
renovate-plan mode="full":
    export SECRETSPEC_REASON="just renovate-plan dry-runs Renovate over the repo's dependency manifests; no secret values are read"

    case "{{ mode }}" in
      # extract: parse manifests only, offline, seconds.
      # lookup:  extract, then query datasources for newer versions.
      # full:    lookup, then resolve grouping into the branches Renovate would open.
      extract | lookup | full) ;;
      *)
        echo "MODE must be extract, lookup, or full (got '{{ mode }}')." >&2
        exit 1
        ;;
    esac

    devenv shell renovate-config-validator .github/renovate.json5

    if [ -n "${GH_TOKEN:-}" ]; then
      export GITHUB_COM_TOKEN="$GH_TOKEN"
    elif [ "{{ mode }}" != "extract" ]; then
      echo "note: GH_TOKEN unset — expect datasource rate-limiting." >&2
    fi

    log="${TMPDIR:-/tmp}/renovate-{{ mode }}.log"
    devenv shell renovate --platform=local --dry-run={{ mode }} > "$log" 2>&1

    sed -n '/"managers": {/,/"total"/p' "$log"
    echo "Full output: $log"

[doc("Runs the cargo-deny supply-chain checks, verifying its target list first.")]
deny: deny-targets
    cargo-deny check

[doc("Fails if .cargo/deny.toml graph.targets drifted from rust-toolchain.toml's targets.")]
[script("nu")]
deny-targets:
    let toolchain = (open rust-toolchain.toml | get toolchain.targets | sort)
    let deny = (open .cargo/deny.toml | get graph.targets | sort)
    let missing = ($toolchain | where {|t| $t not-in $deny})
    let extra = ($deny | where {|t| $t not-in $toolchain})
    if ($missing | is-empty) and ($extra | is-empty) {
        print $"deny.toml graph.targets in sync with rust-toolchain.toml \(($deny | length) targets)"
    } else {
        if ($missing | is-not-empty) {
            print $"missing from deny.toml graph.targets: ($missing | str join ', ')"
        }
        if ($extra | is-not-empty) {
            print $"in deny.toml but not rust-toolchain.toml: ($extra | str join ', ')"
        }
        error make {msg: "deny.toml graph.targets drifted from rust-toolchain.toml"}
    }

[doc("Overlays tailcallhq/gh-workflow at REV (a full SHA or a ref, not an abbreviation) onto tooling/gh-workflow. The local delta in its README is then re-applied by hand.")]
[script]
gh-workflow-sync rev:
    scratch="$(mktemp -d)"
    trap 'rm -rf "$scratch"' EXIT

    git -C "$scratch" init --quiet
    git -C "$scratch" fetch --quiet --depth 1 https://github.com/tailcallhq/gh-workflow {{ rev }}
    git -C "$scratch" checkout --quiet FETCH_HEAD

    # README.md is ours; upstream's crate has none, and its CHANGELOG is not carried.
    rsync -a --delete --exclude README.md --exclude CHANGELOG.md \
      "$scratch/crates/gh-workflow/" tooling/gh-workflow/

    echo "tooling/gh-workflow now holds upstream $(git -C "$scratch" rev-parse --short HEAD)."
    echo "Re-apply the local delta from tooling/gh-workflow/README.md, update its revision, then format the crate (upstream's rustfmt style differs from the workspace's)."
