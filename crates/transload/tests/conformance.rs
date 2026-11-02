//! Properties over generated stores. Each store comes from a seed, so a failure
//! names a reproducible case; `TRANSLOAD_SEEDS` raises the count for a deep run.

mod common;

use bedrock::{Database, Query};
use common::generate::{generate, seeds, wide_chart};
use common::*;
use domain::{
    Chart, ChartID, Mark, MarkID, Terrain, TerrainKind,
    syntax::inline::{LinkName, Links},
};
use std::collections::BTreeMap;
use transload::{Bundle, export, ingest, layout};

async fn live_marks(db: &Database) -> Vec<Mark> {
    let mut marks = db.query::<Mark>(&Query::new()).await.expect("marks");
    marks.sort_by_key(|mark| mark.id);
    marks
}

/// Edges without their regenerated machinery (id and order key), with each
/// parent's children in order.
async fn structure(db: &Database) -> (Vec<String>, BTreeMap<MarkID, Vec<MarkID>>) {
    let edges = db.query::<Terrain>(&Query::new()).await.expect("edges");
    let mut flat: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {:?}",
                e.src_id,
                e.dst_id,
                e.kind.to_storage(),
                e.extra
            )
        })
        .collect();
    flat.sort();
    let mut children: BTreeMap<MarkID, Vec<(String, MarkID)>> = BTreeMap::new();
    for edge in edges.iter().filter(|e| e.kind == TerrainKind::Child) {
        children
            .entry(edge.src_id)
            .or_default()
            .push((edge.order.clone().unwrap_or_default(), edge.dst_id));
    }
    let ordered = children
        .into_iter()
        .map(|(parent, mut kids)| {
            kids.sort();
            (parent, kids.into_iter().map(|(_, id)| id).collect())
        })
        .collect();
    (flat, ordered)
}

/// A refusal with each located problem's source line beside it.
fn explain(bundle: &Bundle, error: transload::IngestError) -> String {
    let transload::IngestError::Refused(refusal) = error else {
        return error.to_string();
    };
    let mut out = String::new();
    for malformed in refusal
        .malformed
        .iter()
        .filter(|m| m.at.file.ends_with(".md"))
    {
        let context: String = malformed
            .at
            .line
            .and_then(|n| {
                let lines = bundle.get(&malformed.at.file)?.lines();
                Some(
                    lines
                        .skip(n - 1)
                        .take(8)
                        .map(|l| format!("\n    | {l}"))
                        .collect(),
                )
            })
            .unwrap_or_default();
        out.push_str(&format!("\n{}: {}{context}", malformed.at, malformed.kind));
    }
    out
}

async fn exported(db: &Database, seed: u64) -> Bundle {
    let exported = export(db, &meta()).await.expect("export");
    assert!(
        exported.warnings.is_empty(),
        "seed {seed}: {:?}",
        exported.warnings
    );
    exported.bundle
}

#[tokio::test]
async fn ingest_of_export_reproduces_every_generated_store() {
    for seed in seeds() {
        let (source, target) = (open().await, open().await);
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        if let Err(error) = ingest(&target, &bundle).await {
            panic!("seed {seed}: {}", explain(&bundle, error));
        }

        let (before, after) = (live_marks(&source).await, live_marks(&target).await);
        assert_eq!(before.len(), after.len(), "seed {seed}: Mark count");
        for (left, right) in before.iter().zip(&after) {
            assert_eq!(left, right, "seed {seed}: Mark {}", left.id);
        }
        assert_eq!(
            structure(&source).await,
            structure(&target).await,
            "seed {seed}: Terrain"
        );
        assert_eq!(
            target
                .get::<Chart>(&ChartID("wide".into()))
                .await
                .expect("get"),
            Some(wide_chart()),
            "seed {seed}: Chart"
        );
    }
}

#[tokio::test]
async fn export_is_byte_stable_and_a_fixed_point_of_ingest() {
    for seed in seeds() {
        let (source, target) = (open().await, open().await);
        generate(&source, seed).await;
        let first = exported(&source, seed).await;
        assert_eq!(first, exported(&source, seed).await, "seed {seed}: twice");

        ingest(&target, &first).await.expect("ingest");
        let again = exported(&target, seed).await;
        for (path, contents) in first.iter() {
            assert_eq!(again.get(path), Some(contents), "seed {seed}: {path}");
        }
        assert_eq!(first, again, "seed {seed}: export(ingest(export(s)))");
    }
}

/// Strip every comment and what remains is text: no `wp:` marker, and a UUID
/// only as a link target — and the text still parses as the dialect.
#[tokio::test]
async fn the_1980_test() {
    // A label may itself hold `[[`, so test what opens each UUID.
    let stray_uuid = |line: &str| {
        line.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
            .any(|token| {
                let at = token.as_ptr() as usize - line.as_ptr() as usize;
                token.len() == 36
                    && token.parse::<uuid::Uuid>().is_ok()
                    && !line[..at].ends_with("[[")
            })
    };
    for seed in seeds() {
        let source = open().await;
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        for (path, contents) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
            let stripped = strip_comments(contents);
            assert!(!stripped.contains("wp:"), "seed {seed} {path}:\n{stripped}");

            let body = stripped
                .splitn(3, "---\n")
                .nth(2)
                .expect("frontmatter fences");
            let mut fence: Option<String> = None;
            for line in body.lines() {
                assert!(
                    !stray_uuid(line),
                    "seed {seed} {path}: a UUID outside a link: {line}"
                );

                // Each line outside a fenced or tabular region is one block.
                let bare = line.trim_start_matches(['>', ' ']);
                if let Some(open) = &fence {
                    if bare == open {
                        fence = None;
                    }
                    continue;
                }
                if bare.starts_with("```") || bare.starts_with("$$") {
                    let marker: String = bare
                        .chars()
                        .take_while(|c| *c == '`' || *c == '$')
                        .collect();
                    fence = Some(if marker.starts_with('$') {
                        "$$".into()
                    } else {
                        marker
                    });
                    continue;
                }
                // Grammar only; name resolution is the round trip's business.
                if !bare.is_empty() && !bare.starts_with(['|', ':']) {
                    domain::syntax::block::parse_with(bare, &AnyName)
                        .unwrap_or_else(|error| panic!("seed {seed} {path}: `{line}`: {error}"));
                }
            }
        }
    }
}

struct AnyName;

impl Links for AnyName {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            page: target.to_string(),
            block: None,
            title: None,
        }
    }

    fn resolve(&self, page: &str, _: Option<&str>) -> Result<String, String> {
        Ok(page.to_string())
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out.lines()
        .filter(|line| !line.trim().is_empty() || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn every_exported_schema_is_valid_draft_07() {
    for seed in seeds() {
        let source = open().await;
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        let schemas: Vec<_> = bundle
            .iter()
            .filter(|(path, _)| path.ends_with(layout::SCHEMA))
            .collect();
        assert!(!schemas.is_empty(), "seed {seed}");
        for (path, contents) in schemas {
            let document: serde_json::Value = serde_json::from_str(contents).expect("JSON");
            jsonschema::draft7::meta::validate(&document)
                .unwrap_or_else(|error| panic!("seed {seed} {path}: {error}"));
        }
    }
}
