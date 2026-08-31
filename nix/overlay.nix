# Pure, flake-independent overlay: every dependency resolves through
# `final`/`prev`, so consumers apply it to their own nixpkgs with no extra
# inputs. Importable without flakes: `import ./nix/overlay.nix {}`.
#
# `rustToolchain` is the escape hatch for consumers who want to pin an exact
# toolchain (e.g. via rust-overlay); by default packages build with the
# consumer's `rustPlatform`.
{rustToolchain ? null}: final: _prev: let
  waypoints-flavor = flavor:
    final.callPackage ./packages/waypoints/package.nix {
      inherit flavor rustToolchain;
    };
in {
  waypoints = waypoints-flavor "stable";
  waypoints-preview = waypoints-flavor "preview";
  waypoints-nightly = waypoints-flavor "nightly";
}
