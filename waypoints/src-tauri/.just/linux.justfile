# Linux — desktop dev + unsigned builds

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
    cargo tauri build --features {{ feature }} --config Tauri.flavor={{ feature }}.toml {{ ARGS }}
