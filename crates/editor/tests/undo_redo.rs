mod fixtures;

use domain::mark::{MarkData, RichText};
use fixtures::*;
use rstest::rstest;
use store::DatabaseConnection;

type Env = (::editor::EditorService, DatabaseConnection);

// ---------------------------------------------------------------------------
// Basic undo
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn undo_nothing_returns_none(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, _) = seed_landmark(&db, 1).await;

    let result = svc.undo(lm).await.unwrap();
    assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn undo_create_mark(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    svc.create_mark_after(children[0], lm, paragraph("new"), lm)
        .await
        .unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 2);

    svc.undo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id, children[0]);
}

#[rstest]
#[tokio::test]
async fn undo_delete_mark(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    svc.delete_mark(children[0], lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 1);

    svc.undo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
}

#[rstest]
#[tokio::test]
async fn undo_split_mark(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let original_text = {
        let m = svc.get_mark(children[0]).await.unwrap();
        match &m.data {
            MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
            _ => panic!(),
        }
    };

    svc.split_mark(children[0], 3, lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 2);

    svc.undo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 1);

    let restored = svc.get_mark(children[0]).await.unwrap();
    let restored_text = match &restored.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!(),
    };
    assert_eq!(restored_text, original_text);
}

#[rstest]
#[tokio::test]
async fn undo_merge_marks(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    svc.merge_marks(children[0], children[1], lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 1);

    svc.undo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
}

#[rstest]
#[tokio::test]
async fn undo_update_text(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let original_text = {
        let m = svc.get_mark(children[0]).await.unwrap();
        match &m.data {
            MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
            _ => panic!(),
        }
    };

    svc.update_mark_text(children[0], RichText::from("changed".to_string()), lm)
        .await
        .unwrap();

    // Wait past coalesce window so the next undo pops this group
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    svc.undo(lm).await.unwrap();

    let restored = svc.get_mark(children[0]).await.unwrap();
    let text = match &restored.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!(),
    };
    assert_eq!(text, original_text);
}

// ---------------------------------------------------------------------------
// Redo
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn redo_nothing_returns_none(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, _) = seed_landmark(&db, 1).await;

    let result = svc.redo(lm).await.unwrap();
    assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn undo_then_redo_restores(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    svc.create_mark_after(children[0], lm, paragraph("redoable"), lm)
        .await
        .unwrap();

    svc.undo(lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 1);

    svc.redo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
}

#[rstest]
#[tokio::test]
async fn new_edit_after_undo_clears_redo(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    svc.create_mark_after(children[0], lm, paragraph("first"), lm)
        .await
        .unwrap();

    svc.undo(lm).await.unwrap();

    svc.create_mark_after(children[0], lm, paragraph("second"), lm)
        .await
        .unwrap();

    let redo_result = svc.redo(lm).await.unwrap();
    assert!(redo_result.is_none(), "redo stack should be empty");
}

// ---------------------------------------------------------------------------
// Multi-step undo
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn multiple_undos(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let r1 = svc
        .create_mark_after(children[0], lm, paragraph("A"), lm)
        .await
        .unwrap();
    svc.create_mark_after(r1.new_mark_id, lm, paragraph("B"), lm)
        .await
        .unwrap();

    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 3);

    svc.undo(lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 2);

    svc.undo(lm).await.unwrap();
    assert_eq!(svc.project_landmark(lm, None).await.unwrap().len(), 1);
}
