---
seo:
  title: Waypoints — Your data. Your machine. Your knowledge.
  description: A local-first, block-based knowledge tool. No cloud accounts, no API keys, no data harvesting. Your notes live on your device and nowhere else.
---

::u-page-hero
---
orientation: horizontal
---
#title
Your data. Your machine. Your knowledge.

#description
Waypoints is a local-first, block-based knowledge tool. No cloud accounts, no API keys, no data harvesting. Your notes live on your device and nowhere else.

#headline
:::u-badge
---
variant: subtle
class: mb-4
---
Early development
:::

#links
  :::u-button
  ---
  size: xl
  to: /download/stable
  trailing-icon: i-lucide-arrow-right
  ---
  Download
  :::

  :::u-button
  ---
  color: neutral
  icon: i-simple-icons-github
  size: xl
  target: _blank
  to: https://github.com/waypoints-hq/waypoints
  variant: subtle
  ---
  Source code
  :::
::

::u-page-section
---
id: principles
---
#title
Principles, not promises

#description
Waypoints is early software. Here is what drives every decision.

#features
  ::::u-page-feature
  ---
  icon: i-lucide-hard-drive
  ---
  #title
  Data locality
  #description
  Your notes are stored in a SQLite database on your device. No sync servers, no vendor lock-in. Back them up however you want.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-cpu
  ---
  #title
  Compute locality
  #description
  Processing happens on your machine. Math rendering, search indexing, and eventually AI inference — all local.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-lock-keyhole
  ---
  #title
  Data ownership
  #description
  Waypoints does not want your data. No telemetry, no analytics, no cloud dependency. If a feature needs a network call, you opt in explicitly.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-box
  ---
  #title
  Blocks as first-class citizens
  #description
  Every piece of content is a block that can be dragged, nested, linked, and referenced. Not an afterthought bolted onto a document model.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-shapes
  ---
  #title
  User-defined types
  #description
  Built-in charts (Pages, Notes, People, Meetings, Bookmarks) are just the starting set. Define your own types with custom fields and relationships.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-code
  ---
  #title
  Open Source
  #description
  The entire codebase is public. Inspect it, fork it, contribute to it. No proprietary core hiding behind an open wrapper.
  ::::
::

::u-page-section
---
id: today
---
#title
What works today

#description
Waypoints is dogfooding toward v0.1. Here is what is already built and usable.

#features
  ::::u-page-feature
  ---
  icon: i-lucide-text-cursor-input
  ---
  #title
  Rich block editor
  #description
  Paragraphs, headings, lists, blockquotes, dividers, and math blocks with live Typst and LaTeX rendering.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-grip-vertical
  ---
  #title
  Drag-and-drop reordering
  #description
  Single and multi-block drag with visual feedback, compound block nesting, and touch support for mobile.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-type
  ---
  #title
  Inline formatting
  #description
  Bold, italic, underline, strikethrough, code, links, inline math, spoilers, superscript, and subscript — all with markdown shortcuts.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-database
  ---
  #title
  Structured data
  #description
  8 built-in chart types with typed readings: text, numbers, dates, selects, references, URLs, emails, and formulas.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-slash
  ---
  #title
  Slash commands
  #description
  Type `/` to convert blocks, insert new content types, or access editor actions with fuzzy search.
  ::::

  ::::u-page-feature
  ---
  icon: i-lucide-monitor-smartphone
  ---
  #title
  Cross-platform
  #description
  Built with Tauri. Runs natively on macOS, Linux, and Windows. Android builds exist. iOS is in progress.
  ::::
::

::u-page-section
#title
Early, honest, and open

#description
Waypoints is not ready for production use. It has known bugs, missing features, and an incomplete UI. If that does not scare you, try it out and file issues.

#links
  :::u-button
  ---
  color: neutral
  to: /download/stable
  trailing-icon: i-lucide-arrow-right
  ---
  Download
  :::

  :::u-button
  ---
  color: neutral
  icon: i-simple-icons-github
  target: _blank
  to: https://github.com/waypoints-hq/waypoints/issues
  variant: outline
  ---
  Issues & feedback
  :::
::
