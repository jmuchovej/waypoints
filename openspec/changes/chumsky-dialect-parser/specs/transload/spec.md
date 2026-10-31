## ADDED Requirements

### Requirement: Parsing is total

Every parse entry of the dialect and the Bundle's text carriers — inline
text, a single block, a Landmark body, frontmatter — SHALL return either a
value or a refusal for any input, SHALL NOT panic, and SHALL terminate on
every input. Recursion in each grammar SHALL be bounded by a fixed ceiling, and
input nested past the ceiling SHALL refuse with a sentence naming the ceiling
rather than exhausting the stack.

#### Scenario: Arbitrary bytes

- **WHEN** any sequence of bytes is given to the inline, block, body or
  frontmatter parser
- **THEN** the parser returns a value or a refusal, and the process is still
  running

#### Scenario: Pathological nesting

- **WHEN** inline text opens a braced script one more time than the ceiling
  allows
- **THEN** the text is refused with a sentence naming the ceiling, at the
  position of the first opener past it
- **AND** a thread with a two-megabyte stack completes the parse

### Requirement: A refusal is located

A refusal of inline or block text SHALL carry the line and column, within the
source given, of the construct it refuses. A refusal of a Landmark body SHALL
carry the file line of the offending construct, not the line of the block or
identity comment it sits under. A refusal of frontmatter SHALL carry the file
line of the offending entry.

#### Scenario: An unclosed shortcut

- **WHEN** the inline text `*unclosed` is parsed
- **THEN** the refusal reads `unclosed `*`` at line 1, column 1

#### Scenario: An unknown token points at itself

- **WHEN** the inline text `[hi]{bold nonsense}` is parsed
- **THEN** the refusal reads `` `nonsense` is not a mark token in this dialect ``
  at line 1, column 11

#### Scenario: A body refusal cites the construct's line

- **WHEN** a Landmark file's twelfth line is a list item, nested in a
  blockquote under an indented parent, whose text ends in `{theme-chartreuse}`
- **THEN** the refusal's location is that file at line 12
- **AND** its detail names `chartreuse` as not a block theme colour and gives
  a column that counts the ` >` prefix

#### Scenario: A frontmatter refusal cites its line

- **WHEN** a Landmark file's frontmatter names a reading of no feature on its
  seventh line
- **THEN** the refusal's location is that file at line 7

### Requirement: A refusal speaks the dialect's vocabulary

A refusal SHALL name what was expected in the dialect's own terms — the
construct, the token, or the sentence the dialect has used for that fault —
and SHALL NOT name a parser combinator, a library type, or an internal
function. A sentence supplied by the caller's `Links` SHALL appear in the
refusal unchanged.

#### Scenario: A bare bracketed run

- **WHEN** the inline text `[bare bracketed]` is parsed
- **THEN** the refusal reads `a bracketed run needs a `(url)`or`{tokens}`
suffix; a literal bracket is escaped`

#### Scenario: A missing identity comment

- **WHEN** a Landmark body line that should open a node is an ordinary HTML
  comment
- **THEN** the refusal is located at that line and reads
  `expected a `wp:mark` comment`

#### Scenario: A caller's sentence passes through

- **WHEN** `[[Nowhere]]` is parsed with a `Links` whose `resolve` returns
  `Err("no page `Nowhere`")`
- **THEN** the refusal's detail is exactly `no page `Nowhere``

#### Scenario: No internal names leak

- **WHEN** every refusal fixture of the inline, block, body and frontmatter
  parsers is rendered
- **THEN** no rendered sentence contains a Rust path, a type name from the
  parsing library, or the word `parser`

### Requirement: Every byte the emitter writes reads back

For every `RichText`, `Content` and frontmatter the emitter accepts,
`parse(emit(value))` SHALL equal `value`, and the set of inputs on which this
is checked SHALL cover every token, character shortcut, escape rule, block
form, attribute key and reading kind the emitter can write — the emitter's
own branches are the checklist. A change to a parser that alters the value
any emitted byte sequence reads back to SHALL fail these checks before it
fails the store-level round trip.

#### Scenario: Every inline spelling

- **WHEN** a rich text carrying each mark kind alone, each parameterised mark,
  each mention spelling, a link, and plain text containing every character the
  emitter escapes is emitted and parsed
- **THEN** the parsed rich text equals the original after canonical
  coalescing

#### Scenario: Every block form

- **WHEN** each `Content` variant, themed and unthemed, with every attribute
  its block form can carry, is emitted and parsed
- **THEN** the parsed content equals the original

#### Scenario: Every reading kind

- **WHEN** a frontmatter carrying each `Reading` variant, empty and
  non-empty, an icon, and a title with dialect syntax is emitted and parsed
  against its Chart's feature kinds
- **THEN** the parsed frontmatter equals the original

#### Scenario: The committed Bundle is a fixed point

- **WHEN** the committed `getting-started` Bundle is ingested and exported
- **THEN** every file is byte-identical to the committed one

### Requirement: Frontmatter is read as a strict YAML subset

A Landmark's frontmatter SHALL be read as the subset of YAML the emitter
writes and nothing beyond it: block mappings and sequences at a two-space
step to a bounded depth, and the scalar and flow forms the emitter uses. Every
frontmatter the emitter writes SHALL be within the subset. YAML outside the
subset SHALL refuse as malformed with the line named.

#### Scenario: The subset's forms

- **WHEN** a frontmatter uses plain scalars, double-quoted scalars escaping
  only `\\`, `\"` and `\n`, strip-chomped literal block scalars (`|-`, `|2-`),
  one-line compact JSON, a one-line flow sequence of scalars, the empty flows
  `[]` and `{}`, and sequence items that are scalars, flows, or mappings whose
  first key shares the item's line
- **THEN** it parses

#### Scenario: A tag is refused with its line

- **WHEN** a Landmark's frontmatter carries `chart: !!str builtin.page` on its
  third line
- **THEN** ingest refuses with a malformed-Bundle error locating that file at
  line 3 and naming the tag as outside the frontmatter grammar
- **AND** the store is unchanged

#### Scenario: Constructs outside the subset are refused

- **WHEN** a frontmatter carries an anchor or alias, a single-quoted scalar, a
  flow spanning lines or nested beyond compact JSON, a `?` complex key, a tab
  as indentation, a three-space step, or nesting past the bound
- **THEN** ingest refuses naming the line of the construct

#### Scenario: The emitter stays inside the subset

- **WHEN** any store is exported
- **THEN** every Landmark file's frontmatter parses under the subset grammar
  to the frontmatter it was written from

### Requirement: Frontmatter comments and spacing are advisory

A trailing `# …` on a frontmatter key line and an own-line `# …` between
entries SHALL be read as advisory and SHALL NOT influence the result. Spaces
beyond the one after `:` or `-`, trailing whitespace, blank lines between
entries and CRLF line ends SHALL be accepted without changing the result.

#### Scenario: Tolerances change nothing

- **WHEN** an exported Landmark file's frontmatter is rewritten with CRLF line
  ends, trailing spaces on every line, a blank line between readings and an
  own-line comment above `readings:`
- **THEN** it ingests to a Landmark equal to the one the untouched file
  ingests to

#### Scenario: Notes are advisory

- **WHEN** the trailing comments on a frontmatter's reading keys are replaced
  with arbitrary text
- **THEN** the ingested Landmark is identical to ingesting the file with those
  comments removed
