# iOS — dev + builds
set script-interpreter := ["bash", "-euo", "pipefail"]
set working-directory := ".."
TAURI_ROOT := module_directory() / "../"
IOS_ROOT := env("TAURI_IOS_PROJECT_PATH", TAURI_ROOT / "gen/apple")
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
    export TAURI_IOS_PROJECT_PATH={{ IOS_ROOT }}
    export WAYPOINTS_FLAVOR={{ flavor }}
    tauri ios dev \
      --features {{ flavor }} \
      --config {{ TAURI_CONFIGS_ROOT }}/platforms/ios.toml \
      --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
      {{ ARGS }}

[private]
[script("nu")]
_build flavor *ARGS:
    # Inside a devenv both SDKROOT and DEVELOPER_DIR point to a Nix apple-sdk,
    # which cc-rs rejects for iOS targets. `env -i` clears them, allowing `xcode-select`
    # to fallback onto the system-wide setting.
    let probe = ^env -i PATH=/usr/bin:/bin xcode-select -p | complete
    let developer_dir = if $probe.exit_code == 0 { $probe.stdout | str trim } else { "" }
    if ($developer_dir | is-empty) {
      error make { msg: "No Xcode found. Install it, then: `sudo xcode-select --switch /Applications/Xcode.app`" }
    }
    if ($developer_dir | str starts-with "/nix/store/") {
      error make { msg: $"xcode-select resolves to the Nix store \(($developer_dir)); iOS needs a real Xcode." }
    }
    if not ($developer_dir | path exists) {
      error make { msg: $"No Xcode at ($developer_dir)" }
    }

    do { cd "{{ IOS_ROOT }}"; ^xcodegen generate }

    rm -rpf "{{ IOS_ROOT }}/build"

    # Built as a record and serialized rather than written as a JSON literal:
    # `to json --raw` is what keeps the quoting out of the string.
    let build_number = $env.WAYPOINTS_BUILD_NUMBER? | default ""
    let version_config = if ($build_number | is-empty) { [] } else {
      ["--config" ({bundle: {iOS: {bundleVersion: $build_number}}} | to json --raw)]
    }

    # Since the *environment* is what contaminates shipped binaries, we use `env -i` to
    # create a clean-room environment. To retain tool-access (for building/etc.) we
    # append the `$PATH` after pushing system paths to higher priorities — with Xcode's
    # toolchain ahead of them, so `swift-build` and friends resolve to Apple's rather
    # than the Nix wrapper, which does not provide them.
    let argv = [
      "-i"
      $"HOME=($env.HOME)"
      $"USER=($env.USER)"
      $"PATH=/usr/bin:/bin:/usr/sbin:/sbin:($developer_dir)/Toolchains/XcodeDefault.xctoolchain/usr/bin:($env.PATH | str join ':')"
      $"DEVELOPER_DIR=($developer_dir)"
      $"SOPS_AGE_KEY=($env.SOPS_AGE_KEY? | default '')"
      "CC_aarch64_apple_ios=/usr/bin/clang"
      "CXX_aarch64_apple_ios=/usr/bin/clang++"
      "CARGO_TARGET_AARCH64_APPLE_IOS_LINKER=/usr/bin/clang"
      "TAURI_IOS_PROJECT_PATH={{ IOS_ROOT }}"
      "WAYPOINTS_FLAVOR={{ flavor }}"
      "WAYPOINTS_BUILD=1"
      "secretspec" "run" "--profile" "release" "--scope" "ios" "--"
      "tauri" "ios" "build"
      "--features" "{{ flavor }}"
      "--config" "{{ TAURI_CONFIGS_ROOT }}/platforms/ios.toml"
      "--config" "{{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml"
      ...$version_config
      {{ ARGS }}
    ]
    ^env ...$argv
