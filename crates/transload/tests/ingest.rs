//! The ingester, against a live engine. Every refusal asserts the target store
//! is untouched afterwards.

mod common;

use bedrock::{Database, Query};
use common::*;
use domain::{
    BlockTheme, Chart, ChartID, Color, Content, Feature, FeatureID, FeatureKind, Mark, MarkData,
    MarkID, Reading, RichText, SelectOption, Terrain, TerrainKind, TextMarkKind,
};
use indexmap::IndexMap;
use transload::{Bundle, IngestError, MalformedKind, export, ingest, layout};

struct Ids {
    root: MarkID,
    nested: MarkID,
    intro: MarkID,
}

/// A store exercising every carrier: nesting of each container kind, a nested
/// Landmark, a reading, Mark extra, edge extra on a child, and a custom edge.
async fn rich_store(db: &Database) -> Ids {
    let (root, nested) = (MarkID::new(), MarkID::new());
    let [
        intro,
        quote,
        quoted,
        item,
        sub,
        callout,
        inside,
        code,
        table,
        rule,
        page,
    ] = std::array::from_fn(|_| MarkID::new());

    db.insert(chart()).await.expect("chart");
    let mut root_mark = landmark(root, "Weekly sync");
    if let MarkData::Landmark(data) = &mut root_mark.data {
        let (feature, reading) = text_reading("some *notes*");
        data.readings.insert(feature, reading);
    }
    root_mark.extra = Some(serde_json::json!({ "so.waypoints.test": { "pinned": true } }));
    db.insert(root_mark).await.expect("root");
    db.insert(landmark(nested, "Child page"))
        .await
        .expect("nested");

    let mut intro_mark = paragraph(
        intro,
        "intro with *bold* and a [link](https://waypoints.so)",
    );
    intro_mark.extra = Some(serde_json::json!({ "so.waypoints.test": 7 }));
    db.insert(intro_mark).await.expect("intro");
    db.insert(content(
        quote,
        Content::Blockquote {
            theme: Some(BlockTheme {
                color: Color::Green,
            }),
        },
    ))
    .await
    .expect("quote");
    db.insert(paragraph(quoted, "quoted"))
        .await
        .expect("quoted");
    db.insert(content(
        item,
        Content::ListItem {
            rich_text: RichText::from("a point".to_string()),
            theme: None,
        },
    ))
    .await
    .expect("item");
    db.insert(paragraph(sub, "a sub-point")).await.expect("sub");
    db.insert(content(
        callout,
        Content::Callout {
            icon: "warning".into(),
            theme: None,
        },
    ))
    .await
    .expect("callout");
    db.insert(paragraph(inside, "mind the gap"))
        .await
        .expect("inside");
    db.insert(content(
        code,
        Content::Code {
            language: "rust".into(),
            lines: vec!["fn main() {".into(), "".into(), "}".into()],
            filename: Some("main.rs".into()),
            highlights: None,
            theme: None,
        },
    ))
    .await
    .expect("code");
    db.insert(content(
        table,
        Content::Table {
            cells: vec![
                vec![
                    RichText::from("a".to_string()),
                    RichText::from("b".to_string()),
                ],
                vec![
                    RichText::from("1".to_string()),
                    RichText::from("2".to_string()),
                ],
            ],
            col_size: None,
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
            theme: None,
        },
    ))
    .await
    .expect("table");
    db.insert(content(rule, Content::Divider { theme: None }))
        .await
        .expect("rule");
    db.insert(paragraph(page, "on the child page"))
        .await
        .expect("page");

    let mut annotated = child(root, intro, "a");
    annotated.extra = Some(serde_json::json!({ "collapsed": false }));
    for edge in [
        annotated,
        child(root, quote, "b"),
        child(quote, quoted, "a"),
        child(root, item, "c"),
        child(item, sub, "a"),
        child(root, callout, "d"),
        child(callout, inside, "a"),
        child(root, code, "e"),
        child(root, nested, "f"),
        child(root, table, "g"),
        child(root, rule, "h"),
        child(nested, page, "a"),
        {
            let mut custom = edge(root, nested, TerrainKind::Custom, None);
            custom.extra = Some(serde_json::json!({ "weight": 3 }));
            custom
        },
    ] {
        db.insert(edge).await.expect("edge");
    }
    Ids {
        root,
        nested,
        intro,
    }
}

async fn exported(db: &Database) -> Bundle {
    let exported = export(db, &meta()).await.expect("export");
    assert!(exported.warnings.is_empty(), "{:?}", exported.warnings);
    exported.bundle
}

fn landmark_path(id: MarkID) -> String {
    layout::landmark(&chart_id(), &id)
}

fn edit(bundle: &mut Bundle, path: &str, edit: impl FnOnce(&str) -> String) {
    let original = bundle.get(path).expect("file to edit").to_string();
    let edited = edit(&original);
    assert_ne!(original, edited, "the edit must change {path}");
    bundle.insert(path, edited);
}

/// Refused, with the target untouched.
async fn refused(target: &Database, bundle: &Bundle) -> transload::Refusal {
    let before = census(target).await;
    let error = ingest(target, bundle).await.expect_err("must refuse");
    assert_eq!(
        census(target).await,
        before,
        "a refusal leaves the store untouched"
    );
    match error {
        IngestError::Refused(refusal) => *refusal,
        other => panic!("expected a refusal, got {other}"),
    }
}

#[tokio::test]
async fn an_export_ingests_into_an_empty_store_and_re_exports_identically() {
    let (source, target) = (open().await, open().await);
    rich_store(&source).await;
    let bundle = exported(&source).await;

    let ingested = ingest(&target, &bundle).await.expect("ingest");
    assert!(ingested.advisories.is_empty(), "{:?}", ingested.advisories);
    assert_eq!(
        exported(&target).await,
        bundle,
        "re-export is byte-identical"
    );

    assert_eq!(
        marks(&source).await,
        marks(&target).await,
        "every Mark, exactly"
    );
    assert_eq!(
        target.get::<Chart>(&chart_id()).await.expect("get"),
        Some(chart()),
        "the custom Chart, id included"
    );
    assert_eq!(
        edges(&source).await,
        edges(&target).await,
        "every edge, minus id and order key"
    );
}

async fn marks(db: &Database) -> Vec<Mark> {
    let mut marks: Vec<Mark> = db.query::<Mark>(&Query::new()).await.expect("marks");
    marks.sort_by_key(|mark| mark.id);
    marks
}

async fn edges(db: &Database) -> Vec<String> {
    let mut edges: Vec<String> = db
        .query::<Terrain>(&Query::new())
        .await
        .expect("edges")
        .into_iter()
        .map(|edge| {
            format!(
                "{} {} {} {:?}",
                edge.src_id,
                edge.dst_id,
                edge.kind.to_storage(),
                edge.extra
            )
        })
        .collect();
    edges.sort();
    edges
}

#[tokio::test]
async fn a_bundle_survives_the_filesystem() {
    let (source, target) = (open().await, open().await);
    rich_store(&source).await;
    let bundle = exported(&source).await;

    let dir = tempfile::tempdir().expect("tempdir");
    bundle.write_dir(dir.path()).expect("write");
    let read = Bundle::read_dir(dir.path()).expect("read");
    ingest(&target, &read).await.expect("ingest");
    assert_eq!(exported(&target).await, bundle);
}

#[tokio::test]
async fn a_bare_mention_gains_its_target_title() {
    let (source, target) = (open().await, open().await);
    let ids = rich_store(&source).await;
    let mut bundle = exported(&source).await;
    edit(&mut bundle, &landmark_path(ids.root), |file| {
        file.replace("\nintro with ", "\nintro with [[Child page]] ")
    });

    ingest(&target, &bundle).await.expect("ingest");
    let intro = target
        .get::<Mark>(&ids.intro)
        .await
        .expect("get")
        .expect("intro");
    let MarkData::Content(Content::Paragraph { rich_text, .. }) = intro.data else {
        panic!("intro is a paragraph");
    };
    let mention = rich_text
        .segments()
        .iter()
        .find(|segment| {
            segment
                .marks
                .iter()
                .any(|m| m.kind == TextMarkKind::Mention)
        })
        .expect("a mention");
    assert_eq!(mention.text, "Child page");
}

#[tokio::test]
async fn a_newer_grammar_is_ingested_with_an_advisory() {
    let (source, target) = (open().await, open().await);
    rich_store(&source).await;
    let mut bundle = exported(&source).await;
    edit(&mut bundle, layout::MANIFEST, |manifest| {
        manifest.replace(
            &format!("\"domain_version\": \"{}\"", domain::VERSION),
            "\"domain_version\": \"99.0.0\"",
        )
    });

    let ingested = ingest(&target, &bundle).await.expect("ingest");
    assert!(
        ingested.advisories.iter().any(|a| a.contains("99.0.0")),
        "{:?}",
        ingested.advisories
    );
}

#[tokio::test]
async fn an_unsupported_format_is_refused_before_anything_else_is_read() {
    let (source, target) = (open().await, open().await);
    rich_store(&source).await;
    let mut bundle = exported(&source).await;
    edit(&mut bundle, layout::MANIFEST, |m| {
        m.replace("\"0.1.0\"", "\"9.9.9\"")
    });
    // Garbage everywhere else: if any of it were read, the error would differ.
    bundle.insert("assets/junk.bin", "not a bundle file");
    bundle.insert(landmark_path(MarkID::new()), "not even frontmatter");

    let before = census(&target).await;
    let error = ingest(&target, &bundle).await.expect_err("must refuse");
    assert!(
        matches!(error, IngestError::UnsupportedFormat { ref declared, .. } if declared == "9.9.9"),
        "got {error}"
    );
    assert_eq!(census(&target).await, before);
}

#[tokio::test]
async fn a_src_keyed_identity_is_refused() {
    let (source, target) = (open().await, open().await);
    let ids = rich_store(&source).await;
    let mut bundle = exported(&source).await;
    edit(&mut bundle, &landmark_path(ids.root), |file| {
        file.replacen(
            &format!("wp:mark {} ", ids.intro),
            "wp:mark {\"src\":\"logseq/page-7\"} ",
            1,
        )
    });

    let refusal = refused(&target, &bundle).await;
    assert!(
        refusal
            .malformed
            .iter()
            .any(|m| m.kind == MalformedKind::SourceKeyedIdentity && m.at.line.is_some()),
        "named with file and line: {refusal}"
    );
}

#[tokio::test]
async fn divergent_duplicates_name_every_file() {
    let (source, target) = (open().await, open().await);
    source.insert(chart()).await.expect("chart");
    let (one, two, shared) = (MarkID::new(), MarkID::new(), MarkID::new());
    source.insert(landmark(one, "One")).await.expect("one");
    source.insert(landmark(two, "Two")).await.expect("two");
    source
        .insert(paragraph(shared, "shared text"))
        .await
        .expect("shared");
    source.insert(child(one, shared, "a")).await.expect("edge");
    source.insert(child(two, shared, "a")).await.expect("edge");

    let clean = exported(&source).await;
    ingest(&open().await, &clean)
        .await
        .expect("identical copies unify");

    let mut bundle = clean;
    edit(&mut bundle, &landmark_path(two), |f| {
        f.replace("shared text", "tampered")
    });
    let refusal = refused(&target, &bundle).await;
    let conflict = refusal
        .duplicates
        .iter()
        .find(|dup| dup.id == shared)
        .expect("a conflict for the shared Mark");
    assert!(conflict.files.contains(&landmark_path(one)), "{conflict:?}");
    assert!(conflict.files.contains(&landmark_path(two)), "{conflict:?}");
}

#[tokio::test]
async fn an_orphan_annotative_line_is_refused() {
    let (source, target) = (open().await, open().await);
    let ids = rich_store(&source).await;
    let mut bundle = exported(&source).await;
    edit(&mut bundle, layout::TERRAIN, |terrain| {
        format!(
            "{terrain}{}\n",
            serde_json::json!({
                "src": ids.nested.to_string(),
                "dst": ids.intro.to_string(),
                "kind": "child",
                "order": 0,
                "extra": { "x": 1 },
            })
        )
    });

    let refusal = refused(&target, &bundle).await;
    assert!(
        refusal.malformed.iter().any(|m| matches!(
            m.kind,
            MalformedKind::OrphanAnnotativeEdge { .. }
        ) && m.at.file == layout::TERRAIN
            && m.at.line.is_some()),
        "{refusal}"
    );
}

#[tokio::test]
async fn a_schema_invalid_reading_is_refused_naming_file_and_feature() {
    let (source, target) = (open().await, open().await);
    let status = FeatureID::derived("status");
    let mut options = IndexMap::new();
    for key in ["open", "done"] {
        let option = SelectOption::builder().with_name(key).build().keyed(key);
        options.insert(option.id.clone(), option);
    }
    let mut chart = chart();
    chart.features.insert(
        status.clone(),
        Feature {
            id: status.clone(),
            name: "Status".into(),
            kind: FeatureKind::Select {
                options,
                default_option: None,
            },
            description: None,
            required: false,
        },
    );
    source.insert(chart).await.expect("chart");
    let root = MarkID::new();
    let mut mark = landmark(root, "Hi");
    if let MarkData::Landmark(data) = &mut mark.data {
        data.readings.insert(
            status.clone(),
            Reading::Select(domain::GlyphID::derived("open")),
        );
    }
    source.insert(mark).await.expect("landmark");

    let mut bundle = exported(&source).await;
    let open_glyph = domain::GlyphID::derived("open").to_string();
    edit(&mut bundle, &landmark_path(root), |f| {
        f.replace(
            &format!("{status}: {open_glyph}"),
            &format!("{status}: zzzz"),
        )
    });

    let refusal = refused(&target, &bundle).await;
    assert!(
        refusal.malformed.iter().any(|m| matches!(
            &m.kind,
            MalformedKind::SchemaViolation { feature, .. } if *feature == status
        ) && m.at.file == landmark_path(root)),
        "{refusal}"
    );
}

#[tokio::test]
async fn unknown_content_is_refused() {
    let (source, target) = (open().await, open().await);
    rich_store(&source).await;
    let mut bundle = exported(&source).await;
    bundle.insert("assets/logo.png", "pretend");

    let refusal = refused(&target, &bundle).await;
    assert!(
        refusal
            .malformed
            .iter()
            .any(|m| matches!(m.kind, MalformedKind::UnknownContent { .. })),
        "{refusal}"
    );
}

#[tokio::test]
async fn a_valid_prefix_with_an_invalid_tail_writes_nothing() {
    let (source, target) = (open().await, open().await);
    source.insert(chart()).await.expect("chart");
    let mut roots: Vec<MarkID> = (0..8).map(|_| MarkID::new()).collect();
    roots.sort();
    for (index, root) in roots.iter().enumerate() {
        source
            .insert(landmark(*root, &format!("Landmark {index}")))
            .await
            .expect("landmark");
        let block = MarkID::new();
        source
            .insert(paragraph(block, "fine"))
            .await
            .expect("block");
        source.insert(child(*root, block, "a")).await.expect("edge");
    }
    let mut bundle = exported(&source).await;
    // The last file in path order is the one ingest reaches last.
    let last = landmark_path(*roots.last().expect("roots"));
    edit(&mut bundle, &last, |f| {
        format!("{f}<!-- wp:mark {{\"id\":\"broken\"}} -->\n")
    });

    // A target already holding unrelated data, which must survive as-is.
    target.insert(chart()).await.expect("chart");
    target
        .insert(landmark(MarkID::new(), "Pre-existing"))
        .await
        .expect("existing");

    let refusal = refused(&target, &bundle).await;
    assert!(
        refusal.malformed.iter().any(|m| m.at.file == last),
        "{refusal}"
    );
}

#[tokio::test]
async fn a_builtin_requiring_what_this_build_lacks_is_a_chart_conflict() {
    let (source, target) = (open().await, open().await);
    let builtin = ChartID::builtin("meeting");
    let extra_feature = Feature {
        id: FeatureID::derived("from-the-future"),
        name: "From the future".into(),
        kind: FeatureKind::Text,
        description: None,
        required: false,
    };
    let ours = Chart {
        id: builtin.clone(),
        name: "Meeting".into(),
        description: None,
        icon: None,
        features: IndexMap::new(),
        extra: None,
    };
    let mut theirs = ours.clone();
    theirs
        .features
        .insert(extra_feature.id.clone(), extra_feature.clone());
    source.insert(theirs).await.expect("their builtin");
    target.insert(ours).await.expect("our builtin");

    let root = MarkID::new();
    let mut mark = landmark(root, "Hi");
    if let MarkData::Landmark(data) = &mut mark.data {
        data.chart = builtin.clone();
    }
    source.insert(mark).await.expect("landmark");

    let refusal = refused(&target, &exported(&source).await).await;
    let conflict = refusal.charts.first().expect("a chart conflict");
    assert_eq!(conflict.chart, builtin);
    assert_eq!(conflict.missing_features, vec![extra_feature.id]);
}

#[tokio::test]
async fn a_builtin_is_resolved_never_overwritten() {
    let (source, target) = (open().await, open().await);
    let builtin = ChartID::builtin("page");
    let page = |name: &str| Chart {
        id: builtin.clone(),
        name: name.into(),
        description: None,
        icon: None,
        features: IndexMap::new(),
        extra: None,
    };
    source.insert(page("Their page")).await.expect("theirs");
    target.insert(page("Our page")).await.expect("ours");
    let root = MarkID::new();
    let mut mark = landmark(root, "Hi");
    if let MarkData::Landmark(data) = &mut mark.data {
        data.chart = builtin.clone();
    }
    source.insert(mark).await.expect("landmark");

    ingest(&target, &exported(&source).await)
        .await
        .expect("ingest");
    let stored = target
        .get::<Chart>(&builtin)
        .await
        .expect("get")
        .expect("page");
    assert_eq!(stored.name, "Our page");
}
