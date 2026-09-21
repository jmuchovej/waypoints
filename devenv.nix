{
  pkgs,
  lib,
  config,
  ...
}:
let
  tauri-app-path = "src-tauri";
  tauri-frontend-path = "src";
  tauri-android-path = "${tauri-app-path}/gen/android";
  tauri-ios-path = "${tauri-app-path}/gen/apple";
in
{
  imports = [
    ./nix/devenv/mise.nix
    ./nix/devenv/android.nix
  ];

  stdenv = pkgs.clangStdenv;

  waypoints.mise = {
    tools = {
      "aqua:casey/just" = pkgs.just;
      "aqua:getsops/sops" = pkgs.sops;
      "aqua:FiloSottile/age" = pkgs.age;
      "npm:wrangler" = pkgs.wrangler;
      "aqua:mikefarah/yq" = pkgs.yq-go;
      "aqua:cachix/secretspec" = pkgs.secretspec;
      # NOTE: renovate omitted — its dependency tree trips mise's npm trust policy in CI.

      # The formatters .config/treefmt.toml names that mise can supply; the
      # nix-only ones sit in `packages` below.
      "aqua:mic92/nixfmt-rs" = pkgs.nixfmt-rs;
      "github:facebook/ktfmt" = pkgs.ktfmt;
      "gem:rufo" = pkgs.rufo;
      "aqua:crate-ci/typos" = pkgs.typos;
      "npm:oxfmt" = pkgs.oxfmt;
      "aqua:oxipng/oxipng" = pkgs.oxipng;
      "aqua:tombi-toml/tombi" = pkgs.tombi;
      "aqua:zizmorcore/zizmor" = pkgs.zizmor;
      "aqua:google/keep-sorted" = pkgs.keep-sorted;

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
      "aqua:numtide/treefmt" = [
        "linux"
        "macos"
      ];
      "gem:fastlane" = [
        "linux"
        "macos"
      ]; # native gem extensions need a Unix toolchain
    };
    env = [
      { PROJECT_ROOT = "{{ config_root }}"; }
      { PROJECT_CONFIG_PATH = "{{ env.PROJECT_ROOT }}/.config"; }
      # The treefmt-* wrappers, as enterShell puts them on PATH in the shell.
      { _.path = [ "tooling/bin" ]; }
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

  enterShell = ''
    export PATH="${config.devenv.root}/tooling/bin:$PATH"
  '';

  packages =
    (lib.attrValues config.waypoints.mise.tools)
    ++ (with pkgs; [
      # nix-only formatters: no mise backend, so treefmt's
      # allow-missing-formatter skips them outside the shell.
      deadnix
      nixf-diagnose
      statix

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
  treefmt.config.settings = lib.importTOML config.env.TREEFMT_CONFIG;

  tasks."waypoints:sync-deny-targets" = {
    description = "Sync .cargo/deny.toml graph.targets from rust-toolchain.toml";
    package = pkgs.nushell;
    after = [ "devenv:enterShell" ];
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
    cargo-test = {
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
    lychee = {
      enable = true;
      settings.configPath = ".config/lychee.toml";
    };
    treefmt = {
      enable = true;
      settings.fail-on-change = false;
    };
  };
}
