# The `outpost` command-line binary: a Station over QUIC — storage, and the
# fleet orderer when handed a fleet key — built from the tree with the `cli`
# feature. Headless: none of the desktop app's GUI toolkit, JavaScript
# runtime, or frontend enters its closure. The version is the crate
# manifest's, which is kept parseable by `builtins.fromTOML` (strict TOML
# 1.0; see the nix-flake-overlay design, D10).
#
# Unlike the app, this has no channels: outpost publishes no release of its
# own yet, so the overlay always applies this recipe to its own tree. When
# outpost gains a release record, it joins the fetch-by-rev path the app
# uses and this argument list is what a newer overlay would call on an older
# tag — so add arguments with defaults, never rename or remove one.
{
  lib,
  rustPlatform,
  makeRustPlatform,
  # The tree to build. A real Nix path, not a string: `lib.fileset` rejects
  # string-encoded paths outright. (Not `root`: nixpkgs has a package by that
  # name, and `callPackage` would fill it in.)
  workingTree,
  # Escape hatch (see nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
}:
let
  # Files read at evaluation are reached by path arithmetic, never by
  # interpolating `workingTree` into a string: that coerces the path to a
  # store path, which copies the whole tree — a working copy's `target/`
  # included — before a single byte is read.
  manifest = fromTOML (builtins.readFile (workingTree + "/crates/outpost/Cargo.toml"));

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
      root = workingTree;
      fileset = unions [
        (workingTree + "/Cargo.toml")
        (workingTree + "/Cargo.lock")
        (workingTree + "/crates")
        (workingTree + "/tooling")
        (workingTree + "/vendor")
        (workingTree + "/src-tauri/Cargo.toml")
        (workingTree + "/src-tauri/build.rs")
        (workingTree + "/src-tauri/src")
      ];
    };

  cargoLock = {
    lockFile = workingTree + "/Cargo.lock";
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
    description = "Waypoints sync Station — the outpost command-line binary";
    homepage = "https://waypoints.so";
    platforms = [
      "aarch64-darwin"
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "outpost";
  };
})
