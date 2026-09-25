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
  flavor,
  # Escape hatch (see nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
  # Commit stamped into build metadata (flake passes self.rev/dirtyRev;
  # build.rs falls back to "unknown" when absent).
  rev ? null,
}:
let
  flavor-config = fromTOML (builtins.readFile ../../../src-tauri/configs/flavors/${flavor}.toml);

  platform = if stdenv.hostPlatform.isDarwin then "macos" else "linux";

  frontend = callPackage ./_frontend.nix { inherit flavor; };

  # Neutralize the dev-pipeline hooks: the frontend is prebuilt (no
  # beforeBuildCommand) and staged into the build tree — a relative path,
  # because tauri serializes its merged config (frontendDist included) into
  # the binary, and an absolute store path there would drag the standalone
  # site into the runtime closure alongside the embedded copy. No updater
  # artifacts: they exist for the self-updating bundler pipeline and demand
  # the signing key, which the sandbox rightly lacks.
  build-config = writeText "waypoints-${flavor}-build.json" (
    builtins.toJSON {
      build = {
        beforeBuildCommand = "";
        frontendDist = "./frontend-dist";
      };
      bundle.createUpdaterArtifacts = false;
    }
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
  inherit (flavor-config) version;

  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ../../../Cargo.toml
      ../../../Cargo.lock
      ../../../crates
      ../../../tooling
      ../../../vendor
      # Desktop build only — the generated mobile projects stay out.
      (lib.fileset.difference ../../../src-tauri ../../../src-tauri/gen)
    ];
  };

  cargoLock = {
    lockFile = ../../../Cargo.lock;
    outputHashes = {
      "gh-workflow-0.9.0" = "sha256-CMzElgOmzseY3JOw7g2Q9uDGrS3usCzzdQt/nVNcn+E=";
    };
  };

  buildAndTestSubdir = "src-tauri";

  # Flavors are additive cargo features over `desktop`; `self-update` (a
  # default feature) is deliberately absent.
  cargoBuildNoDefaultFeatures = true;
  cargoBuildFeatures = [
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
  ++ lib.optionals stdenv.hostPlatform.isLinux [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = lib.optionals stdenv.hostPlatform.isLinux [
    glib-networking
    webkitgtk_4_1
  ];

  # Stage the prebuilt frontend where the relative frontendDist expects it
  # (preBuild runs at the workspace root, before the src-tauri pushd).
  preBuild = ''
    cp -r ${frontend} src-tauri/frontend-dist
  '';

  env = lib.optionalAttrs (rev != null) {
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
