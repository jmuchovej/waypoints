# The `outpost` operator binary: a Station over QUIC — storage, and the
# fleet orderer when handed a fleet key — built from the tree with the `cli`
# feature. Headless: none of the desktop app's GUI toolkit, JavaScript
# runtime, or frontend enters its closure. The version is the crate
# manifest's, which is kept parseable by `builtins.fromTOML` (strict TOML
# 1.0; see the nix-flake-overlay design, D10).
{
  root,
  lib,
  rustPlatform,
  makeRustPlatform,
  # Escape hatch (see nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
}:
let
  manifest = fromTOML (builtins.readFile (root + "/crates/outpost/Cargo.toml"));

  # The git-sourced cargo crates' hashes belong to the workspace lockfile, not
  # to a package, so they are read from the one pins file `nix-pins` keeps
  # (see the header of `tooling/bin/nix-pins`).
  pins = lib.importJSON ../waypoints/hashes.json;

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
  pname = "outpost";
  version = manifest.package.version;

  # Cargo must be able to read every workspace member to resolve the
  # workspace, so the desktop crate's manifest and sources ride along; only
  # `outpost` is built, so none of the desktop's inputs are.
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
        (root + "/src-tauri/Cargo.toml")
        (root + "/src-tauri/build.rs")
        (root + "/src-tauri/src")
      ];
    };

  cargoLock = {
    lockFile = root + "/Cargo.lock";
    outputHashes = pins.cargoOutputHashes;
  };

  cargoBuildFlags = [
    "--package"
    "outpost"
  ];
  # `buildFeatures`, not `cargoBuildFeatures`: buildRustPackage computes the
  # `cargo*` attrs from these and silently overwrites a direct setting.
  buildFeatures = [ "cli" ];

  # The suite runs under cargo in CI; the sandbox build is for the artifact.
  doCheck = false;

  meta = {
    description = "Waypoints sync Station — the outpost operator CLI";
    homepage = "https://waypoints.so";
    platforms = [
      "aarch64-darwin"
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "outpost";
  };
})
