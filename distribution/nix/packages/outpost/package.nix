# SPDX-FileCopyrightText: 2026 John Muchovej
# SPDX-License-Identifier: AGPL-3.0-or-later

# The `outpost` command-line binary: a Station over QUIC — storage, and the
# fleet orderer when handed a fleet key — built from the tree with the `cli`
# feature. Headless: none of the desktop app's GUI toolkit, JavaScript
# runtime, or frontend enters its closure. The version is the crate
# manifest's, which is kept parseable by `builtins.fromTOML` (strict TOML
# 1.0; see the nix-flake-overlay design, D10).
#
# Channel-less: outpost publishes no release of its own yet. On `main` the
# overlay applies this recipe to its own tree; on a series, where the
# workspace pins `outpost` to a tag, it applies the tag's copy of this file
# to the tag's tree (library-minting design D7). This argument list is
# therefore what a newer overlay calls on an older tag — add arguments with
# defaults, never rename or remove one.
{
  lib,
  callPackage,
  # The tree to build. A real Nix path, not a string: `lib.fileset` rejects
  # string-encoded paths outright. (Not `root`: nixpkgs has a package by that
  # name, and `callPackage` would fill it in.)
  workingTree,
  # Escape hatch (see distribution/nix/overlay.nix): a rustc/cargo-bearing toolchain drv,
  # e.g. from rust-overlay. Default: the consumer nixpkgs' rustPlatform.
  rustToolchain ? null,
  # The crane library to build with. Default: the one the tree's `flake.lock`
  # pins — see ../crane.nix.
  craneLib ? null,
}:
let
  # Files read at evaluation are reached by path arithmetic, never by
  # interpolating `workingTree` into a string: that coerces the path to a
  # store path, which copies the whole tree — a working copy's `target/`
  # included — before a single byte is read.
  manifest = fromTOML (builtins.readFile (workingTree + "/crates/outpost/Cargo.toml"));

  crane = callPackage ../crane.nix { inherit workingTree craneLib rustToolchain; };
in
crane.buildPackage {
  pname = "outpost";
  inherit (manifest.package) version;

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
        (workingTree + "/src-tauri/Cargo.toml")
        (workingTree + "/src-tauri/build.rs")
        (workingTree + "/src-tauri/src")
      ];
    };

  # `--locked` holds cargo to the lockfile crane vendored; the rest names the
  # one package and the Station's command line.
  cargoExtraArgs = "--locked --package outpost --features cli";

  # No dependency-only pass — see ../crane.nix.
  cargoArtifacts = null;

  # The suite runs under cargo in CI; the sandbox build is for the artifact.
  doCheck = false;

  meta = {
    description = "Waypoints sync Station — the outpost command-line binary";
    homepage = "https://waypoints.so";
    license = lib.licenses.agpl3Plus;
    platforms = [
      "aarch64-darwin"
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "outpost";
  };
}
