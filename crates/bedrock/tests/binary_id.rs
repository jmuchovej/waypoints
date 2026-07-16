//! Fixed-size byte arrays as native binary primary keys: `[u8; N]` is a
//! BLOB column, so a content hash can be a PK (or an indexed column)
//! without a text-encoding detour.

use bedrock::Database;
use bedrock_derive::{Collection, Scalar};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Scalar)]
struct Hash([u8; 32]);

#[derive(Debug, Clone, PartialEq, Collection)]
#[bedrock(index("track", "topo"))]
struct Record {
    #[bedrock(id)]
    id: Hash,
    #[bedrock(index)]
    track: [u8; 32],
    bytes: Vec<u8>,
    topo: i64,
}

fn record(seed: u8, topo: i64) -> Record {
    Record {
        id: Hash([seed; 32]),
        track: [0xF0; 32],
        bytes: vec![seed; 4],
        topo,
    }
}

#[tokio::test]
async fn binary_pk_crud_roundtrips() {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Record>().await.expect("ensure");

    let a = record(1, 10);
    let b = record(2, 20);
    db.insert(a.clone()).await.expect("insert a");
    db.insert(b.clone()).await.expect("insert b");

    // Point lookup on the BLOB PK.
    let got = db.get::<Record>(&a.id).await.expect("get");
    assert_eq!(got, Some(a.clone()));

    // Duplicate binary PK is a constraint violation, not a silent upsert.
    assert!(db.insert(record(1, 99)).await.is_err());

    // Update keyed on the binary id.
    let mut a2 = a.clone();
    a2.topo = 11;
    assert!(db.update(a2.clone()).await.expect("update"));
    assert_eq!(db.get::<Record>(&a.id).await.expect("get"), Some(a2));

    // Delete keyed on the binary id.
    assert!(db.delete::<Record>(&a.id).await.expect("delete"));
    assert_eq!(db.get::<Record>(&a.id).await.expect("get"), None);
    assert_eq!(db.get::<Record>(&b.id).await.expect("get"), Some(b));
}

#[tokio::test]
async fn blob_columns_filter_and_sort() {
    use bedrock::{Filter, Query};

    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Record>().await.expect("ensure");

    let track_a = [0xF0; 32];
    let mut other = record(3, 5);
    other.track = [0x0A; 32];
    for r in [record(1, 10), record(2, 20), other] {
        db.insert(r).await.expect("insert");
    }

    // Equality on a BLOB column, ordered by the composite index's topo.
    let q = Query::new()
        .filter(Filter::eq("track", track_a))
        .sort_asc("topo");
    let got = db.query::<Record>(&q).await.expect("query");
    assert_eq!(
        got.iter().map(|r| (r.id, r.topo)).collect::<Vec<_>>(),
        vec![(Hash([1; 32]), 10), (Hash([2; 32]), 20)]
    );

    // Range on topo scoped to the blob-track.
    let q = Query::new().filter(Filter::all([
        Filter::eq("track", track_a),
        Filter::gt("topo", 10i64),
    ]));
    let got = db.query::<Record>(&q).await.expect("query");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, Hash([2; 32]));
}

#[tokio::test]
async fn wrong_length_blob_is_a_decode_error() {
    use bedrock::{Field, StoredValue};

    let long = StoredValue::Blob(vec![0u8; 33]);
    assert!(<[u8; 32] as Field>::decode(long).is_err());
    let exact = StoredValue::Blob(vec![7u8; 32]);
    assert_eq!(<[u8; 32] as Field>::decode(exact).unwrap(), [7u8; 32]);
}
