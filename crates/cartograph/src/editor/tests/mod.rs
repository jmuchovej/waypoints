mod maneuver;
mod mutations;
mod projection;
mod readings;
mod undo_redo;

use crate::{Database, editor::service::EditorService};
use domain::{
    Chart, ChartID, Content, Feature, FeatureKind, Landmark, MarkData, MarkID, RichText,
    fractional_index,
};
use rstest::fixture;
use std::sync::Arc;
use store::{
    Store,
    query::{mark, terrain},
};

type Env = (EditorService, Store);

async fn test_db() -> Store {
    store::test_utils::temp_db().await
}

fn shared(db: Store) -> Database {
    Arc::new(tokio::sync::Mutex::new(Some(db)))
}

#[fixture]
async fn env() -> Env {
    let db = test_db().await;
    let shared_db = shared(db.clone());
    let svc = EditorService::new(shared_db);
    (svc, db)
}

fn paragraph(text: &str) -> MarkData {
    MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
}

fn blockquote() -> MarkData {
    MarkData::Content(Content::blockquote())
}

fn landmark(chart_name: &str, title: &str) -> MarkData {
    MarkData::Landmark(
        Landmark::builder()
            .with_chart(ChartID::builtin(chart_name))
            .with_title(title.to_string())
            .build(),
    )
}

fn new_id() -> MarkID {
    MarkID::new()
}

fn now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_utc()
}

fn feature(name: &str, kind: FeatureKind) -> Feature {
    Feature::builder().with_name(name).with_kind(kind).build()
}

/// Upsert a chart with the given features; re-upserting with the same chart
/// replaces it in-place (how tests narrow a feature's constraints).
async fn seed_chart(db: &Store, chart: &Chart) {
    db.db().upsert::<Chart>(chart.clone()).await.unwrap();
}

/// Insert a landmark of an arbitrary chart, without children.
async fn seed_landmark_of(db: &Store, chart_id: ChartID, title: &str) -> MarkID {
    let id = new_id();
    let data = MarkData::Landmark(
        Landmark::builder()
            .with_chart(chart_id)
            .with_title(title.to_string())
            .build(),
    );
    mark::insert(db, id, &data, now()).await.unwrap();
    id
}

async fn seed_landmark(db: &Store, n: usize) -> (MarkID, Vec<MarkID>) {
    let lm = new_id();
    mark::insert(db, lm, &landmark("page", "Test Page"), now())
        .await
        .unwrap();

    let keys = fractional_index::generate_n(n);
    let mut children = Vec::with_capacity(n);
    for (i, key) in keys.iter().enumerate() {
        let child = new_id();
        mark::insert(db, child, &paragraph(&format!("Block {}", i + 1)), now())
            .await
            .unwrap();
        terrain::insert_child_edge(db, lm, child, key)
            .await
            .unwrap();
        children.push(child);
    }

    (lm, children)
}

async fn seed_nested(
    db: &Store,
    n: usize,
    nest_under: usize,
    nested: usize,
) -> (MarkID, Vec<MarkID>, Vec<MarkID>) {
    let (lm, roots) = seed_landmark(db, n).await;

    let parent = roots[nest_under];
    let keys = fractional_index::generate_n(nested);
    let mut nested_ids = Vec::with_capacity(nested);
    for (i, key) in keys.iter().enumerate() {
        let child = new_id();
        mark::insert(db, child, &paragraph(&format!("Nested {}", i + 1)), now())
            .await
            .unwrap();
        terrain::insert_child_edge(db, parent, child, key)
            .await
            .unwrap();
        nested_ids.push(child);
    }

    (lm, roots, nested_ids)
}
