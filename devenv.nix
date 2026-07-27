{
  pkgs,
  lib,
  config,
  ...
}:
{
  stdenv = pkgs.clangStdenv;
  env.LD_LIBRARY_PATH = lib.mkForce "";
  env.PROJECT_ROOT = config.env.DEVENV_ROOT;

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
    openspec
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
      "**/.{devenv,direnv}*"
      "**/.envrc*"
      "**/.gitignore"
      # Patched third-party crates ([patch.crates-io] path forks) — formatting
      # them obscures the local delta against upstream.
      "vendor/**"
      "*.{png,gif,ico}"
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
      swift-format.enable = true;
      tombi.enable = true;
      rufo = {
        enable = true;
        includes = [
          "*.rb"
          "**/Podfile"
          "*.podspec"
        ];
      };
      just.enable = true;
      oxfmt = {
        enable = true;
        excludes = [ "{website,docs}/*/*.{md,mdx}" ];
        includes = [
          "*.{css,scss}"
          "*.graphql"
          "*.hbs"
          "*.html"
          "*.{js,jsx,cjs,mjs}"
          "*.{json,json5,jsonc}"
          "*.mustache"
          "*.{ts,tsx}"
          "*.vue"
          "*.{yaml,yml}"
          "*.{md,mdx}"
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
