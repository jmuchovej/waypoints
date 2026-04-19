---
title: Why Waypoints
description: The design principles and motivations behind Waypoints — a local-first, block-based knowledge tool.
layout: default
navigation: false
---

::u-page-hero
#title
Why Waypoints

#description
Most knowledge tools ask you to trust them with your data. Waypoints asks you to keep it.
::

::u-page-section
---
orientation: vertical
---
#title
The problem

#description
Personal knowledge management is broken in a specific way: the tools that are most capable are also the ones most dependent on someone else's infrastructure. Notion runs entirely in the cloud. Logseq's extension story is minimal. AnyType's open source code lacks core extensibility. Obsidian is local-first but limited to files and folders — no structured data, no typed relationships, no block-level graph. Every tool makes a tradeoff between capability and independence. Waypoints exists because that tradeoff shouldn't be necessary.
::

::u-page-section
---
orientation: vertical
---
#title
What we believe

#features
:::u-page-feature
---
icon: i-lucide-hard-drive
---
#title
Your data stays on your device
#description
Waypoints stores everything in a local SQLite database. No cloud account is required to use the app. When sync is available, the relay servers store encrypted data — we cannot read your notes and do not want to.
:::
:::u-page-feature
---
icon: i-lucide-cpu
---
#title
Compute happens locally
#description
Math rendering, search indexing, and — eventually — AI inference all run on your machine. Network calls are opt-in and transparent. If a feature can work offline, it does.
:::
:::u-page-feature
---
icon: i-lucide-puzzle
---
#title
Extensibility is non-negotiable
#description
A knowledge tool you cannot extend is a knowledge tool you will outgrow. Waypoints is building toward an extension system on par with Obsidian's — but backed by a block-level data model and user-defined types.
:::
:::u-page-feature
---
icon: i-lucide-blocks
---
#title
Blocks are the primitive
#description
Every piece of content is a block — addressable, draggable, linkable, and queryable. Not a line in a file. Not a row in a database. A first-class unit of thought that participates in a graph.
:::
:::u-page-feature
---
icon: i-lucide-mic
---
#title
Voice and agents are local
#description
Speech recognition with speaker diarization. Local language models via MCP interfaces. Transcription as an input method, not just an afterthought. All running on your hardware.
:::
:::u-page-feature
---
icon: i-lucide-shield
---
#title
No dark patterns
#description
No telemetry. No analytics. No "free tier" designed to convert you. The code is open source. The business model is sync and infrastructure — not your attention or your data.
:::
::

::u-page-section
---
orientation: vertical
---
#title
Where Waypoints is today

#description
Waypoints is pre-release software. The editor works. The type system works. Drag-and-drop, math rendering, slash commands, and structured readings all work. Wiki-linking, search, importers, and sync are in progress. It is not ready for production use — but it is real, it is usable, and every piece of it is public.
::
