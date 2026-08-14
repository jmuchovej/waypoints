{
  pkgs,
  lib,
  config,
  ...
}: let
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

      "gem:fastlane" = pkgs.fastlane;
      "npm:@fission-ai/openspec" = pkgs.openspec;
      "npm:@tauri-apps/cli" = pkgs.cargo-tauri;

      "aqua:nextest-rs/nextest/cargo-nextest" = pkgs.cargo-nextest;
      "aqua:numtide/treefmt" = config.treefmt.config.package;

      "core:node" = config.languages.javascript.package;
      "core:deno" = config.languages.deno.package;
      "core:ruby" = config.languages.ruby.package;
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
    "{mise,treefmt}.toml"
  ];
in {
  stdenv = pkgs.clangStdenv;
  env.LD_LIBRARY_PATH = lib.mkForce "";
  env.PROJECT_ROOT = config.env.DEVENV_ROOT;

  enterShell = ''
    export PATH="$DEVENV_ROOT/.bin:$PATH"
  '';

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
  languages.ruby = {
    enable = true;
    lsp.enable = false;
  };
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
  treefmt.config.programs = {
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

    statix = {
      enable = true;
      priority = 1;
    };
    deadnix = {
      enable = true;
      priority = 2;
    };
    alejandra = {
      enable = true;
      priority = 3;
    };
  };
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
      "mise.toml"
      "mise.lock"
    ];
    exec = ''
      set -euo pipefail
      mise lock

      declared="$(yq -p toml -o=json '.tools | length' mise.toml)"
      resolved="$(mise ls --current --json \
        | yq -o=json "to_entries | map(select(.value[0].source.path == \"$DEVENV_ROOT/mise.toml\")) | length")"

      if [ "$declared" -ne "$resolved" ]; then
        echo "mise resolved only $resolved of the $declared tools in mise.toml." >&2
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
    "mise.lock" = {
      copyMode = "seed";
      text = "# This should be overwritten on re-entry.";
    };

    "treefmt.toml" = {
      copyMode = "copy";
      toml =
        lib.filterAttrs (name: _: name != "global") config.treefmt.config.settings
        // {
          formatter = treefmt-formatters;
        };
    };

    "mise.toml" = {
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
        tools = lib.mapAttrs (_: pkg: toString pkg.version) mise-tools;
      };
    };
  };

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
    after = ["cargo-check"];
    language = "system";
    entry = "${config.git-hooks.hooks.cargo-test.package}/bin/cargo test";
    files = "\\.rs$";
    pass_filenames = false;
  };
  git-hooks.hooks.treefmt = {
    enable = true;
    settings.fail-on-change = false;
  };
}
