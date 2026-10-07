# Android — dev + builds
set script-interpreter := ["bash", "-euo", "pipefail"]
set working-directory := ".."
TAURI_ROOT := env("TAURI_APP_PATH")
ANDROID_ROOT := env("TAURI_ANDROID_PROJECT_PATH")
TAURI_CONFIGS_ROOT := TAURI_ROOT / "configs"
RUST_TOOLCHAIN := env("PROJECT_ROOT") / "rust-toolchain.toml"
PLATFORM_CONFIG := TAURI_CONFIGS_ROOT / "platforms/android.toml"

dev *ARGS: (_dev "development" ARGS)

dev-preview *ARGS: (_dev "preview" ARGS)

dev-stable *ARGS: (_dev "stable" ARGS)

build *ARGS: (_build "stable" ARGS)

build-preview *ARGS: (_build "preview" ARGS)

build-nightly *ARGS: (_build "nightly" ARGS)

[private]
[script]
_dev flavor $RUST_LOG="debug" $RUST_BACKTRACE="1" *ARGS:
    export TAURI_ANDROID_PROJECT_PATH={{ ANDROID_ROOT }}
    export WAYPOINTS_FLAVOR={{ flavor }}
    tauri android dev \
      --features {{ flavor }} \
      --config {{ PLATFORM_CONFIG }} \
      --config {{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml \
      {{ ARGS }}

[private]
[script("nu")]
_build flavor *ARGS:
    # Fail loudly rather than half-configure: locally these come from the
    # android devenv profile, in CI from setup-android/setup-java.
    let android_home = $env.ANDROID_HOME? | default ""
    if ($android_home | is-empty) {
      error make { msg: "ANDROID_HOME unset — enter the android devenv profile, or run under setup-android in CI" }
    }
    let java_home = $env.JAVA_HOME? | default ""
    if ($java_home | is-empty) { error make { msg: "JAVA_HOME unset — Gradle needs a JDK" } }

    mut ndk_home = $env.NDK_HOME? | default ($env.ANDROID_NDK_HOME? | default "")
    if ($ndk_home | is-empty) {
      let installed = $"($android_home)/ndk"
      if not ($installed | path exists) {
        error make { msg: $"No Android NDK found: ($installed) does not exist. Set NDK_HOME or install one." }
      }
      $ndk_home = (ls $installed | where type == dir | get name | sort --natural | last)
    }
    if not ($ndk_home | path exists) {
      error make { msg: $"No Android NDK at ($ndk_home). Set `NDK_HOME` or install one." }
    }

    # The NDK names its cross-compilers' directory after the build host; the
    # same table lives in nix/devenv/android.nix.
    let hosts = {
      # NOTE: Android doesn't support building from arm64-linux
      "x86_64-linux": "linux-x86_64"
      # NOTE: Android ships arm64-macos-compatible tools under x86_64-darwin
      "aarch64-macos": "darwin-x86_64"
      "x86_64-macos": "darwin-x86_64"
    }
    # NOTE: `$nu.os-info.name` gives the OS; `sys host`'s gives the distribution
    let platform = $"($nu.os-info.arch)-($nu.os-info.name)"
    let host = $hosts | get -o $platform
    if $host == null {
      error make { msg: $"No Android NDK host toolchain for ($platform); supported: ($hosts | columns | str join ', ')" }
    }
    let ndk_bin = $"($ndk_home)/toolchains/llvm/prebuilt/($host)/bin"
    if not ($ndk_bin | path exists) { error make { msg: $"No NDK toolchain at ($ndk_bin)" } }

    # Read the `minSdk` from the Tauri config for Android.
    let api = open "{{ PLATFORM_CONFIG }}" | get -o bundle.android.minSdkVersion
    if $api == null {
      error make { msg: "minSdkVersion is not declared in {{ PLATFORM_CONFIG }}" }
    }

    # Align targets handed to `tauri` with targets supported by `rust-toolchain.toml`.
    let triples = open "{{ RUST_TOOLCHAIN }}" | get toolchain.targets
      | where $it =~ '-linux-android'
    if ($triples | is-empty) {
      error make { msg: "No Android targets declared in {{ RUST_TOOLCHAIN }}" }
    }

    # `--target` only needs the leading component of the platform triple.
    let target_flags = $triples
      | each { |t| ["--target", ($t | str replace --regex '-linux-android.*' '')] }
      | flatten

    # cc-rs keys off the snake-cased triple, cargo off the upper-cased one. The NDK
    # names armv7's driver `armv7a-...`. Derived so that it matches `rust-toolchain.toml`
    let compilers = $triples | each { |triple|
        let clang = $"($ndk_bin)/($triple | str replace 'armv7-' 'armv7a-')($api)-clang"
        let snake = $triple | str replace --all '-' '_'
        {
          $"CC_($snake)": $clang
          $"CXX_($snake)": $"($clang)++"
          $"AR_($snake)": $"($ndk_bin)/llvm-ar"
          $"CARGO_TARGET_($snake | str uppercase)_LINKER": $clang
        }
      }
      | reduce --fold {} { |it, acc| $acc | merge $it }
      | transpose key value
      | each { |pair| $"($pair.key)=($pair.value)" }

    let base_identifier = {identifier: (open "{{ TAURI_ROOT }}/Tauri.toml" | get identifier)}
      | to json --raw

    # Since the *environment* is what contaminates shipped binaries, we use `env -i` to
    # create a clean-room environment. To retain tool-access (for building/etc.) we
    # append the `$PATH` after pushing system paths to higher priorities.
    let argv = [
        "-i"
        $"HOME=($env.HOME)"
        $"USER=($env.USER)"
        $"PATH=/usr/bin:/bin:/usr/sbin:/sbin:($env.PATH | str join ':')"
        # Which store `secretspec run` reads, and the credential for it. Empty
        # is unset to secretspec, so the profile default applies when the
        # caller names nothing.
        $"SECRETSPEC_PROVIDER=($env.SECRETSPEC_PROVIDER? | default '')"
        $"OP_SERVICE_ACCOUNT_TOKEN=($env.OP_SERVICE_ACCOUNT_TOKEN? | default '')"
        $"SOPS_AGE_KEY_FILE=($env.SOPS_AGE_KEY_FILE? | default '')"
        $"SOPS_AGE_KEY=($env.SOPS_AGE_KEY? | default '')"
        # `build.rs` stamps these into `BuildInfo`, preferring them over
        # shelling out to git. Empty locally, which it treats as absent.
        $"GITHUB_SHA=($env.GITHUB_SHA? | default '')"
        $"GITHUB_REF_NAME=($env.GITHUB_REF_NAME? | default '')"
        $"ANDROID_HOME=($android_home)"
        $"ANDROID_NDK_HOME=($ndk_home)"
        $"NDK_HOME=($ndk_home)"
        $"JAVA_HOME=($java_home)"
        ...$compilers
        "TAURI_ANDROID_PROJECT_PATH={{ ANDROID_ROOT }}"
        "WAYPOINTS_FLAVOR={{ flavor }}"
        "WAYPOINTS_BUILD=1"
        "secretspec" "run" "--profile" "release" "--scope" "android" "--"
        "tauri" "android" "build"
        "--features" "{{ flavor }}"
        "--config" "{{ PLATFORM_CONFIG }}"
        "--config" $"{{ TAURI_CONFIGS_ROOT }}/flavors/{{ flavor }}.toml"
        # After the flavor, because `--config` merges in order and this must win.
        "--config" $base_identifier
        ...$target_flags
        {{ ARGS }}
    ]
    ^env ...$argv
