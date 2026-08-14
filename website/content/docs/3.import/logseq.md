---
title: Importing from Logseq
description: The guarantee, the mechanism, and the failure policy for bringing a Logseq markdown graph into Waypoints — including exactly what survives and what doesn't.
---

<!--
  Genre: CONTRACT DOC — guarantee → mechanism → failure policy → verification.
  Register: SQLite-testing. Numbers as swagger; hedges disclosed; every claim links code or tests.
  Skeleton status: structure + known facts filled; ◆ marks the questions only you can answer.
  The homepage card's "re-running never duplicates" claim is NOT shippable until every ◆ is resolved.
-->

# Importing from Logseq

_Tested against Logseq v◆ (markdown graphs). Last verified: ◆ date. The DB version of Logseq is unsupported — not-yet: upstream hasn't shipped a stable release to target._

## The guarantee

Point Waypoints at your Logseq directory. The importer is **idempotent**: running it twice — or twenty times, or after pulling new changes into the same graph — never creates duplicates. Import is keyed on block identity, so re-import updates what changed and leaves the rest alone.

◆ Confirm the re-import semantics precisely: update-in-place for changed blocks / skip unchanged / what happens to blocks _deleted_ in Logseq since the last run (tombstone? orphan? leave in Waypoints?). The guarantee is only as strong as this paragraph is specific.

## The mechanism (why this works)

Logseq persists a block's `id::` property only when the block has been referenced or embedded somewhere. Every other block has a runtime UUID that never touches the markdown. Any importer claiming idempotency has to answer: what is the stable identity of a block that has no persisted id?

◆ Waypoints' answer, in one paragraph: (deterministic derivation from content + position? hashing scheme? synthesized-and-written-back ids? cite `crates/importer/src/...` with a permalink).

For blocks _with_ persisted ids: the `id::` value becomes the Mark's identity directly, which is what makes cross-file block references and embeds survive the trip intact.

◆ Verify that sentence against the implementation before shipping it.

## Collision policy

Copy-pasting a block in Logseq _with_ its `id::` property produces two blocks claiming one identity. On import, Waypoints ◆ (first-wins / last-wins / both-imported-with-one-remapped / hard error with a report). The rule is deliberate, documented here, and enforced by ◆ (test permalink).

## What survives

<!-- This table IS the "everything native is supported" claim, enumerated. The homepage
     and any forum reply must point here instead of saying "everything." -->

| Logseq construct            | Survives? | Notes                                                   |
| --------------------------- | --------- | ------------------------------------------------------- |
| Blocks + nesting            | ◆         |                                                         |
| Block references `((uuid))` | ◆         |                                                         |
| Block embeds                | ◆         |                                                         |
| Page links `[[...]]`        | ◆         |                                                         |
| Page + block properties     | ◆         | → Readings on which Chart?                              |
| Journals                    | ◆         | dated Landmarks?                                        |
| Tags                        | ◆         |                                                         |
| Assets (images, PDFs)       | ◆         | content-addressed on import?                            |
| Namespaces `a/b/c`          | ◆         |                                                         |
| Queries (simple + advanced) | ◆         | likely: imported as text, not executed — say so plainly |
| Flashcards                  | ◆         |                                                         |
| Whiteboards                 | ◆         |                                                         |
| Org-mode files              | ◆         | can't / won't / not-yet                                 |

Rows marked **won't** get one honest sentence of rationale each — "won't" with a reason converts better than a silent gap.

## What to expect when you run it

◆ Two short paragraphs: where the command/button lives, how long a graph of N pages takes (measure your own graph and print the number — "my 4,300-block graph imports in Xs on a 2021 laptop" is the register), and what the summary report shows.

## Verification

The idempotency guarantee is enforced by tests, not intentions: ◆ permalink to the importer test module (the double-import test, the collision test, the re-import-after-edit test). If these break, the build breaks.

## Found an edge case?

Open a [GitHub Discussion](https://github.com/waypoints-hq/waypoints/discussions) with a minimal reproducing snippet. Answered edge cases graduate into this document — you'll be footnoted.
<!-- Discussions over email/Discord: searchable, compounds into docs, never answered twice. -->
