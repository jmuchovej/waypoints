//! The block layer: every [`Content`] variant ↔ its dialect form.
//!
//! See `docs/dialect.md`, which is normative.
//!
//! Maps one block to its own lines and back; it does not walk the tree.
//! [`Rendered::children`] says how children attach.

use crate::{BlockTheme, Color, Content, MathEngine, RichText, Size, syntax::inline};

use inline::GrammarError;

/// A block's own lines, plus how its children attach beneath it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    /// The lines the block itself contributes, in order.
    pub lines: Vec<String>,
    /// How a child's lines attach under this block.
    pub children: Children,
}

/// How a block's children attach to it — the walker's instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Children {
    /// A leaf. Nothing attaches.
    None,
    /// Every child line takes this prefix. Blockquotes.
    Prefixed(&'static str),
    /// Children indent by this many spaces. List and enum items.
    Indented(usize),
    /// Children sit above this closing line. Block components.
    Closed(&'static str),
}

impl Rendered {
    /// The source a walker hands back to [`parse`] to recover this block.
    ///
    /// Usually the first of [`lines`](Rendered::lines) — but a blockquote
    /// carrying no theme has no line of its own, because its marker *is* the
    /// `> ` prefix its children already wear. Writing a bare `>` above the quote
    /// just to have something to parse would be a line of noise in every
    /// quoted passage, so the walker recovers the block from the prefix
    /// instead, and this is where that correspondence is stated once.
    pub fn opener(&self) -> String {
        match (self.lines.first(), self.children) {
            (Some(line), _) => line.clone(),
            (None, Children::Prefixed(prefix)) => prefix.trim_end().to_string(),
            (None, _) => String::new(),
        }
    }
}

/// The dialect's one indentation step.
pub const INDENT: usize = 2;

/// The component name a themed table wraps itself in.
const TABLE_COMPONENT: &str = "table";

// ── Emit ─────────────────────────────────────────────────────────────────────

/// Render one block to its dialect lines.
pub fn emit(content: &Content) -> Rendered {
    let leaf = |line: String| Rendered {
        lines: vec![line],
        children: Children::None,
    };

    match content {
        Content::Paragraph { rich_text, theme } => {
            leaf(text_line("", rich_text, attrs(theme, &[])))
        }
        Content::Heading1 { rich_text, theme } => {
            leaf(text_line("# ", rich_text, attrs(theme, &[])))
        }
        Content::Heading2 { rich_text, theme } => {
            leaf(text_line("## ", rich_text, attrs(theme, &[])))
        }
        Content::Heading3 { rich_text, theme } => {
            leaf(text_line("### ", rich_text, attrs(theme, &[])))
        }
        Content::ListItem { rich_text, theme } => Rendered {
            lines: vec![text_line("- ", rich_text, attrs(theme, &[]))],
            children: Children::Indented(INDENT),
        },
        // Auto-numbered. Explicit numbers are (pending): `EnumItem` carries no
        // field for one, so `N.` is never emitted.
        Content::EnumItem { rich_text, theme } => Rendered {
            lines: vec![text_line("+ ", rich_text, attrs(theme, &[]))],
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
            let mut out = vec![format!("```{language}{}", attrs(theme, &pairs))];
            out.extend(lines.iter().cloned());
            out.push("```".to_string());
            Rendered {
                lines: out,
                children: Children::None,
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
                children: Children::None,
            }
        }

        Content::Table { .. } => emit_table(content),
    }
}

/// A prefix, the inline text, and an attribute suffix.
///
/// A text ending in `}` would read as an attribute list, so when the block has
/// no attributes of its own that brace is escaped.
fn text_line(prefix: &str, rich: &RichText, attrs: String) -> String {
    let mut body = inline::emit(rich);
    if attrs.is_empty() && body.ends_with('}') {
        body.pop();
        body.push_str("\\}");
    }
    format!("{prefix}{body}{attrs}")
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
    format!("{{{}}}", tokens.join(" "))
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
fn emit_table(content: &Content) -> Rendered {
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
        let mut columns: Vec<String> = row.iter().map(|cell| escape_cell(cell)).collect();
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
            children: Children::None,
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
        children: Children::None,
    }
}

/// A pipe inside a cell would end it, and a newline would end the row.
fn escape_cell(cell: &RichText) -> String {
    inline::emit(cell).replace('|', "\\|").replace('\n', " ")
}

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
    let lines: Vec<&str> = source.lines().collect();
    let first = lines.first().copied().unwrap_or_default();

    if let Some(rest) = first.strip_prefix("```") {
        return parse_code(rest, &lines[1..]);
    }
    if let Some(rest) = first.strip_prefix("$$") {
        return parse_math(rest, &lines[1..]);
    }
    if let Some(rest) = first.strip_prefix("::") {
        return parse_component(rest, &lines[1..]);
    }
    if first.starts_with('|') {
        return parse_gfm_table(&lines, None, None);
    }
    if first.starts_with('>') {
        let (_, attributes) = split_attrs(&first[1..])?;
        return Ok(Content::Blockquote {
            theme: attributes.theme,
        });
    }
    for (prefix, build) in HEADINGS {
        if let Some(rest) = first.strip_prefix(prefix) {
            let (text, attributes) = split_attrs(rest)?;
            return Ok(build(inline::parse(&text)?, attributes.theme));
        }
    }
    if let Some(rest) = first.strip_prefix("- ") {
        let (text, attributes) = split_attrs(rest)?;
        return Ok(Content::ListItem {
            rich_text: inline::parse(&text)?,
            theme: attributes.theme,
        });
    }
    if let Some(rest) = first.strip_prefix("+ ") {
        let (text, attributes) = split_attrs(rest)?;
        return Ok(Content::EnumItem {
            rich_text: inline::parse(&text)?,
            theme: attributes.theme,
        });
    }

    let (text, attributes) = split_attrs(first)?;
    if text.trim() == "---" {
        return Ok(Content::Divider {
            theme: attributes.theme,
        });
    }
    Ok(Content::Paragraph {
        rich_text: inline::parse(&text)?,
        theme: attributes.theme,
    })
}

/// Heading prefixes, longest first so `###` wins over `#`.
#[allow(clippy::type_complexity)]
const HEADINGS: [(&str, fn(RichText, Option<BlockTheme>) -> Content); 3] = [
    ("### ", |rich_text, theme| Content::Heading3 {
        rich_text,
        theme,
    }),
    ("## ", |rich_text, theme| Content::Heading2 {
        rich_text,
        theme,
    }),
    ("# ", |rich_text, theme| Content::Heading1 {
        rich_text,
        theme,
    }),
];

/// What an attribute list said.
#[derive(Debug, Default)]
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

/// Split a trailing `{…}` off a line. The emitter escapes a brace that is text,
/// so an unescaped trailing one is a real list and an unparsable one an error.
fn split_attrs(line: &str) -> Result<(String, Attributes), GrammarError> {
    let trimmed = line.trim_end();
    if !trimmed.ends_with('}') || trimmed.ends_with("\\}") {
        return Ok((line.to_string(), Attributes::default()));
    }
    let bytes = trimmed.as_bytes();
    let mut open = None;
    for index in (0..bytes.len()).rev() {
        if bytes[index] == b'{' && (index == 0 || bytes[index - 1] != b'\\') {
            open = Some(index);
            break;
        }
    }
    let Some(open) = open else {
        return Ok((line.to_string(), Attributes::default()));
    };
    let interior = &trimmed[open + 1..trimmed.len() - 1];
    Ok((trimmed[..open].to_string(), parse_attrs(interior, open)?))
}

/// `theme-blue key="value"` — bare tokens and quoted pairs.
fn parse_attrs(interior: &str, at: usize) -> Result<Attributes, GrammarError> {
    let mut out = Attributes::default();
    for token in split_tokens(interior) {
        if let Some((key, value)) = token.split_once('=') {
            let value = value.trim().trim_matches('"').replace("\\\"", "\"");
            out.pairs.push((key.trim().to_string(), value));
            continue;
        }
        if let Some(color) = token.strip_prefix("theme-") {
            let Some(color) = color_from(color) else {
                return Err(GrammarError {
                    detail: format!("`{color}` is not a block theme colour"),
                    at,
                });
            };
            out.theme = Some(BlockTheme { color });
            continue;
        }
        return Err(GrammarError {
            detail: format!("`{token}` is not a block attribute in this dialect"),
            at,
        });
    }
    Ok(out)
}

/// Whitespace-separated tokens, except inside quotes.
fn split_tokens(interior: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in interior.chars() {
        match character {
            '\\' if quoted && !escaped => {
                escaped = true;
                current.push(character);
            }
            '"' if !escaped => {
                quoted = !quoted;
                current.push(character);
            }
            character if character.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                escaped = false;
            }
            character => {
                current.push(character);
                escaped = false;
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn parse_code(fence: &str, rest: &[&str]) -> Result<Content, GrammarError> {
    let (language, attributes) = split_attrs(fence)?;
    let body: Vec<String> = rest
        .iter()
        .take_while(|line| !line.trim_end().starts_with("```"))
        .map(|line| (*line).to_string())
        .collect();
    Ok(Content::Code {
        language: language.trim().to_string(),
        lines: body,
        filename: attributes.get("filename").map(str::to_string),
        highlights: attributes.get("highlights").map(|value| {
            value
                .split(',')
                .filter_map(|number| number.trim().parse().ok())
                .collect()
        }),
        theme: attributes.theme,
    })
}

fn parse_math(fence: &str, rest: &[&str]) -> Result<Content, GrammarError> {
    let (_, attributes) = split_attrs(fence)?;
    let body: Vec<&str> = rest
        .iter()
        .take_while(|line| line.trim_end() != "$$")
        .copied()
        .collect();
    let engine = match attributes.get("engine") {
        Some("latex") => MathEngine::LaTeX,
        _ => MathEngine::Typst,
    };
    Ok(Content::Math {
        expression: body.join("\n"),
        engine,
        theme: attributes.theme,
    })
}

fn parse_component(opener: &str, rest: &[&str]) -> Result<Content, GrammarError> {
    let (name, attributes) = split_attrs(opener)?;
    let name = name.trim();
    match name {
        "callout" => Ok(Content::Callout {
            icon: attributes.get("icon").unwrap_or_default().to_string(),
            theme: attributes.theme,
        }),
        TABLE_COMPONENT => {
            let rows: Vec<&str> = rest
                .iter()
                .take_while(|line| line.trim_end() != "::")
                .copied()
                .collect();
            parse_gfm_table(&rows, Some(attributes), None)
        }
        other => Err(GrammarError {
            detail: format!("`::{other}` is not a block component in this dialect"),
            at: 0,
        }),
    }
}

/// GFM pipe rows into cells, with any wrapper attributes applied over the top.
fn parse_gfm_table(
    lines: &[&str],
    attributes: Option<Attributes>,
    _reserved: Option<()>,
) -> Result<Content, GrammarError> {
    let mut cells: Vec<Vec<RichText>> = Vec::new();
    let mut saw_delimiter = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            continue;
        }
        if is_delimiter_row(trimmed) {
            saw_delimiter = true;
            continue;
        }
        let mut row = Vec::new();
        for cell in split_cells(trimmed) {
            row.push(inline::parse(&cell)?);
        }
        cells.push(row);
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

fn is_delimiter_row(line: &str) -> bool {
    line.trim_matches('|')
        .split('|')
        .all(|cell| !cell.trim().is_empty() && cell.trim().chars().all(|c| c == '-' || c == ':'))
}

/// A row's cells, honouring `\|` inside one.
fn split_cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    let mut out = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in inner.chars() {
        match character {
            '\\' if !escaped => {
                escaped = true;
                current.push(character);
            }
            '|' if !escaped => out.push(std::mem::take(&mut current)),
            character => {
                current.push(character);
                escaped = false;
            }
        }
    }
    out.push(current);
    out.into_iter()
        .map(|cell| cell.trim().to_string())
        .collect()
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
        assert_eq!(emit(&block).children, Children::None);
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
        assert_eq!(only(&block), "set x = {1\\}");
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
}
