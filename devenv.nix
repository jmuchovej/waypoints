{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: let
  tauri-app-path = "src-tauri";
  tauri-frontend-path = "src";
  tauri-android-path = "${tauri-app-path}/gen/android";
  tauri-ios-path = "${tauri-app-path}/gen/apple";

  is-enabled = prog: prog.enable or false;
  has-package = prog: prog ? package;
  mise-formatters = let
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
    is-available = name: !(builtins.elem name unavailable);
  in
    lib.mapAttrs' (name: prog: lib.nameValuePair (specs.${name} or name) prog.package) (
      lib.filterAttrs (
        name: prog: (is-enabled prog) && (has-package prog) && (is-available name)
      )
      config.treefmt.config.programs
    );

  mise-tools =
    {
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
    }
    // mise-formatters;

  treefmt-formatters = let
    wrappers = {
      just = "fmt-just";
      statix = "fmt-statix";
      xmllint = "fmt-xml";
    };

    bare = name: fmt:
      if wrappers ? ${name}
      then
        fmt
        // {
          command = wrappers.${name};
          options = [];
        }
      else fmt // {command = baseNameOf fmt.command;};
  in
    lib.mapAttrs bare config.treefmt.config.settings.formatter;

  treefmt-excludes = [
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

  # Map a version (e.g. "35.0.0") to its android-nixpkgs attribute suffix ("35-0-0").
  dash-version = lib.replaceStrings ["."] ["-"];

  android = {
    agp.version = "8.13.2";
    build-tools.version = "35.0.0";
    platforms.versions = [
      "36"
      "30"
    ];
    ndk.version = "30.0.16138531";
  };

  # ps16k (16 KB page-size) system images only exist for API >= 35.
  system-images = sdk-pkgs: let
    base-variants = [
      "google-apis"
      "google-apis-playstore"
    ];
    variants = v: base-variants ++ lib.optionals (lib.versionAtLeast v "35") (map (s: "${s}-ps16k") base-variants);
  in
    lib.concatMap (
      v: map (s: sdk-pkgs."system-images-android-${dash-version v}-${s}-arm64-v8a") (variants v)
    )
    android.platforms.versions;

  android-sdk = inputs.android-nixpkgs.sdk.${pkgs.stdenv.hostPlatform.system} (
    sdk-pkgs: (
      [
        sdk-pkgs.cmdline-tools-22-0
        sdk-pkgs.platform-tools
        sdk-pkgs.emulator
        sdk-pkgs."build-tools-${dash-version android.build-tools.version}"
        sdk-pkgs."ndk-${dash-version android.ndk.version}"
      ]
      ++ map (v: sdk-pkgs."platforms-android-${dash-version v}") android.platforms.versions
      ++ (system-images sdk-pkgs)
    )
  );
  android-home = "${android-sdk}/share/android-sdk";
  ndk-version = android.ndk.version;
  build-tools-version = android.build-tools.version;
in {
  stdenv = pkgs.clangStdenv;

  env = {
    PROJECT_ROOT = config.env.DEVENV_ROOT;
    PROJECT_CONFIG_PATH = "${config.env.PROJECT_ROOT}/.config";
    TAURI_FRONTEND_PATH = "${config.env.PROJECT_ROOT}/${tauri-frontend-path}";
    TAURI_APP_PATH = "${config.env.PROJECT_ROOT}/${tauri-app-path}";
    TAURI_ANDROID_PROJECT_PATH = "${config.env.PROJECT_ROOT}/${tauri-android-path}";
    TAURI_IOS_PROJECT_PATH = "${config.env.PROJECT_ROOT}/${tauri-ios-path}";
    TREEFMT_CONFIG = "${config.env.PROJECT_CONFIG_PATH}/treefmt.toml";
    CLIPPY_CONF_DIR = "${config.env.PROJECT_CONFIG_PATH}";

    ANDROID_HOME = android-home;
    ANDROID_NDK_ROOT = "${config.env.ANDROID_HOME}/ndk/${ndk-version}";
    ANDROID_USER_HOME = "${config.env.DEVENV_STATE}/android";
    ANDROID_AVD_HOME = "${config.env.ANDROID_USER_HOME}/avd";
    LD_LIBRARY_PATH = lib.concatStringsSep ":" [
      "$LD_LIBRARY_PATH"
      (pkgs.lib.makeLibraryPath [
        pkgs.vulkan-loader
        pkgs.libGL
      ])
      "${config.env.ANDROID_HOME}/build-tools/${build-tools-version}/lib64/"
      "${config.env.ANDROID_NDK_ROOT}/toolchains/llvm/prebuilt/linux-x86_64/lib/"
      "$LD_LIBRARY_PATH"
    ];
    PATH = lib.concatStringsSep ":" [
      "${config.env.DEVENV_ROOT}/.bin"
      "$PATH"
      "${config.env.ANDROID_HOME}/tools"
      "${config.env.ANDROID_HOME}/tools/bin"
      "${config.env.ANDROID_HOME}/platform-tools"
      "${config.env.ANDROID_HOME}/cmdline-tools/bin"
      "${config.env.ANDROID_HOME}/emulator"
    ];
  };

  enterShell = "";

  packages =
    (lib.attrValues mise-tools)
    ++ [android-sdk]
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
      excludes = treefmt-excludes;
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

  tasks."mise:lock" = {
    description = "Regenerate mise.lock and verify every declared tool resolved";
    package = pkgs.nushell;
    after = ["devenv:enterShell"];
    execIfModified = [
      ".config/mise.toml"
      ".config/mise.lock"
    ];
    exec = builtins.readFile "${config.devenv.root}/.bin/sync-mise";
  };

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

  files = {
    ".config/mise.lock" = {
      copyMode = "seed";
      text = "# This should be overwritten on re-entry.";
    };

    ".config/treefmt.toml" = {
      copyMode = "copy";
      toml =
        lib.filterAttrs (name: _: name != "global") config.treefmt.config.settings
        // {
          formatter = treefmt-formatters;
        };
    };

    ".config/mise.toml" = {
      copyMode = "copy";
      toml = {
        settings = {
          experimental = true;
          idiomatic_version_file_enable_tools = [
            "rust"
            "pnpm"
          ];
          lockfile = true;
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
        tools = lib.mapAttrs (_: pkg: toString pkg.version) mise-tools;
      };
    };
    "${tauri-android-path}/local.properties".text = ''
      # This file was automatically generated by nix-shell.
      sdk.dir=${config.env.ANDROID_HOME}
      ndk.dir=${config.env.ANDROID_NDK_ROOT}
      android.aapt2FromMavenOverride=${config.env.ANDROID_HOME}/build-tools/${build-tools-version}/aapt2
    '';
    "${tauri-android-path}/gradle/libs.versions.toml" = {
      copyMode = "copy";
      toml = {
        versions = {
          kotlin = mise-tools."github:JetBrains/kotlin".version;
          buildTools = android.build-tools.version;
          agp = android.agp.version;
        };
      };
    };
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
