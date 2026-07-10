use bedrock::Database;
use bedrock_core::Collection;
use bedrock_derive::{Collection, Document, Scalar};
use chrono::{NaiveDate, NaiveDateTime};
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
    created: NaiveDateTime,
}

fn at(milli: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 7, 11)
        .unwrap()
        .and_hms_milli_opt(9, 30, 0, milli)
        .unwrap()
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
            created: at(500),
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
            created: at(0),
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

#[tokio::test]
async fn rejects_submillisecond_timestamp() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Note>().await.expect("ensure");

    let note = Note {
        id: NoteId(Uuid::from_u128(3)),
        status: Status::Draft,
        body: Body {
            text: "x".into(),
            tags: vec![],
        },
        pinned: false,
        rank: None,
        // 500_123 microseconds = 500.123 ms — finer than the engine stores.
        created: NaiveDate::from_ymd_opt(2026, 7, 11)
            .unwrap()
            .and_hms_micro_opt(9, 30, 0, 500_123)
            .unwrap(),
    };

    let err = db.insert(&note).await.unwrap_err();
    assert!(matches!(err, bedrock::BedrockError::SubMillisecond { .. }));
}

fn note(n: u128) -> Note {
    Note {
        id: NoteId(Uuid::from_u128(n)),
        status: Status::Draft,
        body: Body {
            text: "x".into(),
            tags: vec![],
        },
        pinned: false,
        rank: None,
        created: at(0),
    }
}

#[tokio::test]
async fn crud_operations() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Note>().await.expect("ensure");

    let mut n = note(7);
    n.rank = Some(1);

    db.insert(&n).await.expect("insert");
    assert_eq!(db.count::<Note>().await.unwrap(), 1);
    assert_eq!(db.get::<Note>(&n.id).await.unwrap().as_ref(), Some(&n));

    // update in place
    n.status = Status::Published;
    n.body.text = "published".into();
    assert!(db.update(&n).await.unwrap());
    assert_eq!(db.get::<Note>(&n.id).await.unwrap().as_ref(), Some(&n));

    // upsert replaces, does not duplicate
    n.rank = Some(99);
    db.upsert(&n).await.expect("upsert");
    assert_eq!(db.count::<Note>().await.unwrap(), 1);
    assert_eq!(db.get::<Note>(&n.id).await.unwrap().unwrap().rank, Some(99));

    // delete
    assert!(db.delete::<Note>(&n.id).await.unwrap());
    assert!(db.get::<Note>(&n.id).await.unwrap().is_none());
    assert!(!db.delete::<Note>(&n.id).await.unwrap()); // already gone
    assert_eq!(db.count::<Note>().await.unwrap(), 0);
}

#[tokio::test]
async fn insert_all_is_atomic() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Note>().await.expect("ensure");

    db.insert_all(&[note(10), note(11)])
        .await
        .expect("insert_all");
    assert_eq!(db.count::<Note>().await.unwrap(), 2);

    // note(10) already exists → primary-key violation → the whole batch rolls back,
    // so note(12) must not survive.
    let result = db.insert_all(&[note(12), note(10)]).await;
    assert!(result.is_err());
    assert_eq!(db.count::<Note>().await.unwrap(), 2);
    assert!(
        db.get::<Note>(&NoteId(Uuid::from_u128(12)))
            .await
            .unwrap()
            .is_none()
    );
}
