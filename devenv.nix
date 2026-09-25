{
  pkgs,
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
      just = {
        package = pkgs.just;
        spec = "aqua:casey/just";
        formatter = false;
      };
      nixfmt = {
        package = pkgs.nixfmt-rs;
        spec = "aqua:mic92/nixfmt-rs";
        formatter = true;
      };
      ktfmt = {
        package = pkgs.ktfmt;
        spec = "github:facebook/ktfmt";
        formatter = true;
      };
      rufo = {
        package = pkgs.rufo;
        spec = "gem:rufo";
        formatter = true;
      };
      typos = {
        package = pkgs.typos;
        spec = "aqua:crate-ci/typos";
        formatter = true;
      };
      oxfmt = {
        package = pkgs.oxfmt;
        spec = "npm:oxfmt";
        formatter = true;
      };
      oxipng = {
        package = pkgs.oxipng;
        spec = "aqua:oxipng/oxipng";
        formatter = true;
      };
      tombi = {
        package = pkgs.tombi;
        spec = "aqua:tombi-toml/tombi";
        formatter = true;
      };
      zizmor = {
        package = pkgs.zizmor;
        spec = "aqua:zizmorcore/zizmor";
        formatter = true;
      };
      hk = {
        package = pkgs.hk;
        spec = "aqua:jdx/hk";
      };
      lychee = {
        package = pkgs.lychee;
        spec = "aqua:lycheeverse/lychee";
      };
      keep-sorted = {
        package = pkgs.keep-sorted;
        spec = "aqua:google/keep-sorted";
        formatter = true;
      };
      # nix-only formatters: in the shell, never in mise. treefmt's
      # allow-missing-formatter lets mise users skip them.
      deadnix = {
        package = pkgs.deadnix;
        include = false;
      };
      statix = {
        package = pkgs.statix;
        include = false;
      };
      nixf-diagnose = {
        package = pkgs.nixf-diagnose;
        include = false;
      };
      treefmt = {
        package = pkgs.treefmt;
        spec = "aqua:numtide/treefmt";
        os = [
          "linux"
          "macos"
        ];
        formatter = true;
      };

      sops = {
        package = pkgs.sops;
        spec = "aqua:getsops/sops";
      };
      age = {
        package = pkgs.age;
        spec = "aqua:FiloSottile/age";
      };
      wrangler = {
        package = pkgs.wrangler;
        spec = "npm:wrangler";
      };
      yq = {
        package = pkgs.yq-go;
        spec = "aqua:mikefarah/yq";
      };
      secretspec = {
        package = pkgs.secretspec;
        spec = "aqua:cachix/secretspec";
      };
      # NOTE: renovate omitted — its dependency tree trips mise's npm trust policy in CI.

      fastlane = {
        package = pkgs.fastlane;
        spec = "gem:fastlane";
        os = [
          "linux"
          "macos"
        ];
      };
      xcodegen = {
        package = pkgs.xcodegen;
        spec = "aqua:yonaskolb/XcodeGen";
        os = [ "macos" ];
      };
      cargo-tauri = {
        package = pkgs.cargo-tauri;
        spec = "aqua:tauri-apps/tauri/cargo-tauri";
      };

      cargo-nextest = {
        package = pkgs.cargo-nextest;
        spec = "aqua:nextest-rs/nextest/cargo-nextest";
      };
      nushell = {
        package = pkgs.nushell;
        spec = "aqua:nushell/nushell";
      };
      cargo-deny = {
        package = pkgs.cargo-deny;
        spec = "aqua:EmbarkStudios/cargo-deny";
      };
      git-cliff = {
        package = pkgs.git-cliff;
        spec = "aqua:orhun/git-cliff";
      };

      node = {
        package = config.languages.javascript.package;
        spec = "core:node";
      };
      deno = {
        package = config.languages.deno.package;
        spec = "core:deno";
      };
      ruby = {
        package = config.languages.ruby.package;
        spec = "core:ruby";
      };
      swift = {
        package = config.languages.swift.package;
        spec = "core:swift";
      };
      kotlin = {
        package = pkgs.kotlin; # NOTE: languages.kotlin doesn't specify a `.package` option
        spec = "github:JetBrains/kotlin";
      };
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
        JJ_HOOKS_RUNNER = "hk";
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
    TREEFMT_TREE_ROOT = config.env.PROJECT_ROOT;
    JJ_HOOKS_RUNNER = "hk";
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

  packages = with pkgs; [
    act
    cargo-binstall
    cargo-expand
    fd
    git
    imagemagick
    jujutsu
    mise
    ripgrep
  ];

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

  tasks."waypoints:sync-deny-targets" = {
    description = "Sync .cargo/deny.toml graph.targets from rust-toolchain.toml";
    package = pkgs.nushell;
    after = [ "devenv:enterShell" ];
    execIfModified = [
      "rust-toolchain.toml"
      ".cargo/deny.toml"
    ];
    exec = builtins.readFile ./nix/devenv/bin/sync-deny-targets;
  };
}
