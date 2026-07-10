mod fixtures;

use fixtures::*;
use rstest::rstest;
use store::Store;
use store::StoreError;
use store::query::mark;

#[rstest]
#[tokio::test]
async fn insert_and_get(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    let data = paragraph("hello world");

    mark::insert(&db, id, &data, now()).await.unwrap();
    let fetched = mark::get(&db, id)
        .await
        .unwrap()
        .expect("mark should exist");

    assert_eq!(fetched.id, id);
    assert_eq!(fetched.data, data);
}

#[rstest]
#[tokio::test]
async fn get_nonexistent_returns_none(#[future] db: Store) {
    let db = db.await;
    let result = mark::get(&db, new_id()).await.unwrap();
    assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn update_data_changes_content(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &paragraph("before"), now())
        .await
        .unwrap();

    let updated = paragraph("after");
    mark::update_data(&db, id, &updated).await.unwrap();

    let fetched = mark::get(&db, id).await.unwrap().unwrap();
    assert_eq!(fetched.data, updated);
}

#[rstest]
#[tokio::test]
async fn update_nonexistent_mark_errors(#[future] db: Store) {
    let db = db.await;
    let err = mark::update_data(&db, new_id(), &paragraph("x"))
        .await
        .unwrap_err();
    assert!(matches!(err, StoreError::MarkNotFound(_)));
}

#[rstest]
#[tokio::test]
async fn soft_delete_hides_from_get(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &paragraph("doomed"), now())
        .await
        .unwrap();

    mark::soft_delete(&db, id).await.unwrap();

    let result = mark::get(&db, id).await.unwrap();
    assert!(result.is_none(), "soft-deleted mark should not be visible");
}

#[rstest]
#[tokio::test]
async fn soft_delete_hides_from_list_landmarks(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &landmark("page", "My Page"), now())
        .await
        .unwrap();

    assert!(
        mark::list(&db, Some("landmark"), None)
            .await
            .unwrap()
            .iter()
            .any(|m| m.id == id)
    );

    mark::soft_delete(&db, id).await.unwrap();

    assert!(
        !mark::list(&db, Some("landmark"), None)
            .await
            .unwrap()
            .iter()
            .any(|m| m.id == id)
    );
}

#[rstest]
#[tokio::test]
async fn hard_delete_removes_permanently(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &paragraph("gone"), now())
        .await
        .unwrap();

    mark::hard_delete(&db, id).await.unwrap();

    assert!(mark::get(&db, id).await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn hard_delete_nonexistent_errors(#[future] db: Store) {
    let db = db.await;
    let err = mark::hard_delete(&db, new_id()).await.unwrap_err();
    assert!(matches!(err, StoreError::MarkNotFound(_)));
}

#[rstest]
#[tokio::test]
async fn list_landmarks_excludes_content_marks(#[future] db: Store) {
    let db = db.await;

    let content_id = new_id();
    mark::insert(&db, content_id, &paragraph("just text"), now())
        .await
        .unwrap();

    let landmark_id = new_id();
    mark::insert(&db, landmark_id, &landmark("page", "A Page"), now())
        .await
        .unwrap();

    let landmarks = mark::list(&db, Some("landmark"), None).await.unwrap();
    assert!(landmarks.iter().any(|m| m.id == landmark_id));
    assert!(!landmarks.iter().any(|m| m.id == content_id));
}

#[rstest]
#[tokio::test]
async fn get_many_returns_subset(#[future] db: Store) {
    let db = db.await;

    let ids: Vec<_> = (0..5).map(|_| new_id()).collect();
    for id in &ids {
        mark::insert(&db, *id, &paragraph("bulk"), now())
            .await
            .unwrap();
    }

    let query_ids: Vec<_> = ids[1..3].iter().map(|id| id.inner()).collect();
    let results = mark::get_many(&db, &query_ids).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[rstest]
#[tokio::test]
async fn get_many_skips_deleted(#[future] db: Store) {
    let db = db.await;
    let a = new_id();
    let b = new_id();
    mark::insert(&db, a, &paragraph("alive"), now())
        .await
        .unwrap();
    mark::insert(&db, b, &paragraph("dead"), now())
        .await
        .unwrap();
    mark::soft_delete(&db, b).await.unwrap();

    let results = mark::get_many(&db, &[a.inner(), b.inner()]).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, a);
}

#[rstest]
#[tokio::test]
async fn insert_preserves_content_variant(#[future] db: Store) {
    let db = db.await;
    let id = new_id();
    let data = heading("Title");
    mark::insert(&db, id, &data, now()).await.unwrap();

    let fetched = mark::get(&db, id).await.unwrap().unwrap();
    assert_eq!(fetched.data, data);
}
