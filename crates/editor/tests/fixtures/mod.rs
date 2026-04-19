use std::sync::Arc;

use domain::chart::ChartID;
use domain::fractional_index;
use domain::mark::{Content, Landmark, MarkData, MarkID, RichText};
use rstest::fixture;
use store::query::{mark, terrain};
use store::{DatabaseConnection, Migrator, MigratorTrait};

/// Create a fresh in-memory SQLite with migrations applied.
async fn test_db() -> DatabaseConnection {
    use sea_orm::Database;
    let db = Database::connect("sqlite::memory:").await.unwrap();
    Migrator::up(&db, None).await.unwrap();
    db
}

/// Wrap a DatabaseConnection in the SharedDb type EditorService expects.
fn shared(db: DatabaseConnection) -> ::editor::SharedDatabase {
    Arc::new(tokio::sync::Mutex::new(Some(db)))
}

/// rstest fixture — returns `(EditorService, DatabaseConnection)`.
/// The raw `db` handle is kept for direct store queries in assertions.
#[fixture]
pub async fn env() -> (::editor::EditorService, DatabaseConnection) {
    let db = test_db().await;
    let shared_db = shared(db.clone());
    let svc = ::editor::EditorService::new(shared_db);
    (svc, db)
}

// -- Factory helpers --

pub fn paragraph(text: &str) -> MarkData {
    MarkData::Content(Content::Paragraph {
        rich_text: RichText::from(text.to_string()),
    })
}

pub fn heading(text: &str) -> MarkData {
    MarkData::Content(Content::Heading1 {
        rich_text: RichText::from(text.to_string()),
    })
}

pub fn blockquote() -> MarkData {
    MarkData::Content(Content::Blockquote)
}

pub fn landmark(chart_name: &str, title: &str) -> MarkData {
    MarkData::Landmark(
        Landmark::builder()
            .with_chart(ChartID::builtin(chart_name))
            .with_title(title.to_string())
            .build(),
    )
}

pub fn new_id() -> MarkID {
    MarkID::new()
}

pub fn now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_utc()
}

/// Seed a landmark with `n` paragraph children. Returns `(landmark_id, child_ids)`.
pub async fn seed_landmark(db: &DatabaseConnection, n: usize) -> (MarkID, Vec<MarkID>) {
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

/// Seed a landmark with `n` root-level paragraphs, then nest `nested` paragraphs under the
/// specified root child (by index). Returns `(landmark_id, root_ids, nested_ids)`.
pub async fn seed_nested(
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
