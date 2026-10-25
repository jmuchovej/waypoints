//! The exporter, against a live engine.
//!
//! Charts are self-contained fixtures: `transload` sits below `store`, so the
//! builtin seed is out of reach.

use bedrock::Database;
use chrono::{NaiveDate, NaiveDateTime};
use domain::{
    Base36ID, Chart, ChartID, Content, Feature, FeatureID, FeatureKind, Landmark, Mark, MarkData,
    MarkID, Reading, RichText, Terrain, TerrainID, TerrainKind,
};
use indexmap::IndexMap;
use transload::{ExportMeta, export, layout};

fn at(milli: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 2)
        .unwrap()
        .and_hms_milli_opt(9, 41, 0, milli)
        .unwrap()
}

fn meta() -> ExportMeta {
    ExportMeta {
        app_version: "0.3.1".into(),
        exported_at: at(0),
        territory: None,
    }
}

fn feature_id(id: &str) -> FeatureID {
    FeatureID(Base36ID(id.to_string()))
}

async fn open() -> Database {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Mark>().await.expect("marks");
    db.ensure_collection::<Terrain>().await.expect("terrains");
    db.ensure_collection::<Chart>().await.expect("charts");
    db
}

fn chart() -> Chart {
    let notes = Feature {
        id: feature_id("a1b2"),
        name: "Notes".into(),
        kind: FeatureKind::Text,
        description: None,
        required: false,
    };
    let mut features = IndexMap::new();
    features.insert(notes.id.clone(), notes);
    Chart {
        id: ChartID("c4d8".into()),
        name: "Meeting".into(),
        description: None,
        icon: None,
        features,
        extra: None,
    }
}

fn landmark(id: MarkID, title: &str, readings: IndexMap<FeatureID, Reading>) -> Mark {
    Mark {
        id,
        data: MarkData::Landmark(Landmark {
            chart: ChartID("c4d8".into()),
            title: RichText::from(title.to_string()),
            icon: None,
            readings,
        }),
        extra: None,
        created_at: at(0),
        updated_at: at(0),
        deleted_at: None,
    }
}

fn paragraph(id: MarkID, text: &str) -> Mark {
    Mark {
        id,
        data: MarkData::Content(Content::Paragraph {
            rich_text: RichText::from(text.to_string()),
            theme: None,
        }),
        extra: None,
        created_at: at(0),
        updated_at: at(0),
        deleted_at: None,
    }
}

fn child(src: MarkID, dst: MarkID, order: &str) -> Terrain {
    Terrain {
        id: TerrainID::new(),
        src_id: src,
        dst_id: dst,
        kind: TerrainKind::Child,
        order: Some(order.to_string()),
        extra: None,
        deleted_at: None,
    }
}

#[tokio::test]
async fn a_landmark_lands_where_its_chart_groups_it() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    db.insert(landmark(root, "Weekly sync", IndexMap::new()))
        .await
        .expect("landmark");

    let exported = export(&db, &meta()).await.expect("export");
    let path = layout::landmark(&ChartID("c4d8".into()), &root);
    assert!(
        exported.bundle.get(&path).is_some(),
        "expected {path}, got {:?}",
        exported.bundle.paths().collect::<Vec<_>>()
    );
    assert!(
        exported.bundle.get("library/c4d8/_schema.json").is_some(),
        "every referenced Chart emits a schema"
    );
    assert!(exported.bundle.get(layout::MANIFEST).is_some());
}

#[tokio::test]
async fn no_reserved_directory_is_emitted() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    db.insert(landmark(MarkID::new(), "Hi", IndexMap::new()))
        .await
        .expect("landmark");

    let exported = export(&db, &meta()).await.expect("export");
    for path in exported.bundle.paths() {
        for reserved in layout::RESERVED {
            assert!(
                !path.starts_with(&format!("{reserved}/")),
                "{path} is under a reserved directory"
            );
        }
    }
}

#[tokio::test]
async fn a_body_carries_identity_comments_and_its_blocks() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let first = MarkID::new();
    let second = MarkID::new();
    db.insert(landmark(root, "Weekly sync", IndexMap::new()))
        .await
        .expect("landmark");
    db.insert(paragraph(first, "first")).await.expect("first");
    db.insert(paragraph(second, "second"))
        .await
        .expect("second");
    // Deliberately inserted out of order, to prove the fractional key orders.
    db.insert(child(root, second, "b")).await.expect("edge b");
    db.insert(child(root, first, "a")).await.expect("edge a");

    let exported = export(&db, &meta()).await.expect("export");
    let file = exported
        .bundle
        .get(&layout::landmark(&ChartID("c4d8".into()), &root))
        .expect("landmark file");

    assert!(
        file.contains(&format!(
            "<!-- wp:mark {first} -->\nfirst\n\n<!-- wp:mark {second} -->\nsecond\n"
        )),
        "a comment carries only the id, and siblings stand a blank line apart:\n{file}"
    );
    let marks = exported.bundle.get(layout::MARKS).expect("marks.jsonl");
    assert!(
        marks.contains(&format!(
            "{{\"id\":\"{first}\",\"created\":\"2026-08-02T09:41:00.000Z\""
        )),
        "a Content Mark's timestamps ride the sidecar:\n{marks}"
    );
    let first_at = file.find("\nfirst\n").expect("first");
    let second_at = file.find("\nsecond\n").expect("second");
    assert!(first_at < second_at, "ordered by fractional key:\n{file}");
    assert!(!file.contains("<!-- /wp:mark -->"), "got\n{file}");
}

#[tokio::test]
async fn a_nested_landmark_keeps_its_place_among_its_siblings() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let (root, before, nested, after) =
        (MarkID::new(), MarkID::new(), MarkID::new(), MarkID::new());
    db.insert(landmark(root, "Parent", IndexMap::new()))
        .await
        .expect("root");
    db.insert(landmark(nested, "Child page", IndexMap::new()))
        .await
        .expect("nested");
    db.insert(paragraph(before, "before"))
        .await
        .expect("before");
    db.insert(paragraph(after, "after")).await.expect("after");
    db.insert(child(root, before, "a")).await.expect("edge");
    db.insert(child(root, nested, "b")).await.expect("edge");
    db.insert(child(root, after, "c")).await.expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let file = exported
        .bundle
        .get(&layout::landmark(&ChartID("c4d8".into()), &root))
        .expect("parent file");
    let link = "[[Child page]]";
    let (b, l, a) = (
        file.find("\nbefore\n").expect("before"),
        file.find(link).expect("nested link"),
        file.find("\nafter\n").expect("after"),
    );
    assert!(
        b < l && l < a,
        "the nested Landmark sits between its siblings:\n{file}"
    );
    assert!(exported.warnings.is_empty(), "{:?}", exported.warnings);
}

#[tokio::test]
async fn a_cycle_is_broken_with_a_warning_not_an_infinite_walk() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let (root, a, b) = (MarkID::new(), MarkID::new(), MarkID::new());
    db.insert(landmark(root, "Hi", IndexMap::new()))
        .await
        .expect("root");
    db.insert(paragraph(a, "a")).await.expect("a");
    db.insert(paragraph(b, "b")).await.expect("b");
    db.insert(child(root, a, "a")).await.expect("edge");
    db.insert(child(a, b, "a")).await.expect("edge");
    db.insert(child(b, a, "a")).await.expect("cycle");

    let exported = export(&db, &meta()).await.expect("export");
    assert!(
        exported.warnings.iter().any(|w| w.contains("cycle")),
        "{:?}",
        exported.warnings
    );
}

#[tokio::test]
async fn unreachable_content_is_reported() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    db.insert(landmark(MarkID::new(), "Hi", IndexMap::new()))
        .await
        .expect("root");
    let stray = MarkID::new();
    db.insert(paragraph(stray, "stray")).await.expect("stray");

    let exported = export(&db, &meta()).await.expect("export");
    assert!(
        exported
            .warnings
            .iter()
            .any(|w| w.contains(&stray.to_string())),
        "{:?}",
        exported.warnings
    );
}

#[tokio::test]
async fn a_reading_edge_is_carried_by_its_own_line() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let (src, dst) = (MarkID::new(), MarkID::new());
    db.insert(landmark(src, "One", IndexMap::new()))
        .await
        .expect("src");
    db.insert(landmark(dst, "Two", IndexMap::new()))
        .await
        .expect("dst");
    db.insert(Terrain {
        id: TerrainID::new(),
        src_id: src,
        dst_id: dst,
        kind: TerrainKind::Reading(ChartID("c4d8".into()), feature_id("a1b2")),
        order: None,
        extra: None,
        deleted_at: None,
    })
    .await
    .expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let terrain = exported.bundle.get(layout::TERRAIN).expect("terrain.jsonl");
    assert!(terrain.contains("reading::c4d8::a1b2"), "{terrain}");
}

#[tokio::test]
async fn a_container_pairs_its_comments_around_its_children() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let quote = MarkID::new();
    let inner = MarkID::new();
    db.insert(landmark(root, "Hi", IndexMap::new()))
        .await
        .expect("landmark");
    db.insert(Mark {
        id: quote,
        data: MarkData::Content(Content::Blockquote { theme: None }),
        extra: None,
        created_at: at(0),
        updated_at: at(0),
        deleted_at: None,
    })
    .await
    .expect("quote");
    db.insert(paragraph(inner, "quoted")).await.expect("inner");
    db.insert(child(root, quote, "a")).await.expect("edge");
    db.insert(child(quote, inner, "a")).await.expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let file = exported
        .bundle
        .get(&layout::landmark(&ChartID("c4d8".into()), &root))
        .expect("landmark file");

    assert!(
        file.contains("<!-- /wp:mark -->"),
        "a container closes:\n{file}"
    );
    assert!(
        file.contains("> <!-- wp:mark"),
        "children wear the blockquote prefix:\n{file}"
    );
}

#[tokio::test]
async fn tombstones_stay_behind() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let gone = MarkID::new();
    db.insert(landmark(root, "Hi", IndexMap::new()))
        .await
        .expect("landmark");
    let mut deleted = paragraph(gone, "deleted text");
    deleted.deleted_at = Some(at(5));
    db.insert(deleted).await.expect("deleted");
    db.insert(child(root, gone, "a")).await.expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    for (path, contents) in exported.bundle.iter() {
        assert!(
            !contents.contains(&gone.to_string()),
            "{path} references a soft-deleted Mark"
        );
        assert!(!contents.contains("deleted text"), "{path} carries it");
    }
}

#[tokio::test]
async fn exporting_twice_is_byte_identical() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    db.insert(landmark(root, "Weekly sync", IndexMap::new()))
        .await
        .expect("landmark");
    for index in 0..5 {
        let id = MarkID::new();
        db.insert(paragraph(id, &format!("block {index}")))
            .await
            .expect("paragraph");
        db.insert(child(root, id, &format!("a{index}")))
            .await
            .expect("edge");
    }

    let first = export(&db, &meta()).await.expect("first");
    let second = export(&db, &meta()).await.expect("second");
    assert_eq!(
        first.bundle, second.bundle,
        "a second export of one store is byte-identical"
    );
}

#[tokio::test]
async fn mark_extra_rides_the_sidecar() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let noted = MarkID::new();
    db.insert(landmark(root, "Hi", IndexMap::new()))
        .await
        .expect("landmark");
    let mut mark = paragraph(noted, "text");
    mark.extra = Some(serde_json::json!({ "so.waypoints.test": 1 }));
    db.insert(mark).await.expect("mark");
    db.insert(child(root, noted, "a")).await.expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let marks = exported.bundle.get(layout::MARKS).expect("marks.jsonl");
    assert!(marks.contains(&noted.to_string()), "got {marks}");
    assert!(marks.contains("so.waypoints.test"), "got {marks}");
}

#[tokio::test]
async fn a_constitutive_edge_becomes_a_terrain_line() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let src = MarkID::new();
    let dst = MarkID::new();
    db.insert(landmark(src, "One", IndexMap::new()))
        .await
        .expect("src");
    db.insert(landmark(dst, "Two", IndexMap::new()))
        .await
        .expect("dst");
    db.insert(Terrain {
        id: TerrainID::new(),
        src_id: src,
        dst_id: dst,
        kind: TerrainKind::Custom,
        order: None,
        extra: Some(serde_json::json!({ "weight": 3 })),
        deleted_at: None,
    })
    .await
    .expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let terrain = exported.bundle.get(layout::TERRAIN).expect("terrain.jsonl");
    assert!(terrain.contains("\"kind\":\"custom\""), "got {terrain}");
    assert!(terrain.contains("\"weight\":3"), "got {terrain}");
    // No TerrainID ever reaches a Bundle.
    assert!(!terrain.contains("\"id\""), "got {terrain}");
}

#[tokio::test]
async fn a_structural_edge_writes_no_line_unless_it_carries_extra() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let plain = MarkID::new();
    let noted = MarkID::new();
    db.insert(landmark(root, "Hi", IndexMap::new()))
        .await
        .expect("landmark");
    db.insert(paragraph(plain, "plain")).await.expect("plain");
    db.insert(paragraph(noted, "noted")).await.expect("noted");
    db.insert(child(root, plain, "a"))
        .await
        .expect("plain edge");
    let mut annotated = child(root, noted, "b");
    annotated.extra = Some(serde_json::json!({ "pinned": true }));
    db.insert(annotated).await.expect("annotated edge");

    let exported = export(&db, &meta()).await.expect("export");
    let terrain = exported.bundle.get(layout::TERRAIN).unwrap_or("");
    assert!(terrain.contains("\"pinned\":true"), "got {terrain}");
    assert!(
        !terrain.contains(&plain.to_string()),
        "a plain child edge is carried by nesting alone: {terrain}"
    );
    // An annotative line always says which parallel edge it annotates.
    assert!(terrain.contains("\"order\""), "got {terrain}");
}

#[tokio::test]
async fn the_manifest_counts_and_censuses_what_was_exported() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    let root = MarkID::new();
    let block = MarkID::new();
    let mut readings = IndexMap::new();
    readings.insert(
        feature_id("a1b2"),
        Reading::Text(RichText::from("some notes".to_string())),
    );
    db.insert(landmark(root, "Weekly sync", readings))
        .await
        .expect("landmark");
    db.insert(Mark {
        id: block,
        data: MarkData::Content(Content::Divider { theme: None }),
        extra: None,
        created_at: at(0),
        updated_at: at(0),
        deleted_at: None,
    })
    .await
    .expect("divider");
    db.insert(child(root, block, "a")).await.expect("edge");

    let exported = export(&db, &meta()).await.expect("export");
    let manifest: serde_json::Value =
        serde_json::from_str(exported.bundle.get(layout::MANIFEST).expect("manifest"))
            .expect("manifest is JSON");

    assert_eq!(manifest["bundle_format"], "0.1.0");
    assert_eq!(manifest["domain_version"], domain::VERSION);
    assert_eq!(manifest["counts"]["landmarks"], 1);
    assert_eq!(manifest["counts"]["charts"], 1);
    assert_eq!(manifest["uses"]["content"], serde_json::json!(["divider"]));
    assert_eq!(manifest["uses"]["readings"], serde_json::json!(["text"]));
    assert_eq!(
        manifest["uses"]["terrain_kinds"],
        serde_json::json!(["child"])
    );
    assert_eq!(
        manifest["uses"]["feature_kinds"],
        serde_json::json!(["text"])
    );
}

#[tokio::test]
async fn an_export_warns_rather_than_refusing_when_the_store_disagrees_with_itself() {
    let db = open().await;
    db.insert(chart()).await.expect("chart");
    // A reading for a feature the Chart does not define.
    let mut readings = IndexMap::new();
    readings.insert(
        feature_id("zzzz"),
        Reading::Text(RichText::from("orphaned".to_string())),
    );
    let root = MarkID::new();
    db.insert(landmark(root, "Weekly sync", readings))
        .await
        .expect("landmark");

    let exported = export(&db, &meta()).await.expect("export must not refuse");
    assert!(
        !exported.warnings.is_empty(),
        "an inconsistent store is reported"
    );
    assert!(
        exported.warnings.iter().any(|w| w.contains("zzzz")),
        "the warning names the feature: {:?}",
        exported.warnings
    );
    let file = exported
        .bundle
        .get(&layout::landmark(&ChartID("c4d8".into()), &root))
        .expect("landmark file");
    assert!(file.contains("orphaned"), "got\n{file}");
}
