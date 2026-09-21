# Owns treefmt's projection into `.config/treefmt.toml`: store-path commands
# are rewritten to bare names (mise supplies them outside the shell), wrapper
# scripts from `.bin` replace formatters that need shell-level glue, and the
# global exclude list feeds both the live shell config and the projection.
{
  lib,
  config,
  ...
}: let
  cfg = config.waypoints.treefmt;

  bare = name: fmt:
    if cfg.wrappers ? ${name}
    then
      fmt
      // {
        command = cfg.wrappers.${name};
        options = [];
      }
    else fmt // {command = baseNameOf fmt.command;};

  treefmt-formatters = lib.mapAttrs bare config.treefmt.config.settings.formatter;
in {
  options.waypoints.treefmt = {
    wrappers = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = ''
        Formatters whose projected command is replaced by a wrapper script
        (resolved from `.bin` via PATH). The wrapper owns its own flags, so
        `options` is emptied for these entries.
      '';
    };
    excludes = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [];
      description = "Global treefmt exclude globs.";
    };
  };

  config = {
    treefmt.config.settings.excludes = cfg.excludes;

    files.".config/treefmt.toml" = {
      copyMode = "copy";
      toml =
        lib.filterAttrs (name: _: name != "global") config.treefmt.config.settings
        // {
          formatter = treefmt-formatters;
        };
    };
  };
}
