# Throwaway spike (task 1.3): pnpm_11 FOD viability against pnpm-lock.yaml.
# Build with:
#   nix build --impure --expr \
#     '(import ./nix/packages/waypoints/_spike-fod.nix { pkgs = import (builtins.getFlake "github:NixOS/nixpkgs/nixos-unstable") {}; })'
# Deleted once the result is recorded in design.md.
{pkgs}: let
  inherit (pkgs) lib;
  src = lib.fileset.toSource {
    root = ../../..;
    fileset = lib.fileset.unions [
      ../../../package.json
      ../../../pnpm-lock.yaml
      ../../../pnpm-workspace.yaml
      ../../../patches
      ../../../src/package.json
      ../../../website/package.json
      ../../../crates/domain/package.json
      ../../../crates/cartograph/package.json
      ../../../tooling/icongen/package.json
    ];
  };
in
  pkgs.stdenv.mkDerivation (finalAttrs: {
    pname = "waypoints-pnpm-fod-spike";
    version = "0.0.0";
    inherit src;

    nativeBuildInputs = [
      pkgs.nodejs
      pkgs.pnpm_11
      pkgs.pnpmConfigHook
    ];

    pnpmDeps = pkgs.fetchPnpmDeps {
      inherit (finalAttrs) pname version src;
      pnpm = pkgs.pnpm_11;
      fetcherVersion = 4;
      hash = lib.fakeHash;
    };

    # configHook already ran `pnpm install --offline --frozen-lockfile`;
    # the build just proves node_modules materialized.
    buildPhase = ''
      test -d node_modules/.pnpm || (echo "no node_modules" && exit 1)
    '';
    installPhase = ''
      ls node_modules > $out
    '';
  })
