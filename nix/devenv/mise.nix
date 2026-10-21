# handles projecting devenv into `mise` (all written to `.config/mise/conf.d/`).
# each tool's version is synchronized with devenv. the base toolchain is stored
# in `tools.toml` and supplemental environments are written to
# `tools.<env>.toml`. this project exists for non-nix consumers (i.e., CI
# runners, non-Nix contributors, etc.).
{
  lib,
  config,
  pkgs,
  ...
}:
let
  inherit (lib)
    attrValues
    filter
    filterAttrs
    mapAttrsToList
    mkOption
    optionalAttrs
    ;

  cfg = config.waypoints.mise;
  tools-prefix = ".config/mise/conf.d";
  tools-to-include = filterAttrs (_: tool: tool.envs != null) cfg.tools;

  # every lane a tool set is written for, keyed by environment. `base` is
  # reserved for tools with no environment, which every environment loads.
  lanes = {
    base = filterAttrs (_: tool: tool.envs == [ ]) tools-to-include;
  }
  // lib.genAttrs cfg.environments (
    env: filterAttrs (_: tool: lib.elem env tool.envs) tools-to-include
  );

  tool-type =
    with lib.types;
    submodule {
      options = {
        package = mkOption {
          type = nullOr package;
          default = null;
          description = "The nix package providing the tool in the shell; its version is what mise pins.";
        };
        spec = mkOption {
          type = nullOr str;
          default = null;
          description = "Backend-qualified mise tool spec, e.g. `aqua:casey/just`.";
        };
        os = mkOption {
          # mise also accepts `os/arch` pairs ("macos/arm64"); add them if a tool
          # ever needs that granularity.
          type = listOf (enum [
            "linux"
            "macos"
            "windows"
            "unix"
          ]);
          default = [ ];
          description = ''
            Operating systems mise installs the tool on (its `os`  option);
            empty means everywhere.
          '';
        };
        envs = mkOption {
          type = nullOr (listOf (enum cfg.environments));
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
          type = listOf str;
          default = [ ];
          description = ''
            Specs of other projected tools this one needs at run time — the
            runtime a `gem:`/`npm:` tool executes on. Written and projected as
            the spec (`core:ruby`); mise matches a dependency on any of its
            identities, so the full spec is the unambiguous choice.

            mise does not enforce this: a config naming an absent dependency
            resolves the dependent tool and exits zero. So the dependency is not
            pulled in here either — an environment missing one is an assertion
            failure, and the fix is to say where the dependency belongs.
          '';
        };
      };
    };
in
{
  options.waypoints.mise = with lib.types; {
    environments = mkOption {
      type = listOf str;
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
      type = attrsOf tool-type;
      default = { };
      description = "Tools by name: nix package, mise spec, and OS restriction together.";
    };
    env = mkOption {
      type = attrs;
      default = { };
      description = ''
        The `[env]` table of the projected `tools.toml`, `_` directives
        included. Normally empty: the environment mise needs — `PROJECT_ROOT`,
        the `.env` pointer, the PATH additions — is policy rather than something
        derived from the tool set, so it is hand-maintained in
        `.config/mise.toml`; see nix/devenv/default.nix for why each side
        resolves `PROJECT_ROOT` its own way.
      '';
    };
  };

  config = {
    assertions =
      let
        base-is-reserved = {
          assertion = !lib.elem "base" cfg.environments;
          message = "waypoints.mise.environments: `base` is reserved for tools with `envs = [ ]`.";
        };

        has-package-spec-or-null-envs = mapAttrsToList (name: tool: {
          assertion = tool.envs == null || (tool.package != null && tool.spec != null);
          message = "waypoints.mise.tools.${name}: set both `package` and `spec`, or `envs = null`.";
        }) cfg.tools;

        # the lanes `tool` is written into that `dep` isn't.
        missing-lanes =
          tool: dep:
          if dep.envs == [ ] then
            [ ]
          else if tool.envs == [ ] then
            [ "base" ]
          else
            lib.subtractLists dep.envs tool.envs;

        by-spec = lib.pipe tools-to-include [
          attrValues
          (map (tool: lib.nameValuePair tool.spec tool))
          lib.listToAttrs
        ];

        depends-will-resolve-assertion =
          {
            name,
            tool,
            dep,
          }:
          let
            prefix = "waypoints.mise.tools.${name}.depends: `${dep}`";
            missing = missing-lanes tool by-spec.${dep};
            quoted = lib.concatMapStringsSep ", " (lane: "\"${lane}\"") missing;
          in
          if !(by-spec ? ${dep}) then
            {
              assertion = false;
              message = "${prefix} is not a projected tool's spec.";
            }
          else
            {
              assertion = missing == [ ];
              message =
                "${prefix} is missing from ${quoted}. "
                + (
                  if missing == [ "base" ] then
                    "Give that tool `envs = [ ]`."
                  else
                    "Add ${quoted} to that tool's `envs`."
                );
            };

        # mise resolves a `depends` on an unprojected tool to nothing and exits
        # zero, so the generator has to refuse it.
        depends-will-resolve = lib.pipe tools-to-include [
          (mapAttrsToList (name: tool: map (dep: { inherit name tool dep; }) tool.depends))
          lib.concatLists
          (map depends-will-resolve-assertion)
        ];
      in
      [ base-is-reserved ] ++ has-package-spec-or-null-envs ++ depends-will-resolve;

    # every tool with a package, except what can't be built on this platform
    # (e.g., xcodegen off macOS); mise still pins those.
    packages = lib.pipe cfg.tools [
      attrValues
      (filter (tool: tool.package != null))
      (map (tool: tool.package))
      (filter (lib.meta.availableOn pkgs.stdenv.hostPlatform))
    ];

    # Runs after devenv has written its files, so the expected set is on disk
    # and anything left over is genuinely orphaned.
    tasks."mise:prune-environments" = {
      description = "Remove generated mise environment configs whose environment no longer has members";
      package = pkgs.nushell;
      after = [ "devenv:files" ];
      before = [
        "devenv:enterShell"
        "devenv:enterTest"
      ];
      exec =
        let
          # the basenames the declared environments own.
          expected-files = lib.pipe config.files [
            lib.attrNames
            (filter (lib.hasPrefix tools-prefix))
            (map lib.baseNameOf)
            (lib.concatStringsSep ",")
          ];
        in
        ''
          $env.WAYPOINTS_MISE_ENV_FILES = "${expected-files}"
          ${builtins.readFile ./bin/prune-mise-environments}
        '';
    };

    files =
      let
        # a tool's mise entry: a bare version string unless something forces the
        # table form.
        mk-tool-value =
          tool:
          let
            version = toString tool.package.version;
          in
          if tool.os == [ ] && tool.depends == [ ] then
            version
          else
            {
              inherit version;
            }
            // optionalAttrs (tool.os != [ ]) { inherit (tool) os; }
            // optionalAttrs (tool.depends != [ ]) { inherit (tool) depends; };

        filename-for = lane: if lane == "base" then "tools" else "tools.${lane}";

        toolfile = lane: tools: {
          "${tools-prefix}/${filename-for lane}.toml" = {
            copyMode = "copy";
            toml = {
              tools = lib.mapAttrs' (_: tool: lib.nameValuePair tool.spec (mk-tool-value tool)) tools;
            }
            // optionalAttrs (lane == "base") {
              # project whatever `nix` computes. all other `[settings]` are
              # hand-maintained in `.config/mise.toml`.
              inherit (cfg) env;
              # ensure that `mise` is at least the same as what generated `tools`
              min_version = pkgs.mise.version;
            };
          };
        };
      in
      # each config fragment per-declared environment. these are selectable
      # via `MISE_ENV`.
      lib.concatMapAttrs toolfile lanes;
  };
}
