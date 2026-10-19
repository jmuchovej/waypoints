# The only reader of the release index. One file per channel sits beside this
# expression — `stable.json`, `preview.json`, `nightly.json` — each the record
# of that channel's most recent publish:
#
#   {
#     "tag": "v0.2.0-nightly.7+2026.08.29",
#     "rev": "<the commit the tag points at>",
#     "hash": "<SRI NAR hash of the fetched tree at that rev>"
#   }
#
# The tag is the sole source of the version: `version` here is the tag minus
# its `v`, and the file carries no version field of its own. Nothing else is
# recorded — the tree at `rev` carries its own recipe and dependency pins. A
# channel that has not published under the index holds `{}`, and asking for
# it is an evaluation error rather than a fallback: a build standing in for
# a release would misrepresent it.
#
# Written by `cargo xtask release-index` from CI once a tag exists. Editing a
# file by hand is a repair, not an interface.
{ channel }:
let
  file = ./. + "/${channel}.json";
  entry = if builtins.pathExists file then builtins.fromJSON (builtins.readFile file) else { };

  fields = [
    "tag"
    "rev"
    "hash"
  ];
  recorded = builtins.all (field: entry ? ${field} && entry.${field} != "") fields;

  inherit (entry) tag;
  version = builtins.substring 1 (builtins.stringLength tag) tag;
in
if !recorded then
  throw ''
    waypoints.${channel}: no release is recorded for the ${channel} channel.

    nix/releases/${channel}.json holds no entry: ${channel} has not published
    under the release index yet. Pin a ref of main whose index records a
    ${channel} release, or wait for the channel's next publish.
  ''
else if builtins.substring 0 1 tag != "v" then
  throw "nix/releases/${channel}.json: tag `${tag}` does not start with `v`"
else
  entry // { inherit version; }
