# The channel a tree ships as, read from its app manifest's version — the
# one reading shared by the overlay and the flake, and the same one the
# release planner applies to tags (`release_plan::channel_of`):
#
#   X.Y.Z-nightly…   nightly   (main)
#   X.Y.Z-<other>…   preview   (a series branch until promoted)
#   X.Y.Z            stable    (a series branch once promoted)
#
# Builtins only, so a flake can call it before it has a nixpkgs. Returns the
# channel and the series the version belongs to (`vX.Y.x`), which is what
# the overlay's messages name.
tree:
let
  version = (fromTOML (builtins.readFile (tree + "/src-tauri/Cargo.toml"))).package.version;

  # `1.2.3-pre.4+build` → "pre"; `1.2.3` → "".
  withoutBuild = builtins.head (builtins.split "\\+" version);
  prerelease = builtins.elemAt (builtins.match "[^-]*(-([^.]*).*)?" withoutBuild) 1;
  identifier = if prerelease == null then "" else prerelease;

  numbers = builtins.match "([0-9]+)\\.([0-9]+)\\..*" version;
in
{
  inherit version;
  channel =
    if identifier == "" then
      "stable"
    else if identifier == "nightly" then
      "nightly"
    else
      "preview";
  series = "v${builtins.elemAt numbers 0}.${builtins.elemAt numbers 1}.x";
}
