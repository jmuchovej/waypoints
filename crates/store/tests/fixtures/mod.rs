use domain::ChartID;
use domain::fractional_index;
use domain::{Content, Landmark, MarkData, MarkID, RichText};
use rstest::fixture;
use store::Store;
use store::query::{mark, terrain};

/// rstest fixture — each test gets a fresh bedrock Store in a tempdir.
#[fixture]
pub async fn db() -> Store {
    let dir = tempfile::tempdir().unwrap().keep();
    store::open(&dir.join("test.db")).await.unwrap()
}

/// A paragraph content mark with the given plain text.
pub fn paragraph(text: &str) -> MarkData {
    MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
}

/// A heading1 content mark.
pub fn heading(text: &str) -> MarkData {
    MarkData::Content(Content::heading1(RichText::from(text.to_string())))
}

/// A blockquote container mark.
pub fn blockquote() -> MarkData {
    MarkData::Content(Content::blockquote())
}

/// A minimal landmark mark using a builtin chart.
pub fn landmark(chart_name: &str, title: &str) -> MarkData {
    MarkData::Landmark(
        Landmark::builder()
            .with_chart(ChartID::builtin(chart_name))
            .with_title(title.to_string())
            .build(),
    )
}

/// Generate a fresh MarkID.
pub fn new_id() -> MarkID {
    MarkID::new()
}

/// Current naive UTC timestamp.
pub fn now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_utc()
}

/// Insert a parent landmark with `n` paragraph children (M1..Mn).
/// Returns `(parent_id, child_ids, fractional_keys)`.
pub async fn parent_with_children(db: &Store, n: usize) -> (MarkID, Vec<MarkID>, Vec<String>) {
    let parent = new_id();
    mark::insert(db, parent, &landmark("page", "Parent"), now())
        .await
        .unwrap();

    let keys = fractional_index::generate_n(n);
    let mut children = Vec::with_capacity(n);
    for (i, key) in keys.iter().enumerate() {
        let child = new_id();
        mark::insert(db, child, &paragraph(&format!("M{}", i + 1)), now())
            .await
            .unwrap();
        terrain::insert_child_edge(db, parent, child, key)
            .await
            .unwrap();
        children.push(child);
    }

    (parent, children, keys)
}
