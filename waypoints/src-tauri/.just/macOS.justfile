# macOS — desktop dev + Apple-signed builds

set working-directory := ".."

dev *ARGS: (_dev "development" ARGS)

dev-preview *ARGS: (_dev "preview" ARGS)

dev-stable *ARGS: (_dev "stable" ARGS)

build *ARGS: (_build "stable" ARGS)

build-preview *ARGS: (_build "preview" ARGS)

[private]
_dev feature $RUST_LOG="debug" $RUST_BACKTRACE="1" *ARGS:
    cargo tauri dev \
      --features {{ feature }} \
      --config Tauri.flavor={{ feature }}.toml \
      {{ ARGS }}

[private]
_build feature *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    repo_root=$(git rev-parse --show-toplevel)
    sops_file="${repo_root}/.secrets/signing-apple.sops.yaml"
    key_path=$(mktemp "${TMPDIR:-/tmp}/AuthKey.XXXXXX.p8")
    trap 'rm -f "${key_path}"' EXIT
    secrets=$(sops -d "${sops_file}")
    echo "${secrets}" | yq '.developer-id.key-prv' > "${key_path}"
    env -i \
        HOME="$HOME" \
        USER="$USER" \
        PATH="/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.nix-profile/bin:$HOME/.cargo/bin:$HOME/.bun/bin" \
        APPLE_API_ISSUER="$(echo "${secrets}" | yq '.developer-id.issuer-id')" \
        APPLE_API_KEY="$(echo "${secrets}" | yq '.developer-id.key-id')" \
        APPLE_API_KEY_PATH="${key_path}" \
        APPLE_CERTIFICATE="$(echo "${secrets}" | yq '.developer-id.certificate')" \
        APPLE_CERTIFICATE_PASSWORD="$(echo "${secrets}" | yq '.developer-id.password')" \
        cargo tauri build \
          --features {{ feature }} \
          --config Tauri.flavor={{ feature }}.toml \
          {{ ARGS }}
