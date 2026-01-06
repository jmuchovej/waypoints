{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
{
  overlays = [ (import inputs.rust-overlay) ];
  # https://devenv.sh/packages/
  packages = with pkgs; [
    git
    fd
    ripgrep
    cargo-expand
    # Custom wrapper for `just` that auto-selects devenv profile
    (pkgs.writeShellScriptBin "just" ''
      # Get the current directory relative to git root
      CURRENT_DIR="$(pwd)"
      REL_PATH="''${CURRENT_DIR#${config.git.root}/}"

      # Check if first argument is a module path (contains ::)
      FIRST_ARG="''${1:-}"
      if [[ "$FIRST_ARG" =~ :: ]]; then
        # Extract module prefix (e.g., "app" from "app::frb-codegen")
        MODULE="''${FIRST_ARG%%::*}"
      else
        MODULE=""
      fi

      SPECIALIZED_PROFILE=""
      # Determine which devenv profile to use
      # Priority: 1) module prefix, 2) current directory
      if [[ "$MODULE" =~ ^(app|playground)$ ]] || [[ -z "$MODULE" && "$REL_PATH" =~ ^(app|playground) ]]; then
        SPECIALIZED_PROFILE="--profile app"
      fi

      cd "${config.git.root}"
      exec devenv $SPECIALIZED_PROFILE shell bash <<EOF
        cd '$CURRENT_DIR'
        ${pkgs.just}/bin/just $*
      EOF
    '')
  ];

  # Use stable Rust for development
  languages.rust = {
    enable = true;
    channel = "stable"; # Your code compiles against stable
  };

  languages.deno.enable = true;

  enterShell = '''';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = '''';

  # https://devenv.sh/git-hooks/
  git-hooks.hooks.trim-trailing-whitespace.enable = true;
  git-hooks.hooks.end-of-file-fixer.enable = true;
  git-hooks.hooks.mixed-line-endings.enable = true;
  git-hooks.hooks.check-executables-have-shebangs.enable = true;
  git-hooks.hooks.check-shebang-scripts-are-executable.enable = true;
  git-hooks.hooks.check-yaml.enable = true;
  git-hooks.hooks.check-json.enable = true;
  git-hooks.hooks.check-symlinks.enable = true;
  git-hooks.hooks.check-added-large-files.enable = true;
  git-hooks.hooks.clippy.enable = true;
  git-hooks.hooks.cargo-check.enable = true;
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

  # See full reference at https://devenv.sh/reference/options/

  # https://devenv.sh/profiles/
  profiles.app.module =
    { config, ... }:
    {
      stdenv = lib.mkDefault pkgs.stdenvNoCC;
      # env.LD_LIBRARY_PATH = lib.makeLibraryPath [ ];
      env.LD_LIBRARY_PATH = "\$LD_LIBRARY_PATH";
      env.CHROME_EXECUTABLE = "${pkgs.google-chrome}/bin/google-chrome-stable";

      # https://devenv.sh/packages/
      packages =
        with pkgs;
        [
          flutter_rust_bridge_codegen
          google-chrome
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          cocoapods
          xcodegen
          xcodes
          xcbuild
          darwin.cctools
          # https://developer.apple.com/services-account/download?path=/Developer_Tools/Xcode_26.1/Xcode_26.1_Universal.xip
          darwin.xcode_26_1
        ];

      env.VERBOSE_SCRIPT_LOGGING = "1";

      # https://devenv.sh/languages/
      languages.rust.enable = true;
      languages.kotlin.enable = true;
      languages.swift.enable = true;
      android = {
        enable = true;
        ndk.enable = true;
        platforms.version = [
          "32"
          "34"
          "36"
        ];
        buildTools.version = [ "28.0.3" ];
        # TODO(jmuchovej) re-enable once https://github.com/flutter/flutter/pull/175846 is merged
        flutter.enable = true;
      };
    };

  env.TREEFMT_ALLOW_MISSING_FORMATTER = true;
  env.TREEFMT_VERBOSE = 1; # 0 = warn, 1 = info, 2 = debug
  treefmt = {
    enable = true;
    # Exclude generated/vendored code
    config.settings.global.excludes = [
      "app/rust_builder/cargokit/**"
      "**/macos/Runner/*"
      "**/macos/Flutter/*"
      "**/windows/runner/*"
      "**/windows/flutter/*"
      "**/frb_generated*"
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
    ];
    config.settings.global.on-unmatched = "info";
    config.programs = {
      statix.enable = true;
      clang-format.enable = true;
      rustfmt.enable = true;
      ktfmt.enable = true;
      dart-format.enable = true;
      swift-format.enable = true;
      rufo = {
        enable = true;
        includes = [
          "*.rb"
          "**/Podfile"
          "*.podspec"
        ];
      };
      just.enable = true;
      # biome.enable = true;
      deno.enable = true;
      yamlfmt = {
        enable = true;
        includes = [ "*.metadata" ];
      };
      jsonfmt = {
        enable = true;
        includes = [ "*.arb" ];
      };
      taplo.enable = true;
      cmake-format = {
        enable = true;
        includes = [
          "**/*.cmake"
          "**/CMakeLists.txt"
        ];
      };
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
