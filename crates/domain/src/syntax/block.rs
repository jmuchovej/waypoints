// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The block layer: every [`Content`] variant ↔ its dialect form.
//!
//! See `docs/dialect.md`, which is normative.
//!
//! Maps one block to its own lines and back; it does not walk the tree.
//! [`Rendered::children`] says how children attach.

use chumsky::{input::MapExtra, prelude::*};

use crate::{BlockTheme, Color, Content, MathEngine, RichText, Size, syntax::inline};

use inline::{Extra, GrammarError, Links, Verbatim};

/// What a `map_with` sees: where in the source its output came from.
type Span<'a, 'b> = MapExtra<'a, 'b, &'a str, Extra<'a>>;

/// A block's own lines, plus how its children attach beneath it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    pub lines: Vec<String>,
    pub children: Children,
}

/// How a block's children attach. Any Mark may have children, so every block
/// has an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Children {
    /// Every child line takes this prefix. Blockquotes.
    Prefixed(&'static str),
    /// Children indent by this many spaces. Everything else.
    Indented(usize),
    /// Children sit above this closing line. Container components.
    Closed(&'static str),
}

impl Rendered {
    /// The source to hand back to [`parse`]. An unthemed blockquote has no line
    /// of its own — its marker is the prefix its children wear.
    pub fn opener(&self) -> String {
        match (self.lines.first(), self.children) {
            (Some(line), _) => line.clone(),
            (None, Children::Prefixed(prefix)) => prefix.trim_end().to_string(),
            (None, _) => String::new(),
        }
    }
}

pub const INDENT: usize = 2;

const TABLE_COMPONENT: &str = "table";

/// Render one block to its dialect lines, mention targets written verbatim.
pub fn emit(content: &Content) -> Rendered {
    emit_with(content, &Verbatim)
}

/// Render one block, spelling mention targets through `links`.
pub fn emit_with(content: &Content, links: &dyn Links) -> Rendered {
    let leaf = |line: String| Rendered {
        lines: vec![line],
        children: Children::Indented(INDENT),
    };

    match content {
        Content::Paragraph { rich_text, theme } => {
            leaf(text_line(links, "", rich_text, attrs(theme, &[])))
        }
        Content::Heading1 { rich_text, theme } => {
            leaf(text_line(links, "# ", rich_text, attrs(theme, &[])))
        }
        Content::Heading2 { rich_text, theme } => {
            leaf(text_line(links, "## ", rich_text, attrs(theme, &[])))
        }
        Content::Heading3 { rich_text, theme } => {
            leaf(text_line(links, "### ", rich_text, attrs(theme, &[])))
        }
        Content::ListItem { rich_text, theme } => Rendered {
            lines: vec![text_line(links, "- ", rich_text, attrs(theme, &[]))],
            children: Children::Indented(INDENT),
        },
        // Auto-numbered. Explicit numbers are (pending): `EnumItem` carries no
        // field for one, so `N.` is never emitted.
        Content::EnumItem { rich_text, theme } => Rendered {
            lines: vec![text_line(links, "+ ", rich_text, attrs(theme, &[]))],
            children: Children::Indented(INDENT),
        },
        Content::Divider { theme } => leaf(format!("---{}", attrs(theme, &[]))),

        Content::Blockquote { theme } => Rendered {
            lines: match attrs(theme, &[]).as_str() {
                "" => Vec::new(),
                rendered => vec![format!(">{rendered}")],
            },
            children: Children::Prefixed("> "),
        },
        Content::Callout { icon, theme } => Rendered {
            lines: vec![format!(
                "::callout{}",
                attrs(theme, &[("icon", icon.clone())])
            )],
            children: Children::Closed("::"),
        },

        Content::Code {
            language,
            lines,
            filename,
            highlights,
            theme,
        } => {
            let mut pairs = Vec::new();
            if let Some(filename) = filename {
                pairs.push(("filename", filename.clone()));
            }
            if let Some(highlights) = highlights {
                pairs.push((
                    "highlights",
                    highlights
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ));
            }
            let fence = code_fence(lines);
            let mut out = vec![format!("{fence}{language}{}", attrs(theme, &pairs))];
            out.extend(lines.iter().cloned());
            out.push(fence);
            Rendered {
                lines: out,
                children: Children::Indented(INDENT),
            }
        }

        Content::Math {
            expression,
            engine,
            theme,
        } => {
            let pairs = match engine {
                MathEngine::Typst => Vec::new(),
                MathEngine::LaTeX => vec![("engine", "latex".to_string())],
            };
            let mut out = vec![format!("$${}", attrs(theme, &pairs))];
            out.extend(expression.lines().map(str::to_string));
            out.push("$$".to_string());
            Rendered {
                lines: out,
                children: Children::Indented(INDENT),
            }
        }

        Content::Table { .. } => emit_table(content, links),
    }
}

/// Characters that open another block when they start a bare paragraph line.
const LINE_OPENERS: [char; 6] = ['#', '>', '-', '+', ':', '<'];

/// A prefix, the inline text, and an attribute suffix.
///
/// Text ending in `}` — a literal brace or an inline `{tokens}` suffix — would
/// read as the block's attribute list, so such a line always carries one, even
/// if empty. A bare paragraph opening like another block is escaped.
fn text_line(links: &dyn Links, prefix: &str, rich: &RichText, attrs: String) -> String {
    let mut body = inline::emit_with(rich, links);
    let attrs = if attrs.is_empty() && body.trim_end().ends_with('}') {
        "{}".into()
    } else {
        attrs
    };
    if prefix.is_empty() && body.starts_with(LINE_OPENERS) {
        body.insert(0, '\\');
    }
    format!("{prefix}{body}{attrs}")
}

/// A fence longer than any backtick run in the code, so no line closes it.
fn code_fence(lines: &[String]) -> String {
    let longest = lines
        .iter()
        .map(|line| line.split(|c| c != '`').map(str::len).max().unwrap_or(0))
        .max()
        .unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// `{theme-blue key="value"}`, or `""` when there is nothing to say. The theme
/// token comes first, then pairs in the caller's order.
fn attrs(theme: &Option<BlockTheme>, pairs: &[(&str, String)]) -> String {
    let mut tokens = Vec::new();
    if let Some(theme) = theme {
        tokens.push(format!("theme-{}", color_token(&theme.color)));
    }
    for (key, value) in pairs {
        tokens.push(format!("{key}=\"{}\"", value.replace('"', "\\\"")));
    }
    if tokens.is_empty() {
        return String::new();
    }
    format!("{{{attrs}}}", attrs = tokens.join(" "))
}

/// A colour's token spelling, via serde so this cannot drift from [`Color`].
fn color_token(color: &Color) -> String {
    serde_json::to_value(color)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "default".to_string())
}

/// A colour from its token spelling.
fn color_from(token: &str) -> Option<Color> {
    serde_json::from_value(serde_json::Value::String(token.to_string())).ok()
}

/// A table as GFM pipe rows — wrapped in a `::table` component when the model
/// carries more than GFM can say (anything beyond a header row).
fn emit_table(content: &Content, links: &dyn Links) -> Rendered {
    let Content::Table {
        cells,
        col_size,
        row_size,
        col_header,
        row_header,
        fill_width,
        col_colors,
        row_colors,
        theme,
    } = content
    else {
        unreachable!("emit_table is only called for a table");
    };

    let width = cells.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows: Vec<String> = Vec::new();
    for (index, row) in cells.iter().enumerate() {
        let mut columns: Vec<String> = row.iter().map(|cell| escape_cell(cell, links)).collect();
        columns.resize(width, String::new());
        rows.push(format!("| {} |", columns.join(" | ")));
        if index == 0 && *col_header == Some(true) {
            let dashes = vec!["---".to_string(); width];
            rows.push(format!("| {} |", dashes.join(" | ")));
        }
    }

    let plain_gfm = *col_header == Some(true)
        && col_size.is_none()
        && row_size.is_none()
        && row_header.is_none()
        && fill_width.is_none()
        && col_colors.is_none()
        && row_colors.is_none()
        && theme.is_none();
    if plain_gfm {
        return Rendered {
            lines: rows,
            children: Children::Indented(INDENT),
        };
    }

    let mut pairs = Vec::new();
    if let Some(flag) = col_header {
        pairs.push(("col-header", flag.to_string()));
    }
    if let Some(flag) = row_header {
        pairs.push(("row-header", flag.to_string()));
    }
    if let Some(flag) = fill_width {
        pairs.push(("fill-width", flag.to_string()));
    }
    if let Some(sizes) = col_size {
        pairs.push(("col-size", join_sizes(sizes)));
    }
    if let Some(sizes) = row_size {
        pairs.push(("row-size", join_sizes(sizes)));
    }
    if let Some(colors) = col_colors {
        pairs.push(("col-colors", join_colors(colors)));
    }
    if let Some(colors) = row_colors {
        pairs.push(("row-colors", join_colors(colors)));
    }

    let mut lines = vec![format!("::{TABLE_COMPONENT}{}", attrs(theme, &pairs))];
    lines.extend(rows);
    lines.push("::".to_string());
    Rendered {
        lines,
        children: Children::Indented(INDENT),
    }
}

/// Escape what the row syntax would eat. Every `|` gets a backslash that
/// [`split_cells`] removes again before inline parsing, as GFM does; edge
/// spaces would be trimmed away, so they are escaped for the inline layer.
fn escape_cell(cell: &RichText, links: &dyn Links) -> String {
    let mut out = inline::emit_with(cell, links).replace('|', "\\|");
    // A cell of only `-`/`:` would make its row read as the delimiter row.
    if !out.is_empty() && out.chars().all(|c| c == '-' || c == ':') {
        out.insert(0, '\\');
    }
    // Trailing first: a lone space then needs only the one escape.
    if out.ends_with(' ') {
        out.insert(out.len() - 1, '\\');
    }
    if out.starts_with(' ') {
        out.insert(0, '\\');
    }
    out
}

/// A table row starts here; a line starting with `|` alone may be a spoiler.
pub const TABLE_ROW: &str = "| ";

fn join_sizes(sizes: &[Option<Size>]) -> String {
    sizes
        .iter()
        .map(|size| match size {
            None => String::new(),
            Some(Size::Auto) => "auto".to_string(),
            Some(Size::Min) => "min".to_string(),
            Some(Size::Max) => "max".to_string(),
            Some(Size::Fixed(pixels)) => format!("{pixels}px"),
            Some(Size::Ratio(fraction)) => format!("{fraction}fr"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_sizes(value: &str) -> Vec<Option<Size>> {
    value
        .split(',')
        .map(|token| match token.trim() {
            "" => None,
            "auto" => Some(Size::Auto),
            "min" => Some(Size::Min),
            "max" => Some(Size::Max),
            other => {
                if let Some(pixels) = other.strip_suffix("px") {
                    pixels.parse().ok().map(Size::Fixed)
                } else if let Some(fraction) = other.strip_suffix("fr") {
                    fraction.parse().ok().map(Size::Ratio)
                } else {
                    None
                }
            }
        })
        .collect()
}

fn join_colors(colors: &[Option<Color>]) -> String {
    colors
        .iter()
        .map(|color| color.as_ref().map(color_token).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_colors(value: &str) -> Vec<Option<Color>> {
    value
        .split(',')
        .map(|token| color_from(token.trim()))
        .collect()
}

/// Component names whose body is child Marks rather than the block's own
/// content — the walker consumes their closing `::` after the children.
pub const CONTAINER_COMPONENTS: [&str; 1] = ["callout"];

/// One block's dialect source (prefix and indent already stripped) back into
/// [`Content`]. A container's source is just its opening marker.
pub fn parse(source: &str) -> Result<Content, GrammarError> {
    parse_with(source, &Verbatim)
}

/// [`parse`], reading mention targets back through `links`.
///
/// Only the first line says what the block is; a fenced or tabular block
/// reads its body from the lines after it.
pub fn parse_with(source: &str, links: &dyn Links) -> Result<Content, GrammarError> {
    let lines = lines().parse(source).into_output().unwrap_or_default();
    let (first, _) = lines.first().copied().unwrap_or_default();
    let (opener, errors) = opener().parse(first).into_output_errors();
    if let Some(error) = errors.first() {
        return Err(inline::refusal(source, error));
    }
    let Some(opener) = opener else {
        return Err(GrammarError::new(source, 0, "not a block in this dialect"));
    };
    let rest = lines.get(1..).unwrap_or_default();
    let until = |closer: &'static str| {
        rest.iter()
            .copied()
            .take_while(move |(line, _)| line.trim_end() != closer)
    };
    let read = |text: &str, at: usize| {
        inline::parse_with(text, links).map_err(|error| error.shifted(source, at))
    };

    Ok(match opener {
        Opener::Code {
            fence,
            language,
            attributes,
        } => Content::Code {
            language: language.trim().to_string(),
            lines: rest
                .iter()
                .take_while(|(line, _)| line.trim_end() != fence)
                .map(|(line, _)| (*line).to_string())
                .collect(),
            filename: attributes.get("filename").map(str::to_string),
            highlights: attributes.get("highlights").map(|value| {
                value
                    .split(',')
                    .filter_map(|number| number.trim().parse().ok())
                    .collect()
            }),
            theme: attributes.theme,
        },
        Opener::Math(attributes) => Content::Math {
            expression: until("$$")
                .map(|(line, _)| line)
                .collect::<Vec<_>>()
                .join("\n"),
            engine: match attributes.get("engine") {
                Some("latex") => MathEngine::LaTeX,
                _ => MathEngine::Typst,
            },
            theme: attributes.theme,
        },
        Opener::Component(TABLE_COMPONENT, attributes) => {
            table(source, until("::"), Some(attributes), links)?
        }
        Opener::Component(_, attributes) => Content::Callout {
            icon: attributes.get("icon").unwrap_or_default().to_string(),
            theme: attributes.theme,
        },
        Opener::Table => table(source, lines.iter().copied(), None, links)?,
        Opener::Quote(attributes) => Content::Blockquote {
            theme: attributes.theme,
        },
        Opener::Text(Text::Paragraph, text, _, attributes) if text.trim() == "---" => {
            Content::Divider {
                theme: attributes.theme,
            }
        }
        Opener::Text(kind, text, at, attributes) => {
            let rich_text = read(text, at)?;
            let theme = attributes.theme;
            match kind {
                Text::Paragraph => Content::Paragraph { rich_text, theme },
                Text::Heading1 => Content::Heading1 { rich_text, theme },
                Text::Heading2 => Content::Heading2 { rich_text, theme },
                Text::Heading3 => Content::Heading3 { rich_text, theme },
                Text::ListItem => Content::ListItem { rich_text, theme },
                Text::EnumItem => Content::EnumItem { rich_text, theme },
            }
        }
    })
}

/// What an attribute list said.
#[derive(Clone, Debug, Default)]
struct Attributes {
    theme: Option<BlockTheme>,
    pairs: Vec<(String, String)>,
}

impl Attributes {
    fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    }

    fn flag(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|value| value.parse().ok())
    }
}

#[derive(Clone, Copy, Debug)]
enum Text {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    ListItem,
    EnumItem,
}

/// What a block's first line opens.
#[derive(Clone, Debug)]
enum Opener<'a> {
    Code {
        fence: &'a str,
        language: &'a str,
        attributes: Attributes,
    },
    Math(Attributes),
    /// A component's name, already known to be one the dialect has.
    Component(&'a str, Attributes),
    Table,
    Quote(Attributes),
    /// Inline text, the byte it starts at, and its block's attributes.
    Text(Text, &'a str, usize, Attributes),
}

/// The lines `str::lines` would give, each with the byte it starts at.
fn lines<'a>() -> impl Parser<'a, &'a str, Vec<(&'a str, usize)>, Extra<'a>> {
    let terminated = none_of('\n')
        .repeated()
        .to_slice()
        .map_with(|line: &str, extra: &mut Span<'a, '_>| {
            (line.strip_suffix('\r').unwrap_or(line), extra.span().start)
        })
        .then_ignore(just('\n'));
    let last = none_of('\n')
        .repeated()
        .at_least(1)
        .to_slice()
        .map_with(|line, extra: &mut Span<'a, '_>| (line, extra.span().start));
    terminated
        .repeated()
        .collect::<Vec<_>>()
        .then(last.or_not())
        .map(|(mut lines, last)| {
            lines.extend(last);
            lines
        })
}

/// A block's first line, by the prefix that opens it.
fn opener<'a>() -> impl Parser<'a, &'a str, Opener<'a>, Extra<'a>> {
    let text = |prefix: &'static str, kind: Text| {
        just(prefix)
            .ignore_then(attributed())
            .map(move |((text, at), attributes)| Opener::Text(kind, text, at, attributes))
    };
    choice((
        just('`')
            .repeated()
            .at_least(3)
            .to_slice()
            .then(attributed())
            .map(|(fence, ((language, _), attributes))| Opener::Code {
                fence,
                language,
                attributes,
            }),
        just("$$")
            .ignore_then(attributed())
            .map(|(_, attributes)| Opener::Math(attributes)),
        just("::")
            .ignore_then(attributed())
            .validate(|((name, at), attributes), _, emitter| {
                let trimmed = name.trim();
                if !["callout", TABLE_COMPONENT].contains(&trimmed) {
                    let start = at + name.len() - name.trim_start().len();
                    emitter.emit(Rich::custom(
                        SimpleSpan::from(start..start + trimmed.len()),
                        format!("`::{trimmed}` is not a block component in this dialect"),
                    ));
                }
                Opener::Component(trimmed, attributes)
            }),
        just(TABLE_ROW)
            .rewind()
            .ignore_then(any().repeated())
            .map(|()| Opener::Table),
        just('>')
            .ignore_then(attributed())
            .map(|(_, attributes)| Opener::Quote(attributes)),
        text("### ", Text::Heading3),
        text("## ", Text::Heading2),
        text("# ", Text::Heading1),
        text("- ", Text::ListItem),
        text("+ ", Text::EnumItem),
        attributed()
            .map(|((text, at), attributes)| Opener::Text(Text::Paragraph, text, at, attributes)),
    ))
}

/// The rest of a line: its text, the byte that text starts at, and the
/// attribute list that ends the line, if one does.
fn attributed<'a>() -> impl Parser<'a, &'a str, ((&'a str, usize), Attributes), Extra<'a>> + Clone {
    any()
        .and_is(attribute_list().not())
        .repeated()
        .to_slice()
        .map_with(|text, extra| (text, extra.span().start))
        .then(attribute_list().or_not().map(Option::unwrap_or_default))
}

/// `{…}` reaching the end of the line, trailing whitespace aside. A `\` escapes
/// the character after it, and a `{` or `}` inside quotes is the value's; so
/// the list is the last brace group no quote or backslash claims.
fn attribute_list<'a>() -> impl Parser<'a, &'a str, Attributes, Extra<'a>> + Clone {
    let escaped = just('\\').then(any()).ignored();
    let quoted = just('"')
        .then(choice((escaped, none_of("\"\\").ignored())).repeated())
        .then(just('"'))
        .ignored();
    let close = just('}')
        .then(any().filter(|c: &char| c.is_whitespace()).repeated())
        .then(end());
    let inner_brace = close.not().ignore_then(just('}')).ignored();
    just('{')
        .ignore_then(
            choice((escaped, quoted, inner_brace, none_of("{}\"\\").ignored()))
                .repeated()
                .to_slice()
                .map_with(|interior, extra: &mut Span<'a, '_>| (interior, extra.span().start)),
        )
        .then_ignore(close)
        .validate(|(interior, at), _, emitter| {
            let mut out = Attributes::default();
            let words = words().parse(interior).into_output().unwrap_or_default();
            for (word, span) in words {
                let span = SimpleSpan::from(at + span.start..at + span.end);
                if let Some((key, value)) = word.split_once('=') {
                    let value = value.trim().trim_matches('"').replace("\\\"", "\"");
                    out.pairs.push((key.trim().to_string(), value));
                } else if let Some(color) = word.strip_prefix("theme-") {
                    match color_from(color) {
                        Some(color) => out.theme = Some(BlockTheme { color }),
                        None => emitter.emit(Rich::custom(
                            SimpleSpan::from(span.start + "theme-".len()..span.end),
                            format!("`{color}` is not a block theme colour"),
                        )),
                    }
                } else {
                    emitter.emit(Rich::custom(
                        span,
                        format!("`{word}` is not a block attribute in this dialect"),
                    ));
                }
            }
            out
        })
}

/// An attribute list's words: whitespace separates them except inside quotes,
/// where a `\` keeps the character after it from closing the quote.
fn words<'a>() -> impl Parser<'a, &'a str, Vec<(&'a str, SimpleSpan)>, Extra<'a>> {
    let space = any().filter(|c: &char| c.is_whitespace());
    let inside = choice((
        just('\\').then(any()).ignored(),
        just('\\').ignored(),
        none_of("\"\\").ignored(),
    ));
    let quoted = just('"')
        .then(inside.repeated())
        .then(just('"').ignored().or(end()));
    let bare = any().filter(|c: &char| !c.is_whitespace() && *c != '"');
    let word = choice((quoted.ignored(), bare.ignored()))
        .repeated()
        .at_least(1)
        .to_slice()
        .map_with(|word, extra| (word, extra.span()));
    space
        .repeated()
        .ignore_then(word.then_ignore(space.repeated()).repeated().collect())
}

/// GFM pipe rows into cells, with any wrapper attributes applied over the top.
fn table<'a>(
    source: &str,
    lines: impl Iterator<Item = (&'a str, usize)>,
    attributes: Option<Attributes>,
    links: &dyn Links,
) -> Result<Content, GrammarError> {
    let mut cells: Vec<Vec<RichText>> = Vec::new();
    let mut saw_delimiter = false;
    for (line, at) in lines {
        let trimmed = line.trim();
        let at = at + line.len() - line.trim_start().len();
        match row().parse(trimmed).into_output() {
            Some(Row::Delimiter) => saw_delimiter = true,
            Some(Row::Cells(row)) => cells.push(
                row.into_iter()
                    .map(|(cell, start)| {
                        let start = at + start + cell.len() - cell.trim_start().len();
                        inline::parse_with(&cell_text(&cell), links)
                            .map_err(|error| error.shifted(source, start))
                    })
                    .collect::<Result<_, _>>()?,
            ),
            Some(Row::Other) | None => {}
        }
    }

    let attributes = attributes.unwrap_or_default();
    // An explicit `col-header` wins over the delimiter-row inference.
    let col_header =
        attributes
            .flag("col-header")
            .or(if saw_delimiter { Some(true) } else { None });
    Ok(Content::Table {
        cells,
        col_size: attributes.get("col-size").map(parse_sizes),
        row_size: attributes.get("row-size").map(parse_sizes),
        col_header,
        row_header: attributes.flag("row-header"),
        fill_width: attributes.flag("fill-width"),
        col_colors: attributes.get("col-colors").map(parse_colors),
        row_colors: attributes.get("row-colors").map(parse_colors),
        theme: attributes.theme,
    })
}

/// One trimmed line of a table.
#[derive(Clone, Debug)]
enum Row {
    /// `| --- | :-: |`: every cell dashes and colons.
    Delimiter,
    /// Each cell's text with `\|` read as `|`, and the byte it starts at.
    Cells(Vec<(String, usize)>),
    /// Not a row at all.
    Other,
}

/// A trimmed line as a table row. The run of pipes that ends the line closes
/// the row, so a `\` before it escapes nothing.
fn row<'a>() -> impl Parser<'a, &'a str, Row, Extra<'a>> {
    let edge = just('|').repeated().at_least(1).then(end());
    let space = any().filter(|c: &char| c.is_whitespace()).repeated();
    let dashes = space.then(one_of("-:").repeated().at_least(1)).then(space);
    let delimiter = just('|')
        .repeated()
        .at_least(1)
        .ignore_then(dashes.separated_by(just('|')).at_least(1))
        .then_ignore(just('|').repeated())
        .then_ignore(end())
        .map(|()| Row::Delimiter);

    let pipe = just('\\').ignore_then(edge.not().ignore_then(just('|')));
    let cell = choice((pipe, none_of('|')))
        .repeated()
        .collect::<String>()
        .map_with(|cell, extra: &mut Span<'a, '_>| (cell, extra.span().start));
    let cells = just('|')
        .repeated()
        .at_least(1)
        .ignore_then(
            cell.separated_by(edge.not().ignore_then(just('|')))
                .collect(),
        )
        .then_ignore(edge.or_not())
        .then_ignore(end())
        .map(Row::Cells);

    choice((delimiter, cells, any().repeated().map(|()| Row::Other)))
}

/// A cell's inline source. Content never ends in an odd run of backslashes,
/// so one here escapes a trailing space the row's spacing trimmed away.
fn cell_text(cell: &str) -> String {
    let mut text = cell.trim().to_string();
    if text.chars().rev().take_while(|c| *c == '\\').count() % 2 == 1 {
        text.push(' ');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InlineMark, TextMarkKind, TextSegment};

    fn rich(text: &str) -> RichText {
        RichText::new(vec![TextSegment {
            text: text.into(),
            marks: Vec::new(),
        }])
    }

    fn themed(color: Color) -> Option<BlockTheme> {
        Some(BlockTheme { color })
    }

    fn only(content: &Content) -> String {
        emit(content).lines.join("\n")
    }

    #[test]
    fn a_paragraph_is_the_bare_line() {
        let block = Content::Paragraph {
            rich_text: rich("hello"),
            theme: None,
        };
        assert_eq!(only(&block), "hello");
        assert_eq!(emit(&block).children, Children::Indented(INDENT));
    }

    #[test]
    fn headings_are_atx_and_stop_at_three() {
        assert_eq!(
            only(&Content::Heading1 {
                rich_text: rich("one"),
                theme: None
            }),
            "# one"
        );
        assert_eq!(
            only(&Content::Heading2 {
                rich_text: rich("two"),
                theme: None
            }),
            "## two"
        );
        assert_eq!(
            only(&Content::Heading3 {
                rich_text: rich("three"),
                theme: None
            }),
            "### three"
        );
    }

    #[test]
    fn a_theme_is_one_coupled_token() {
        assert_eq!(
            only(&Content::Heading1 {
                rich_text: rich("hi"),
                theme: themed(Color::Blue)
            }),
            "# hi{theme-blue}"
        );
    }

    #[test]
    fn bullets_are_dashes_and_enums_are_pluses() {
        let list = Content::ListItem {
            rich_text: rich("one"),
            theme: None,
        };
        assert_eq!(only(&list), "- one");
        assert_eq!(emit(&list).children, Children::Indented(2));

        let enumerated = Content::EnumItem {
            rich_text: rich("one"),
            theme: None,
        };
        assert_eq!(only(&enumerated), "+ one");
        assert_eq!(emit(&enumerated).children, Children::Indented(2));
    }

    #[test]
    fn a_blockquote_is_a_prefix_with_no_line_of_its_own() {
        let plain = emit(&Content::Blockquote { theme: None });
        assert!(plain.lines.is_empty(), "nothing to say, so nothing is said");
        assert_eq!(plain.children, Children::Prefixed("> "));

        let themed = emit(&Content::Blockquote {
            theme: themed(Color::Green),
        });
        assert_eq!(themed.lines, vec![">{theme-green}"]);
    }

    #[test]
    fn a_callout_is_a_block_component() {
        let block = Content::Callout {
            icon: "warning".into(),
            theme: themed(Color::Red),
        };
        assert_eq!(only(&block), "::callout{theme-red icon=\"warning\"}");
        assert_eq!(emit(&block).children, Children::Closed("::"));
    }

    #[test]
    fn a_divider_is_exactly_three_dashes() {
        assert_eq!(only(&Content::Divider { theme: None }), "---");
    }

    #[test]
    fn code_is_fenced_and_tagged_with_its_attributes_on_the_fence() {
        let block = Content::Code {
            language: "rust".into(),
            lines: vec!["fn main() {}".into()],
            filename: Some("main.rs".into()),
            highlights: Some(vec![1]),
            theme: None,
        };
        assert_eq!(
            only(&block),
            "```rust{filename=\"main.rs\" highlights=\"1\"}\nfn main() {}\n```"
        );
    }

    #[test]
    fn math_says_its_engine_only_when_it_is_not_the_default() {
        let typst = Content::Math {
            expression: "x + y".into(),
            engine: MathEngine::Typst,
            theme: None,
        };
        assert_eq!(only(&typst), "$$\nx + y\n$$");

        let latex = Content::Math {
            expression: "x + y".into(),
            engine: MathEngine::LaTeX,
            theme: None,
        };
        assert_eq!(only(&latex), "$${engine=\"latex\"}\nx + y\n$$");
    }

    #[test]
    fn a_plain_table_is_plain_gfm() {
        let block = Content::Table {
            cells: vec![vec![rich("a"), rich("b")], vec![rich("1"), rich("2")]],
            col_size: None,
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        };
        assert_eq!(only(&block), "| a | b |\n| --- | --- |\n| 1 | 2 |");
    }

    #[test]
    fn a_table_carrying_more_than_gfm_can_say_wraps_itself() {
        let block = Content::Table {
            cells: vec![vec![rich("a"), rich("b")]],
            col_size: Some(vec![Some(Size::Fixed(200)), Some(Size::Auto)]),
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        };
        let rendered = only(&block);
        assert!(
            rendered.starts_with("::table{col-header=\"true\" col-size=\"200px,auto\"}"),
            "got {rendered}"
        );
        assert!(rendered.ends_with("\n::"), "got {rendered}");
    }

    #[test]
    fn a_headerless_table_cannot_be_plain_gfm() {
        let block = Content::Table {
            cells: vec![vec![rich("a")]],
            col_size: None,
            row_size: None,
            col_header: None,
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        };
        assert!(only(&block).starts_with("::table"), "{}", only(&block));
    }

    #[test]
    fn a_pipe_in_a_cell_is_escaped() {
        let block = Content::Table {
            cells: vec![vec![rich("a|b")]],
            col_size: None,
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        };
        assert!(only(&block).contains("a\\|b"), "{}", only(&block));
    }

    #[test]
    fn a_text_ending_in_a_brace_does_not_read_as_attributes() {
        let block = Content::Paragraph {
            rich_text: rich("set x = {1}"),
            theme: None,
        };
        assert_eq!(only(&block), "set x = {1}{}");
        let spaced = Content::Paragraph {
            rich_text: rich("set x = {1} "),
            theme: None,
        };
        assert_eq!(only(&spaced), "set x = {1} {}");
    }

    #[test]
    fn inline_marks_reach_the_block_line() {
        let block = Content::Paragraph {
            rich_text: RichText::new(vec![TextSegment {
                text: "hi".into(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                }],
            }]),
            theme: None,
        };
        assert_eq!(only(&block), "*hi*");
    }

    #[test]
    fn sizes_and_colors_round_trip_through_their_tokens() {
        let sizes = vec![
            Some(Size::Fixed(200)),
            Some(Size::Auto),
            Some(Size::Min),
            Some(Size::Max),
            None,
        ];
        assert_eq!(parse_sizes(&join_sizes(&sizes)), sizes);

        let colors = vec![Some(Color::Blue), None, Some(Color::Red)];
        assert_eq!(parse_colors(&join_colors(&colors)), colors);
    }

    #[test]
    fn color_tokens_follow_the_enum() {
        assert_eq!(color_token(&Color::Blue), "blue");
        assert_eq!(color_from("blue"), Some(Color::Blue));
        assert_eq!(color_from("chartreuse"), None);
    }

    /// Parses from the block's own lines, or from its opener where it has none.
    fn survives(content: Content) {
        let rendered = emit(&content);
        let source = match rendered.lines.is_empty() {
            true => rendered.opener(),
            false => rendered.lines.join("\n"),
        };
        let parsed = parse(&source).unwrap_or_else(|e| panic!("parse {source:?}: {e}"));
        assert_eq!(parsed, content, "via {source:?}");
        assert_eq!(emit(&parsed).lines, rendered.lines);
    }

    #[test]
    fn every_content_variant_survives_its_block_form() {
        survives(Content::Paragraph {
            rich_text: rich("hello"),
            theme: None,
        });
        survives(Content::Paragraph {
            rich_text: rich("themed"),
            theme: themed(Color::Purple),
        });
        survives(Content::Heading1 {
            rich_text: rich("one"),
            theme: None,
        });
        survives(Content::Heading2 {
            rich_text: rich("two"),
            theme: themed(Color::Blue),
        });
        survives(Content::Heading3 {
            rich_text: rich("three"),
            theme: None,
        });
        survives(Content::Blockquote { theme: None });
        survives(Content::Blockquote {
            theme: themed(Color::Green),
        });
        survives(Content::ListItem {
            rich_text: rich("bullet"),
            theme: None,
        });
        survives(Content::EnumItem {
            rich_text: rich("numbered"),
            theme: None,
        });
        survives(Content::Callout {
            icon: "warning".into(),
            theme: themed(Color::Red),
        });
        survives(Content::Divider { theme: None });
        survives(Content::Divider {
            theme: themed(Color::Gray),
        });
        survives(Content::Code {
            language: "rust".into(),
            lines: vec!["fn main() {}".into(), "// two lines".into()],
            filename: Some("main.rs".into()),
            highlights: Some(vec![1, 4, 5]),
            theme: None,
        });
        survives(Content::Code {
            language: String::new(),
            lines: vec!["plain".into()],
            filename: None,
            highlights: None,
            theme: None,
        });
        survives(Content::Math {
            expression: "x + y".into(),
            engine: MathEngine::Typst,
            theme: None,
        });
        survives(Content::Math {
            expression: "\\frac{1}{2}".into(),
            engine: MathEngine::LaTeX,
            theme: None,
        });
    }

    #[test]
    fn tables_survive_in_both_spellings() {
        survives(Content::Table {
            cells: vec![vec![rich("a"), rich("b")], vec![rich("1"), rich("2")]],
            col_size: None,
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        });
        survives(Content::Table {
            cells: vec![vec![rich("a"), rich("b")]],
            col_size: Some(vec![Some(Size::Fixed(200)), Some(Size::Auto)]),
            row_size: Some(vec![None]),
            col_header: Some(true),
            row_header: Some(false),
            fill_width: Some(true),
            col_colors: Some(vec![Some(Color::Blue), None]),
            row_colors: Some(vec![Some(Color::Red)]),
            theme: themed(Color::Yellow),
        });
        survives(Content::Table {
            cells: vec![
                vec![rich(" "), rich(" a "), rich("b\\ ")],
                vec![rich("-"), rich(":-:"), rich("--")],
            ],
            col_size: None,
            row_size: None,
            col_header: Some(false),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        });
    }

    #[test]
    fn inline_content_survives_inside_a_block() {
        survives(Content::Paragraph {
            rich_text: RichText::new(vec![
                TextSegment {
                    text: "a ".into(),
                    marks: Vec::new(),
                },
                TextSegment {
                    text: "bold".into(),
                    marks: vec![InlineMark {
                        kind: TextMarkKind::Bold,
                        data: None,
                    }],
                },
                TextSegment {
                    text: " end".into(),
                    marks: Vec::new(),
                },
            ]),
            theme: None,
        });
    }

    #[test]
    fn a_brace_ending_text_survives() {
        survives(Content::Paragraph {
            rich_text: rich("set x = {1}"),
            theme: None,
        });
    }

    #[test]
    fn a_bare_gfm_table_infers_its_header_from_the_delimiter_row() {
        let parsed = parse("| a | b |\n| --- | --- |\n| 1 | 2 |").expect("parse");
        let Content::Table {
            col_header, cells, ..
        } = parsed
        else {
            panic!("expected a table");
        };
        assert_eq!(col_header, Some(true));
        assert_eq!(cells.len(), 2, "the delimiter row is not a row of cells");
    }

    #[test]
    fn unknown_blocks_and_attributes_are_refused() {
        assert!(parse("::nonsense{}").is_err());
        assert!(parse("# hi{theme-chartreuse}").is_err());
        assert!(parse("# hi{nonsense}").is_err());
    }

    #[test]
    fn a_callout_is_a_container_component() {
        assert!(CONTAINER_COMPONENTS.contains(&"callout"));
        assert!(
            !CONTAINER_COMPONENTS.contains(&TABLE_COMPONENT),
            "a table's rows are its own content, not child Marks"
        );
    }

    /// The emitter's branches as a checklist — every variant themed and
    /// unthemed, every attribute key, the `{}` suffix, every line opener and
    /// every cell escape — each read back to the value it came from.
    #[test]
    fn every_block_form_the_emitter_writes_reads_back() {
        type Text = fn(RichText, Option<BlockTheme>) -> Content;
        let text_kinds: [Text; 6] = [
            |rich_text, theme| Content::Paragraph { rich_text, theme },
            |rich_text, theme| Content::Heading1 { rich_text, theme },
            |rich_text, theme| Content::Heading2 { rich_text, theme },
            |rich_text, theme| Content::Heading3 { rich_text, theme },
            |rich_text, theme| Content::ListItem { rich_text, theme },
            |rich_text, theme| Content::EnumItem { rich_text, theme },
        ];
        let spanned = RichText::new(vec![TextSegment {
            text: "hi".into(),
            marks: vec![
                InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                },
                InlineMark {
                    kind: TextMarkKind::Italic,
                    data: None,
                },
            ],
        }]);
        let table = |cells: Vec<Vec<RichText>>, col_header, theme| Content::Table {
            cells,
            col_size: None,
            row_size: None,
            col_header,
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme,
        };

        for theme in [None, themed(Color::Orange)] {
            for build in text_kinds {
                survives(build(rich("hello"), theme.clone()));
                survives(build(rich("set x = {1}"), theme.clone()));
                survives(build(rich("set x = {1} "), theme.clone()));
                survives(build(spanned.clone(), theme.clone()));
                survives(build(rich("# - > + : < not openers here"), theme.clone()));
            }
            survives(Content::Divider {
                theme: theme.clone(),
            });
            survives(Content::Blockquote {
                theme: theme.clone(),
            });
            survives(Content::Callout {
                icon: "a \"quoted\" icon".into(),
                theme: theme.clone(),
            });
            survives(Content::Callout {
                icon: String::new(),
                theme: theme.clone(),
            });
            survives(Content::Code {
                language: "rust".into(),
                lines: vec!["let s = \"```\";".into(), "".into(), "}".into()],
                filename: Some("src/main \"x\".rs".into()),
                highlights: Some(vec![1, 3]),
                theme: theme.clone(),
            });
            survives(Content::Code {
                language: String::new(),
                lines: Vec::new(),
                filename: None,
                highlights: Some(Vec::new()),
                theme: theme.clone(),
            });
            for engine in [MathEngine::Typst, MathEngine::LaTeX] {
                survives(Content::Math {
                    expression: "\\frac{1}{2}\n+ 3".into(),
                    engine,
                    theme: theme.clone(),
                });
            }
            survives(table(
                vec![vec![rich("a"), rich("b")], vec![rich("1"), rich("2")]],
                Some(true),
                theme.clone(),
            ));
            survives(table(vec![vec![rich("a")]], None, theme.clone()));
            survives(table(vec![vec![rich("a")]], Some(false), theme.clone()));
            survives(Content::Table {
                cells: vec![vec![rich("a"), rich("b"), rich("c")]],
                col_size: Some(vec![
                    Some(Size::Fixed(200)),
                    Some(Size::Ratio(1.5)),
                    Some(Size::Min),
                ]),
                row_size: Some(vec![Some(Size::Max), None, Some(Size::Auto)]),
                col_header: Some(true),
                row_header: Some(true),
                fill_width: Some(false),
                col_colors: Some(vec![Some(Color::Blue), None, Some(Color::Pink)]),
                row_colors: Some(vec![None]),
                theme: theme.clone(),
            });
        }

        for opener in LINE_OPENERS {
            survives(Content::Paragraph {
                rich_text: rich(&format!("{opener} x")),
                theme: None,
            });
            survives(Content::Paragraph {
                rich_text: rich(&format!("{opener}{opener}{opener}")),
                theme: None,
            });
        }
        for text in ["---", "| a", "``` x", "$$ x", "::x", "::"] {
            survives(Content::Paragraph {
                rich_text: rich(text),
                theme: None,
            });
        }

        let cells = [
            "a|b", "|", "-", "---", ":-:", "--:", " a", "b ", " ", "b\\ ", "\\", "*x*",
        ];
        let mut row: Vec<RichText> = cells.iter().map(|cell| rich(cell)).collect();
        row.push(RichText::default());
        survives(table(vec![row], Some(false), None));
        survives(table(
            vec![
                vec![rich("h")],
                cells.iter().take(1).map(|cell| rich(cell)).collect(),
            ],
            Some(true),
            None,
        ));
    }
}
