{
  pkgs,
  lib,
  config,
  ...
}: let
  tauri-lib-path = "waypoints/src-tauri";
  tauri-app-path = "waypoints";
  tauri-android-path = "gen/android";
  tauri-ios-path = "gen/apple";

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

      "gem:fastlane" = pkgs.fastlane;
      "npm:@tauri-apps/cli" = pkgs.cargo-tauri;

      "aqua:nextest-rs/nextest/cargo-nextest" = pkgs.cargo-nextest;
      "aqua:numtide/treefmt" = config.treefmt.config.package;
      "aqua:nushell/nushell" = pkgs.nushell;

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
    "*.{gif,ico}"
    "*.{properties,gradle}"
    "*.{pbxproj,xcconfig}"
    "*{mise,treefmt}.toml"
  ];
in {
  stdenv = pkgs.clangStdenv;
  env.LD_LIBRARY_PATH = lib.mkForce "";
  env.PROJECT_ROOT = config.env.DEVENV_ROOT;
  env.PROJECT_CONFIG_PATH = "${config.env.PROJECT_ROOT}/.config";
  env.TAURI_FRONTEND_PATH = "${config.env.PROJECT_ROOT}/${tauri-app-path}";
  env.TAURI_APP_PATH = "${config.env.PROJECT_ROOT}/${tauri-lib-path}";
  env.TAURI_ANDROID_PROJECT_PATH = "${config.env.TAURI_APP_PATH}/${tauri-android-path}";
  env.TAURI_IOS_PROJECT_PATH = "${config.env.TAURI_APP_PATH}/${tauri-ios-path}";
  env.TREEFMT_CONFIG = "${config.env.PROJECT_CONFIG_PATH}/treefmt.toml";
  env.CLIPPY_CONF_DIR = "${config.env.PROJECT_CONFIG_PATH}";

  enterShell = ''
    export PATH="$DEVENV_ROOT/.bin:$PATH"
  '';

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

  packages =
    (lib.attrValues mise-tools)
    ++ [
      (pkgs.writeShellScriptBin "tauri" ''exec ${pkgs.cargo-tauri}/bin/cargo-tauri "$@"'')
    ]
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
          "website/*/*.{md,mdx}"
          "docs/*/*.{md,mdx}"
        ];
      };
    }
  ];

  tasks."mise:lock" = {
    after = ["devenv:enterShell"];
    execIfModified = [
      ".config/mise.toml"
      ".config/mise.lock"
    ];
    exec = ''
      set -euo pipefail
      mise lock

      declared="$(yq -p toml -o=json '.tools | length' .config/mise.toml)"
      resolved="$(mise ls --current --json \
        | yq -o=json "to_entries | map(select(.value[0].source.path == \"$DEVENV_ROOT/.config/mise.toml\")) | length")"

      if [ "$declared" -ne "$resolved" ]; then
        echo "mise resolved only $resolved of the $declared tools in .config/mise.toml." >&2
        echo "Declared:" >&2
        yq -p toml -o=json '.tools | keys | .[]' mise.toml | tr -d '"' | sed 's/^/  /' >&2
        echo "Resolved:" >&2
        mise ls --current --json | yq -o=json 'keys | .[]' | tr -d '"' | sed 's/^/  /' >&2
        echo 'Add the missing one to the mise-formatters `unavailable` list in' >&2
        echo 'devenv.nix, or give it a correct backend spec.' >&2
        exit 1
      fi
    '';
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
        env = {
          PROJECT_ROOT = "{{ config_root }}";
          PROJECT_CONFIG_PATH = "{{ env.PROJECT_ROOT }}/.config";
          TAURI_FRONTEND_PATH = "{{ env.PROJECT_ROOT }}/${tauri-app-path}";
          TAURI_APP_PATH = "{{ env.PROJECT_ROOT }}/${tauri-lib-path}";
          TAURI_ANDROID_PROJECT_PATH = "{{ env.TAURI_APP_PATH }}/${tauri-android-path}";
          TAURI_IOS_PROJECT_PATH = "{{ env.TAURI_APP_PATH }}/${tauri-ios-path}";
          TREEFMT_CONFIG = "{{ env.PROJECT_CONFIG_PATH }}/treefmt.toml";
          CLIPPY_CONF_DIR = "{{ env.PROJECT_CONFIG_PATH }}";
        };
        tools = lib.mapAttrs (_: pkg: toString pkg.version) mise-tools;
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
