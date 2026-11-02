//! The hand-written body walker, kept as the differential oracle while the
//! views-and-grammars walker replaces it.

use domain::{
    MarkID, TextMarkKind,
    syntax::{
        block,
        inline::{self, Links},
    },
};

use super::{Node, Refused};
use crate::{
    identity::{self, Marker},
    refusal::MalformedKind,
};

pub(crate) fn parse(file: &str, links: &dyn Links) -> Result<Vec<Node>, Refused> {
    parse_region(&body_lines(file), links)
}

fn body_lines(file: &str) -> Vec<(usize, String)> {
    let first_body_line = file
        .lines()
        .enumerate()
        .filter(|(_, line)| *line == "---")
        .nth(1)
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    file.lines()
        .enumerate()
        .skip(first_body_line)
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect()
}

type Region = [(usize, String)];
type Parsed<T> = Result<T, Refused>;

fn grammar(line: usize, detail: impl Into<String>) -> Refused {
    (
        line,
        MalformedKind::Grammar {
            detail: detail.into(),
        },
    )
}

fn parse_region(lines: &Region, links: &dyn Links) -> Parsed<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        if lines[at].1.trim().is_empty() {
            at += 1;
            continue;
        }
        let (node, next) = parse_node(lines, at, links)?;
        nodes.push(node);
        at = next;
    }
    Ok(nodes)
}

fn parse_node(lines: &Region, start: usize, links: &dyn Links) -> Parsed<(Node, usize)> {
    let (line_no, line) = &lines[start];
    let identity = match identity::parse(line).map_err(|kind| (*line_no, kind))? {
        Some(Marker::Open(identity)) => identity,
        _ => return Err(grammar(*line_no, "expected a `wp:mark` comment")),
    };
    let mut at = start + 1;
    let Some((first_no, first)) = lines.get(at) else {
        return Err(grammar(
            *line_no,
            "a `wp:mark` comment with no block after it",
        ));
    };
    let (first_no, first) = (*first_no, first.clone());

    if is_self_link(&first, identity.id, links) {
        let node = Node {
            identity,
            content: None,
            children: Vec::new(),
            line: *line_no,
        };
        return Ok((node, at + 1));
    }

    let take_until = |from: usize, end: &str| -> Parsed<usize> {
        (from + 1..lines.len())
            .find(|&index| lines[index].1 == end)
            .ok_or_else(|| grammar(first_no, format!("unclosed block, expected `{end}`")))
    };

    let mut children: Vec<(usize, String)> = Vec::new();
    let mut indented_children = true;
    let own: String;
    if first.starts_with("```") {
        let fence = &first[..first.len() - first.trim_start_matches('`').len()];
        let end = take_until(at, fence)?;
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if first.starts_with("$$") {
        let end = take_until(at, "$$")?;
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if let Some(name) = first.strip_prefix("::") {
        let end = matching_closer(lines, at)
            .ok_or_else(|| grammar(first_no, "unclosed `::` component"))?;
        let name = name.split('{').next().unwrap_or_default();
        if block::CONTAINER_COMPONENTS.contains(&name) {
            own = first.clone();
            children = lines[at + 1..end].to_vec();
            indented_children = false;
        } else {
            own = join(&lines[at..=end]);
        }
        at = end + 1;
    } else if first.starts_with(block::TABLE_ROW) {
        let end = (at..lines.len())
            .take_while(|&index| lines[index].1.starts_with(block::TABLE_ROW))
            .last()
            .unwrap_or(at);
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if first.starts_with('>') {
        indented_children = false;
        if first.starts_with(">{") {
            own = first.clone();
            at += 1;
        } else {
            own = ">".into();
        }
        while let Some((number, line)) = lines.get(at) {
            let stripped = if line == ">" {
                Some("")
            } else {
                line.strip_prefix("> ")
            };
            let Some(stripped) = stripped else { break };
            children.push((*number, stripped.to_string()));
            at += 1;
        }
    } else {
        own = first.clone();
        at += 1;
    }

    if indented_children {
        let indent = " ".repeat(block::INDENT);
        while let Some((number, line)) = lines.get(at) {
            let continues = line.starts_with(&indent)
                || (line.is_empty()
                    && lines[at + 1..]
                        .iter()
                        .find(|(_, next)| !next.is_empty())
                        .map(|(_, next)| next.starts_with(&indent))
                        .unwrap_or(false));
            if !continues {
                break;
            }
            children.push((
                *number,
                line.strip_prefix(&indent).unwrap_or("").to_string(),
            ));
            at += 1;
        }
    }

    let content =
        block::parse_with(&own, links).map_err(|error| grammar(first_no, error.to_string()))?;
    let children = parse_region(&children, links)?;
    if !children.is_empty() {
        match lines.get(at) {
            Some((_, line)) if matches!(identity::parse(line), Ok(Some(Marker::Close))) => at += 1,
            _ => return Err(grammar(*line_no, "a container's comment is never closed")),
        }
    }
    Ok((
        Node {
            identity,
            content: Some(content),
            children,
            line: *line_no,
        },
        at,
    ))
}

fn join(lines: &Region) -> String {
    lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn matching_closer(lines: &Region, start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut fence: Option<&str> = None;
    for (index, (_, line)) in lines.iter().enumerate().skip(start) {
        if let Some(open) = fence {
            if line == open {
                fence = None;
            }
            continue;
        }
        if line.starts_with("```") {
            fence = Some(&line[..line.len() - line.trim_start_matches('`').len()]);
        } else if line.starts_with("$$") {
            fence = Some("$$");
        } else if line == "::" {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        } else if line.starts_with("::") {
            depth += 1;
        }
    }
    None
}

fn is_self_link(line: &str, id: MarkID, links: &dyn Links) -> bool {
    let Ok(rich) = inline::parse_with(line, links) else {
        return false;
    };
    match rich.segments() {
        [segment] => segment.marks.iter().any(|mark| {
            mark.kind == TextMarkKind::Mention
                && mark.data.as_ref().and_then(|data| data.as_str()) == Some(&id.to_string())
        }),
        _ => false,
    }
}
