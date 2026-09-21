# Owns the mise projection: every tool the shell provides is mirrored into
# `.config/mise.toml` with pinned versions, `.config/mise.lock` is seeded, and
# the `mise:lock` task keeps the lockfile's provenance entries fresh. mise never
# provides dependencies inside the shell — nix does; the projection exists for
# consumers outside it (CI runners).
{
  lib,
  config,
  pkgs,
  ...
}:
let
  inherit (lib) mkOption types;
  cfg = config.waypoints.mise;
in
{
  options.waypoints.mise = {
    tools = mkOption {
      type = types.attrsOf types.package;
      default = { };
      description = "mise tool spec -> nix package providing it in the shell; versions project into `.config/mise.toml`.";
    };
    os = mkOption {
      # mise also accepts `os/arch` pairs ("macos/arm64"); add them if a tool
      # ever needs that granularity.
      type = types.attrsOf (
        types.listOf (
          types.enum [
            "linux"
            "macos"
            "windows"
            "unix" # everything but windows
          ]
        )
      );
      default = { };
      description = "mise tool spec -> operating systems it installs on (mise's `os` option); unlisted tools install everywhere.";
    };
    env = mkOption {
      type = types.listOf types.attrs;
      default = [ ];
      description = "Entries for the `[[env]]` array of the projected `.config/mise.toml`, `_` directives included.";
    };
  };

  config = {
    tasks."mise:lock" = {
      description = "Regenerate mise.lock and verify every declared tool resolved";
      package = pkgs.nushell;
      after = [ "devenv:enterShell" ];
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
          # A bare version string unless the tool is OS-restricted, in which
          # case mise wants the table form.
          tools = lib.mapAttrs (
            name: pkg:
            if cfg.os ? ${name} then
              {
                version = toString pkg.version;
                os = cfg.os.${name};
              }
            else
              toString pkg.version
          ) cfg.tools;
        };
      };
    };
  };
}
