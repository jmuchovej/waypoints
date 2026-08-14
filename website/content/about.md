---
title: Why Waypoints
description: There was no single moment I decided to build instead of adapt. It was death by a thousand cuts, not being stabbed.
layout: default
navigation: false
---

<!--
  EXTRACTION NOTES (delete before deploy)
  Frame: ANNOUNCEMENT DAY. GATE-marked claims must be true before shipping.
  Markers: GATE / FACT / BRIDGE / QUARRY — same legend as index.md.
  Routing: this page targets the TINKERER — internals, opinions, disclosed flaws, receipts.
  Voice: first-person singular per 1.5. Register: Litestream / Ink & Switch (essayistic).
  Pronoun sweep applied: quarry lines dictated as "we" converted to "I"/neutral per 1.5.
-->

## The origin

I'm organizationally chaotic — not messy, but not apparently organized either — and for years my research was spread across a dozen apps that each did one thing and refused to talk to the others. The cuts added up. Most of these tools are macOS and iOS only, which means buying into the Apple ecosystem to use them everywhere; that's fine for a laptop, but iPhone hardware is sorely lacking next to Android. Logseq's development timeline was inconsistent and its mobile API non-functional. Notion wouldn't let me catalog research papers and tasks in one place, and kept everything in someone else's always-online storage. Things only does tasks. Timing only does time. Obsidian's flat markdown files are supremely overrated — markdown should be how data enters and leaves a system, not how it lives inside one.
<!-- QUARRY: 2.1, 1.1 PI answer, Graveyard db — verify firing-line wording against your database rows. -->

The thing that finally made building viable wasn't a breaking point — it was agentic programming getting good enough to act as a multiplier for one person with about forty hours per five-week sprint.

And underneath all of it, the reason tracking is a pillar and not a plugin: time-blindness means I don't know how much time things actually take. When someone asks me to estimate the duration of something, I'm making up numbers. If I underestimate, I end up highly stressed and appear over-promising even when that's not my intention. It means not knowing how much time I spend working versus with friends and family or my dog. It means missing internal deadlines with my advisor, which sets back publications and jeopardizes PhD progression. No app was going to fix that by asking me to remember to start a timer.
<!-- QUARRY: 2.3 verbatim + one BRIDGE closer. -->

## Where your data actually lives

Short answer: on your computer, in a Turso database with SQLite-incompatible features enabled.

Longer answer: your data sits in the application data directory, backed by Turso with `experimental_custom_types` and `experimental_materialized_views` turned on — check `Database.open_with` in [`crates/bedrock/src/database.rs`](https://github.com/waypoints-hq/waypoints/blob/main/crates/bedrock/src/database.rs). Whatever database viewer you use must support Turso directly; I don't check for SQLite compatibility, because egress doesn't depend on it. Turso exited beta with v0.7.0, and every release of Waypoints is checked against a vanilla `tursodb` — the experimental features can't silently lock you in.
<!-- QUARRY: 3.c + 5.2 #1, pronoun-swept. FACT: confirm the repo path renders as a valid link. -->

The exit is a first-class feature. Export is a derivative of [Comark](https://comark.dev) — an extension of CommonMark with parameterized admonitions and inline parameterization. Every Landmark becomes a markdown file keyed by its UUID, fully projected: a Mark shared across multiple Landmarks appears in each file, duplicated exactly. Assets keep content-based addressing with directory-local links you can follow by hand. Annotating PDFs and images is harder to replicate in plain text, so annotations export as companion markdown files that sit beside the asset. The `getting-started` collection in [`crates/store/collections/getting-started`](https://github.com/waypoints-hq/waypoints/tree/main/crates/store/collections/getting-started) always mirrors the export format. **Your data is yours, and the text can be read in 1980 or 2180.**
<!-- GATE: Comark export shipped + roundtrip stable. QUARRY: 3.c near-verbatim. -->

The stack: Nuxt (Vue/TypeScript) + Tauri (Rust). I push as much into Rust as is reasonable and keep TypeScript for UI and the planned extension API.
<!-- QUARRY: 5.1, pronoun-swept. -->

## The bet

Every knowledge tool makes an ontological bet. Notion requires pages to have parents and types to live inside databases. Obsidian requires everything to be a text file — which doesn't work well for time series, calendar events, or document annotation. And some things can't be plugins at all: local AI needs low-level machine-learning APIs, not a TypeScript extension sandbox, and Logseq's plugins can't run on mobile at all.

Waypoints bets on blocks in a database. Blocks are the core data structure — plain markdown or richly structured with Charts — referenceable and embeddable across the app. Flat files are incredible as an interchange format. As a _storage_ format for graphical knowledge, a database wins.
<!-- QUARRY: reply 3 + 4.3, ASR-cleaned. Dropped the SQLite 35%-faster citation per review — wrong receipt for a representational claim. -->

## How this outlives my motivation

I can't make guarantees yet — this is a problem I'm acutely aware of. The steps I'm taking: the code is GPL-3.0, so improvements are either contributed upstream or downstream derivatives function as continuations of the project. The [data covenant](/covenant) is the benchmark for cutting v1 — pre-v1 breaking changes are still possible, but they will be very loud and come with a graceful degradation path so your data is not lost; post-v1, the data schema is highly durable and development moves to surfaces like the extension API and MCP. <!-- FACT: /covenant link target must exist. -->

There isn't a second maintainer. There is a design goal that there never needs to be one for your data to survive. If you'd rather audit than wait, commit rights are earnable — things have been going well with agentic programming, but I'd like more humans working on this.
<!-- QUARRY: reply 2 + 3.e reply, pronoun-swept and tightened. -->

## How the code gets written

Nearly every diff in this repository is AI-assisted and human-reviewed. Saying "I wrote all the code" would be disingenuous — and the speed at which this project moves isn't because I'm superhuman. What you can audit instead of trusting: agent commits land under a distinct identity while my merges are signed and branch-protected, so "every merge is mine" is a repo setting, not a promise; feature-scale work starts from specs in the repo, so direction is visibly human; and CI enforces the invariants — including the vanilla-`tursodb` compatibility check and the encryption-at-rest tests that break the build if a fleet operator could read your data. The disclosure convention (`Assisted-by`) and the tiering live in CONTRIBUTING.
<!-- QUARRY: 1.5 + 5.2 #3 + gates row. GATE: review machinery public (identities, branch protection, CONTRIBUTING). -->

## The honest state

Five things I'd rather tell you than have you discover. One: rather than battle-tested SQLite, Waypoints runs on Turso, which exited beta with v0.7.0 — experimental features enabled, checked against vanilla `tursodb` every release, with the Comark export as the durable escape hatch. Two: import is a continuously evolving feature set — not every PKM is supported natively, but anything that exports raw markdown should come in, and importers for task managers and calendars are a moving goalpost. Three: self-hosting the sync fleet works in principle and is untested in practice — right now "cloud sync" is a Kubernetes cluster that I manage, and your data is encrypted at rest such that no fleet operator can read it without explicit access, enforced by the test suite. Four: the data covenant gates v1, and until then breaking changes remain on the table — loud ones, with graceful degradation. Five: one maintainer, academic day job, releases every 3–5 weeks, and some sprints will be quiet.
<!-- QUARRY: 5.2 #1–#5 near-verbatim, pronoun-swept, stitched into prose. -->

If that does not scare you, [try it out](/download/stable) and [file issues](https://github.com/waypoints-hq/waypoints/issues).
