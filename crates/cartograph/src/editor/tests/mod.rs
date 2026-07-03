mod mutations;
mod projection;
mod undo_redo;

use std::sync::Arc;

use domain::chart::ChartID;
use domain::fractional_index;
use domain::mark::{Content, Landmark, MarkData, MarkID, RichText};
use rstest::fixture;
use store::DatabaseConnection;
use store::query::{mark, terrain};

use crate::Database;
use crate::editor::service::EditorService;

type Env = (EditorService, DatabaseConnection);

async fn test_db() -> DatabaseConnection {
    store::test_utils::temp_db().await
}

fn shared(db: DatabaseConnection) -> Database {
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

async fn seed_landmark(db: &DatabaseConnection, n: usize) -> (MarkID, Vec<MarkID>) {
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
    db: &DatabaseConnection,
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
