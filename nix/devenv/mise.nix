# Owns the mise projection: the dev shell's tool set (declared tools plus
# treefmt formatters that mise can supply) is mirrored into `.config/mise.toml`
# with pinned versions, `.config/mise.lock` is seeded, and the `mise:lock`
# task keeps the lockfile's provenance entries fresh. mise never provides
# dependencies inside the shell — nix does; the projection exists for
# consumers outside it (CI runners).
{
  lib,
  config,
  pkgs,
  ...
}: let
  cfg = config.waypoints.mise;

  is-enabled = prog: prog.enable or false;
  has-package = prog: prog ? package;
  is-available = name: !(builtins.elem name cfg.unavailable);

  mise-formatters = lib.mapAttrs' (name: prog: lib.nameValuePair (cfg.specs.${name} or name) prog.package) (
    lib.filterAttrs (
      name: prog: (is-enabled prog) && (has-package prog) && (is-available name)
    )
    config.treefmt.config.programs
  );
in {
  options.waypoints.mise = {
    specs = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "treefmt formatter name -> mise tool spec (backend-qualified); unmapped formatters project under their own name.";
    };
    unavailable = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "treefmt formatters with no usable mise backend; excluded from the projection.";
    };
    tools = lib.mkOption {
      type = lib.types.attrsOf lib.types.package;
      default = {};
      description = "mise tool spec -> nix package providing it in the shell; versions project into `.config/mise.toml`.";
    };
    env = lib.mkOption {
      type = lib.types.listOf (lib.types.attrsOf lib.types.str);
      default = [];
      description = "Entries for the `[[env]]` array of the projected `.config/mise.toml`.";
    };
    resolved-tools = lib.mkOption {
      type = lib.types.attrsOf lib.types.package;
      readOnly = true;
      description = "Declared tools merged with the projected treefmt formatters (read-only).";
    };
  };

  config = {
    waypoints.mise.resolved-tools = cfg.tools // mise-formatters;

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

    files = {
      ".config/mise.lock" = {
        copyMode = "seed";
        text = "# This should be overwritten on re-entry.";
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
          inherit (cfg) env;
          tools = lib.mapAttrs (_: pkg: toString pkg.version) cfg.resolved-tools;
        };
      };
    };
  };
}
