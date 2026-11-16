# SPDX-FileCopyrightText: 2026 John Muchovej
# SPDX-License-Identifier: AGPL-3.0-or-later

# The git pins a tree's `Cargo.lock` carries, as cargo locks one:
# `git+<url>?tag=<tag>#<rev>` for a tag, `?branch=` or `?rev=` otherwise. On
# a series every library group is pinned to a tag of this repository
# (library-minting design D7); on `main` nothing is, and `pinOf` answers null.
#
# Builtins only, read by path arithmetic, so the overlay and the recipes can
# call it before they have a nixpkgs and without copying the tree.
tree:
let
  lock = fromTOML (builtins.readFile (tree + "/Cargo.lock"));

  parse =
    package:
    let
      parts = builtins.match "git\\+([^?#]+)(\\?([^#]*))?#([0-9a-f]+)" (package.source or "");
      query = if parts == null then null else builtins.elemAt parts 2;
      tag = if query == null then null else builtins.match "tag=(.*)" query;
    in
    if parts == null then
      null
    else
      {
        inherit (package) name;
        url = builtins.elemAt parts 0;
        rev = builtins.elemAt parts 3;
        # Null where the pin names a branch or a bare revision.
        tag = if tag == null then null else builtins.head tag;
      };

  pins = builtins.filter (pin: pin != null) (map parse lock.package);
in
{
  # The git pin of the package called `name` — `{ name, url, tag, rev }`,
  # `tag` null unless the pin names one — or null where the lockfile carries
  # none: an in-tree crate, or a registry crate of the same name.
  pinOf =
    name:
    let
      found = builtins.filter (pin: pin.name == name) pins;
    in
    if found == [ ] then null else builtins.head found;
}
