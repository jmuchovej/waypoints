---
uuid: 45f453a4-b942-41ed-b144-57207eb74ce5
title: Welcome to Waypoints
---

Waypoints is a structured editor for capturing and organizing your work. Everything you write lives inside **Marks** — discrete, identifiable blocks of content that can be referenced, linked, and composed.

> [!TIP]
> Type `/` at the start of any block to open the slash menu and convert it to a different type — heading, list, blockquote, callout, code, and more.

---

## Formatting

Inline styles: **bold**, _italic_, **underline**, ~~strikethrough~~, and `inline code`. You can also use ^superscript^ and ~subscript~ for technical notation, or ||spoilers|| to hide sensitive content until revealed.

> Blockquotes pull out key ideas or reference external sources. Nest them for layered commentary.

> [!NOTE]
> Callouts draw attention to important information. They come in five flavors: note, tip, important, warning, and caution.

### Headings

Three levels of headings help structure your marks. Use them to create a scannable hierarchy within longer documents.

### Code Blocks

Embed code with language-specific syntax highlighting:

```rust
fn main() {
    let waypoint = Mark::new("hello-world");
    waypoint.publish();
}
```

### Lists

- Marks are the atomic unit of content
- Each mark gets a unique identifier
- Marks compose into larger structures
  - Nested items organize sub-points
  - Indentation conveys hierarchy

1. Numbered lists track ordered sequences
2. Use them for steps, rankings, or progressions

---

## Keyboard Shortcuts

Most formatting is a keystroke away:

- **Bold** — `Cmd+B`
- **Italic** — `Cmd+I`
- **Underline** — `Cmd+U`
- **Inline Code** — `Cmd+E`
- **Strikethrough** — `Cmd+Shift+S`
- **Spoiler** — `Cmd+Shift+H`
- **Highlight** — `Cmd+Shift+E`
- **Superscript** — `Cmd+.`
- **Subscript** — `Cmd+,`

> [!IMPORTANT]
> Use `@` to mention other marks inline. Mentions are live references — click them to navigate, hover for a preview.

---

> [!CAUTION]
> This is a dev playground. Break things, experiment, and have fun. The database can be reset from the Developer menu at any time.
