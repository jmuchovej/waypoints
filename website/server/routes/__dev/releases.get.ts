export default defineEventHandler(() => {
  if (!import.meta.dev) {
    throw createError({ statusCode: 404 });
  }

  return {
    releases: [
      {
        id: 1,
        tag: "v0.1.0",
        author: "john",
        name: "v0.1.0",
        draft: false,
        prerelease: false,
        createdAt: "2026-04-28T12:00:00Z",
        publishedAt: "2026-04-28T12:00:00Z",
        markdown: `## Highlights

- Initial public release of Waypoints
- Rich block-based editor with drag handles and context menus
- Callout blocks with customizable colors
- Reading list management with book import support
- Math rendering with Typst and LaTeX engines

## Bug Fixes

- Fixed missing \`isEmpty\` import that broke bottom-doc click targets
- Resolved compound block creation edge cases
- Fixed undo/redo pipeline for block operations`,
        html: "",
      },
      {
        id: 2,
        tag: "v0.1.1",
        author: "john",
        name: "v0.1.1",
        draft: false,
        prerelease: false,
        createdAt: "2026-05-05T12:00:00Z",
        publishedAt: "2026-05-05T12:00:00Z",
        markdown: `## Bug Fixes

- Fixed drag-and-drop into empty compound blocks
- Dedent (Shift+Tab) now correctly places block as parent sibling
- Resolved Vue Fragment reconciliation crash on block focus transitions`,
        html: "",
      },
      {
        id: 3,
        tag: "v0.2.0-pre.1",
        author: "john",
        name: "v0.2.0-pre.1",
        draft: false,
        prerelease: true,
        createdAt: "2026-05-10T12:00:00Z",
        publishedAt: "2026-05-10T12:00:00Z",
        markdown: `## Highlights

- Wiki-style \`[[linking]]\` with autocomplete
- Full-text search across all marks
- Command palette (Cmd+K) with navigation and actions

## Known Issues

- Search index rebuild can be slow on large territories
- Backlink panel does not update in real-time`,
        html: "",
      },
      {
        id: 4,
        tag: "v0.2.0-pre.2",
        author: "john",
        name: "v0.2.0-pre.2",
        draft: false,
        prerelease: true,
        createdAt: "2026-05-15T12:00:00Z",
        publishedAt: "2026-05-15T12:00:00Z",
        markdown: `## Highlights

- Backlink panel now updates in real-time
- Search results show highlighted matches
- Mention inline mark with hover popover

## Bug Fixes

- Fixed search index not including reading values
- Resolved command palette focus trap on close`,
        html: "",
      },
      {
        id: 5,
        tag: "v0.2.0-nightly.20260531",
        author: "john",
        name: "v0.2.0 (31 May 2026)",
        draft: false,
        prerelease: true,
        createdAt: "2026-05-31T12:00:00Z",
        publishedAt: "2026-05-31T12:00:00Z",
        markdown: `## Highlights

- Initial public release of Waypoints
- Rich block-based editor with drag handles and context menus
- Callout blocks with customizable colors
- Reading list management with book import support
- Math rendering with Typst and LaTeX engines

## Bug Fixes

- Fixed missing \`isEmpty\` import that broke bottom-doc click targets
- Resolved compound block creation edge cases
- Fixed undo/redo pipeline for block operations`,
        html: "",
      },
    ],
  };
});
