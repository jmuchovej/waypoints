# Owns `.env` — the one file describing this repo's environment — and the two
# substituters that read it.
#
# `.env` is shared with mise, which loads it via `[env._.file]` and expands it
# at *runtime*, where `PROJECT_ROOT` is already set from `{{ config_root }}`.
# devenv's own `dotenv.substitution` expands at *eval* time instead, and its
# scope is the ambient process environment plus the file's own earlier lines —
# `config.env.PROJECT_ROOT` does not exist yet, because devenv derives `env`
# *from* the dotenv file. So the two want opposite things from the same line:
# mise needs `.env` to carry no `PROJECT_ROOT` (its own is correct, and a line
# in the file shadows it), while devenv's eval-time pass has nothing to expand
# against unless the file anchors itself.
#
# `${PROJECT_ROOT:-...}` cannot bridge them: mise does not implement default
# expansion, and drops the whole reference silently.
#
# So devenv's substitution is turned off and done here instead, against
# `config.devenv.root` — a value nix knows and the file therefore need not
# carry. `.env` keeps no anchor line, and each side supplies the root from
# something it actually knows.
{
  lib,
  config,
  ...
}:
let
  inherit (lib)
    attrNames
    concatStringsSep
    filter
    foldlAttrs
    mapAttrs
    ;

  anchors = {
    PROJECT_ROOT = config.devenv.root;
  };

  # `PATH` is deliberately not taken from `.env`. Its value there ends in
  # `${PATH}`, a *runtime* self-reference that eval-time expansion would freeze
  # to whatever the evaluator happened to have. devenv assembles the shell's
  # PATH itself; `enterShell` prepends the same two entries mise reads from the
  # file, so both agree without either resolving `${PATH}` early.
  runtime-owned = [ "PATH" ];

  literals = removeAttrs config.dotenv.resolved runtime-owned;

  # Replace every `${NAME}` for which `known` has a value. Names it does not
  # cover are left alone for a later pass.
  substitute =
    known: value:
    foldlAttrs (
      acc: name: v:
      lib.replaceStrings [ "\${${name}}" ] [ v ] acc
    ) value known;

  # `.env` chains — PROJECT_ROOT → PROJECT_CONFIG_PATH → TREEFMT_CONFIG — and an
  # attrset has no order, so one pass cannot resolve a reference to an entry
  # that has not been substituted yet. Re-substituting from the literals each
  # round converges.
  #
  # The bound is passes, not layers, and the two are not proportional:
  # `substitute` folds over names alphabetically, so a round collapses a whole
  # chain when each name sorts after what it references, and advances one step
  # when it does not. Measured against this file, one pass leaves
  # TAURI_ANDROID_PROJECT_PATH and TAURI_IOS_PROJECT_PATH unresolved (they sort
  # before TAURI_APP_PATH) and two passes suffice; three leaves a single pass of
  # slack. Raise it only in response to the assertion below — never pre-emptively,
  # or an accidental cycle becomes a silent truncation instead of an error.
  resolve =
    depth: acc:
    if depth == 0 then
      acc
    else
      resolve (depth - 1) (mapAttrs (_: substitute (anchors // acc)) literals);

  resolved = resolve 3 literals;

  # Fail closed. An unresolved `${...}` means `.env` referenced a name nothing
  # defines — silently exporting the literal would put a `${` into a path and
  # surface as a baffling file-not-found much later.
  unresolved = filter (name: lib.hasInfix "\${" resolved.${name}) (attrNames resolved);

  # `.env`'s PATH, with every name but PATH itself expanded and the trailing
  # `${PATH}` handed to the shell to expand at entry. Derived from the same line
  # mise reads, so the two cannot disagree about what is prepended.
  path-line = lib.replaceStrings [ "\${PATH}" ] [ "$PATH" ] (
    substitute (anchors // resolved) config.dotenv.resolved.PATH
  );
in
{
  imports = [
    ./mise.nix
    ./android.nix
  ];

  dotenv = {
    enable = true;
    filename = ".env";
    substitution = false;
  };

  # A plain definition, so it takes precedence over the `mkDefault` the dotenv
  # module applies to the unexpanded values.
  env = resolved;

  enterShell = ''
    export PATH="${path-line}"
  '';

  assertions = [
    {
      assertion = unresolved == [ ];
      message =
        "${config.dotenv.filename} references undefined variables in: "
        + concatStringsSep ", " unresolved
        + ". Define them earlier in the file, or add an anchor in nix/devenv/default.nix.";
    }
  ];
}
