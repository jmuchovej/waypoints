# Flavor-parameterized frontend build: `nuxi generate` over `src/`, with
# `VITE_WAYPOINTS_FLAVOR` taken from the flavor's Tauri config identifier
# (mirroring `src/justfile`). Build-graph intermediate only — consumed by
# `package.nix` as `frontendDist`, never exposed as a package.
#
# The pnpm dependency FOD's `src` is manifests-only: all flavors share one
# fetch, and app-code edits never invalidate it. To refresh the hash after a
# `pnpm-lock.yaml` change, see the comment at `hash` below.
{
  lib,
  stdenv,
  fetchPnpmDeps,
  pnpmConfigHook,
  pnpm_11,
  nodejs,
  deno,
  flavor,
}:
let
  inherit ((fromTOML (builtins.readFile ../../../src-tauri/configs/flavors/${flavor}.toml)))
    identifier
    ;

  root = ../../..;
  # Everything pnpm needs to resolve the workspace — and nothing that would
  # vary per flavor or per app-code edit.
  manifests = lib.fileset.unions [
    ../../../package.json
    ../../../pnpm-lock.yaml
    ../../../pnpm-workspace.yaml
    ../../../patches
    ../../../src/package.json
    ../../../website/package.json
    ../../../crates/domain/package.json
    ../../../crates/cartograph/package.json
    ../../../tooling/icongen/package.json
  ];
  # `devEngines` in package.json (node ^24.4, deno ^2.9.4, onFail: error) is
  # enforced at every pnpm boot, including inside the fetch sandbox.
  runtimes = [
    nodejs
    deno
  ];
in
stdenv.mkDerivation (finalAttrs: {
  pname = "waypoints-frontend-${flavor}";
  version = (fromTOML (builtins.readFile ../../../src-tauri/Cargo.toml)).package.version;

  src = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions [
      manifests
      # Workspace packages the app imports: plain checked-in TS exports.
      ../../../crates/domain/typescript
      ../../../crates/cartograph/typescript
      (lib.fileset.difference ../../../src (
        lib.fileset.unions [
          (lib.fileset.maybeMissing ../../../src/node_modules)
          (lib.fileset.maybeMissing ../../../src/.nuxt)
          (lib.fileset.maybeMissing ../../../src/.output)
        ]
      ))
    ];
  };

  nativeBuildInputs = [
    pnpm_11
    pnpmConfigHook
  ]
  ++ runtimes;

  pnpmDeps = fetchPnpmDeps {
    pname = "waypoints-frontend";
    inherit (finalAttrs) version;
    src = lib.fileset.toSource {
      inherit root;
      fileset = manifests;
    };
    pnpm = pnpm_11;
    fetcherVersion = 4;
    nativeBuildInputs = [ deno ];
    # Refresh after any pnpm-lock.yaml change:
    #   set to lib.fakeHash, run `nix build .#waypoints`, copy the
    #   "got:" hash from the mismatch error back here.
    hash = "sha256-zFz9iUIhZQ66U5lSjCQ+RKiAXfUC7HcWuEAMd2CKgcU=";
  };

  env = {
    VITE_WAYPOINTS_FLAVOR = identifier;
    NUXT_TELEMETRY_DISABLED = "1";
  };

  buildPhase = ''
    runHook preBuild

    # Verification hook (task 5.2): production DCE strips the only consumer
    # of VITE_WAYPOINTS_FLAVOR (dev-only httpTransport), so the identifier
    # never appears in emitted assets. Assert the injection at build time
    # instead — auditable via `nix log`.
    [ -n "$VITE_WAYPOINTS_FLAVOR" ] || {
      echo "VITE_WAYPOINTS_FLAVOR is empty" >&2
      exit 1
    }
    echo "building frontend flavor=${flavor} VITE_WAYPOINTS_FLAVOR=$VITE_WAYPOINTS_FLAVOR"

    cd src
    pnpm exec nuxi generate

    runHook postBuild
  '';

  # `nuxi generate` emits the browsable site under `.output/public`
  # (`.output` itself carries nitro metadata). Normalize: $out is the
  # directory that holds index.html.
  installPhase = ''
    runHook preInstall

    if [ -f .output/public/index.html ]; then
      cp -r .output/public $out
    elif [ -f .output/index.html ]; then
      cp -r .output $out
    else
      echo "no index.html under src/.output — nuxi generate layout changed?" >&2
      exit 1
    fi

    runHook postInstall
  '';
})
