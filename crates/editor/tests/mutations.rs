mod fixtures;

use domain::mark::{Content, MarkData, MarkID, RichText};
use fixtures::*;
use rstest::rstest;
use store::DatabaseConnection;

type Env = (::editor::EditorService, DatabaseConnection);

// ---------------------------------------------------------------------------
// create_mark_after
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// update_mark_text
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn update_mark_text_changes_content(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let new_rt = RichText::from("updated text".to_string());
    svc.update_mark_text(children[0], new_rt, lm).await.unwrap();

    let fetched = svc.get_mark(children[0]).await.unwrap();
    if let MarkData::Content(Content::Paragraph { rich_text }) = fetched.data {
        assert_eq!(rich_text.plain_text(), "updated text");
    } else {
        panic!("expected paragraph");
    }
}

// ---------------------------------------------------------------------------
// update_mark (block type conversion)
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn update_mark_converts_block_type(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    // Grab the original text before conversion
    let original = svc.get_mark(children[0]).await.unwrap();
    let original_rt = match &original.data {
        MarkData::Content(c) => c.rich_text().unwrap().clone(),
        _ => panic!("expected content"),
    };

    // Convert paragraph → heading, preserving the same rich text
    let converted = Content::Heading1 {
        rich_text: original_rt.clone(),
    };
    svc.update_mark(children[0], converted, lm).await.unwrap();

    let fetched = svc.get_mark(children[0]).await.unwrap();
    match &fetched.data {
        MarkData::Content(Content::Heading1 { rich_text }) => {
            assert_eq!(
                *rich_text, original_rt,
                "text should survive type conversion"
            );
        }
        other => panic!("expected Heading1, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// delete_mark
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// delete_marks (batch)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// split_mark
// ---------------------------------------------------------------------------

#[rstest]
#[tokio::test]
async fn split_mark_at_offset(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    // "Block 1" is 7 bytes; split at offset 5 → "Block" | " 1"
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

// ---------------------------------------------------------------------------
// merge_marks
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// split then merge roundtrip
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// move_marks
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// reparent_mark
// ---------------------------------------------------------------------------

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

    svc.reparent_mark(children[1], bq, lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[2].id, children[1]);
    assert_eq!(blocks[2].parent_id, Some(bq));
    assert_eq!(blocks[2].depth, 1);
}

// ---------------------------------------------------------------------------
// update_mark_title
// ---------------------------------------------------------------------------

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
