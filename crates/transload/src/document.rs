//! A Landmark file's body: the tree of nodes its identity comments open.
//!
//! Each level reads the file's lines through its prefix — two spaces under an
//! indented parent, `> ` under a blockquote — as slices of the file, so every
//! node, block, and refusal keeps its own line and column. The lines, identity
//! comments, and each block's opener are grammars; nesting is plain recursion,
//! bounded by [`MAX_DEPTH`].

use chumsky::prelude::*;
use domain::{
    Content, MarkID, TextMarkKind,
    syntax::{
        block,
        inline::{self, GrammarError, Links},
    },
};

use crate::{
    identity::{self, Identity, Marker},
    refusal::MalformedKind,
};

#[cfg(test)]
pub(crate) mod legacy;

/// How many nodes may enclose one another in a body.
pub(crate) const MAX_DEPTH: usize = 256;

type Extra<'a> = extra::Err<Rich<'a, char>>;

/// One body node: a Mark, or (with no content) a nested Landmark's position.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Node {
    pub identity: Identity,
    pub content: Option<Content>,
    pub children: Vec<Node>,
    /// The file line of its identity comment.
    pub line: usize,
}

/// A refusal: the file line it cites and what is wrong there.
pub(crate) type Refused = (usize, MalformedKind);

/// The nodes of a Landmark file's body: every line after its frontmatter's
/// closing fence.
pub(crate) fn parse(file: &str, links: &dyn Links) -> Result<Vec<Node>, Refused> {
    let lines = lines().parse(file).into_output().unwrap_or_default();
    let body = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == "---")
        .nth(1)
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    let views: Vec<View> = lines
        .iter()
        .enumerate()
        .skip(body)
        .map(|(index, text)| View {
            text,
            line: index + 1,
            prefix: 0,
        })
        .collect();
    Walk { links }.region(&views, 0)
}

/// The lines `str::lines` would give.
fn lines<'a>() -> impl Parser<'a, &'a str, Vec<&'a str>, Extra<'a>> {
    let terminated = none_of('\n')
        .repeated()
        .to_slice()
        .map(|line: &str| line.strip_suffix('\r').unwrap_or(line))
        .then_ignore(just('\n'));
    let last = none_of('\n').repeated().at_least(1).to_slice();
    terminated
        .repeated()
        .collect::<Vec<_>>()
        .then(last.or_not())
        .map(|(mut lines, last)| {
            lines.extend(last);
            lines
        })
}

/// A file line as one level reads it: past the prefixes of the levels above.
#[derive(Clone, Copy, Debug)]
struct View<'a> {
    text: &'a str,
    /// The 1-indexed file line.
    line: usize,
    /// Characters of prefix before `text` on that line.
    prefix: usize,
}

impl<'a> View<'a> {
    fn past(self, text: &'a str) -> Self {
        Self {
            text,
            line: self.line,
            prefix: self.prefix + self.text.len() - text.len(),
        }
    }
}

/// How a block's first line says its own lines and children run.
#[derive(Clone, Debug)]
enum Shape<'a> {
    /// A fenced block, closed by a line that is exactly its fence.
    Fence(&'a str),
    /// A `::` component, by the name before its attribute list.
    Component(&'a str),
    /// `| ` rows, for as long as they run.
    Table,
    /// A blockquote, opening on a line of its own when it carries attributes.
    Quote { own_line: bool },
    /// One line.
    Line,
}

fn shape<'a>() -> impl Parser<'a, &'a str, Shape<'a>, Extra<'a>> {
    choice((
        just('`')
            .repeated()
            .at_least(3)
            .to_slice()
            .then_ignore(any().repeated())
            .map(Shape::Fence),
        just("$$")
            .then_ignore(any().repeated())
            .map(|_| Shape::Fence("$$")),
        just("::")
            .ignore_then(none_of('{').repeated().to_slice())
            .then_ignore(any().repeated())
            .map(Shape::Component),
        just(block::TABLE_ROW)
            .then(any().repeated())
            .map(|_| Shape::Table),
        just('>')
            .ignore_then(just('{').or_not())
            .then_ignore(any().repeated())
            .map(|brace| Shape::Quote {
                own_line: brace.is_some(),
            }),
        any().repeated().map(|()| Shape::Line),
    ))
}

struct Walk<'l> {
    links: &'l dyn Links,
}

/// A node read up to its children: what [`Walk::plan`] hands back before the
/// walk descends, so the recursion carries only this.
struct Planned<'a> {
    node: Node,
    children: Vec<View<'a>>,
    /// The first line after the node's own lines and children.
    at: usize,
}

impl Walk<'_> {
    /// Sibling nodes, with blank lines between them.
    fn region(&self, views: &[View<'_>], depth: usize) -> Result<Vec<Node>, Refused> {
        let mut nodes = Vec::new();
        let mut at = 0;
        while at < views.len() {
            if views[at].text.trim().is_empty() {
                at += 1;
                continue;
            }
            if depth > MAX_DEPTH {
                return Err(too_deep(views[at].line));
            }
            let (node, next) = self.node(views, at, depth)?;
            nodes.push(node);
            at = next;
        }
        Ok(nodes)
    }

    /// The node an identity comment opens: its block's own lines, the
    /// children beneath them, and the comment that closes it if it has any.
    fn node(
        &self,
        views: &[View<'_>],
        start: usize,
        depth: usize,
    ) -> Result<(Node, usize), Refused> {
        let Planned {
            mut node,
            children,
            mut at,
        } = self.plan(views, start)?;
        if node.content.is_none() {
            return Ok((node, at));
        }
        node.children = self.region(&children, depth + 1)?;
        if !node.children.is_empty() {
            match views.get(at) {
                Some(view) if matches!(identity::parse(view.text), Ok(Some(Marker::Close))) => {
                    at += 1;
                }
                _ => return Err(never_closed(node.line)),
            }
        }
        Ok((node, at))
    }

    /// A node's identity, its block, and the lines its children hold.
    #[inline(never)]
    fn plan<'a>(&self, views: &[View<'a>], start: usize) -> Result<Planned<'a>, Refused> {
        let opener = views[start];
        let identity = match identity::parse(opener.text).map_err(|kind| (opener.line, kind))? {
            Some(Marker::Open(identity)) => identity,
            _ => return Err(grammar(opener.line, "expected a `wp:mark` comment")),
        };
        let mut at = start + 1;
        let Some(&first) = views.get(at) else {
            return Err(grammar(
                opener.line,
                "a `wp:mark` comment with no block after it",
            ));
        };

        // A nested Landmark: its own comment over a link to itself.
        if is_self_link(first.text, identity.id, self.links) {
            let node = Node {
                identity,
                content: None,
                children: Vec::new(),
                line: opener.line,
            };
            return Ok(Planned {
                node,
                children: Vec::new(),
                at: at + 1,
            });
        }

        let until = |end: &str| {
            (at + 1..views.len())
                .find(|&index| views[index].text == end)
                .ok_or_else(|| grammar(first.line, format!("unclosed block, expected `{end}`")))
        };
        let shape = shape()
            .parse(first.text)
            .into_output()
            .unwrap_or(Shape::Line);
        let mut children = Vec::new();
        let mut indented = true;
        let own: &[View] = match shape {
            Shape::Fence(fence) => {
                let end = until(fence)?;
                let own = &views[at..=end];
                at = end + 1;
                own
            }
            Shape::Component(name) => {
                let end = closer(views, at)
                    .ok_or_else(|| grammar(first.line, "unclosed `::` component"))?;
                let own = if block::CONTAINER_COMPONENTS.contains(&name) {
                    children.extend_from_slice(&views[at + 1..end]);
                    indented = false;
                    &views[at..=at]
                } else {
                    &views[at..=end]
                };
                at = end + 1;
                own
            }
            Shape::Table => {
                let end = (at..views.len())
                    .take_while(|&index| views[index].text.starts_with(block::TABLE_ROW))
                    .last()
                    .unwrap_or(at);
                let own = &views[at..=end];
                at = end + 1;
                own
            }
            Shape::Quote { own_line } => {
                indented = false;
                let own = if own_line {
                    at += 1;
                    &views[at - 1..at]
                } else {
                    &[]
                };
                while let Some(view) = views.get(at) {
                    let text = match view.text {
                        ">" => "",
                        text => match text.strip_prefix("> ") {
                            Some(text) => text,
                            None => break,
                        },
                    };
                    children.push(view.past(text));
                    at += 1;
                }
                own
            }
            Shape::Line => {
                at += 1;
                &views[at - 1..at]
            }
        };

        if indented {
            while let Some(view) = views.get(at) {
                let child = match view.text.strip_prefix("  ") {
                    Some(text) => view.past(text),
                    None if view.text.is_empty() && continues(&views[at + 1..]) => *view,
                    None => break,
                };
                children.push(child);
                at += 1;
            }
        }

        let source = match own {
            [] => ">".to_string(),
            [only] => only.text.to_string(),
            lines => lines
                .iter()
                .map(|view| view.text)
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let content =
            block::parse_with(&source, self.links).map_err(|error| located(own, first, &error))?;
        Ok(Planned {
            node: Node {
                identity,
                content: Some(content),
                children: Vec::new(),
                line: opener.line,
            },
            children,
            at,
        })
    }
}

#[inline(never)]
fn too_deep(line: usize) -> Refused {
    grammar(
        line,
        format!("the body nests deeper than {MAX_DEPTH} levels"),
    )
}

#[inline(never)]
fn never_closed(line: usize) -> Refused {
    grammar(line, "a container's comment is never closed")
}

/// Does a blank line continue an indented region? Only if the next line that
/// is not blank is still indented.
fn continues(rest: &[View<'_>]) -> bool {
    rest.iter()
        .find(|view| !view.text.is_empty())
        .is_some_and(|view| view.text.starts_with("  "))
}

/// The `::` closing the component opened at `start`, counting nested ones and
/// passing over fenced code and math, whose lines are verbatim.
fn closer(views: &[View<'_>], start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut fence: Option<&str> = None;
    for (index, view) in views.iter().enumerate().skip(start) {
        let line = view.text;
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

/// A block's refusal at the file line and column it points at: the line of
/// the block's own that it names, past that line's prefixes.
fn located(own: &[View<'_>], first: View<'_>, error: &GrammarError) -> Refused {
    let view = own
        .get(error.line.saturating_sub(1))
        .copied()
        .unwrap_or(first);
    grammar(
        view.line,
        format!("{} (column {})", error.detail, view.prefix + error.column),
    )
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

fn grammar(line: usize, detail: impl Into<String>) -> Refused {
    (
        line,
        MalformedKind::Grammar {
            detail: detail.into(),
        },
    )
}

#[cfg(test)]
mod tests {
    use domain::syntax::inline::LinkName;

    use super::*;
    use crate::{bundle::Bundle, common, export::export, names::Lenient};

    /// Resolves every target to itself.
    struct Pages;

    impl Links for Pages {
        fn name(&self, target: &str) -> LinkName {
            LinkName {
                page: target.to_string(),
                block: None,
                title: None,
            }
        }

        fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
            Ok(match block {
                Some(block) => format!("{page}#{block}"),
                None => page.to_string(),
            })
        }
    }

    /// The walker reads `file` as the hand walker did: refused by both, or
    /// read by both to the same nodes.
    fn agree(file: &str) {
        for links in [&Lenient as &dyn Links, &Pages] {
            let (new, old) = (parse(file, links), legacy::parse(file, links));
            assert_eq!(new.is_ok(), old.is_ok(), "{file}\n{new:?}\nagainst {old:?}");
            if let (Ok(new), Ok(old)) = (new, old) {
                assert_eq!(new, old, "{file}");
            }
        }
    }

    /// `file`, and files one edit away from it: a line dropped, doubled, or
    /// given an indent or a quote marker it did not have.
    fn mutations(file: &str) -> Vec<String> {
        let lines: Vec<&str> = file.lines().collect();
        let mut out = vec![file.to_string()];
        for at in (0..lines.len()).step_by((lines.len() / 6).max(1)) {
            let mut dropped = lines.clone();
            dropped.remove(at);
            let mut doubled = lines.clone();
            doubled.insert(at, lines[at]);
            let (indented, quoted) = (format!("  {}", lines[at]), format!("> {}", lines[at]));
            let mut shifted = lines.clone();
            shifted[at] = &indented;
            let mut marked = lines.clone();
            marked[at] = &quoted;
            out.extend([dropped, doubled, shifted, marked].map(|lines| lines.join("\n")));
        }
        out
    }

    #[test]
    fn the_walker_reads_the_committed_bundle_as_the_hand_walker_did() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../store/src/collections/getting-started");
        let bundle = Bundle::read_dir(&root).expect("the committed Bundle");
        for (_, file) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
            for file in mutations(file) {
                agree(&file);
            }
        }
    }

    #[tokio::test]
    async fn the_walker_reads_generated_exports_as_the_hand_walker_did() {
        for seed in common::generate::seeds() {
            let db = common::open().await;
            common::generate::generate(&db, seed).await;
            let bundle = export(&db, &common::meta()).await.expect("export").bundle;
            for (_, file) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
                let files = match seed % 8 {
                    0 => mutations(file),
                    _ => vec![file.to_string()],
                };
                for file in &files {
                    agree(file);
                }
            }
        }
    }

    /// `nodes` paragraphs, each the only child of the one before.
    fn chain(nodes: usize) -> String {
        let mut out = String::new();
        for depth in 0..nodes {
            let indent = "  ".repeat(depth);
            let id = MarkID(uuid::Uuid::from_u128(depth as u128 + 1));
            out.push_str(&format!("{indent}<!-- wp:mark {id} -->\n{indent}p\n"));
        }
        for depth in (0..nodes.saturating_sub(1)).rev() {
            out.push_str(&format!("{}<!-- /wp:mark -->\n", "  ".repeat(depth)));
        }
        out
    }

    #[test]
    fn a_body_at_the_ceiling_parses_and_one_deeper_refuses() {
        let (at, past) = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                (
                    parse(&chain(MAX_DEPTH + 1), &Lenient),
                    parse(&chain(MAX_DEPTH + 2), &Lenient),
                )
            })
            .expect("thread")
            .join()
            .expect("the walk completes on a two-megabyte stack");
        at.expect("a body nested 256 levels parses");
        assert_eq!(
            past.expect_err("a body nested 257 levels refuses"),
            grammar(
                2 * (MAX_DEPTH + 1) + 1,
                "the body nests deeper than 256 levels"
            )
        );
    }
}
