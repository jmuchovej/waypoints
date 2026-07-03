use super::*;

use domain::mark::{Content, MarkData, MarkID, RichText};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn create_mark_after_inserts_in_order(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    let result = svc
        .create_mark_after(children[0], lm, paragraph("inserted"), lm)
        .await
        .unwrap();

    assert_eq!(result.blocks.len(), 3);
    assert_eq!(result.blocks[0].id, children[0]);
    assert_eq!(result.blocks[1].id, result.new_mark_id);
    assert_eq!(result.blocks[2].id, children[1]);
}

#[rstest]
#[tokio::test]
async fn update_mark_text_changes_content(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let new_rt = RichText::from("updated text".to_string());
    svc.update_mark_text(children[0], new_rt, lm).await.unwrap();

    let fetched = svc.get_mark(children[0]).await.unwrap();
    if let MarkData::Content(Content::Paragraph { rich_text, .. }) = fetched.data {
        assert_eq!(rich_text.plain_text(), "updated text");
    } else {
        panic!("expected paragraph");
    }
}

#[rstest]
#[tokio::test]
async fn update_mark_converts_block_type(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let original = svc.get_mark(children[0]).await.unwrap();
    let original_rt = match &original.data {
        MarkData::Content(c) => c.rich_text().unwrap().clone(),
        _ => panic!("expected content"),
    };

    let converted = Content::heading1(original_rt.clone());
    svc.update_mark(children[0], converted, lm).await.unwrap();

    let fetched = svc.get_mark(children[0]).await.unwrap();
    match &fetched.data {
        MarkData::Content(Content::Heading1 { rich_text, .. }) => {
            assert_eq!(
                *rich_text, original_rt,
                "text should survive type conversion"
            );
        }
        other => panic!("expected Heading1, got {:?}", other),
    }
}

#[rstest]
#[tokio::test]
async fn delete_mark_removes_from_projection(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 3).await;

    svc.delete_mark(children[1], lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, children[0]);
    assert_eq!(blocks[1].id, children[2]);
}

#[rstest]
#[tokio::test]
async fn delete_marks_batch(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 5).await;

    svc.delete_marks(&[children[1], children[3]], lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![children[0], children[2], children[4]]);
}

#[rstest]
#[tokio::test]
async fn split_mark_at_offset(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let result = svc.split_mark(children[0], 5, lm).await.unwrap();
    assert_eq!(result.blocks.len(), 2);

    let left = svc.get_mark(children[0]).await.unwrap();
    let right = svc.get_mark(result.new_mark_id).await.unwrap();

    let left_text = match &left.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!("expected content"),
    };
    let right_text = match &right.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!("expected content"),
    };

    assert_eq!(left_text, "Block");
    assert_eq!(right_text, " 1");
}

#[rstest]
#[tokio::test]
async fn split_preserves_sibling_order(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 3).await;

    let result = svc.split_mark(children[1], 3, lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[0].id, children[0]);
    assert_eq!(blocks[1].id, children[1]);
    assert_eq!(blocks[2].id, result.new_mark_id);
    assert_eq!(blocks[3].id, children[2]);
}

#[rstest]
#[tokio::test]
async fn merge_marks_combines_text(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    svc.merge_marks(children[0], children[1], lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 1);

    let merged = svc.get_mark(children[0]).await.unwrap();
    let text = match &merged.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!("expected content"),
    };
    assert_eq!(text, "Block 1Block 2");
}

#[rstest]
#[tokio::test]
async fn split_then_merge_roundtrips(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let original_text = {
        let m = svc.get_mark(children[0]).await.unwrap();
        match &m.data {
            MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
            _ => panic!("expected content"),
        }
    };

    let split = svc.split_mark(children[0], 5, lm).await.unwrap();
    svc.merge_marks(children[0], split.new_mark_id, lm)
        .await
        .unwrap();

    let after = svc.get_mark(children[0]).await.unwrap();
    let after_text = match &after.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!("expected content"),
    };

    assert_eq!(original_text, after_text);
}

#[rstest]
#[tokio::test]
async fn move_marks_reorders(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 4).await;

    svc.move_marks(lm, &[children[0]], Some(children[2]), lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(
        ids,
        vec![children[1], children[2], children[0], children[3]]
    );
}

#[rstest]
#[tokio::test]
async fn reparent_mark_into_container(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    let result = svc
        .create_mark_after(children[0], lm, blockquote(), lm)
        .await
        .unwrap();
    let bq = result.new_mark_id;

    svc.reparent_mark(children[1], bq, None, lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[2].id, children[1]);
    assert_eq!(blocks[2].parent_id, Some(bq));
    assert_eq!(blocks[2].depth, 1);
}

#[rstest]
#[tokio::test]
async fn dedent_places_block_after_parent(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, roots, nested) = seed_nested(&db, 3, 1, 1).await;

    svc.reparent_mark(nested[0], lm, Some(roots[1]), lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![roots[0], roots[1], nested[0], roots[2]]);
    assert_eq!(blocks[2].depth, 0, "dedented block should be at root depth");
}

#[rstest]
#[tokio::test]
async fn dedent_last_child_places_after_parent(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, roots, nested) = seed_nested(&db, 2, 1, 2).await;

    svc.reparent_mark(nested[1], lm, Some(roots[1]), lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![roots[0], roots[1], nested[0], nested[1]]);
    assert_eq!(blocks[2].depth, 1, "n1 stays nested");
    assert_eq!(blocks[3].depth, 0, "n2 is dedented to root");
}

#[rstest]
#[tokio::test]
async fn reparent_with_none_after_appends(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, roots, nested) = seed_nested(&db, 3, 0, 1).await;

    svc.reparent_mark(roots[2], roots[1], None, lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![roots[0], nested[0], roots[1], roots[2]]);
    assert_eq!(blocks[3].parent_id, Some(roots[1]));
}

#[rstest]
#[tokio::test]
async fn update_mark_title_on_landmark(#[future] env: Env) {
    let (svc, db) = env.await;
    let (lm, _) = seed_landmark(&db, 0).await;

    svc.update_mark_title(lm, RichText::from("New Title".to_string()))
        .await
        .unwrap();

    let fetched = svc.get_mark(lm).await.unwrap();
    if let MarkData::Landmark(l) = fetched.data {
        assert_eq!(l.title.plain_text(), "New Title");
    } else {
        panic!("expected landmark");
    }
}

#[rstest]
#[tokio::test]
async fn update_mark_title_on_content_errors(#[future] env: Env) {
    let (svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let result = svc
        .update_mark_title(children[0], RichText::from("nope".to_string()))
        .await;
    assert!(result.is_err());
}

/// Every mutation through the editor service is also logged to sync
/// (#131 acceptance): one local-origin `sync_log` group per command.
#[rstest]
#[tokio::test]
async fn mutations_log_local_sync_groups(#[future] env: Env) {
    use sea_orm::EntityTrait;
    use store::entities::SyncLog;

    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    svc.create_mark_after(children[0], lm, paragraph("synced"), lm)
        .await
        .unwrap();
    svc.delete_mark(children[1], lm).await.unwrap();

    let rows = SyncLog::find().all(&db).await.unwrap();
    assert_eq!(rows.len(), 2, "one group per mutation");
    assert!(rows.iter().all(|r| r.origin == 0), "local origin");

    // Undo is a synced write too.
    svc.undo(lm).await.unwrap();
    let rows = SyncLog::find().all(&db).await.unwrap();
    assert_eq!(rows.len(), 3);
}
