# Windows — desktop dev + unsigned builds

set working-directory := ".."

dev *ARGS: (_dev "development" ARGS)

dev-preview *ARGS: (_dev "preview" ARGS)

dev-stable *ARGS: (_dev "stable" ARGS)

build *ARGS: (_build "stable" ARGS)

build-preview *ARGS: (_build "preview" ARGS)

[private]
_dev feature $RUST_LOG="debug" $RUST_BACKTRACE="1" *ARGS:
    cargo tauri dev --features {{ feature }} --config Tauri.flavor={{ feature }}.toml {{ ARGS }}

[private]
_build feature *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    just icons::{{ feature }}
    repo_root=$(git rev-parse --show-toplevel)
    updater_secrets=$(sops -d "${repo_root}/.secrets/updater.sops.yaml")
    export TAURI_SIGNING_PRIVATE_KEY="$(echo "${updater_secrets}" | yq '.private-key')"
    export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(echo "${updater_secrets}" | yq '.password')"
    cargo tauri build --features {{ feature }} --config Tauri.flavor={{ feature }}.toml {{ ARGS }}
