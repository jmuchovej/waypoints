# SPDX-FileCopyrightText: 2026 John Muchovej
# SPDX-License-Identifier: AGPL-3.0-or-later

# The crane library a recipe builds with, resolved for the tree it builds:
# the caller's `craneLib` when given, else crane as that tree's `flake.lock`
# pins it. Read from the tree rather than taken from the overlay, so a tag's
# recipe builds with the crane it was written for, and the overlay stays
# flake-independent with no crane of its own.
#
# crane vendors a lockfile's registry crates by their checksums and its git
# sources by their locked revisions, with no hash per pin: a series'
# `Cargo.lock` pins each library group's tag, and the sandbox builds offline
# from those checkouts (library-minting design D7).
#
# Neither recipe runs crane's dependency-only pass (`cargoArtifacts = null`):
# it dummifies the sources by re-reading every manifest with `fromTOML`,
# which is strict TOML 1.0, and the crate manifests are TOML 1.1 — only the
# root and app manifests are held to 1.0, for the readers nix has (the
# nix-flake-overlay design, D10). The vendoring is the point; the cache of
# compiled dependencies is not.
#
# `rustToolchain` is the recipes' escape hatch — a rustc/cargo-bearing
# toolchain drv in place of the consumer's rustPlatform — applied here.
{
  lib,
  callPackage,
  workingTree,
  craneLib ? null,
  rustToolchain ? null,
}:
let
  # Path arithmetic, never interpolation: a string of `workingTree` would
  # copy the whole tree into the store — see the recipes. The fetched source
  # is turned back into a path value as the overlay does for a release: the
  # context dropped carries no derivation, a fetched tree is already in the
  # store.
  locked = (lib.importJSON (workingTree + "/flake.lock")).nodes.crane.locked;
  source = /. + builtins.unsafeDiscardStringContext (fetchTree locked).outPath;
  pinned = callPackage (source + "/lib") { };
  base = if craneLib != null then craneLib else pinned;
in
if rustToolchain != null then base.overrideToolchain rustToolchain else base
