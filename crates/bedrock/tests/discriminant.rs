//! `#[bedrock(discriminant)]` drives materialized-view scoping. The annotation
//! records which document field is the discriminant (and its serde tag), so a
//! collection can hand back a correctly-typed `ViewScope` without the caller
//! restating the path. Verified against a live view: a scope selects only the
//! matching kind, and a projected reading reads back typed.

use bedrock::{
    Collection, ColumnType, Database, Document, Scalar, ViewColumn, ViewScope, ViewSpec,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct EntryId(Uuid);

// A serde-tagged enum: the tag `"kind"` is the discriminant path, inferred by
// the Document derive and read by `#[bedrock(discriminant)]` below.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Body {
    Note { title: String },
    Task { title: String, done: bool },
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Entry {
    #[bedrock(id)]
    id: EntryId,
    #[bedrock(discriminant)]
    body: Body,
}

fn entry(n: u128, body: Body) -> Entry {
    Entry {
        id: EntryId(Uuid::from_u128(n)),
        body,
    }
}

#[tokio::test]
async fn inferred_discriminant_scopes_a_view_and_projects_a_reading() {
    // The tag path is inferred from `#[serde(tag = "kind")]`.
    assert_eq!(Entry::DISCRIMINANT_DOCUMENT, "body");
    assert_eq!(
        Entry::discriminant_scope("note"),
        ViewScope {
            document: "body".into(),
            ns: None,
            path: "kind".into(),
            equals: "note".into(),
        }
    );

    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Entry>().await.expect("ensure");
    db.insert_all(vec![
        entry(1, Body::Note { title: "a".into() }),
        entry(
            2,
            Body::Task {
                title: "b".into(),
                done: true,
            },
        ),
        entry(3, Body::Note { title: "c".into() }),
    ])
    .await
    .expect("insert");

    // A view scoped to notes, projecting the reading `body.title` as a typed column.
    let view = ViewSpec {
        name: "entry_notes".into(),
        scope: Some(Entry::discriminant_scope("note")),
        columns: vec![ViewColumn {
            name: "title".into(),
            document: "body".into(),
            ns: None,
            path: "title".into(),
            column_type: ColumnType::Text,
        }],
    };
    db.declare_view::<Entry>(&view).await.expect("declare");

    // Only the two Note rows are in the view; each row is [id, title].
    let mut rows = db.view_rows("entry_notes").await.expect("rows");
    let mut titles: Vec<String> = rows
        .iter_mut()
        .map(|r| match &r[1] {
            bedrock::StoredValue::Text(s) => s.clone(),
            other => panic!("expected text title, got {other:?}"),
        })
        .collect();
    titles.sort();
    assert_eq!(titles, ["a", "c"]);
}

// A document with no serde tag uses the explicit path form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct Meta {
    kind: String,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Tagged {
    #[bedrock(id)]
    id: EntryId,
    #[bedrock(discriminant = "kind")]
    meta: Meta,
}

#[test]
fn explicit_discriminant_and_general_scope() {
    assert_eq!(Tagged::DISCRIMINANT_DOCUMENT, "meta");
    // Explicit path form (no serde tag on `Meta`).
    assert_eq!(
        Tagged::discriminant_scope("chart-x"),
        ViewScope {
            document: "meta".into(),
            ns: None,
            path: "kind".into(),
            equals: "chart-x".into(),
        }
    );
    // The general `scope` targets an arbitrary path in the same document — the
    // deeper-granularity escape hatch the app uses for chart/feature views.
    assert_eq!(
        Tagged::scope("label", "hi"),
        ViewScope {
            document: "meta".into(),
            ns: None,
            path: "label".into(),
            equals: "hi".into(),
        }
    );
}
