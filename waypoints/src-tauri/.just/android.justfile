# Android — dev + builds

set working-directory := ".."

dev *ARGS: (_dev "development" ARGS)

dev-preview *ARGS: (_dev "preview" ARGS)

dev-stable *ARGS: (_dev "stable" ARGS)

build *ARGS: (_build "stable" ARGS)

build-preview *ARGS: (_build "preview" ARGS)

[private]
_dev feature $RUST_LOG="debug" $RUST_BACKTRACE="1" *ARGS:
    WAYPOINTS_FLAVOR={{ feature }} cargo tauri android dev --features {{ feature }} {{ ARGS }}

[private]
_build feature *ARGS:
    WAYPOINTS_FLAVOR={{ feature }} WAYPOINTS_BUILD=1 cargo tauri android build --features {{ feature }} {{ ARGS }}
