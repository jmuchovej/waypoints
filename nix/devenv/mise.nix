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
# package, its backend-qualified mise spec, any OS restriction, and the
# environments it belongs to. `envs = null` is a nix-only tool: it lives in the
# shell and is never projected. Otherwise `envs = [ ]` means the base config and
# a non-empty list means those environments' files; an assertion refuses
# anything projected without both a package and a spec.
#
# Nothing is inferred. An environment holds exactly the tools that named it, and
# a tool whose `depends` is absent from a lane is an evaluation error rather than
# something quietly pulled in — mise resolves such a config and exits zero, so
# the generator has to be the one that notices.
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
    mkOption
    nameValuePair
    types
    ;
  cfg = config.waypoints.mise;

  environment-names = cfg.environments;

  # What mise calls a tool: its spec, minus the `core:` prefix mise strips when
  # displaying and when matching `depends`.
  displayed = lib.removePrefix "core:";

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
        type = types.nullOr (types.listOf (types.enum environment-names));
        default = [ ];
        description = ''
          Environments this tool belongs to. `[ ]` puts it in the base config,
          which every environment loads; a non-empty list writes it into each
          named environment's file instead.

          `null` keeps the tool out of mise entirely — a nix-only tool that
          lives in the shell and has no projected entry. There is no separate
          `include` flag: a tool is projected exactly when it names where it
          belongs.
        '';
      };
      depends = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = ''
          Specs of other projected tools this one needs at run time — the
          runtime a `gem:`/`npm:` tool executes on. Written as the spec
          (`core:ruby`) and projected as mise's displayed name (`ruby`), which
          is what mise matches.

          mise does not enforce this: a config naming an absent dependency
          resolves the dependent tool and exits zero. So the dependency is not
          pulled in here either — an environment missing one is an assertion
          failure, and the fix is to say where the dependency belongs.
        '';
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
  included = filterAttrs (_: tool: tool.envs != null) cfg.tools;

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

  # What an environment's own file contains: exactly the tools that named it.
  # Nothing is pulled in — a tool is in an environment because it says so.
  membership = env: attrNames (filterAttrs (_: tool: lib.elem env tool.envs) included);

  # Every lane a tool set is written for, base included, so the dependency
  # assertion can ask the same question of each.
  lanes = {
    base = base-tools;
  }
  // lib.genAttrs environment-names membership;

  # The mise-displayed names resolvable in a lane: its own tools plus the base,
  # which every environment loads on top of.
  resolvable =
    lane: lib.unique (map (name: displayed included.${name}.spec) (lanes.base ++ lanes.${lane}));

  # The basenames the declared environments own. Anything else matching
  # `mise.<something>.{toml,lock}` under `.config` is a leftover from an
  # environment that has been renamed or removed.
  expected-files = lib.concatMap (env: [
    "mise.${env}.toml"
    "mise.${env}.lock"
  ]) environment-names;
in
{
  options.waypoints.mise = {
    environments = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = ''
        Environments a consumer can select, each naming a unit of work rather
        than an actor. Declaring one here is what makes it nameable in a tool's
        `envs`, so a typo there is an evaluation error instead of a config file
        that silently never loads.

        A consumer needing several names them together — mise reads `MISE_ENV`
        as a comma-separated list and loads each config additively. There is no
        union environment: composition happens at selection, so building one
        here would only duplicate what the caller already says.
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
        included. Normally empty: the environment mise needs — `PROJECT_ROOT`,
        the `.env` pointer, the PATH additions — is policy rather than something
        derived from the tool set, so it is hand-maintained in
        `.config/mise/conf.d/base.toml`; see nix/devenv/default.nix for why
        each side resolves `PROJECT_ROOT` its own way.
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
          assertion = tool.envs == null || (tool.package != null && tool.spec != null);
          message = "waypoints.mise.tools.${name}: set both `package` and `spec`, or `envs = null`.";
        }
      ) (attrNames cfg.tools)
      # A dependency on an unprojected tool would write a `depends` entry mise
      # resolves to nothing and reports success for — the failure this option
      # exists to prevent, reintroduced one level up.
      ++ lib.concatMap (
        name:
        map (dependency: {
          assertion = lib.elem dependency (map (tool: tool.spec) (attrValues included));
          message = "waypoints.mise.tools.${name}.depends: `${dependency}` is not a projected tool's spec.";
        }) included.${name}.depends
      ) (attrNames included)
      # mise will not enforce the closure and will not complain: a config whose
      # `depends` target is absent installs the dependent tool, exits zero, and
      # fails at run time with no interpreter. So every lane is checked to
      # already contain what its tools depend on. Deliberately an assertion
      # rather than a fixup — the generator pulling the dependency in silently
      # would mean an environment's contents were never quite what it declared.
      ++ lib.concatMap (
        lane:
        lib.concatMap (
          name:
          map (dependency: {
            assertion = lib.elem (displayed dependency) (resolvable lane);
            message = "waypoints.mise.${lane}: `${included.${name}.spec}` depends on `${dependency}`, which is not a member. Add `\"${lane}\"` to that tool's `envs`.";
          }) included.${name}.depends
        ) lanes.${lane}
      ) (attrNames lanes);

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
        text = "# Placeholder. Run `just mise-lock` to populate; nothing writes this on shell entry.";
      };

      ".config/mise.toml" = {
        copyMode = "copy";
        toml = {
          # Only what nix computes is projected. `[settings]` is hand-maintained
          # in `.config/mise/conf.d/base.toml`: nothing there derives from the
          # package set, and a generated file cannot be linted — treefmt has to
          # exclude it or tombi reformats what devenv writes.
          inherit (cfg) env;
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
    # One config and one seeded lockfile for every declared environment, member
    # or not. An environment whose tools are all in the base gets a file with an
    # empty `[tools]` rather than no file at all: a consumer names the same
    # environment whatever it currently contains, so every workflow job carries
    # the same `MISE_ENV` shape, and the day a lane-specific tool appears it
    # lands in a file the jobs already select.
    // lib.concatMapAttrs (name: members: {
      ".config/mise.${name}.toml" = {
        copyMode = "copy";
        # `[tools]` and nothing else — settings, `min_version` and `[env]`
        # stay in the base, so each is written in exactly one place.
        toml.tools = tool-table members;
      };
      ".config/mise.${name}.lock" = {
        copyMode = "seed";
        text = "# Placeholder. Run `just mise-lock` to populate; nothing writes this on shell entry.";
      };
    }) (removeAttrs lanes [ "base" ]);
  };
}
