{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  stdenv = pkgs.clangStdenv;
  env.LD_LIBRARY_PATH = "\$LD_LIBRARY_PATH";
  env.PROJECT_ROOT = config.env.DEVENV_ROOT;

  # Cross-compilation: point cc-rs at platform-specific toolchains
  # instead of Nix's host-only clang wrapper.
  enterShell = ''
    # Android NDK
    NDK_VERSION=$(ls "$ANDROID_HOME/ndk" | head -1)
    export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
    NDK_TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin"
    export CC_aarch64_linux_android="$NDK_TOOLCHAIN/aarch64-linux-android28-clang"
    export CXX_aarch64_linux_android="$NDK_TOOLCHAIN/aarch64-linux-android28-clang++"
    export AR_aarch64_linux_android="$NDK_TOOLCHAIN/llvm-ar"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/aarch64-linux-android28-clang"

    # iOS/macOS — cc-rs checks SDKROOT before calling the compiler. Nix sets
    # it to its macOS SDK, which cc-rs rejects for iOS and then falls back to
    # xcrun (which also fails under Nix). Fix: unset SDKROOT (Nix's clang has
    # its sysroot baked in) and set DEVELOPER_DIR so xcrun can resolve any SDK.
    if [ -d "/Applications/Xcode.app/Contents/Developer" ]; then
      unset SDKROOT
      export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
      # Nix's clang bakes in -mmacos-version-min which conflicts with iOS
      # targets. Use /usr/bin/clang (Xcode's) for iOS cross-compilation.
      export CC_aarch64_apple_ios="/usr/bin/clang"
      export CXX_aarch64_apple_ios="/usr/bin/clang++"
      export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="/usr/bin/clang"
      export CC_aarch64_apple_ios_sim="/usr/bin/clang"
      export CXX_aarch64_apple_ios_sim="/usr/bin/clang++"
      export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="/usr/bin/clang"
      # Nix's Swift wrapper lacks swift-build (SPM). Prepend Xcode's
      # toolchain so swift-rs can compile Swift packages.
      export PATH="$DEVELOPER_DIR/Toolchains/XcodeDefault.xctoolchain/usr/bin:$PATH"
    fi
  '';

  packages = with pkgs; [
    git
    jujutsu
    fd
    ripgrep
    cargo-expand
    cargo-tauri
    cargo-binstall
    imagemagick
    fastlane
    just
    sops
    age
  ];

  languages.rust = {
    enable = true;
    lsp.enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };
  languages.deno.enable = true;
  languages.javascript = {
    enable = true;
    lsp.enable = true;
    bun.enable = true;
    bun.install.enable = true;
  };
  languages.typescript = {
    enable = true;
    lsp.enable = true;
  };
  languages.kotlin.enable = true;
  languages.swift.enable = true;
  android = {
    enable = true;
    ndk.enable = true;
    platforms.version = [
      "36"
      "34"
    ];
    buildTools.version = [
      "35.0.0"
      "34.0.0"
    ];
  };

  # https://devenv.sh/git-hooks/
  git-hooks.hooks.trim-trailing-whitespace.enable = true;
  git-hooks.hooks.end-of-file-fixer.enable = true;
  git-hooks.hooks.mixed-line-endings.enable = true;
  git-hooks.hooks.check-yaml.enable = true;
  git-hooks.hooks.check-symlinks.enable = true;
  git-hooks.hooks.check-added-large-files.enable = true;
  git-hooks.hooks.clippy.enable = true;
  git-hooks.hooks.cargo-check.enable = true;
  git-hooks.hooks.keep-sorted.enable = true;
  git-hooks.hooks.cargo-test = {
    enable = true;
    package = pkgs.cargo;
    id = "cargo-test";
    name = "cargo-test";
    after = [ "cargo-check" ];
    language = "system";
    entry = "${config.git-hooks.hooks.cargo-test.package}/bin/cargo test";
    files = "\\.rs$";
    pass_filenames = false;
  };
  git-hooks.hooks.treefmt = {
    enable = true;
    settings.fail-on-change = false;
  };

  env.TREEFMT_ALLOW_MISSING_FORMATTER = true;
  env.TREEFMT_VERBOSE = 1; # 0 = warn, 1 = info, 2 = debug
  treefmt = {
    enable = true;
    # Exclude generated/vendored code
    config.settings.global.excludes = [
      "**/node_modules*"
      "**/.devenv*"
      "**/.direnv*"
      "**/.envrc*"
      "**/.gitignore"
      "*.png"
      "*.gif"
      "*.ico"
      "*.lock"
      "*.pbxproj"
      "*.properties"
      "*.gradle"
      "*.xcconfig"
      "**/content/**/*.md"
    ];
    config.settings.global.on-unmatched = "info";
    config.programs = {
      statix.enable = true;
      clang-format.enable = true;
      rustfmt.enable = true;
      ktfmt.enable = true;
      swift-format.enable = true;
      taplo.enable = true;
      rufo = {
        enable = true;
        includes = [
          "*.rb"
          "**/Podfile"
          "*.podspec"
        ];
      };
      just.enable = true;
      oxfmt.enable = true;
      xmllint = {
        enable = true;
        includes = [
          "*.xml"
          "*.svg"
          "*.xhtml"
          "*.xsl"
          "*.xslt"
          "*.dtd"
          "*.xsd"
          "*.entitlements"
          "*.plist"
          "*.manifest"
          "*.xcscheme"
          "*.xcworkspacedata"
          "*.xcsettings"
          "*.storyboard"
        ];
      };
    };
  };
}
