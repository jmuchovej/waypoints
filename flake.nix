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

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake.overlays.default = import ./nix/overlay.nix { };

      perSystem =
        { pkgs, ... }:
        let
          # build.rs falls back to "unknown" on dirty/unknown trees.
          rev = inputs.self.rev or inputs.self.dirtyRev or null;
          overlaid = pkgs.extend (import ./nix/overlay.nix { inherit rev; });
          # `default` is the channel this tree's version names; on main, where
          # every channel is a recorded release, it is stable.
          inherit (import ./nix/channel.nix ./.) channel;
          default = if channel == "nightly" then "stable" else channel;
        in
        {
          packages = {
            inherit (overlaid.waypoints) stable preview nightly;
            default = overlaid.waypoints.${default};
          };

          # The recipe applied to this very tree under the development flavor,
          # which no channel ships: exercised on every ref, presented as no
          # release.
          checks.waypoints-in-tree = pkgs.callPackage ./nix/packages/waypoints/package.nix {
            inherit rev;
            workingTree = ./.;
            flavor = "development";
          };
        };
    };
}
