{
  pkgs,
  lib,
  config,
  ...
}: let
  tauri-app-path = "src-tauri";
  tauri-frontend-path = "src";
  tauri-android-path = "${tauri-app-path}/gen/android";
  tauri-ios-path = "${tauri-app-path}/gen/apple";
in {
  imports = [
    ./nix/devenv/treefmt.nix
    ./nix/devenv/mise.nix
    ./nix/devenv/android.nix
  ];

  stdenv = pkgs.clangStdenv;

  waypoints.mise = {
    unavailable = [
      "alejandra" # crates.io lags nixpkgs
      "clang-format" # comes from the system toolchain
      "deadnix" # crates.io lags nixpkgs
      "rustfmt" # ships within rust
      "statix" # an un-related crate
      "swift-format" # comes from the Xcode toolchain
      "xmllint" # comes from libxml2
    ];
    specs = {
      just = "aqua:casey/just";
      ktfmt = "github:facebook/ktfmt";
      rufo = "gem:rufo";
      typos = "aqua:crate-ci/typos";
      oxfmt = "npm:oxfmt";
      oxipng = "aqua:oxipng/oxipng";
      tombi = "aqua:tombi-toml/tombi";
      actionlint = "aqua:rhysd/actionlint";
      zizmor = "aqua:zizmorcore/zizmor";
    };
    tools = {
      "aqua:casey/just" = pkgs.just;
      "aqua:getsops/sops" = pkgs.sops;
      "aqua:FiloSottile/age" = pkgs.age;
      "npm:wrangler" = pkgs.wrangler;
      "aqua:mikefarah/yq" = pkgs.yq-go;
      "aqua:cachix/secretspec" = pkgs.secretspec;
      "npm:renovate" = pkgs.renovate;

      "gem:fastlane" = pkgs.fastlane;
      "aqua:tauri-apps/tauri/cargo-tauri" = pkgs.cargo-tauri;

      "aqua:nextest-rs/nextest/cargo-nextest" = pkgs.cargo-nextest;
      "aqua:numtide/treefmt" = config.treefmt.config.package;
      "aqua:nushell/nushell" = pkgs.nushell;
      "aqua:EmbarkStudios/cargo-deny" = pkgs.cargo-deny;
      "aqua:orhun/git-cliff" = pkgs.git-cliff;

      "core:node" = config.languages.javascript.package;
      "core:deno" = config.languages.deno.package;
      "core:ruby" = config.languages.ruby.package;
      "core:swift" = config.languages.swift.package;
      "github:JetBrains/kotlin" = pkgs.kotlin; # NOTE: languages.kotlin doesn't specify a `.package` option
    };
    # Tools with no Windows build. Restricting them lets mise skip them there
    # instead of failing the whole install; the Windows test job needs neither.
    os = {
      "aqua:numtide/treefmt" = ["linux" "macos"];
      "gem:fastlane" = ["linux" "macos"]; # native gem extensions need a Unix toolchain
    };
    env = [
      {PROJECT_ROOT = "{{ config_root }}";}
      {PROJECT_CONFIG_PATH = "{{ env.PROJECT_ROOT }}/.config";}
      {
        CLIPPY_CONF_DIR = "{{ env.PROJECT_CONFIG_PATH }}";
        TAURI_APP_PATH = "{{ env.PROJECT_ROOT }}/${tauri-app-path}";
        TAURI_FRONTEND_PATH = "{{ env.PROJECT_ROOT }}/${tauri-frontend-path}";
        TAURI_ANDROID_PROJECT_PATH = "{{ env.PROJECT_ROOT }}/${tauri-android-path}";
        TAURI_IOS_PROJECT_PATH = "{{ env.PROJECT_ROOT }}/${tauri-ios-path}";
        TREEFMT_CONFIG = "{{ env.PROJECT_CONFIG_PATH }}/treefmt.toml";
      }
    ];
  };

  waypoints.treefmt = {
    wrappers = {
      just = "fmt-just";
      statix = "fmt-statix";
      xmllint = "fmt-xml";
    };
    excludes = [
      "*/node_modules*"
      ".{dev,dir}env/*"
      "*/.envrc*"
      "vendor/*"
      ".scratch/*"
      "*.{gif,ico}"
      "*.{properties,gradle}"
      "*.{pbxproj,xcconfig}"
      "*{mise,treefmt}.toml"
    ];
  };

  env = {
    PROJECT_ROOT = config.devenv.root;
    PROJECT_CONFIG_PATH = "${config.env.PROJECT_ROOT}/.config";
    TAURI_FRONTEND_PATH = "${config.env.PROJECT_ROOT}/${tauri-frontend-path}";
    TAURI_APP_PATH = "${config.env.PROJECT_ROOT}/${tauri-app-path}";
    TAURI_ANDROID_PROJECT_PATH = "${config.env.PROJECT_ROOT}/${tauri-android-path}";
    TAURI_IOS_PROJECT_PATH = "${config.env.PROJECT_ROOT}/${tauri-ios-path}";
    TREEFMT_CONFIG = "${config.env.PROJECT_CONFIG_PATH}/treefmt.toml";
    CLIPPY_CONF_DIR = "${config.env.PROJECT_CONFIG_PATH}";
  };

  waypoints.android = {
    agp.version = "8.13.2";
    build-tools.version = "35.0.0";
    platforms.versions = [
      "36"
      "30"
    ];
    ndk.version = "30.0.16138531";
    project-path = tauri-android-path;
  };

  enterShell = "";

  packages =
    (lib.attrValues config.waypoints.mise.resolved-tools)
    ++ (with pkgs; [
      act
      cargo-binstall
      cargo-expand
      fd
      git
      imagemagick
      jujutsu
      mise
      ripgrep
    ]);

  languages.rust = {
    enable = true;
    lsp.enable = true;
    toolchainFile = ./rust-toolchain.toml;
  };
  languages.deno.enable = true;
  languages.ruby.enable = true;
  languages.kotlin.enable = true;
  languages.swift.enable = true;
  languages.java.enable = true;
  languages.javascript = {
    enable = true;
    lsp.enable = true;
    pnpm.enable = true;
    pnpm.package = pkgs.pnpm_11;
    pnpm.install.enable = true;
  };
  languages.typescript = {
    enable = true;
    lsp.enable = true;
  };

  treefmt.enable = true;
  treefmt.config.programs = lib.mkMerge [
    {
      actionlint.enable = true;
      clang-format.enable = true;
      just.enable = true;
      ktfmt.enable = true;
      oxfmt.enable = true;
      oxipng.enable = true;
      rufo.enable = true;
      rustfmt.enable = true;
      swift-format.enable = true;
      tombi.enable = true;
      typos.enable = true;
      xmllint.enable = true;
      statix.enable = true;
      deadnix.enable = true;
      alejandra.enable = true;
      zizmor.enable = true;
    }
    {
      statix.priority = 1;
      deadnix.priority = 2;
      alejandra.priority = 3;
    }
  ];
  treefmt.config.settings = lib.mkMerge [
    {
      on-unmatched = "info";
      allow-missing-formatter = true;
      verbose = 1;
      formatter = {
        rufo.includes = [
          "**/Podfile"
          "*.podspec"
        ];
        xmllint.includes = [
          "*.entitlements"
          "*.manifest"
          "*.plist"
          "*.storyboard"
          "*.xc{scheme,settings,workspacedata}"
        ];
        oxfmt.excludes = [
          "website/*/*.md"
        ];
      };
    }
  ];

  tasks."waypoints:sync-deny-targets" = {
    description = "Sync .cargo/deny.toml graph.targets from rust-toolchain.toml";
    package = pkgs.nushell;
    after = ["devenv:enterShell"];
    execIfModified = [
      "rust-toolchain.toml"
      ".cargo/deny.toml"
    ];
    exec = builtins.readFile "${config.devenv.root}/.bin/sync-deny-targets";
  };

  git-hooks.hooks = {
    trim-trailing-whitespace.enable = true;
    end-of-file-fixer.enable = true;
    mixed-line-endings.enable = true;
    check-yaml.enable = true;
    check-symlinks.enable = true;
    check-added-large-files.enable = true;
    clippy.enable = true;
    cargo-check.enable = true;
    keep-sorted.enable = true;
    cargo-test = {
      enable = true;
      package = pkgs.cargo;
      id = "cargo-test";
      name = "cargo-test";
      after = ["cargo-check"];
      language = "system";
      entry = "${config.git-hooks.hooks.cargo-test.package}/bin/cargo test";
      files = "\\.rs$";
      pass_filenames = false;
    };
    treefmt = {
      enable = true;
      settings.fail-on-change = false;
    };
  };
}
