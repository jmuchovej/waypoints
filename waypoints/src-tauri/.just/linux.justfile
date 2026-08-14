# Linux — desktop dev + unsigned builds
set script-interpreter := ["bash", "-euo", "pipefail"]
set working-directory := ".."
TAURI_ROOT := module_directory() / "../"
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
    tauri dev \
      --features {{ flavor }} \
      --config {{ TAURI_CONFIGS_ROOT }}/platforms/linux.toml \
      --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
      {{ ARGS }}

[private]
[script]
_build flavor *ARGS:
    export WAYPOINTS_FLAVOR={{ flavor }}
    export WAYPOINTS_BUILD=1
    secretspec run --profile release --scope linux -- \
      tauri build \
        --features {{ flavor }} \
        --config {{ TAURI_CONFIGS_ROOT }}/platforms/linux.toml \
        --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
        {{ ARGS }}
