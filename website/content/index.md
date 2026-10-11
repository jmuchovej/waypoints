---
title: Waypoints — a toolkit for non-linear thought
description: >-
  Thinking doesn't happen in a straight line. Your tools shouldn't force that
  either. Local-first, block-based, GPL — built in the open by one researcher.
---

<!-- Frame: Announcement Day. -->
<!-- Markers: GATE | FACT | BRIDGE | QUARRY
  GATE   = licensed by a launch gate (see Notion gates table)
  FACT   = number/detail you must confirm
  BRIDGE = connective tissue in my words, not quarry — rewrite in yours if it grates
-->
<!--
  Routing: this page targets the EARLY ADOPTER. Tinkerers get routed to /about.
  Voice: first-person singular per 1.5. Triad budget: one (spent in "Not for you" intro).
  index.v2 — SCAN PAGE rebuild per site-ia.md (delete this block before deploy)
  Changes from v1: narrative section removed (→ /why); redditor's "losses" line cut
  (not your quarry); Beacons card removed (not built — origin material lives on /why);
  offline reframed as invariant; sync/encryption unfused; denotative headers throughout;
  readiness ladder replaces "who shouldn't download this yet"; email removed everywhere.
  Triad budget: spent on the invariant strip, on purpose.
-->

::u-page-hero{orientation="vertical"}
#title
  Thinking doesn't happen in a straight line.

#description
  Your tools shouldn't force that either. I got tired of needing twelve apps to track the papers I'm reading, my notes on them, my projects, and the time each one eats — so I built the one I wanted. [The long version.](/why)

#headline
  :::u-badge{size="lg" color="secondary" class="mb-4"}
    Early, honest, and open
  :::

#links
  :::u-button
  ---
  size: xl
  color: primary
  variant: solid
  to: /download/stable
  trailing-icon: i-lucide-arrow-right
  ---
    Download
  :::

  :::u-button
  ---
  size: xl
  to: https://github.com/waypoints-hq/waypoints
  leading-icon: i-simple-icons-github
  color: neutral
  target: _blank
  ---
    Source code
  :::
::

::u-theme
---
ui:
  pageSection:
    root: "bg-elevated/50"
    container: "py-0 sm:py-0 lg:py-0"
    body: "mt-0"
    features: "grid gap-1 divide-x divide-accented"
  pageFeature:
    root: "rounded-none divide-inherit p-2"
  prose:
    p:
      base: "my-0"
---
  ::u-page-section{id="invariants"}
  
  #features
    :::u-page-feature{as="li" to="/docs/storage" icon="i-lucide-plane"}
    
    #title
      Offline always, online sometimes

    #description
    Core features will never wait on a remote server. Its baked into the architecture, not an afterthought.
    :::

    :::u-page-feature{as="li" to="/docs/export" icon="i-lucide-door-open"}
    #title
    Leaves the way it came
    #description
    Export is already built as plain markdown. **Your data is yours, and the text can be read in 1980 or 2180.**
    :::

    :::u-page-feature{as="li" to="/why#sustainability" icon="i-lucide-scale"}
    #title
    GPL-3.0, one public repo

    #description
    Fork it, audit it, continue it without me.
    :::
  ::
::

::u-page-section{id="today"}
#title
What works today

#description
Each card links to the document that proves it.

#features
  :::u-page-feature{as="li" icon="i-lucide-text-cursor-input"}
  #title
  Rich block editor
  
  #description
  Paragraphs, headings, lists, blockquotes, callouts. Math blocks render Typst and LaTeX live.
  :::

  :::u-page-feature{as="li" to="/docs/charts" icon="i-lucide-shapes"}
  #title
  Charts — typing that's optional
  
  #description
  Everything is at least a page; typecast exactly as far as you want. Ships with 8 built-in Charts; define your own when you find new use cases.
  :::

  :::u-page-feature{as="li" to="/docs/import" icon="i-lucide-import"}
  #title
  Import from Logseq, Notion, and Obsidian
  
  #description
  Logseq imports keyed on block UUIDs — re-running never duplicates. The migration matrix shows exactly what survives from each source.
  :::

  :::u-page-feature{as="li" to="/docs/sync" icon="i-lucide-refresh-cw"}
  #title
  Multi-device sync
  
  #description
  Has carried my own devices for months and survived a half-dozen testers. Young, and disclosed as such.
  :::

  :::u-page-feature{as="li" to="/docs/sync#encryption" icon="i-lucide-lock-keyhole"}
  #title
  Encrypted at rest
  
  #description
  Whoever runs the sync fleet cannot read your notes without explicit access. That guarantee is in the test suite — violating it breaks the build.
  :::

  :::u-page-feature{as="li" icon="i-lucide-terminal"}
  #title
  Cross-platform
  
  #description
  macOS, Linux, Windows; Android and iOS.
  :::
::

::u-page-section{id="readiness" orientation="vertical"}
#title
Should you download this today?

#description
Honest answer: depends on your appetite. Pick your rung.

#features
  :::u-page-feature{as="li" to="/download/stable" icon="i-lucide-flame"}
  #title
  Try it today
  
  #description
  You don't need every byte of your old graph to survive import, and breakage is closer to entertainment than catastrophe. Known bugs, missing features, an incomplete UI — if that does not scare you, download it and file issues.
  :::

  :::u-page-feature{as="li" to="https://github.com/waypoints-hq/waypoints/releases" icon="i-lucide-eye"}
  #title
  Watch the releases
  
  #description
  You want this, but you need a specific importer to land, or collaboration — the multiplayer foundations exist and none of it has been tested as multiplayer. Watch the repo, or grab the release-ping role on Discord.
  :::

  :::u-page-feature{as="li" to="/covenant" icon="i-lucide-hourglass"}
  #title
  Wait for the covenant
  
  #description
  You need stability guarantees, not promises. The data covenant defines exactly what v1 means — read it, then take the "ping me at v1" role and close this tab with my blessing.
  :::
::

::u-page-section{id="cost" orientation="vertical"}
#title
What this costs

#description
The app is GPL-3.0 and free. Everything I plan to charge for is availability, not capability — a synced peer that never sleeps, a bigger model than your laptop carries, a scheduler that runs while you don't. All of it ships self-hostable.

Built by one maintainer with an academic day job; releases every 3–5 weeks, and some sprints will be quiet. [How this outlives my motivation.](/why#sustainability)
::

