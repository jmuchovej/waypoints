# The Waypoints markdown dialect

This document is normative. The dialect is Waypoints' own — CommonMark is its
base and its departures are deliberate, but no external dialect is an authority
here, and none is tracked.

The dialect exists so a `Mark` can be written as text a person reads. It is the
body half of the Bundle format; the Bundle's own grammar — frontmatter, identity
comments, the manifest — is not the dialect's business and lives in the
`transload` crate.

## Principles

1. **A strict subset.** One canonical spelling per construct. Where CommonMark
   offers alternatives (`-`/`*`/`+` bullets, Setext/ATX headings, indented/fenced
   code) the dialect picks one and the emitter writes only that.
2. **Well-formedness is a round trip**, not a validator: a document is
   well-formed when `emit(parse(doc)) == doc`. There is no hand-maintained list
   of rules to drift from the code.
3. **One shared path.** Emit and parse live in one module per layer, so a
   spelling cannot diverge between the directions.
4. **Spans are canonical; characters are shortcuts.** Every inline mark can be
   written `[text]{tokens}`. Character spellings exist for the common marks and
   the emitter prefers one wherever it is unambiguous.
5. **Readers see content; machines read comments.** Anything a reader should
   perceive rides in syntax or attributes. Identity and provenance ride in
   comments, and nothing else does.

## Blocks

Each `Content` variant has exactly one block form. Two variants are containers —
they hold no text of their own and their children supply it.

| `Content`     | Dialect form                       |
| ------------- | ---------------------------------- |
| `Paragraph`   | the bare line                      |
| `Heading1..3` | `# `, `## `, `### ` (ATX only)     |
| `Blockquote`  | `> ` prefix (container)            |
| `ListItem`    | `- ` (container-capable)           |
| `EnumItem`    | `+ ` auto-numbered, `N. ` explicit |
| `Callout`     | `::callout{…}` … `::` (container)  |
| `Code`        | a backtick fence, tagged           |
| `Table`       | GFM pipe table, rows open `\| `    |
| `Math`        | `$$` … `$$`                        |
| `Divider`     | `---`                              |

Rules the emitter holds to:

- **Bullets** are `-`. Never `*`, never `+`.
- **Enums** are `+` when the number is implied by position, and `N.` when the
  number is explicit. Explicit numbers are **(pending)**: `EnumItem` carries no
  field for one, so the emitter writes `+` unconditionally until that domain
  change lands.
- **Indentation** is two spaces per level on emit. On parse, nesting is read by
  _relative_ indentation, Typst-style — the one deliberate leniency in the
  dialect — and normalizes to two spaces when re-emitted. Any block's children
  sit indented beneath it, not only a list item's — save the two containers,
  whose children wear the `> ` prefix or sit between the callout's markers.
- **Headings** are ATX. There is no `####`: the model stops at three levels.
- **Code** is always fenced — three backticks, or one more than the longest
  backtick run inside, so no line of the code closes it — and always carries its
  language, even when empty. `filename` and `highlights` ride as block
  attributes on the opening fence, after the language tag:
  `rust{filename="main.rs" highlights="1,4-6"}`.
- **Dividers** are exactly `---`. Never `***`, never `___`.
- **Containers carry their attributes on their opening marker.** A callout
  always has one (`::callout{…}`); a blockquote's marker is the `> ` prefix its
  children already wear, so an unthemed quote contributes no line of its own and
  a themed one opens with `>{theme-green}`. Writing a bare `>` above every
  quoted passage just to have something to hang attributes on would be a line of
  noise in the common case.
- **Tables** are plain GFM whenever that is honest — a header row and nothing
  else set. Sizes, colours, a row header, or a fill-width cannot be said in GFM,
  and a table with no header row cannot be plain GFM at all without growing one,
  so those wrap themselves in `::table{…}` … `::` rather than losing anything.
  A row opens with `| ` — a line opening with `|` alone may be a `||` spoiler.
- **A text line ending in `}`** — a literal brace or an inline `{tokens}`
  suffix, trailing whitespace aside — always carries an attribute list, empty
  if need be (`set x = {1}{}`), since its last brace would otherwise read as one.

### Block themes

A block's `theme` is the single coupled token `theme-{color}`, matching
`BlockTheme`'s single color. It rides in the block's attribute list:

```md
# A heading{theme-blue}
```

Independent ink and surface are inline-only; a block cannot set them apart.

### Block components

`::name{attrs}` … `::` is the dialect's structured-block extension point.
`callout` is its only current use:

```md
::callout{icon="warning" theme-red}
Mind the gap.
::
```

The component grammar is Comark/MDC-inspired. This document, not that
inspiration, defines it.

## Inlines

### Canonical form

```md
[text]{token token …}
```

Tokens are bare words, space-separated, in the order listed under "Token
order" below. `[hi]{bold italic}` is bold italic text.

### Character shortcuts

| Mark            | Shortcut        | Note                                          |
| --------------- | --------------- | --------------------------------------------- |
| `Bold`          | `*text*`        | **a departure**: CommonMark reads this italic |
| `Italic`        | `_text_`        | **a departure**                               |
| `Underline`     | `__text__`      | **a departure**: CommonMark has no underline  |
| `Strikethrough` | `~~text~~`      |                                               |
| `Code`          | `` `text` ``    |                                               |
| `Highlight`     | `==text==`      | the unparameterized highlight only            |
| `Spoiler`       | `\|\|text\|\|`  |                                               |
| `Superscript`   | `^{text}`       | braced always — see below                     |
| `Subscript`     | `_{text}`       | braced always — see below                     |
| `Link`          | `[text](url)`   | external references                           |
| `Mention`       | `[[id\|label]]` | internal references                           |
| `Math`          | `$expr$`        |                                               |

`**bold**` never appears in an emitted file.

The emphasis remapping is a departure chosen for how it reads. It means the
parser cannot take `pulldown-cmark`'s merged `Emphasis`/`Strong` events at face
value: it resolves each by the delimiter character at its source span, so `*` is
bold, `_` is italic, and `__` is underline.

#### Why the scripts are always braced

`_` already opens italic, so a bare `_c` subscript would be ambiguous with it.
Superscript has no such collision and could have taken a bare `^c` — but the two
scripts reading differently is worse than either rule, so both are braced:
`^{text}` and `_{text}`, or the span forms `[text]{sup}` and `[text]{sub}`.

One consequence is a gain: `^` opens nothing on its own, so `2^10` is ordinary
text that needs no escape. Only `^{` is hidden, as `\^{` — and a `^` ending a
run, whose neighbour may begin with `{`.

The cost is one case: an italic run whose text begins with `{` would make `_{`
read as the subscript opener. Rather than add an escaping rule for it, that one
segment takes the span form — `[{a}]{italic}`. The span is both the canonical
spelling and the fallback, so nothing needs a special escape.

Braced-for-consistency is John's ruling, 2026-10-20, on a point D8 left as
"`^`/`_` with Typst/LaTeX brace rules".

### Span tokens

| Mark            | Token       |
| --------------- | ----------- |
| `Bold`          | `bold`      |
| `Italic`        | `italic`    |
| `Underline`     | `underline` |
| `Strikethrough` | `strike`    |
| `Superscript`   | `sup`       |
| `Subscript`     | `sub`       |
| `Code`          | `code`      |
| `Highlight`     | `highlight` |
| `Spoiler`       | `spoiler`   |
| `Math`          | `math`      |

### Parameterized marks

These carry data and so have no character shortcut — the span form is their only
spelling. Tokens keep the evergreen Tailwind names; the domain layer calls the
two color channels ink and surface internally.

| Mark         | Token           | Data                        |
| ------------ | --------------- | --------------------------- |
| `Color`      | `text-{color}`  | a color name, as a string   |
| `Highlight`  | `bg-{color}`    | a color name, as a string   |
| `FontSize`   | `text-{size}`   | a size token, as a string   |
| `FontFamily` | `font-{family}` | a family token, as a string |

`text-` prefixes both ink color and font size, exactly as Tailwind does. The two
value spaces are disjoint — color names on one side, size tokens on the other —
so no token is ambiguous.

### Token order

A span's tokens are emitted in `TextMarkKind` declaration order, so one set of
marks has one spelling. Parse accepts any order.

### Mentions

A mention is a wiki link whose interior is a bare `MarkID`:

```md
Asimov wrote [[018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55|Foundation]].
```

The interior is normative and identifies the target losslessly. The label after
the pipe is the **anchor text** — what a person chose to call the target in this
sentence — and it survives verbatim in both directions. "the Foundation series"
stays "the Foundation series"; replacing it with the target's title on every
export would quietly discard what someone wrote.

Only a **bare** mention, `[[id]]`, has no anchor text, and that is the one case
the label is generated: the ingester resolves the target's title, since it is
the side holding the store. (John's ruling, 2026-10-20, amending the spec
delta's "advisory and regenerated on export".)

Frontmatter is the other story: a relation Reading's `{ref, label?}` label _is_
advisory and ignored on ingest, because `Reading::Relation` holds refs and has
nowhere for a label to land.

A bare `MarkID`, rather than a relative Landmark path, because a mention may
target a Content Mark, which has no Landmark file to point at — and because
embedding the Chart directory in body text would couple content bytes to export
layout, so re-grouping a Chart would rewrite every mentioning file.

`Mention.data` is a bare JSON string holding the UUID. That is what
`cartograph::editor::projection` reads and what the legacy importer writes.

Legacy content whose mention text was the raw UUID — the `@uuid` grammar the
demo corpus used — therefore exports with a UUID for its anchor text. That is
legal (a UUID inside `[[…]]` is a link target) but ugly, and it is one of the
things the corpus audit exists to catch.

A mention needing more than a target — a non-default reference display mode —
escalates to an inline component. That is the only reserved use of inline
components in the dialect.

## Escaping

The emitter escapes exactly what it must for the parse direction to be exact,
and no more. A character is escaped when leaving it bare would open a construct:

- `\`, `[`, `]`, `*`, `_`, `` ` ``, `$` — always, since each opens a construct
  on its own
- `~`, `=`, `|` — only when doubled or at a run's edge, since only `~~`, `==`
  and `||` are significant and a neighbouring run may supply the double. Prose
  keeps its lone interior tildes and pipes.
- `^` — only before `{` or at a run's end, since the scripts are braced. `2^10`
  stays bare.
- `}` inside a braced script, which would close it early
- `{` opening a plain run, which would read as the preceding span's tokens
- `)` and `\` in a link target, which would end it early
- `#`, `>`, `-`, `+` at the start of a line — the block layer's business, since
  only there do they begin anything

Table cells add their own, all removed again before inline parsing: every `|`
becomes `\|` (as GFM), an edge space is escaped so the row's trim keeps it, and a
cell of nothing but `-` and `:` gets a leading `\` so its row cannot read as
the delimiter row.

Code and math interiors are **verbatim**: nothing inside them is escaped, so a
text carrying their own delimiter has no shortcut and takes the span form.

Escaping is fixture-tested per `Content` variant and per segment shape, round
trip exact, and `transload`'s conformance suite drives it with generated stores
(`TRANSLOAD_SEEDS` raises the default 64).

## What is normalized

Two things are regenerated rather than preserved. They join edge identities and
order strings in the set that round-trip equality exempts — each is an artifact
of how an editor built the list, not content a reader could point at:

1. **Adjacent runs with identical marks coalesce.** Two neighbouring bold runs
   are one bold run; emitting them separately would write `*a**b*` and read back
   as one anyway.
2. **A segment's marks come out in one order** — the table above, top to bottom.
   Parse accepts any order.

A mention's label is **not** among them — see Mentions above.

An empty run is dropped as an artifact — _unless_ it carries data. An unlabelled
mention, `[[id]]`, is still a mention, and its target is not recoverable from
text.

## Pending

Two constructs cannot be spelled yet, both awaiting domain changes outside the
dialect:

- **Explicit enum numbers** — `EnumItem` carries no number field.
- **The `comment` mark** — `TextMarkKind` has no `Comment` variant. (The
  frontend's `registries.ts` already reaches for one; that gap is its own.)
