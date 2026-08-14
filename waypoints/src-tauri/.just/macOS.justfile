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
    just icons::{{ feature }}
    repo_root=$(git rev-parse --show-toplevel)
    apple_sops="${repo_root}/.secrets/apple.sops.yaml"
    updater_sops="${repo_root}/.secrets/updater.sops.yaml"
    key_path=$(mktemp "${TMPDIR:-/tmp}/AuthKey.XXXXXX.p8")
    trap 'rm -f "${key_path}"' EXIT
    js_bins="$(dirname "$(command -v node)"):$(dirname "$(command -v pnpm)")"
    apple_secrets=$(sops -d "${apple_sops}")
    updater_secrets=$(sops -d "${updater_sops}")
    echo "${apple_secrets}" | yq '.developer-id.key-prv' > "${key_path}"
    env -i \
        HOME="$HOME" \
        USER="$USER" \
        PATH="/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.nix-profile/bin:$HOME/.cargo/bin:${js_bins}" \
        APPLE_API_ISSUER="$(echo "${apple_secrets}" | yq '.developer-id.issuer-id')" \
        APPLE_API_KEY="$(echo "${apple_secrets}" | yq '.developer-id.key-id')" \
        APPLE_API_KEY_PATH="${key_path}" \
        APPLE_CERTIFICATE="$(echo "${apple_secrets}" | yq '.developer-id.certificate')" \
        APPLE_CERTIFICATE_PASSWORD="$(echo "${apple_secrets}" | yq '.developer-id.password')" \
        TAURI_SIGNING_PRIVATE_KEY="$(echo "${updater_secrets}" | yq '.private-key')" \
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(echo "${updater_secrets}" | yq '.password')" \
        cargo tauri build \
          --features {{ feature }} \
          --config Tauri.flavor={{ feature }}.toml \
          {{ ARGS }}
