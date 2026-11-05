# `outpost` covenant fixtures

Frozen envelope and ACL records, owned by the `outpost` release group.

Everything here but this README is append-only. Add new files freely. A pull
request that modifies, deletes, or renames one fails `version-integrity` unless
its description carries the `Covenant: break` trailer, because changing a fixture
changes what old data means. The formatter skips this directory, and the owning
crate's tests read these files.
