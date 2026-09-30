{
  pkgs,
  lib,
  config,
  ...
}:
{
  stdenv = pkgs.clangStdenv;

  imports = [ ./nix/devenv ];

  waypoints.mise = {
    # Units of work, not actors: a job names what it does, and `MISE_ENV` takes
    # a comma-separated list, so a consumer composes the sets it needs.
    environments = {
      xtask = { };
      test = { };
      format = { };
      build-desktop = { };
      build-mobile = { };
      development = { };
    };

    tools = {
      just = {
        package = pkgs.just;
        spec = "aqua:casey/just";
        formatter = false;
      };
      nixfmt = {
        package = pkgs.nixfmt-rs;
        spec = "github:Mic92/nixfmt-rs";
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
        depends = [ "ruby" ];
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
        depends = [ "node" ];
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

      # `github:` rather than `aqua:` so the SLSA provenance sops publishes is
      # checked against the workflow that actually signed it. The aqua registry
      # declares no `signer_identity`/`signer_issuer`, so mise verified any
      # valid Sigstore signature — including one from an unrelated workflow —
      # until it began refusing that check outright in 2026.9.16.
      #
      # The identity is slsa-github-generator's, not sops': sops calls it as a
      # reusable workflow, and the certificate records the callee. Read from the
      # signing certificate rather than inferred. It therefore tracks the
      # *generator's* version, so a sops release built with a newer generator
      # fails closed here until this is updated — which is the point.
      sops = {
        package = pkgs.sops;
        spec = "github:getsops/sops";
        options = {
          slsa_signer_identity = "https://github.com/slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@refs/tags/v2.1.0";
          slsa_signer_issuer = "https://token.actions.githubusercontent.com";
        };
      };
      age = {
        package = pkgs.age;
        spec = "aqua:FiloSottile/age";
      };
      wrangler = {
        package = pkgs.wrangler;
        spec = "npm:wrangler";
        depends = [ "node" ];
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
        depends = [ "ruby" ];
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
      pnpm = {
        package = config.languages.javascript.pnpm.package;
        spec = "aqua:pnpm/pnpm";
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
    env = {
      PROJECT_ROOT = "{{ config_root }}";
      _.file = {
        path = config.dotenv.filename;
        expand = true;
      };
    };
  };

  # `.env` and both of its substituters are owned by nix/devenv/default.nix.
  env.PROJECT_ROOT = config.devenv.root;

  delta.enable = true;

  waypoints.android = {
    agp.version = "8.13.2";
    build-tools.version = "35.0.0";
    platforms.versions = [
      "36"
      "30"
    ];
    ndk.version = "30.0.16138531";
    # devenv writes `files."<project-path>/..."` relative to the root, while
    # `.env` carries the absolute path both it and mise need. Re-relativize
    # rather than restating the path in nix.
    project-path = lib.removePrefix "${config.devenv.root}/" config.env.TAURI_ANDROID_PROJECT_PATH;
  };

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
      ".config/deny.toml"
    ];
    exec = builtins.readFile ./nix/devenv/bin/sync-deny-targets;
  };
}
