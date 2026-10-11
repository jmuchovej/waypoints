# Pure, flake-independent overlay: every dependency resolves through
# `final`/`prev`, so consumers apply it to their own nixpkgs with no extra
# inputs. Importable without flakes: `import ./nix/overlay.nix {}`.
#
# `rustToolchain` is the escape hatch for consumers who want to pin an exact
# toolchain (e.g. via rust-overlay); by default packages build with the
# consumer's `rustPlatform`.
#
# `root` is the single relative path in the tree — every package derives its
# sources from it. It must be a real Nix path, not a string or a flake input
# attrset: `lib.fileset` rejects string-encoded paths outright.
{
  root ? ../.,
  rustToolchain ? null,
}:
final: _prev:
let
  waypoints-flavor =
    flavor:
    final.callPackage ./packages/waypoints/package.nix {
      inherit flavor root rustToolchain;
    };
in
{
  waypoints = waypoints-flavor "stable";
  waypoints-preview = waypoints-flavor "preview";
  waypoints-nightly = waypoints-flavor "nightly";
  # The sync server's operator binary — headless, no flavor.
  outpost = final.callPackage ./packages/outpost/package.nix { inherit root rustToolchain; };
}
