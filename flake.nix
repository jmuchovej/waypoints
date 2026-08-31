# Packaging-only flake (see openspec distribution/nix-overlay): exposes the
# desktop app through nix/overlay.nix. The dev shell stays on devenv.yaml —
# `nix develop` is not a supported entry point, and this flake's inputs are
# deliberately independent of devenv's lock.
{
  description = "Waypoints — flake overlay for the desktop app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake.overlays.default = import ./nix/overlay.nix {};

      perSystem = _: {
        # `packages`/`checks` are wired in task 6.4 once the derivations exist.
      };
    };
}
