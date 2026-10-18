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
    # a comma-separated list, so a consumer composes the sets it needs. A
    # contributor working without nix names the lanes they work in — there is no
    # `development` union, because selection already composes.
    environments = [
      "xtask"
      "test"
      "devtools"
      "build-desktop"
      "build-mobile"
      "website"
      "deploy"
    ];

    tools = {
      # The base is the common working set: `envs = [ ]` puts a tool in
      # `.config/mise.toml`, which every environment loads on top of. Every
      # script that launches, compiles or maintains waypoints runs through
      # `just`, and these are what those recipes reach for — so they are not
      # something a consumer should have to name a lane to get. An environment
      # below carries only what is genuinely specific to that lane.
      just = {
        package = pkgs.just;
        spec = "aqua:casey/just";
      };
      nixfmt = {
        package = pkgs.nixfmt-rs;
        spec = "github:Mic92/nixfmt-rs";
        envs = [ "devtools" ];
      };
      ktfmt = {
        package = pkgs.ktfmt;
        spec = "github:facebook/ktfmt";
        envs = [ "devtools" ];
      };
      rufo = {
        package = pkgs.rufo;
        spec = "gem:rufo";
        depends = [ config.waypoints.mise.tools.ruby.spec ];
        envs = [ "devtools" ];
      };
      typos = {
        package = pkgs.typos;
        spec = "aqua:crate-ci/typos";
        envs = [ "devtools" ];
      };
      oxfmt = {
        package = pkgs.oxfmt;
        spec = "npm:oxfmt";
        depends = [ config.waypoints.mise.tools.node.spec ];
        envs = [ "devtools" ];
      };
      oxipng = {
        package = pkgs.oxipng;
        spec = "aqua:oxipng/oxipng";
        envs = [ "devtools" ];
      };
      tombi = {
        package = pkgs.tombi;
        spec = "aqua:tombi-toml/tombi";
        envs = [ "devtools" ];
      };
      zizmor = {
        package = pkgs.zizmor;
        spec = "aqua:zizmorcore/zizmor";
        envs = [ "devtools" ];
      };
      hk = {
        package = pkgs.hk;
        spec = "aqua:jdx/hk";
        envs = [ "devtools" ];
      };
      lychee = {
        package = pkgs.lychee;
        spec = "aqua:lycheeverse/lychee";
        envs = [ "devtools" ];
      };
      keep-sorted = {
        package = pkgs.keep-sorted;
        spec = "aqua:google/keep-sorted";
        envs = [ "devtools" ];
      };
      # nix-only: in the shell, never in mise. treefmt's
      # allow-missing-formatter lets mise users skip them.
      deadnix = {
        package = pkgs.deadnix;
        envs = null;
      };
      statix = {
        package = pkgs.statix;
        envs = null;
      };
      nixf-diagnose = {
        package = pkgs.nixf-diagnose;
        envs = null;
      };
      treefmt = {
        package = pkgs.treefmt;
        spec = "aqua:numtide/treefmt";
        os = [
          "linux"
          "macos"
        ];
        envs = [ "devtools" ];
      };

      # The deploy lane: validating the Station manifests under deploy/ and
      # standing them up on kind (`just deploy::…`).
      kubeconform = {
        package = pkgs.kubeconform;
        spec = "aqua:yannh/kubeconform";
        envs = [ "deploy" ];
      };
      kubectl = {
        package = pkgs.kubectl;
        spec = "aqua:kubernetes/kubernetes/kubectl";
        envs = [ "deploy" ];
      };
      kind = {
        package = pkgs.kind;
        spec = "aqua:kubernetes-sigs/kind";
        envs = [ "deploy" ];
      };

      # Shell-only: the local development store for secrets. No CI lane
      # decrypts a sops file — CI reads 1Password through `op` — so neither is
      # projected.
      sops = {
        package = pkgs.sops;
        envs = null;
      };
      age = {
        package = pkgs.age;
        envs = null;
      };
      # What secretspec's 1Password provider shells out to. In the base with
      # `secretspec` itself: every lane that resolves a secret needs both.
      op = {
        package = pkgs._1password-cli;
        spec = "aqua:1password/cli";
      };
      wrangler = {
        package = pkgs.wrangler;
        spec = "npm:wrangler";
        depends = [ config.waypoints.mise.tools.node.spec ];
        envs = [ "website" ];
      };
      yq = {
        package = pkgs.yq-go;
        spec = "aqua:mikefarah/yq";
      };
      secretspec = {
        package = pkgs.secretspec;
        spec = "aqua:cachix/secretspec";
      };

      fastlane = {
        package = pkgs.fastlane;
        spec = "gem:fastlane";
        depends = [ config.waypoints.mise.tools.ruby.spec ];
        os = [
          "linux"
          "macos"
        ];
        envs = [ "build-mobile" ];
      };
      xcodegen = {
        package = pkgs.xcodegen;
        spec = "aqua:yonaskolb/XcodeGen";
        os = [ "macos" ];
        envs = [ "build-mobile" ];
      };
      # The Android project ships no gradlew (the wrapper jar is gitignored),
      # so Tauri runs whichever `gradle` is on PATH. Projected so CI gets the
      # same major as the shell: the runner image's own Gradle is a release
      # ahead of what the pinned Android Gradle Plugin supports.
      gradle = {
        package = pkgs.gradle;
        spec = "aqua:gradle/gradle";
        envs = [ "build-mobile" ];
      };
      # The CI build cache: sccache fronts rustc and the C compilers, and
      # rclone moves directory snapshots to and from the same R2 bucket. The
      # build lanes, and the xtask lane whose planners compile the toolchain
      # on every run — every lane that compiles under an environment.
      sccache = {
        package = pkgs.sccache;
        spec = "aqua:mozilla/sccache";
        envs = [
          "build-desktop"
          "build-mobile"
          "xtask"
        ];
      };
      rclone = {
        package = pkgs.rclone;
        spec = "aqua:rclone/rclone";
        envs = [
          "build-desktop"
          "build-mobile"
          "xtask"
        ];
      };
      cargo-tauri = {
        package = pkgs.cargo-tauri;
        spec = "aqua:tauri-apps/tauri/cargo-tauri";
      };

      cargo-nextest = {
        package = pkgs.cargo-nextest;
        spec = "aqua:nextest-rs/nextest/cargo-nextest";
        envs = [ "test" ];
      };
      nushell = {
        package = pkgs.nushell;
        spec = "aqua:nushell/nushell";
      };
      cargo-deny = {
        package = pkgs.cargo-deny;
        spec = "aqua:EmbarkStudios/cargo-deny";
        envs = [ "devtools" ];
      };
      git-cliff = {
        package = pkgs.git-cliff;
        spec = "aqua:orhun/git-cliff";
        envs = [ "xtask" ];
      };

      node = {
        package = config.languages.javascript.package;
        spec = "core:node";
      };
      pnpm = {
        package = config.languages.javascript.pnpm.package;
        spec = "aqua:pnpm/pnpm";
        depends = [ config.waypoints.mise.tools.node.spec ];
      };
      deno = {
        package = config.languages.deno.package;
        spec = "core:deno";
        envs = [ "website" ];
      };
      # Not base: only `rufo` and `fastlane` run on it, so it rides with them.
      ruby = {
        package = config.languages.ruby.package;
        spec = "core:ruby";
        envs = [
          "devtools"
          "build-mobile"
        ];
      };
      # LOAD-BEARING, despite `envs = null`: nix/devenv/android.nix reads this
      # package's version into the generated `libs.versions.toml`, which is what
      # pins the Gradle Kotlin plugin. Gradle resolves `kotlin-compiler-embeddable`
      # from Maven at that version and runs it on the JDK — it never looks for
      # `kotlinc` on PATH — so projecting the tool into mise would install a
      # second compiler nothing calls. The entry is also what puts `kotlinc` in
      # the shell, since `languages.kotlin` exposes no `.package` option.
      kotlin = {
        package = pkgs.kotlin;
        envs = null;
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

  # `devenv shell -P emulator` for an AVD. Off by default: the emulator and its
  # system images are ~20 GB that neither a build nor CI needs, and nothing
  # generated into the tree depends on them.
  profiles.emulator.module.waypoints.android.emulator.enable = true;

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
