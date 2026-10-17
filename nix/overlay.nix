# Pure, flake-independent overlay: every dependency resolves through
# `final`/`prev`, so consumers apply it to their own nixpkgs with no extra
# inputs. Importable without flakes: `import ./nix/overlay.nix {}`.
#
# One attribute set, `waypoints`, with a package per channel. What each one
# builds is decided by the channel this tree's manifest version names (see
# ./channel.nix):
#
#   nightly (main)   every channel is the release its index entry records
#                    (nix/releases/), fetched by rev and built with that
#                    tree's own recipe — a latest-tracking target
#   preview, stable  that channel is this very tree, built with the recipe
#                    beside this file; the other two fail, saying where to
#                    pin — a series branch or tag is its own release
#
# `rustToolchain` is the escape hatch for consumers who want to pin an exact
# toolchain (e.g. via rust-overlay); by default packages build with the
# consumer's `rustPlatform`. `rev` is this tree's own commit, stamped into a
# self-build; the flake passes it.
{
  rustToolchain ? null,
  rev ? null,
}:
final: _prev:
let
  inherit (final) lib;

  tree = ../.;
  recipe = root: root + "/nix/packages/waypoints/package.nix";
  this = import ./channel.nix tree;

  # The recorded release of `channel`, built with the recipe its own tree
  # carries. The fetch happens at evaluation, through the github fetcher a
  # flake ref resolves with (so a private repository needs only the
  # access-tokens a consumer already has); its store path is turned back
  # into a path value — the context dropped carries no derivation, a fetched
  # source is already in the store.
  recorded =
    channel:
    let
      entry = import ./releases { inherit channel; };
      fetched = fetchTree {
        type = "github";
        owner = "jmuchovej";
        repo = "waypoints";
        inherit (entry) rev;
        narHash = entry.hash;
      };
      source = /. + builtins.unsafeDiscardStringContext fetched.outPath;
    in
    final.callPackage (recipe source) {
      inherit rustToolchain;
      flavor = channel;
      workingTree = source;
      inherit (entry) rev version;
    };

  # This tree, as the channel it names.
  own =
    channel:
    final.callPackage (recipe tree) {
      inherit rustToolchain rev;
      flavor = channel;
      workingTree = tree;
    };

  unavailable =
    channel:
    throw (
      if channel == "nightly" then
        ''
          waypoints.nightly: nightlies publish from main only; this ref is the ${this.series} ${this.channel} series (${this.version}).
          Pin main for the latest nightly.
        ''
      else if this.channel == "preview" then
        ''
          waypoints.stable: ${this.series} has no stable release yet; this ref is its preview series (${this.version}).
          Pin main for the latest stable, or this branch once it is promoted.
        ''
      else
        ''
          waypoints.preview: ${this.series} ships no further previews; this ref is its stable series (${this.version}).
          Pin main for the latest preview, or the next series.
        ''
    );

  package =
    channel:
    if this.channel == "nightly" then
      recorded channel
    else if this.channel == channel then
      own channel
    else
      unavailable channel;
in
{
  waypoints = lib.recurseIntoAttrs {
    stable = package "stable";
    preview = package "preview";
    nightly = package "nightly";
  };
}
