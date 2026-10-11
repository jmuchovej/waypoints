# The desktop app, one flavor per package (stable/preview/nightly — never
# development). Reuses the existing pipeline's config layering
# (platform toml + flavor toml) via nixpkgs' cargo-tauri.hook, which builds
# the bare platform bundle: `.app` into $out/Applications on darwin,
# deb-unpacked bin/.desktop/icons into $out on Linux. dmg/nsis/appimage stay
# with the release pipeline.
#
# Nix builds compile without the `self-update` feature: store paths cannot
# rewrite themselves, so the updater notifies and never installs.
{
  root,
  lib,
  stdenv,
  rustPlatform,
  makeRustPlatform,
  callPackage,
  cargo-tauri,
  writeText,
  pkg-config,
  wrapGAppsHook4,
  glib-networking,
  webkitgtk_4_1,
  libayatana-appindicator,
  flavor,
  # Escape hatch (see nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
  # Commit stamped into build metadata (flake passes self.rev/dirtyRev;
  # build.rs falls back to "unknown" when absent).
  rev ? null,
  # Replaces the flavor's updater endpoints, for exercising the notify-only
  # update path against a controlled feed without editing a tracked config:
  #   (pkgs.waypoints.override { updaterEndpoints = [ "https://…/test.json" ]; })
  # Leave null for real builds — a package that ships pointing at a test feed
  # would never see genuine releases.
  updaterEndpoints ? null,
}:
let
  inherit (stdenv.hostPlatform) isLinux;
  inherit (lib) optionals;

  # Channel identity (product name, identifier, icons, update endpoint) — never
  # the version: a Tauri config `version` would override the manifest's.
  flavor-config = fromTOML (builtins.readFile "${root}/src-tauri/configs/flavors/${flavor}.toml");

  # The single source of the version, tracking the branch this tree sits on.
  # Kept parseable by `builtins.fromTOML` (strict TOML 1.0) — see design.md D10.
  app-version = (fromTOML (builtins.readFile "${root}/src-tauri/Cargo.toml")).package.version;

  platform = if stdenv.hostPlatform.isDarwin then "macos" else "linux";

  frontend = callPackage ./_frontend.nix {
    inherit flavor root;
    version = app-version;
  };

  # Neutralize the dev-pipeline hooks: the frontend is prebuilt (no
  # beforeBuildCommand) and staged into the build tree — a relative path,
  # because tauri serializes its merged config (frontendDist included) into
  # the binary, and an absolute store path there would drag the standalone
  # site into the runtime closure alongside the embedded copy. No updater
  # artifacts: they exist for the self-updating bundler pipeline and demand
  # the signing key, which the sandbox rightly lacks.
  build-config = writeText "waypoints-${flavor}-build.json" (
    builtins.toJSON (
      {
        build = {
          beforeBuildCommand = "";
          frontendDist = "./frontend-dist";
        };
        bundle.createUpdaterArtifacts = false;
      }
      // lib.optionalAttrs (updaterEndpoints != null) {
        plugins.updater.endpoints = updaterEndpoints;
      }
    )
  );

  rust-platform =
    if rustToolchain != null then
      makeRustPlatform {
        cargo = rustToolchain;
        rustc = rustToolchain;
      }
    else
      rustPlatform;
in
rust-platform.buildRustPackage (_finalAttrs: {
  pname = if flavor == "stable" then "waypoints" else "waypoints-${flavor}";
  version = app-version;

  src =
    with lib.fileset;
    toSource {
      inherit root;
      fileset = unions [
        (root + "/Cargo.toml")
        (root + "/Cargo.lock")
        (root + "/crates")
        (root + "/tooling")
        (root + "/vendor")
        # Desktop build only — the generated mobile projects stay out.
        (difference (root + "/src-tauri") (root + "/src-tauri/gen"))
      ];
    };

  cargoLock = {
    lockFile = root + "/Cargo.lock";
    outputHashes = {
      "gh-workflow-0.9.0" = "sha256-CMzElgOmzseY3JOw7g2Q9uDGrS3usCzzdQt/nVNcn+E=";
    };
  };

  buildAndTestSubdir = "src-tauri";

  # Flavors are additive cargo features over `desktop`; `self-update` (a
  # default feature) is deliberately absent.
  #
  # These must be `buildFeatures`/`buildNoDefaultFeatures`, not the `cargo*`
  # spellings the build hook reads: `buildRustPackage` *computes* the `cargo*`
  # attrs from these, so setting the `cargo*` names directly is silently
  # overwritten by the defaults (`[]` and `false`) and the build quietly gets
  # every default feature — `self-update` included.
  buildNoDefaultFeatures = true;
  buildFeatures = [
    "desktop"
    flavor
  ];

  tauriBuildFlags = [
    "--config"
    "configs/platforms/${platform}.toml"
    "--config"
    "configs/flavors/${flavor}.toml"
    "--config"
    "${build-config}"
  ];

  nativeBuildInputs = [
    cargo-tauri.hook
  ]
  ++ optionals isLinux [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = optionals isLinux [
    glib-networking
    webkitgtk_4_1
    # StatusNotifierItem client library — how `builders/tray.rs` publishes its
    # tray icon on Linux, so load-bearing at runtime rather than build-only.
    # The deb bundler is just where a missing one first shows up (it panics
    # picking between the libayatana-/libappindicator3-1 `Depends:`), which
    # means a green Rust build says nothing about this being present.
    libayatana-appindicator
  ];

  # Stage the prebuilt frontend where the relative frontendDist expects it
  # (preBuild runs at the workspace root, before the src-tauri pushd).
  preBuild = ''
    cp -r ${frontend} src-tauri/frontend-dist
  '';

  # `WAYPOINTS_FLAVOR` is what build.rs reads for the channel it stamps, and
  # `src-tauri/justfile` exports the same — without it every package reports
  # itself as `development`, whatever its cargo features say. `WAYPOINTS_BUILD`
  # stays unset on purpose: it only enables a wall-clock timestamp, and the
  # store path already identifies the build.
  env = {
    WAYPOINTS_FLAVOR = flavor;
  }
  // lib.optionalAttrs (rev != null) {
    GITHUB_SHA = rev;
  };

  doCheck = false;

  meta = {
    description = "Local-first Markdown knowledge management (${flavor-config.productName})";
    homepage = "https://waypoints.so";
    platforms = [
      "aarch64-darwin"
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "waypoints";
  };
})
