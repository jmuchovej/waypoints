# macOS — desktop dev + Apple-signed builds
set script-interpreter := ["bash", "-euo", "pipefail"]
set working-directory := ".."
TAURI_ROOT := env("TAURI_APP_PATH")
TAURI_CONFIGS_ROOT := TAURI_ROOT / "configs"

dev *ARGS: (_dev "development" ARGS)

dev-preview *ARGS: (_dev "preview" ARGS)

dev-stable *ARGS: (_dev "stable" ARGS)

build *ARGS: (_build "stable" ARGS)

build-preview *ARGS: (_build "preview" ARGS)

build-nightly *ARGS: (_build "nightly" ARGS)

[private]
[script]
_dev flavor $RUST_LOG="debug" $RUST_BACKTRACE="1" *ARGS:
    export WAYPOINTS_FLAVOR={{ flavor }}
    cargo tauri dev \
        --features {{ flavor }} \
        --config {{ TAURI_CONFIGS_ROOT }}/platforms/macos.toml \
        --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
        {{ ARGS }}

[private]
[script]
_build flavor *ARGS:
    # `env -i` because the *environment* is what contaminates a shipped binary:
    # Nix's SDKROOT/NIX_LDFLAGS make it link against store paths instead of
    # /usr/lib. PATH is not the problem, so the ambient one is kept whole —
    # every tool the build shells out to (yq, deno, pnpm, …) stays reachable
    # without maintaining a list. Prepending the system directories is what
    # makes /usr/bin/clang win over Nix's wrapper.
    env -i \
      HOME="$HOME" \
      USER="$USER" \
      PATH="/usr/bin:/bin:/usr/sbin:/sbin:$PATH" \
      SECRETSPEC_PROVIDER="${SECRETSPEC_PROVIDER:-}" \
      OP_SERVICE_ACCOUNT_TOKEN="${OP_SERVICE_ACCOUNT_TOKEN:-}" \
      SOPS_AGE_KEY_FILE="${SOPS_AGE_KEY_FILE:-}" \
      SOPS_AGE_KEY="${SOPS_AGE_KEY:-}" \
      GITHUB_SHA="${GITHUB_SHA:-}" \
      GITHUB_REF_NAME="${GITHUB_REF_NAME:-}" \
      WAYPOINTS_FLAVOR={{ flavor }} \
      WAYPOINTS_BUILD=1 \
      secretspec run --profile release --scope macos -- \
        cargo tauri build \
          --features {{ flavor }} \
          --config {{ TAURI_CONFIGS_ROOT }}/platforms/macos.toml \
          --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
          {{ ARGS }}
