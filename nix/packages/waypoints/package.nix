# The desktop app, one flavor per package (stable/preview/nightly — never
# development). Reuses the existing pipeline's config layering
# (platform toml + flavor toml) via nixpkgs' cargo-tauri.hook, which builds
# the bare platform bundle: `.app` into $out/Applications on darwin,
# deb-unpacked bin/.desktop/icons into $out on Linux. dmg/nsis/appimage stay
# with the release pipeline.
#
# The recipe for the tree it lives in. `nix/overlay.nix` applies it to its
# own tree on a series ref, and to a fetched release's tree on main — by
# evaluating that tree's copy of this file. The argument list is therefore
# an interface a newer overlay calls on an older tag: add arguments with
# defaults, never rename or remove one.
#
# Nix builds compile without the `self-update` feature: store paths cannot
# rewrite themselves, so the updater notifies and never installs.
{
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
  # The tree to build. A real Nix path, not a string: `lib.fileset` rejects
  # string-encoded paths outright. (Not `root`: nixpkgs has a package by that
  # name, and `callPackage` would fill it in.)
  workingTree,
  # Escape hatch (see nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
  # Commit stamped into build metadata (the flake passes self.rev/dirtyRev,
  # the overlay a release's recorded rev; build.rs falls back to "unknown"
  # when absent).
  rev ? null,
  # The published version, where the tag knows more than the manifest: a
  # nightly's tag carries the counter and date stamp its manifest never does.
  # Must extend the manifest's version. Null means the manifest's.
  version ? null,
  # Replaces the flavor's updater endpoints, for exercising the notify-only
  # update path against a controlled feed without editing a tracked config:
  #   (pkgs.waypoints.stable.override { updaterEndpoints = [ "https://…/test.json" ]; })
  # Leave null for real builds — a package that ships pointing at a test feed
  # would never see genuine releases.
  updaterEndpoints ? null,
}:
let
  inherit (stdenv.hostPlatform) isLinux;
  inherit (lib) optionals;

  # Files read at evaluation are reached by path arithmetic, never by
  # interpolating `workingTree` into a string: that coerces the path to a
  # store path, which copies the whole tree — a working copy's `target/`
  # included — before a single byte is read.
  read = file: builtins.readFile (workingTree + file);

  # Channel identity (product name, identifier, icons, update endpoint) — never
  # the version: a Tauri config `version` would override the manifest's.
  flavor-config = fromTOML (read "/src-tauri/res/configs/flavors/${flavor}.toml");

  # The single source of the version, tracking the branch this tree sits on.
  # Kept parseable by `builtins.fromTOML` (strict TOML 1.0) — see the
  # nix-flake-overlay design, D10.
  manifest-version = (fromTOML (read "/src-tauri/Cargo.toml")).package.version;

  # A publish-time suffix extends the manifest's version; it never replaces
  # it: `0.3.0-nightly` becomes `0.3.0-nightly.7+2026.08.29`, never anything
  # whose stem differs. A recorded tag that disagrees with the manifest it
  # points at is a wrong entry, and fails here rather than building under a
  # false name.
  app-version =
    if version == null then
      manifest-version
    else
      let
        suffix = lib.removePrefix manifest-version version;
        extends = builtins.any (sep: lib.hasPrefix sep suffix) [
          "."
          "-"
          "+"
        ];
      in
      assert lib.assertMsg (lib.hasPrefix manifest-version version && (suffix == "" || extends))
        "waypoints.${flavor}: version ${version} does not extend the manifest version ${manifest-version}";
      version;

  platform = if stdenv.hostPlatform.isDarwin then "macos" else "linux";

  # Hashes nix cannot derive from the tree: the pnpm dependency closure and the
  # git-sourced cargo crates. They live in data rather than in these
  # expressions so `nix-pins` refreshes one by writing a JSON key — see the
  # header of `tooling/bin/nix-pins`.
  pins = lib.importJSON ./hashes.json;

  frontend = callPackage ./_frontend.nix {
    inherit flavor;
    root = workingTree;
    version = app-version;
    pnpmDepsHash = pins.pnpmDeps;
  };

  # Neutralize the dev-pipeline hooks: the frontend is prebuilt (no
  # beforeBuildCommand) and staged into the build tree — a relative path,
  # because tauri serializes its merged config (frontendDist included) into
  # the binary, and an absolute store path there would drag the standalone
  # site into the runtime closure alongside the embedded copy. No updater
  # artifacts: they exist for the self-updating bundler pipeline and demand
  # the signing key, which the sandbox rightly lacks.
  #
  # A published version also travels here, the one place a Tauri config
  # `version` is wanted: the manifest says the bare target, and the bundle
  # must report what was published.
  build-config = writeText "waypoints-${flavor}-build.json" (
    builtins.toJSON (
      {
        build = {
          beforeBuildCommand = "";
          frontendDist = "./frontend-dist";
        };
        bundle.createUpdaterArtifacts = false;
      }
      // lib.optionalAttrs (version != null) { version = app-version; }
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
      root = workingTree;
      fileset = unions [
        (workingTree + "/Cargo.toml")
        (workingTree + "/Cargo.lock")
        (workingTree + "/crates")
        (workingTree + "/tooling")
        (workingTree + "/vendor")
        # Desktop build only — the generated mobile projects stay out.
        (difference (workingTree + "/src-tauri") (workingTree + "/src-tauri/gen"))
      ];
    };

  cargoLock = {
    lockFile = workingTree + "/Cargo.lock";
    outputHashes = pins.cargoOutputHashes;
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
    "res/configs/platforms/${platform}.toml"
    "--config"
    "res/configs/flavors/${flavor}.toml"
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

  # Exposed so the pnpm dependency closure is addressable on its own
  # (`.frontend.pnpmDeps`). `nix-pins` builds that attribute rather than the
  # whole app, so refreshing its hash costs one fetch.
  passthru = { inherit frontend; };

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
