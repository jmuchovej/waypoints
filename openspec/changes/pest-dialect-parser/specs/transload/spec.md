## ADDED Requirements

### Requirement: The dialect's inline and block syntax have one executable grammar

The syntax of inline text and of a single block SHALL be defined by
machine-readable grammar files shipped alongside the dialect, and the parser
SHALL be derived from those files rather than written beside them. The
reference document (`dialect.md`) explains the grammar and MAY restate rules
for the reader; where the document and the grammar disagree, the grammar is
normative and the document SHALL say so.

#### Scenario: The grammar is what the parser runs

- **WHEN** a production in the grammar file is changed
- **THEN** the parser's acceptance changes accordingly without any other
  source edit

#### Scenario: The emitter writes only what the grammar reads

- **WHEN** every mark, token, and block form the emitter can write is
  rendered and parsed
- **THEN** each parses back to the value it was rendered from

### Requirement: A grammar refusal names the construct and the position

When inline or block source is not in the dialect, the refusal SHALL state
which construct was expected in the dialect's own vocabulary — never an
internal rule identifier — and SHALL locate the fault by line and column
within the source handed to the parser. A refusal raised while ingesting a
Bundle SHALL additionally carry the file and file line, as today.

#### Scenario: An unclosed span is located

- **WHEN** the inline source `a [b` is parsed
- **THEN** the refusal names an unclosed bracketed run and points at the `[`

#### Scenario: An unknown attribute is named

- **WHEN** a block line carries `{highlight}` as a block attribute
- **THEN** the refusal says `highlight` is not a block attribute and points
  at it

### Requirement: Parsing is total

For any input string, inline and block parsing SHALL terminate and SHALL
return either a value or a refusal; no input SHALL cause a panic.

#### Scenario: Arbitrary bytes are refused, not crashed on

- **WHEN** an arbitrary sequence of characters, including unmatched
  delimiters, lone backslashes, and deeply nested openers, is parsed
- **THEN** the parser returns a value or a refusal within bounded time

## MODIFIED Requirements

### Requirement: The projection reads as text once comments are stripped

Stripping every comment from every projection file SHALL yield documents
that read as text, proper — every remaining marker a legible dialect
convention, with zero machine-shaped residue (`wp:` markers, and UUIDs save
as the target of a link to a Mark the Bundle does not carry). Renderer
conformance is not the bar; the claim is Org-Mode-style plain-text
legibility. Content a reader should see rides in dialect syntax or
attributes; identity and provenance ride in comments; nothing else exists.

Every block line of a stripped file SHALL parse under the dialect's
executable grammar, so "still parses" is a check against the shipped grammar
files rather than against prose.

#### Scenario: The 1980 test

- **WHEN** all comments are stripped from every `.md` file in an exported
  Bundle
- **THEN** no file contains a `wp:` marker, a UUID, or other machine-shaped
  residue, save the bare target of a link to a Mark the Bundle does not carry
- **AND** every block line still parses under the dialect's executable
  grammar
