//! The YAML a Landmark's frontmatter is written in: the subset the emitter
//! writes, and nothing beyond it.
//!
//! Block mappings and sequences step two spaces, to a bounded depth. Scalars
//! are plain, double-quoted (escaping only `\\`, `\"` and `\n`), strip-chomped
//! literal blocks (`|-`, `|2-`), one line of compact JSON, or a one-line flow
//! sequence; `[]` and `{}` are the empty collections. A trailing `# …` is an
//! entry's advisory note. A file outside the subset is refused at its line.

use chumsky::{
    input::{Emitter, MapExtra},
    prelude::*,
};

/// How deep mappings and sequences may nest below the top level.
pub(crate) const MAX_DEPTH: usize = 4;

const STEP: &str = "  ";

type Extra<'a> = extra::Err<Rich<'a, char>>;

/// A value as the subset spells it. The style is kept, so a written value
/// reads back as itself.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Node {
    /// A bare scalar: ids, `null`, booleans, numbers, timestamps.
    Plain(String),
    /// `"…"`.
    Quoted(String),
    /// `|-`, or `|2-` when the text opens with a space; lines one step deeper.
    Literal(String),
    /// One line of compact JSON.
    Json(serde_json::Value),
    /// `[a, b]` of scalars, on one line.
    Flow(Vec<Node>),
    /// `- item` lines one step deeper, or `[]` when empty.
    Seq(Vec<Node>),
    /// `key: value` lines one step deeper, or `{}` when empty.
    Map(Vec<Entry>),
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub key: String,
    pub value: Node,
    /// A trailing `# …`: written and read, never interpreted.
    pub note: Option<String>,
    /// The byte its line starts at, where it was read from; nothing compares it.
    pub at: usize,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        (&self.key, &self.value, &self.note) == (&other.key, &other.value, &other.note)
    }
}

impl Entry {
    pub(crate) fn new(key: impl Into<String>, value: Node) -> Self {
        Self {
            key: key.into(),
            value,
            note: None,
            at: 0,
        }
    }

    pub(crate) fn noted(mut self, note: Option<String>) -> Self {
        self.note = note;
        self
    }
}

impl Node {
    /// An id: bare, unless YAML would read it as a number, `null`, or a boolean.
    pub(crate) fn id(text: &str) -> Self {
        match retyped(text) {
            true => Self::Quoted(text.to_string()),
            false => Self::Plain(text.to_string()),
        }
    }

    /// Rich text's spelling: a literal block, or `""` when there is no text.
    pub(crate) fn text(text: String) -> Self {
        match text.is_empty() {
            true => Self::Quoted(text),
            false => Self::Literal(text),
        }
    }

    /// The text of a scalar, whatever its style.
    pub(crate) fn scalar(&self) -> Option<&str> {
        match self {
            Self::Plain(text) | Self::Quoted(text) | Self::Literal(text) => Some(text),
            _ => None,
        }
    }

    /// The items of a sequence, block or flow.
    pub(crate) fn items(&self) -> Option<&[Node]> {
        match self {
            Self::Seq(items) | Self::Flow(items) => Some(items),
            _ => None,
        }
    }
}

/// `0001` reads as an integer, `12e3` as a float, `null` and `true` as such.
fn retyped(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_digit())
        || matches!(text, "null" | "true" | "false" | "~")
}

/// A top-level mapping's lines.
pub(crate) fn write(entries: &[Entry]) -> String {
    let mut out = String::new();
    for entry in entries {
        write_entry(&mut out, 0, entry);
    }
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(STEP);
    }
}

fn write_entry(out: &mut String, depth: usize, entry: &Entry) {
    indent(out, depth);
    match retyped(&entry.key) {
        true => out.push_str(&quote(&entry.key)),
        false => out.push_str(&entry.key),
    }
    out.push(':');
    let note = |out: &mut String| {
        if let Some(note) = &entry.note {
            out.push_str(" # ");
            out.push_str(note);
        }
        out.push('\n');
    };
    match &entry.value {
        Node::Literal(text) => {
            out.push_str(if text.starts_with(' ') { " |2-" } else { " |-" });
            note(out);
            for line in text.split('\n') {
                indent(out, depth + 1);
                out.push_str(line);
                out.push('\n');
            }
        }
        Node::Seq(items) if !items.is_empty() => {
            note(out);
            for item in items {
                write_item(out, depth + 1, item);
            }
        }
        Node::Map(entries) if !entries.is_empty() => {
            note(out);
            for entry in entries {
                write_entry(out, depth + 1, entry);
            }
        }
        scalar => {
            out.push(' ');
            out.push_str(&inline(scalar));
            note(out);
        }
    }
}

/// `- item`; a mapping's first entry shares the dash line.
fn write_item(out: &mut String, depth: usize, item: &Node) {
    indent(out, depth);
    out.push_str("- ");
    match item {
        Node::Map(entries) if !entries.is_empty() => {
            let mut first = String::new();
            write_entry(&mut first, 0, &entries[0]);
            let mut lines = first.split_terminator('\n');
            out.push_str(lines.next().unwrap_or_default());
            out.push('\n');
            for line in lines {
                indent(out, depth + 1);
                out.push_str(line);
                out.push('\n');
            }
            for entry in &entries[1..] {
                write_entry(out, depth + 1, entry);
            }
        }
        scalar => {
            out.push_str(&inline(scalar));
            out.push('\n');
        }
    }
}

/// A value on its key's line.
fn inline(node: &Node) -> String {
    match node {
        Node::Plain(text) => text.clone(),
        Node::Quoted(text) | Node::Literal(text) => quote(text),
        Node::Json(value) => serde_json::to_string(value).unwrap_or_else(|_| "null".into()),
        Node::Flow(items) | Node::Seq(items) => {
            let items: Vec<String> = items.iter().map(inline).collect();
            format!("[{}]", items.join(", "))
        }
        Node::Map(_) => "{}".into(),
    }
}

/// A double-quoted scalar, escaping only what would end or break it.
pub(crate) fn quote(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// A top-level mapping, or the byte its first problem starts at and what it is.
pub(crate) fn read(source: &str) -> Result<Vec<Entry>, (usize, String)> {
    let (entries, errors) = document().parse(source).into_output_errors();
    if let Some(error) = errors.first() {
        return Err((error.span().start, as_sentence(error)));
    }
    Ok(entries.unwrap_or_default())
}

fn as_sentence(error: &Rich<'_, char>) -> String {
    use chumsky::error::{RichPattern::*, RichReason::*};
    match error.reason() {
        Custom(sentence) => sentence.clone(),
        ExpectedFound { expected, found } => {
            let mut expected: Vec<String> = expected
                .iter()
                .filter_map(|pattern| match pattern {
                    Label(label) => Some(label.to_string()),
                    EndOfInput => Some("the end of the frontmatter".into()),
                    _ => None,
                })
                .collect();
            expected.dedup();
            let found = match found {
                Some(found) => format!("`{}`", found.escape_default()),
                None => "the end of the frontmatter".into(),
            };
            match expected.is_empty() {
                true => format!("{found} is outside the frontmatter grammar"),
                false => format!("expected {}, found {found}", expected.join(" or ")),
            }
        }
    }
}

/// Spaces and tabs, then the line's end.
fn eol<'a>() -> impl Parser<'a, &'a str, (), Extra<'a>> + Clone {
    one_of(" \t")
        .repeated()
        .then(just('\r').or_not())
        .then(choice((just('\n').ignored(), end())))
        .ignored()
}

/// A line with nothing on it, or only a `# …` comment.
fn skip<'a>() -> impl Parser<'a, &'a str, (), Extra<'a>> + Clone {
    let blank = one_of(" \t")
        .repeated()
        .then(just('\r').or_not())
        .then(just('\n'))
        .ignored();
    let trailing = one_of(" \t").repeated().at_least(1).then(end()).ignored();
    let comment = one_of(" \t")
        .repeated()
        .then(just('#'))
        .then(none_of("\n").repeated())
        .then(choice((just('\n').ignored(), end())))
        .ignored();
    choice((blank, trailing, comment))
}

/// ` # …` to the line's end.
fn note<'a>() -> impl Parser<'a, &'a str, String, Extra<'a>> + Clone {
    one_of(" \t")
        .repeated()
        .at_least(1)
        .ignore_then(just('#'))
        .ignore_then(just(' ').or_not())
        .ignore_then(none_of("\r\n").repeated().to_slice())
        .map(|note: &str| note.trim_end().to_string())
}

fn quoted<'a>() -> impl Parser<'a, &'a str, String, Extra<'a>> + Clone {
    let escape = just('\\')
        .ignore_then(any())
        .validate(|escaped: char, extra, emitter| match escaped {
            '\\' | '"' => escaped,
            'n' => '\n',
            other => {
                emitter.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "`\\{other}` is an escape outside the frontmatter grammar, which has only \
                         `\\\\`, `\\\"` and `\\n`"
                    ),
                ));
                other
            }
        });
    just('"')
        .ignore_then(choice((escape, none_of("\"\\\n"))).repeated().collect())
        .then_ignore(just('"'))
}

type LineValue = (Node, Option<String>);

/// What a value cannot open with, and why.
fn refused<'a>() -> impl Parser<'a, &'a str, LineValue, Extra<'a>> + Clone {
    one_of("!&*'>%@`?|\"[]{},")
        .then_ignore(none_of("\n").repeated())
        .validate(|opener: char, extra, emitter| {
            let what = match opener {
                '!' => "a YAML tag",
                '&' => "a YAML anchor",
                '*' => "a YAML alias",
                '\'' => "a single-quoted scalar",
                '>' => "a folded scalar",
                '|' => "a block scalar other than `|-` or `|2-`",
                '"' => "a quoted scalar that does not close on its line",
                '[' => "a flow sequence that does not close on its line",
                ']' | '}' | ',' => "a stray flow indicator",
                _ => "a YAML directive or reserved indicator",
            };
            emitter.emit(Rich::custom(
                extra.span(),
                format!("{what} is outside the frontmatter grammar"),
            ));
            (Node::Plain(String::new()), None)
        })
}

fn json_line_validator<'src, 'b>(
    text: &str,
    extra: &mut MapExtra<'src, 'b, &'src str, Extra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> LineValue {
    let mut values = serde_json::Deserializer::from_str(text).into_iter();
    match values.next() {
        Some(Ok(value)) => {
            let after = text[values.byte_offset()..].trim();
            let note = after.strip_prefix('#').map(|note| {
                note.strip_prefix(' ')
                    .unwrap_or(note)
                    .trim_end()
                    .to_string()
            });
            if !after.is_empty() && note.is_none() {
                emitter.emit(Rich::custom(
                    extra.span(),
                    "a JSON value ends its line, a note aside",
                ));
            }
            let node = match value {
                // `{}` is the empty mapping, not a JSON value.
                serde_json::Value::Object(fields) if fields.is_empty() => Node::Map(Vec::new()),
                value => Node::Json(value),
            };
            (node, note)
        }
        _ => {
            emitter.emit(Rich::custom(
                extra.span(),
                "a `{` value is one line of JSON",
            ));
            (Node::Map(Vec::new()), None)
        }
    }
}

/// A value on its key's line, and the note after it.
fn line_value<'a>() -> impl Parser<'a, &'a str, LineValue, Extra<'a>> + Clone {
    let rest = none_of("\r\n").repeated().to_slice();
    let json = just('{')
        .rewind()
        .ignore_then(rest)
        .validate(json_line_validator);
    let flow_item = choice((
        quoted().map(Node::Quoted),
        none_of(",[]{}\"#\r\n")
            .repeated()
            .at_least(1)
            .to_slice()
            .map(|item: &str| Node::Plain(item.trim().to_string())),
    ))
    .padded_by(just(' ').repeated());
    let flow = just('[')
        .ignore_then(flow_item.separated_by(just(',')).collect::<Vec<_>>())
        .then_ignore(just(']'))
        .map(|items: Vec<Node>| match items.is_empty() {
            true => Node::Seq(items),
            false => Node::Flow(items),
        })
        .then(note().or_not())
        .labelled("a flow sequence closed on its own line");
    let empty = just('[')
        .then(just(' ').repeated())
        .then(just(']'))
        .to(Node::Seq(Vec::new()))
        .then(note().or_not());
    let plain = any()
        .and_is(choice((one_of(" \t").then(just('#')).ignored(), eol())).not())
        .repeated()
        .at_least(1)
        .to_slice()
        .validate(|text: &str, extra, emitter| {
            let text = text.trim_end();
            if text.contains(": ") || text.starts_with("- ") {
                emitter.emit(Rich::custom(
                    extra.span(),
                    format!("`{text}` is a plain value holding YAML structure; quote it"),
                ));
            }
            Node::Plain(text.to_string())
        })
        .then(note().or_not());
    choice((
        quoted().map(Node::Quoted).then(note().or_not()),
        json,
        empty,
        flow,
        refused(),
        plain,
    ))
}

/// A literal block's lines, each past `width` spaces; a line of only spaces is
/// empty, and trailing empty lines are not the text's.
fn literal_body<'a>(width: usize) -> impl Parser<'a, &'a str, String, Extra<'a>> + Clone {
    let content = just(" ".repeat(width))
        .ignore_then(none_of("\n").repeated().to_slice())
        .then_ignore(choice((just('\n').ignored(), end())))
        .map(|line: &str| line.strip_suffix('\r').unwrap_or(line).to_string());
    let blank = just(' ')
        .repeated()
        .then(just('\r').or_not())
        .then(just('\n'))
        .to(String::new());
    choice((content, blank))
        .repeated()
        .collect::<Vec<String>>()
        .map(|mut lines| {
            while lines.last().is_some_and(String::is_empty) {
                lines.pop();
            }
            lines.join("\n")
        })
}

type Level<'a> = Boxed<'a, 'a, &'a str, Node, Extra<'a>>;

/// The whole frontmatter: a top-level mapping.
fn document<'a>() -> impl Parser<'a, &'a str, Vec<Entry>, Extra<'a>> {
    // Each depth's mapping and sequence, built from the depth below it.
    let mut deeper: Option<(Level<'a>, Level<'a>)> = None;
    for depth in (0..=MAX_DEPTH + 1).rev() {
        let level = match deeper.take() {
            None => {
                let too_deep = just(STEP.repeat(depth))
                    .rewind()
                    .ignore_then(any().repeated())
                    .validate(|(), extra, emitter| {
                        emitter.emit(Rich::custom(
                            extra.span(),
                            "frontmatter nests deeper than a Landmark's readings do",
                        ));
                        Node::Map(Vec::new())
                    })
                    .boxed();
                (too_deep.clone(), too_deep)
            }
            Some((map, seq)) => levels(depth, map, seq),
        };
        deeper = Some(level);
    }
    let (top, _) = deeper.expect("built");
    top.map(|node| match node {
        Node::Map(entries) => entries,
        _ => Vec::new(),
    })
    .then_ignore(skip().repeated())
}

/// The mapping and the sequence whose lines sit `depth` steps in, given those
/// one step deeper.
fn levels<'a>(depth: usize, map: Level<'a>, seq: Level<'a>) -> (Level<'a>, Level<'a>) {
    let indent = just(STEP.repeat(depth));
    let key = choice((
        quoted(),
        none_of(": \t\r\n\"#'!&*[]{}|>%@`,")
            .repeated()
            .at_least(1)
            .to_slice()
            .map(str::to_string),
    ));
    let header = just('|')
        .ignore_then(just('2').or_not())
        .then_ignore(just('-'))
        .ignore_then(note().or_not())
        .then_ignore(eol())
        .then(literal_body(STEP.len() * (depth + 1)))
        .map(|(note, text)| (Node::Literal(text), note));
    // A `#` after the key's space opens a note, so the value is nested below.
    let on_line = just(' ')
        .repeated()
        .at_least(1)
        .then(just('#').not())
        .ignore_then(choice((header, line_value().then_ignore(eol()))));
    let nested = note()
        .or_not()
        .then_ignore(eol())
        .then(choice((seq.clone(), map.clone())))
        .map(|(note, value)| (value, note));
    let entry = indent
        .clone()
        .ignore_then(key.clone())
        .then_ignore(just(':'))
        .then(choice((on_line, nested)))
        .map_with(|(key, (value, note)), extra| Entry {
            key,
            value,
            note,
            at: extra.span().start,
        })
        .labelled("a `key: value` line");
    let mapping = skip()
        .repeated()
        .ignore_then(entry)
        .repeated()
        .at_least(1)
        .collect::<Vec<Entry>>()
        .map(Node::Map)
        .boxed();

    let first = key
        .then_ignore(just(':'))
        .then_ignore(just(' ').repeated().at_least(1))
        .then(line_value())
        .then_ignore(eol())
        .map_with(|(key, (value, note)), extra| Entry {
            key,
            value,
            note,
            at: extra.span().start,
        });
    let item_map = first.then(map.or_not()).map(|(first, rest)| {
        let mut entries = vec![first];
        if let Some(Node::Map(rest)) = rest {
            entries.extend(rest);
        }
        Node::Map(entries)
    });
    let item = indent
        .ignore_then(just('-'))
        .ignore_then(just(' ').repeated().at_least(1))
        .ignore_then(choice((
            item_map,
            line_value().then_ignore(eol()).map(|(value, _)| value),
        )))
        .labelled("a `- item` line");
    let sequence = skip()
        .repeated()
        .ignore_then(item)
        .repeated()
        .at_least(1)
        .collect::<Vec<Node>>()
        .map(Node::Seq)
        .boxed();
    (mapping, sequence)
}

#[cfg(test)]
mod tests {
    use rand::{RngExt, SeedableRng, rngs::StdRng};

    use super::*;

    /// A writer-domain node: what the frontmatter emitter can build.
    fn node(rng: &mut StdRng, depth: usize) -> Node {
        let text = |rng: &mut StdRng, alphabet: &[char]| -> String {
            (0..rng.random_range(1..9))
                .map(|_| alphabet[rng.random_range(0..alphabet.len())])
                .collect()
        };
        let plain: &[char] = &['a', 'b', '0', '9', '.', ':', '-', 'T', 'Z', 'e'];
        let any_text: &[char] = &[
            'a', ' ', '"', '\\', '\n', '#', ':', '[', ']', '{', '}', '-', '|', '\'', 'é',
        ];
        let scalar = |rng: &mut StdRng| match rng.random_range(0..3) {
            0 => {
                let text = text(rng, plain);
                match text.starts_with('-') || text.contains(": ") {
                    true => Node::Quoted(text),
                    false => Node::id(&text),
                }
            }
            1 => Node::Quoted(text(rng, any_text)),
            _ => Node::Plain("null".into()),
        };
        // A mapping's entries sit a step deeper, and an item mapping's later
        // keys two steps deeper, than the value that holds them.
        let kinds = match depth {
            depth if depth + 2 <= MAX_DEPTH => 8,
            depth if depth < MAX_DEPTH => 6,
            _ => 4,
        };
        match rng.random_range(0..kinds) {
            0 => scalar(rng),
            1 => {
                let body = text(rng, any_text);
                Node::text(body.trim_end_matches('\n').to_string())
            }
            2 => Node::Json(serde_json::json!({ "identifier": text(rng, any_text), "n": 3 })),
            3 => Node::Flow(vec![scalar(rng), scalar(rng)]),
            4 => Node::Seq(Vec::new()),
            5 => Node::Map(Vec::new()),
            6 => Node::Seq(
                (0..rng.random_range(1..3))
                    .map(|_| match rng.random_bool(0.5) {
                        true => scalar(rng),
                        false => Node::Map(vec![
                            Entry::new("ref", scalar(rng)),
                            Entry::new("label", Node::Quoted(text(rng, any_text))),
                        ]),
                    })
                    .collect(),
            ),
            _ => Node::Map(entries(rng, depth + 1)),
        }
    }

    fn entries(rng: &mut StdRng, depth: usize) -> Vec<Entry> {
        (0..rng.random_range(1..4))
            .map(|index| {
                let key = match rng.random_bool(0.3) {
                    true => format!("{index:04}"),
                    false => format!("k{index}"),
                };
                let note = rng
                    .random_bool(0.3)
                    .then(|| format!("Note {}", rng.random_range(0..99)));
                Entry::new(key, node(rng, depth)).noted(note)
            })
            .collect()
    }

    #[test]
    fn every_written_node_reads_back_as_itself() {
        for seed in 0..1000 {
            let mut rng = StdRng::seed_from_u64(seed);
            let written = entries(&mut rng, 0);
            let source = write(&written);
            let read = read(&source).unwrap_or_else(|(at, e)| panic!("{source}\nat {at}: {e}"));
            assert_eq!(read, written, "seed {seed}:\n{source}");
            assert_eq!(write(&read), source, "seed {seed}");
        }
    }
}
