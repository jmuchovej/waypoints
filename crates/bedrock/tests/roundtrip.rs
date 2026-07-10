use bedrock::Database;
use bedrock_core::Collection;
use bedrock_derive::{Collection, Document, Scalar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct NoteId(Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Scalar)]
#[serde(rename_all = "kebab-case")]
enum Status {
    Draft,
    Published,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct Body {
    text: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Note {
    #[bedrock(id)]
    id: NoteId,
    status: Status,
    body: Body,
    pinned: bool,
    rank: Option<i64>,
}

#[tokio::test]
async fn create_insert_readback_roundtrips() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Note>().await.expect("ensure");
    // idempotent — a second ensure is a no-op.
    db.ensure_collection::<Note>().await.expect("ensure again");

    let notes = vec![
        Note {
            id: NoteId(Uuid::from_u128(1)),
            status: Status::Draft,
            body: Body {
                text: "first".into(),
                tags: vec!["a".into(), "b".into()],
            },
            pinned: true,
            rank: Some(10),
        },
        Note {
            id: NoteId(Uuid::from_u128(2)),
            status: Status::Published,
            body: Body {
                text: "second".into(),
                tags: vec![],
            },
            pinned: false,
            rank: None,
        },
    ];
    for note in &notes {
        db.insert(note).await.expect("insert");
    }

    let mut got = db.all::<Note>().await.expect("all");
    got.sort_by_key(|n| n.id.0);
    let mut want = notes.clone();
    want.sort_by_key(|n| n.id.0);

    assert_eq!(got, want);
}
