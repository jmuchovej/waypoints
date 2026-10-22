# Flavor-parameterized frontend build: `nuxi generate` over `src/`, with
# `VITE_WAYPOINTS_FLAVOR` taken from the flavor's Tauri config identifier
# (mirroring `src/justfile`). Build-graph intermediate only — consumed by
# `package.nix` as `frontendDist`, never exposed as a package.
#
# The pnpm dependency FOD's `src` is manifests-only: all flavors share one
# fetch, and app-code edits never invalidate it. Its hash comes from
# `hashes.json` by way of package.nix; refresh it with `just nix-pins` after a
# `pnpm-lock.yaml` change.
{
  # The tree being built: package.nix's `workingTree`.
  root,
  lib,
  stdenv,
  fetchPnpmDeps,
  pnpmConfigHook,
  pnpm_11,
  nodejs,
  deno,
  flavor,
  # Passed down from package.nix, which resolves it once — the app manifest's,
  # or the published version a tag carries. Not re-read here: one parse, one
  # source. See the nix-flake-overlay design, D9.
  version,
  # Likewise from package.nix, which reads it from `hashes.json`. Refresh it
  # with `just nix-pins` after any `pnpm-lock.yaml` change; never by hand.
  pnpmDepsHash,
}:
let
  # Path arithmetic, not interpolation — see the note in package.nix.
  inherit ((fromTOML (builtins.readFile (root + "/src-tauri/res/configs/flavors/${flavor}.toml"))))
    identifier
    ;

  # Everything pnpm needs to resolve the workspace — and nothing that would
  # vary per flavor or per app-code edit.
  manifests = lib.fileset.unions [
    (root + "/package.json")
    (root + "/pnpm-lock.yaml")
    (root + "/pnpm-workspace.yaml")
    # `patchedDependencies` lives beside the vendored Rust crates; only the
    # `.patch` files concern pnpm.
    (lib.fileset.fileFilter (file: file.hasExt "patch") (root + "/tooling/patches"))
    (root + "/src/package.json")
    (root + "/website/package.json")
    (root + "/crates/domain/package.json")
    (root + "/crates/cartograph/package.json")
    (root + "/tooling/icongen/package.json")
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
  inherit version;

  src =
    with lib.fileset;
    toSource {
      inherit root;
      fileset = unions [
        manifests
        # Workspace packages the app imports: plain checked-in TS exports.
        (root + "/crates/domain/typescript")
        (root + "/crates/cartograph/typescript")
        (difference (root + "/src") (unions [
          (maybeMissing (root + "/src/node_modules"))
          (maybeMissing (root + "/src/.nuxt"))
          (maybeMissing (root + "/src/.output"))
        ]))
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
    hash = pnpmDepsHash;
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
