//! The hand-written block reader, kept as the differential oracle while the
//! grammar replaces it.

use super::{TABLE_COMPONENT, TABLE_ROW, color_from, parse_colors, parse_sizes};
use crate::{
    BlockTheme, Content, MathEngine, RichText,
    syntax::inline::{GrammarError, Links, legacy as inline},
};

pub(crate) fn parse_with(source: &str, links: &dyn Links) -> Result<Content, GrammarError> {
    let lines: Vec<&str> = source.lines().collect();
    let first = lines.first().copied().unwrap_or_default();

    if first.starts_with("```") {
        let fence = &first[..first.len() - first.trim_start_matches('`').len()];
        return parse_code(fence, &first[fence.len()..], &lines[1..]);
    }
    if let Some(rest) = first.strip_prefix("$$") {
        return parse_math(rest, &lines[1..]);
    }
    if let Some(rest) = first.strip_prefix("::") {
        return parse_component(rest, &lines[1..], links);
    }
    if first.starts_with(TABLE_ROW) {
        return parse_gfm_table(&lines, None, links);
    }
    if let Some(rest) = first.strip_prefix('>') {
        let (_, attributes) = split_attrs(rest)?;
        return Ok(Content::Blockquote {
            theme: attributes.theme,
        });
    }
    for (prefix, build) in HEADINGS {
        if let Some(rest) = first.strip_prefix(prefix) {
            let (text, attributes) = split_attrs(rest)?;
            return Ok(build(inline::parse_with(&text, links)?, attributes.theme));
        }
    }
    if let Some(rest) = first.strip_prefix("- ") {
        let (text, attributes) = split_attrs(rest)?;
        return Ok(Content::ListItem {
            rich_text: inline::parse_with(&text, links)?,
            theme: attributes.theme,
        });
    }
    if let Some(rest) = first.strip_prefix("+ ") {
        let (text, attributes) = split_attrs(rest)?;
        return Ok(Content::EnumItem {
            rich_text: inline::parse_with(&text, links)?,
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
        rich_text: inline::parse_with(&text, links)?,
        theme: attributes.theme,
    })
}

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

fn split_attrs(line: &str) -> Result<(String, Attributes), GrammarError> {
    let trimmed = line.trim_end();
    let bytes = trimmed.as_bytes();
    let escaped = |index: usize| {
        bytes[..index]
            .iter()
            .rev()
            .take_while(|byte| **byte == b'\\')
            .count()
            % 2
            == 1
    };
    if !trimmed.ends_with('}') || escaped(bytes.len() - 1) {
        return Ok((line.to_string(), Attributes::default()));
    }
    let mut open = None;
    let mut quoted = false;
    for index in (0..bytes.len() - 1).rev() {
        match bytes[index] {
            b'"' if !escaped(index) => quoted = !quoted,
            b'{' if !quoted && !escaped(index) => {
                open = Some(index);
                break;
            }
            _ => {}
        }
    }
    let Some(open) = open else {
        return Ok((line.to_string(), Attributes::default()));
    };
    let interior = &trimmed[open + 1..trimmed.len() - 1];
    Ok((
        trimmed[..open].to_string(),
        parse_attrs(line, interior, open)?,
    ))
}

fn parse_attrs(line: &str, interior: &str, at: usize) -> Result<Attributes, GrammarError> {
    let mut out = Attributes::default();
    for token in split_tokens(interior) {
        if let Some((key, value)) = token.split_once('=') {
            let value = value.trim().trim_matches('"').replace("\\\"", "\"");
            out.pairs.push((key.trim().to_string(), value));
            continue;
        }
        if let Some(color) = token.strip_prefix("theme-") {
            let Some(color) = color_from(color) else {
                return Err(GrammarError::new(
                    line,
                    at,
                    format!("`{color}` is not a block theme colour"),
                ));
            };
            out.theme = Some(BlockTheme { color });
            continue;
        }
        return Err(GrammarError::new(
            line,
            at,
            format!("`{token}` is not a block attribute in this dialect"),
        ));
    }
    Ok(out)
}

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

fn parse_code(fence: &str, opener: &str, rest: &[&str]) -> Result<Content, GrammarError> {
    let (language, attributes) = split_attrs(opener)?;
    let body: Vec<String> = rest
        .iter()
        .take_while(|line| line.trim_end() != fence)
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

fn parse_component(
    opener: &str,
    rest: &[&str],
    links: &dyn Links,
) -> Result<Content, GrammarError> {
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
            parse_gfm_table(&rows, Some(attributes), links)
        }
        other => Err(GrammarError::new(
            opener,
            0,
            format!("`::{other}` is not a block component in this dialect"),
        )),
    }
}

fn parse_gfm_table(
    lines: &[&str],
    attributes: Option<Attributes>,
    links: &dyn Links,
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
            row.push(inline::parse_with(&cell, links)?);
        }
        cells.push(row);
    }

    let attributes = attributes.unwrap_or_default();
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

fn split_cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    let mut out = Vec::new();
    let mut current = String::new();
    for character in inner.chars() {
        match character {
            '|' if current.ends_with('\\') => {
                current.pop();
                current.push('|');
            }
            '|' => out.push(std::mem::take(&mut current)),
            character => current.push(character),
        }
    }
    out.push(current);
    out.into_iter()
        .map(|cell| {
            let mut trimmed = cell.trim().to_string();
            let run = trimmed.chars().rev().take_while(|c| *c == '\\').count();
            if run % 2 == 1 {
                trimmed.push(' ');
            }
            trimmed
        })
        .collect()
}
