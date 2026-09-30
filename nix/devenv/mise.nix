# Owns the mise projection: every tool the shell provides is mirrored into
# `.config/mise.toml` with pinned versions, `.config/mise.lock` is seeded, and
# the `mise:lock` task keeps the lockfile's provenance entries fresh. mise never
# provides dependencies inside the shell — nix does; the projection exists for
# consumers outside it (CI runners).
#
# Every tool is one `waypoints.mise.tools.<name>` entry carrying its nix
# package, its backend-qualified mise spec, and any OS restriction. A tool with
# a package but no mise backend (the nix-only formatters) lives in the shell
# alone: `include = false` keeps it out of the projection. The assertion refuses
# anything projected without both halves.
{
  lib,
  config,
  pkgs,
  ...
}:
let
  inherit (lib)
    attrNames
    attrValues
    filter
    filterAttrs
    mapAttrs
    mapAttrs'
    mkOption
    nameValuePair
    types
    ;
  cfg = config.waypoints.mise;

  environment-names = attrNames cfg.environments;

  environment-type = types.submodule {
    options.composes = mkOption {
      type = types.listOf (types.enum environment-names);
      default = [ ];
      description = ''
        Environments whose members this one also contains, transitively. mise
        composes environments at *selection* time (`MISE_ENV=a,b`) but a config
        has no include directive, so a union has to be built here.
      '';
    };
  };

  # What mise calls the tool: its spec, minus the `core:` prefix mise strips
  # when displaying and when matching `depends`.
  displayed = name: lib.removePrefix "core:" cfg.tools.${name}.spec;

  tool-type = types.submodule {
    options = {
      package = mkOption {
        type = types.nullOr types.package;
        default = null;
        description = "The nix package providing the tool in the shell; its version is what mise pins.";
      };
      spec = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Backend-qualified mise tool spec, e.g. `aqua:casey/just`.";
      };
      os = mkOption {
        # mise also accepts `os/arch` pairs ("macos/arm64"); add them if a tool
        # ever needs that granularity.
        type = types.listOf (
          types.enum [
            "linux"
            "macos"
            "windows"
            "unix" # everything but windows
          ]
        );
        default = [ ];
        description = "Operating systems mise installs the tool on (its `os` option); empty means everywhere.";
      };
      envs = mkOption {
        type = types.listOf (types.enum environment-names);
        default = [ ];
        description = ''
          Environments this tool belongs to. Empty puts it in the base config,
          which every environment loads; a non-empty list writes it into each
          named environment's file instead.
        '';
      };
      depends = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = ''
          Other declared tools this one needs at run time — the runtime a
          `gem:`/`npm:` tool executes on. Named as `waypoints.mise.tools.<name>`
          and projected as mise's displayed name.

          mise does not enforce this: a config naming an absent dependency
          resolves the dependent tool and exits zero, so the closure is computed
          here instead.
        '';
      };
      include = mkOption {
        type = types.bool;
        default = true;
        description = "Project the tool into `.config/mise.toml`. `false` keeps a nix-only tool in the shell without a mise entry.";
      };
      formatter = mkOption {
        type = types.bool;
        default = false;
        description = "Exists only to format: listed in `WAYPOINTS_FORMATTERS` so release CI can skip it.";
      };
      options = mkOption {
        type = types.attrsOf types.str;
        default = { };
        description = ''
          Extra backend options for the tool's mise entry, e.g. the
          `slsa_signer_identity`/`slsa_signer_issuer` a `github:` tool needs to
          verify SLSA provenance against the workflow that actually signed it.
          Forces the table form.
        '';
      };
    };
  };

  in-shell = filter (tool: tool.package != null) (attrValues cfg.tools);
  included = filterAttrs (_: tool: tool.include) cfg.tools;
  by-spec = mapAttrs' (_: tool: nameValuePair tool.spec tool) included;

  # Emitted as a `[vars]` entry so release CI can hand the list to
  # MISE_DISABLE_TOOLS without a second config file.
  formatter-specs = lib.sort lib.lessThan (attrNames (filterAttrs (_: tool: tool.formatter) by-spec));
in
{
  options.waypoints.mise = {
    environments = mkOption {
      type = types.attrsOf environment-type;
      default = { };
      description = ''
        Environments a consumer can select, each naming a unit of work rather
        than an actor. Declaring one here is what makes it nameable in a tool's
        `envs`, so a typo there is an evaluation error instead of a config file
        that silently never loads.
      '';
    };
    tools = mkOption {
      type = types.attrsOf tool-type;
      default = { };
      description = "Tools by name: nix package, mise spec, and OS restriction together.";
    };
    env = mkOption {
      type = types.attrs;
      default = { };
      description = ''
        The `[env]` table of the projected `.config/mise.toml`, `_` directives
        included. `_.file` points mise at `.env` — the environment description
        devenv reads too, so the two cannot disagree; see nix/devenv/default.nix
        for why each side resolves `PROJECT_ROOT` its own way.
      '';
    };
  };

  config = {
    assertions =
      map (
        name:
        let
          tool = cfg.tools.${name};
        in
        {
          assertion = !tool.include || (tool.package != null && tool.spec != null);
          message = "waypoints.mise.tools.${name}: set both `package` and `spec`, or `include = false`.";
        }
      ) (attrNames cfg.tools)
      # A dependency on an undeclared tool would project a `depends` entry mise
      # resolves to nothing and reports success for — the failure this option
      # exists to prevent, reintroduced one level up.
      ++ lib.concatMap (
        name:
        map (dependency: {
          assertion = cfg.tools ? ${dependency} && cfg.tools.${dependency}.include;
          message = "waypoints.mise.tools.${name}.depends: no projected tool named `${dependency}`.";
        }) cfg.tools.${name}.depends
      ) (attrNames cfg.tools);

    # Every tool with a package, minus what cannot be built on this platform
    # (xcodegen off macOS); those still project their versions into mise.toml.
    packages = filter (lib.meta.availableOn pkgs.stdenv.hostPlatform) (
      map (tool: tool.package) in-shell
    );

    tasks."mise:lock" = {
      description = "Regenerate mise.lock and verify every declared tool resolved";
      package = pkgs.nushell;
      after = [ "devenv:enterShell" ];
      execIfModified = [
        ".config/mise.toml"
        ".config/mise.lock"
      ];
      exec = builtins.readFile ./bin/sync-mise;
    };

    files = {
      ".config/mise.lock" = {
        copyMode = "seed";
        text = "# This should be overwritten on re-entry.";
      };

      ".config/mise.toml" = {
        copyMode = "copy";
        toml = {
          # Only what nix computes is projected. `[settings]` is hand-maintained
          # in `.config/mise/conf.d/base.toml`, where a `#:schema` comment makes
          # it lintable — treefmt has to exclude generated `mise.toml` outright,
          # so anything projected here is unlintable by construction, and none
          # of those settings derive from the package set anyway.
          inherit (cfg) env;
          vars.WAYPOINTS_FORMATTERS = lib.concatStringsSep "," formatter-specs;
          # The mise devenv provides — the one that writes `.config/mise.lock`.
          # mise reads this itself and refuses an older one, and CI installs
          # exactly it rather than whatever released most recently: a lockfile
          # is only as reproducible as the tool reading it. mise 2026.9.16
          # tightened SLSA provenance and broke a nightly hours after
          # publishing, with nothing changed here.
          min_version = pkgs.mise.version;
          # A bare version string unless the tool is OS-restricted, in which
          # case mise wants the table form.
          tools = mapAttrs (
            _: tool:
            if tool.os == [ ] && tool.options == { } && tool.depends == [ ] then
              toString tool.package.version
            else
              {
                version = toString tool.package.version;
              }
              // lib.optionalAttrs (tool.os != [ ]) { inherit (tool) os; }
              // lib.optionalAttrs (tool.depends != [ ]) { depends = map displayed tool.depends; }
              // tool.options
          ) by-spec;
        };
      };
    };
  };
}
