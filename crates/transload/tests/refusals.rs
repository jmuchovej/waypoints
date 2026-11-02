//! Every refusal sentence the dialect's readers write, at the position it
//! names. A refusal sentence in the readers' sources with no fixture here fails
//! `every_refusal_sentence_has_a_fixture`.

mod common;

use common::*;
use domain::{
    MarkID,
    syntax::{
        block,
        inline::{self, GrammarError, LinkName, Links, Verbatim},
    },
};
use transload::{Bundle, IngestError, Location, MalformedKind, export, ingest, layout};

/// A refusal of inline or block source, and where in that source it points.
struct Fixture {
    /// The string literal in the source, placeholders and all.
    template: &'static str,
    source: &'static str,
    detail: &'static str,
    at: usize,
    line: usize,
    column: usize,
}

/// A refusal of a Landmark body.
struct Body {
    template: &'static str,
    source: &'static str,
    detail: &'static str,
    /// The offending line, counted from the body's first.
    line: usize,
}

const INLINE: &[Fixture] = &[
    Fixture {
        template: "unclosed `{close}`",
        source: "*unclosed",
        detail: "unclosed `*`",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "unclosed `{close}`",
        source: "a\n*b",
        detail: "unclosed `*`",
        at: 2,
        line: 2,
        column: 1,
    },
    Fixture {
        template: "unclosed `]`",
        source: "[a",
        detail: "unclosed `]`",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "unclosed `{close}`",
        source: "x^{y",
        detail: "unclosed `}`",
        at: 1,
        line: 1,
        column: 2,
    },
    Fixture {
        template: "unclosed `[[`",
        source: "[[",
        detail: "unclosed `[[`",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "a mention with no target",
        source: "[[|label]]",
        detail: "a mention with no target",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "a bracketed run needs a `(url)` or `{tokens}` suffix; a literal bracket is escaped",
        source: "see [bare bracketed]",
        detail: "a bracketed run needs a `(url)` or `{tokens}` suffix; a literal bracket is escaped",
        at: 4,
        line: 1,
        column: 5,
    },
    Fixture {
        template: "`{token}` is not a mark token in this dialect",
        source: "[hi]{bold nonsense}",
        detail: "`nonsense` is not a mark token in this dialect",
        at: 10,
        line: 1,
        column: 11,
    },
    Fixture {
        template: "unclosed `(`",
        source: "[a](unclosed",
        detail: "unclosed `(`",
        at: 3,
        line: 1,
        column: 4,
    },
    Fixture {
        template: "unclosed `{`",
        source: "[a]{bold",
        detail: "unclosed `{`",
        at: 3,
        line: 1,
        column: 4,
    },
    Fixture {
        template: "unclosed `{delim}`",
        source: "`unclosed",
        detail: "unclosed ```",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "unclosed `{delim}`",
        source: "$x",
        detail: "unclosed `$`",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "a block link needs names to resolve against",
        source: "[[page#4f2a]]",
        detail: "a block link needs names to resolve against",
        at: 0,
        line: 1,
        column: 1,
    },
    Fixture {
        template: "inline marks nest deeper than {MAX_DEPTH} levels",
        source: "^{^{^{^{^{^{^{^{^{^{^{^{^{^{^{^{^{x",
        detail: "inline marks nest deeper than 16 levels",
        at: 32,
        line: 1,
        column: 33,
    },
];

const BLOCK: &[Fixture] = &[
    Fixture {
        template: "`{color}` is not a block theme colour",
        source: "# hi{theme-chartreuse}",
        detail: "`chartreuse` is not a block theme colour",
        at: 11,
        line: 1,
        column: 12,
    },
    Fixture {
        template: "`{word}` is not a block attribute in this dialect",
        source: "# hi{nonsense}",
        detail: "`nonsense` is not a block attribute in this dialect",
        at: 5,
        line: 1,
        column: 6,
    },
    Fixture {
        template: "`::{trimmed}` is not a block component in this dialect",
        source: "::nonsense{}",
        detail: "`::nonsense` is not a block component in this dialect",
        at: 2,
        line: 1,
        column: 3,
    },
    Fixture {
        template: "unclosed `{close}`",
        source: "- *x",
        detail: "unclosed `*`",
        at: 2,
        line: 1,
        column: 3,
    },
    Fixture {
        template: "unclosed `{close}`",
        source: "| a | *b |",
        detail: "unclosed `*`",
        at: 6,
        line: 1,
        column: 7,
    },
    Fixture {
        template: "unclosed `{close}`",
        source: "::table\n| *b |",
        detail: "unclosed `*`",
        at: 10,
        line: 2,
        column: 3,
    },
];

const BODY: &[Body] = &[
    Body {
        template: "expected a `wp:mark` comment",
        source: "<!-- just a note -->\nhello\n",
        detail: "expected a `wp:mark` comment",
        line: 0,
    },
    Body {
        template: "a `wp:mark` comment with no block after it",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n",
        detail: "a `wp:mark` comment with no block after it",
        line: 0,
    },
    Body {
        template: "unclosed block, expected `{end}`",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n```rust\nfn x\n",
        detail: "unclosed block, expected `````",
        line: 1,
    },
    Body {
        template: "unclosed block, expected `{end}`",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n$$\nx\n",
        detail: "unclosed block, expected `$$`",
        line: 1,
    },
    Body {
        template: "unclosed `::` component",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n::callout{icon=\"x\"}\n",
        detail: "unclosed `::` component",
        line: 1,
    },
    Body {
        template: "`{word}` is not a block attribute in this dialect",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n# hi{nonsense}\n",
        detail: "`nonsense` is not a block attribute in this dialect (column 6)",
        line: 1,
    },
    Body {
        template: "`{token}` is not a mark token in this dialect",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\nfirst\n\n\
                 <!-- wp:mark 00000000-0000-0000-0000-00000000000b -->\n\
                 > <!-- wp:mark 00000000-0000-0000-0000-00000000000c -->\n\
                 > - item\n\
                 >   <!-- wp:mark 00000000-0000-0000-0000-00000000000d -->\n\
                 >   text [x]{nonsense} and more\n\
                 > <!-- /wp:mark -->\n\
                 <!-- /wp:mark -->\n",
        detail: "`nonsense` is not a mark token in this dialect (column 14)",
        line: 7,
    },
    Body {
        template: "`{color}` is not a block theme colour",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\nparent\n\
                 \x20 <!-- wp:mark 00000000-0000-0000-0000-00000000000b -->\n\
                 \x20 > <!-- wp:mark 00000000-0000-0000-0000-00000000000c -->\n\
                 \x20 > - item{theme-chartreuse}\n\
                 \x20 <!-- /wp:mark -->\n\
                 <!-- /wp:mark -->\n",
        detail: "`chartreuse` is not a block theme colour (column 18)",
        line: 4,
    },
    Body {
        template: "a container's comment is never closed",
        source: "<!-- wp:mark 00000000-0000-0000-0000-00000000000a -->\n- item\n  \
                 <!-- wp:mark 00000000-0000-0000-0000-00000000000b -->\n  child\n",
        detail: "a container's comment is never closed",
        line: 0,
    },
    Body {
        template: "a `{MARKER}` comment takes a Mark id, not `{payload}`",
        source: "<!-- wp:mark not-a-uuid -->\nhello\n",
        detail: "a `wp:mark` comment takes a Mark id, not `not-a-uuid`",
        line: 0,
    },
];

const EDGE_KIND: &str = "`{raw_kind}` is not an edge kind";

/// The readers whose sentences must each have a fixture, relative to this crate.
const SOURCES: &[&str] = &[
    "../domain/src/syntax/inline.rs",
    "../domain/src/syntax/block.rs",
    "src/document.rs",
    "src/ingest.rs",
    "src/identity.rs",
];

/// Resolves every page by name, except `Nowhere`.
struct Pages;

impl Links for Pages {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            page: target.into(),
            block: None,
            title: None,
        }
    }

    fn resolve(&self, page: &str, _: Option<&str>) -> Result<String, String> {
        match page {
            "Nowhere" => Err("no page `Nowhere`".into()),
            page => Ok(page.into()),
        }
    }
}

fn located(fixture: &Fixture, error: &GrammarError) {
    assert_eq!(error.detail, fixture.detail, "{:?}", fixture.source);
    assert_eq!(
        (error.at, error.line, error.column),
        (fixture.at, fixture.line, fixture.column),
        "{:?}",
        fixture.source
    );
    assert!(
        renders(fixture.template, &error.detail),
        "{:?}",
        fixture.source
    );
}

#[test]
fn inline_refusals_name_their_sentence_and_position() {
    for fixture in INLINE {
        located(
            fixture,
            &inline::parse(fixture.source).expect_err(fixture.source),
        );
    }
}

#[test]
fn a_callers_sentence_passes_through_unchanged() {
    let error = inline::parse_with("[[Nowhere]]", &Pages).expect_err("unresolvable");
    assert_eq!(error.detail, "no page `Nowhere`");
    assert_eq!((error.at, error.line, error.column), (0, 1, 1));
}

#[test]
fn block_refusals_name_their_sentence_and_position() {
    for fixture in BLOCK {
        located(
            fixture,
            &block::parse_with(fixture.source, &Verbatim).expect_err(fixture.source),
        );
    }
}

#[test]
fn no_refusal_names_the_machinery() {
    let inline = INLINE
        .iter()
        .map(|fixture| inline::parse(fixture.source).expect_err(fixture.source));
    let block = BLOCK
        .iter()
        .map(|fixture| block::parse_with(fixture.source, &Verbatim).expect_err(fixture.source));
    for error in inline.chain(block) {
        let rendered = error.to_string();
        for word in ["chumsky", "Rich", "Span", "parser", "Parser", "syntax::"] {
            assert!(!rendered.contains(word), "{rendered}");
        }
    }
}

#[tokio::test]
async fn body_refusals_cite_their_line() {
    for fixture in BODY {
        let (bundle, path, first) = with_body(fixture.source).await;
        let refusal = refused(&bundle).await;
        let expected = Location::line(&path, first + fixture.line);
        assert!(
            refusal.malformed.iter().any(|malformed| {
                malformed.at == expected
                    && malformed.kind
                        == MalformedKind::Grammar {
                            detail: fixture.detail.into(),
                        }
            }),
            "{:?}: expected `{}` at {expected}, got {refusal}",
            fixture.source,
            fixture.detail
        );
        assert!(
            renders(fixture.template, fixture.detail),
            "{}",
            fixture.source
        );
    }
}

#[tokio::test]
async fn frontmatter_refusals_cite_their_file_line() {
    let refusal_at = |refusal: &transload::Refusal, path: &str, line: usize, says: &str| {
        refusal.malformed.iter().any(|malformed| {
            malformed.at == Location::line(path, line)
                && matches!(&malformed.kind, MalformedKind::Grammar { detail } if detail.contains(says))
        })
    };

    let (mut bundle, path, _) = with_body("").await;
    let file = bundle.get(&path).expect("landmark").to_string();
    let chart = file
        .lines()
        .position(|line| line.starts_with("chart:"))
        .expect("a chart line")
        + 1;
    bundle.insert(&path, file.replacen("chart: ", "chart: !!str ", 1));
    let refusal = refused(&bundle).await;
    assert!(refusal_at(&refusal, &path, chart, "tag"), "{refusal}");

    let (mut bundle, path, _) = with_body("").await;
    let file = bundle.get(&path).expect("landmark").to_string();
    let fence = file.rfind("---\n").expect("closing fence");
    let reading = file[..fence].lines().count() + 2;
    bundle.insert(
        &path,
        format!(
            "{}readings:\n  zzzz: true\n{}",
            &file[..fence],
            &file[fence..]
        ),
    );
    let refusal = refused(&bundle).await;
    assert!(
        refusal_at(&refusal, &path, reading, "names no feature"),
        "{refusal}"
    );
}

#[tokio::test]
async fn an_unknown_edge_kind_cites_its_sidecar_line() {
    let (mut bundle, _, _) = with_body("").await;
    let id = MarkID::new();
    bundle.insert(
        layout::TERRAIN,
        format!("{{\"src\":\"{id}\",\"dst\":\"{id}\",\"kind\":\"nonsense\"}}\n"),
    );
    let refusal = refused(&bundle).await;
    let detail = "`nonsense` is not an edge kind";
    assert!(
        refusal.malformed.iter().any(|malformed| {
            malformed.at == Location::line(layout::TERRAIN, 1)
                && malformed.kind
                    == MalformedKind::Grammar {
                        detail: detail.into(),
                    }
        }),
        "{refusal}"
    );
    assert!(renders(EDGE_KIND, detail));
}

#[test]
fn every_refusal_sentence_has_a_fixture() {
    let covered: Vec<&str> = INLINE
        .iter()
        .chain(BLOCK)
        .map(|fixture| fixture.template)
        .chain(BODY.iter().map(|fixture| fixture.template))
        .chain([EDGE_KIND])
        .collect();
    let root = env!("CARGO_MANIFEST_DIR");
    let mut written = Vec::new();
    for file in SOURCES {
        let text = std::fs::read_to_string(format!("{root}/{file}")).expect(file);
        for literal in literals(&text) {
            let refuses = literal.starts_with("unclosed")
                || literal.contains("is not a")
                || literal.contains("expected a");
            assert!(
                !refuses || covered.contains(&literal.as_str()),
                "{file}: `{literal}` has no fixture"
            );
            written.push(literal);
        }
    }
    for template in covered {
        assert!(
            written.iter().any(|literal| literal == template),
            "`{template}` is no longer written by any reader"
        );
    }
}

/// A Bundle holding one Landmark whose body is `body`, its path, and the file
/// line the body opens on.
async fn with_body(body: &str) -> (Bundle, String, usize) {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    db.insert(landmark(root, "Fixture")).await.expect("root");
    let mut bundle = export(&db, &meta()).await.expect("export").bundle;
    let path = layout::landmark(&chart_id(), &root);
    let front = bundle.get(&path).expect("landmark").to_string();
    let first = front.lines().count() + 2;
    bundle.insert(&path, format!("{front}\n{body}"));
    (bundle, path, first)
}

async fn refused(bundle: &Bundle) -> transload::Refusal {
    match ingest(&open().await, bundle).await {
        Err(IngestError::Refused(refusal)) => *refusal,
        Err(other) => panic!("expected a refusal, got {other}"),
        Ok(_) => panic!("expected a refusal"),
    }
}

/// Does `detail` read as `template` with each `{…}` filled in? A trailing
/// position, as a block refusal carries through the body, is not the sentence.
fn renders(template: &str, detail: &str) -> bool {
    let detail = match detail.rsplit_once(" (") {
        Some((sentence, position))
            if position.ends_with(')')
                && ["at ", "line ", "column "]
                    .iter()
                    .any(|lead| position.starts_with(lead)) =>
        {
            sentence
        }
        _ => detail,
    };
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        pieces.push(&rest[..open]);
        rest = &rest[open + close + 1..];
    }
    pieces.push(rest);

    let (first, last) = (pieces[0], pieces[pieces.len() - 1]);
    if pieces.len() == 1 {
        return detail == first;
    }
    let Some(mut tail) = detail.strip_prefix(first) else {
        return false;
    };
    for piece in &pieces[1..pieces.len() - 1] {
        match tail.find(piece) {
            Some(at) => tail = &tail[at + piece.len()..],
            None => return false,
        }
    }
    tail.ends_with(last)
}

/// The string literals of a source file's non-test code, with Rust's `\`
/// line continuations joined.
fn literals(source: &str) -> Vec<String> {
    let code = source
        .split("#[cfg(test)]\nmod tests")
        .next()
        .unwrap_or(source);
    let chars: Vec<char> = code.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '\'' if chars.get(i + 1) == Some(&'\\') => {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            }
            '\'' if chars.get(i + 2) == Some(&'\'') => i += 3,
            '"' => {
                let mut literal = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && chars.get(i + 1) == Some(&'\n') {
                        i += 1;
                        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
                            i += 1;
                        }
                        continue;
                    }
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    if let Some(&c) = chars.get(i) {
                        literal.push(c);
                    }
                    i += 1;
                }
                i += 1;
                out.push(literal);
            }
            _ => i += 1,
        }
    }
    out
}
