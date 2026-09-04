//! Real-engine tests for sync-owned persistence: winning arbitration keys
//! per fold target, and the incremental fold cursor.

use outpost_protocol::{HLC, NoticeID};
use store::query::sync::{self, StoredKey};

fn key(hlc: i64, author: u8, notice: u8) -> StoredKey {
    StoredKey {
        hlc: HLC(hlc),
        author: [author; 32],
        notice: NoticeID::from_bytes([notice; 32]),
    }
}

#[tokio::test]
async fn winner_round_trips_and_overwrites() {
    let db = store::test_utils::temp_db().await;

    assert_eq!(sync::winner(&db, "mark:x").await.unwrap(), None);

    let first = key(7, 1, 1);
    sync::set_winner(&db, "mark:x", first).await.unwrap();
    assert_eq!(sync::winner(&db, "mark:x").await.unwrap(), Some(first));

    // Overwrite in place — one row per target.
    let second = key(9, 2, 2);
    sync::set_winner(&db, "mark:x", second).await.unwrap();
    assert_eq!(sync::winner(&db, "mark:x").await.unwrap(), Some(second));

    // Targets are independent rows.
    sync::set_winner(&db, "title:x", first).await.unwrap();
    assert_eq!(sync::winner(&db, "title:x").await.unwrap(), Some(first));
    assert_eq!(sync::winner(&db, "mark:x").await.unwrap(), Some(second));
}

#[tokio::test]
async fn fold_cursor_defaults_and_advances() {
    let db = store::test_utils::temp_db().await;

    assert_eq!(sync::fold_cursor(&db).await.unwrap(), -1);
    sync::set_fold_cursor(&db, 41).await.unwrap();
    assert_eq!(sync::fold_cursor(&db).await.unwrap(), 41);
    sync::set_fold_cursor(&db, 42).await.unwrap();
    assert_eq!(sync::fold_cursor(&db).await.unwrap(), 42);
}
