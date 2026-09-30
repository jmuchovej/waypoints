# Owns the mise projection: every tool the shell provides is mirrored into
# `.config/mise.toml` with pinned versions, and into `.config/mise.<env>.toml`
# for the environments it belongs to. mise never provides dependencies inside
# the shell — nix does; the projection exists for consumers outside it (CI
# runners).
#
# Lockfiles are seeded here but never written on entry. `tooling/bin/mise-lock`
# refreshes them when asked: mise reports success after a rate-limited run that
# wrote only part of the file, so that cannot happen unattended.
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

  # Emitted as a `[vars]` entry so release CI can hand the list to
  # MISE_DISABLE_TOOLS without a second config file.
  formatter-specs = lib.sort lib.lessThan (
    map (tool: tool.spec) (attrValues (filterAttrs (_: tool: tool.formatter) included))
  );

  # A tool's mise entry: a bare version string unless something forces the
  # table form.
  entry =
    tool:
    if tool.os == [ ] && tool.options == { } && tool.depends == [ ] then
      toString tool.package.version
    else
      {
        version = toString tool.package.version;
      }
      // lib.optionalAttrs (tool.os != [ ]) { inherit (tool) os; }
      // lib.optionalAttrs (tool.depends != [ ]) { depends = map displayed tool.depends; }
      // tool.options;

  tool-table =
    names: lib.listToAttrs (map (n: nameValuePair included.${n}.spec (entry included.${n})) names);

  # Tools with no environment go in the base, which every environment loads.
  base-tools = attrNames (filterAttrs (_: tool: tool.envs == [ ]) included);

  # Environments this one pulls in, itself included. Bounded rather than
  # iterated to a fixpoint so a cycle in `composes` fails instead of hanging.
  composed =
    depth: env:
    if depth == 0 then
      [ env ]
    else
      lib.unique ([ env ] ++ lib.concatMap (composed (depth - 1)) cfg.environments.${env}.composes);

  # Grow a tool set over `depends` until it stops changing. Terminates even on
  # a dependency cycle: the set only grows and the tool set is finite.
  close =
    names:
    let
      next = lib.unique (lib.sort lib.lessThan (names ++ lib.concatMap (n: included.${n}.depends) names));
    in
    if next == names then names else close next;

  # What an environment's file contains: everything assigned to it or to any
  # environment it composes, closed over `depends`. A dependency already in the
  # base is still written out — generated duplication is free, and relying on
  # the base would break silently if the tool later moved.
  membership =
    env:
    close (
      lib.unique (
        lib.sort lib.lessThan (
          lib.concatMap (e: attrNames (filterAttrs (_: tool: lib.elem e tool.envs) included)) (composed 8 env)
        )
      )
    );

  # Environments that currently have members, and so have files on disk.
  materialised = filter (env: membership env != [ ]) environment-names;

  # The basenames those environments own. Anything else matching
  # `mise.<something>.{toml,lock}` under `.config` is a leftover.
  expected-files = lib.concatMap (env: [
    "mise.${env}.toml"
    "mise.${env}.lock"
  ]) materialised;
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

    # Runs after devenv has written its files, so the expected set is on disk
    # and anything left over is genuinely orphaned.
    tasks."mise:prune-environments" = {
      description = "Remove generated mise environment configs whose environment no longer has members";
      package = pkgs.nushell;
      after = [ "devenv:files" ];
      before = [ "devenv:enterShell" ];
      # `exec` runs under nushell, so the expected set is bound the nu way
      # rather than exported by a shell.
      exec = ''
        $env.WAYPOINTS_MISE_ENV_FILES = "${lib.concatStringsSep "," expected-files}"
        ${builtins.readFile ./bin/prune-mise-environments}
      '';
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
          # in `.config/mise/conf.d/base.toml`: nothing there derives from the
          # package set, and a generated file cannot be linted — treefmt has to
          # exclude it or tombi reformats what devenv writes.
          inherit (cfg) env;
          vars.WAYPOINTS_FORMATTERS = lib.concatStringsSep "," formatter-specs;
          # The mise devenv provides — the one that writes `.config/mise.lock`.
          # mise reads this itself and refuses an older one, and CI installs
          # exactly it rather than whatever released most recently: a lockfile
          # is only as reproducible as the tool reading it. mise 2026.9.16
          # tightened SLSA provenance and broke a nightly hours after
          # publishing, with nothing changed here.
          min_version = pkgs.mise.version;
          tools = tool-table base-tools;
        };
      };
    }
    # One config and one seeded lockfile per environment that has members. An
    # environment with none emits nothing: mise treats a missing
    # `mise.<env>.toml` as nothing to add, so selecting it degrades to
    # base-only rather than failing.
    // lib.concatMapAttrs (
      name: _:
      let
        members = membership name;
      in
      lib.optionalAttrs (members != [ ]) {
        ".config/mise.${name}.toml" = {
          copyMode = "copy";
          # `[tools]` and nothing else — settings, `min_version` and `[env]`
          # stay in the base, so each is written in exactly one place.
          toml.tools = tool-table members;
        };
        ".config/mise.${name}.lock" = {
          copyMode = "seed";
          text = "# This should be overwritten on re-entry.";
        };
      }
    ) cfg.environments;
  };
}
