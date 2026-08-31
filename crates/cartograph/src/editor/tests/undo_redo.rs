use super::*;
use domain::{Content, MarkData, RichText};
use rstest::rstest;

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

async fn mark_text(svc: &EditorService, id: domain::MarkID) -> String {
    let m = svc.get_mark(id).await.unwrap();
    match &m.data {
        MarkData::Content(c) => c.rich_text().unwrap().plain_text().to_owned(),
        _ => panic!("expected content mark"),
    }
}

#[rstest]
#[tokio::test]
async fn undo_restores_previous_text(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    svc.update_mark_text(children[0], RichText::from("changed".to_string()), lm)
        .await
        .unwrap();
    assert_eq!(mark_text(&svc, children[0]).await, "changed");

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(mark_text(&svc, children[0]).await, "Block 1");
}

#[rstest]
#[tokio::test]
async fn text_burst_undoes_as_one_step(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    // Three updates well inside the 500ms coalesce window.
    for text in ["o", "on", "one"] {
        svc.update_mark_text(children[0], RichText::from(text.to_string()), lm)
            .await
            .unwrap();
    }

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(mark_text(&svc, children[0]).await, "Block 1");

    // The burst is also one redo step.
    svc.redo(lm).await.unwrap().unwrap();
    assert_eq!(mark_text(&svc, children[0]).await, "one");
}

#[rstest]
#[tokio::test]
async fn structural_edit_interrupts_coalescing(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 2).await;

    svc.update_mark_text(children[0], RichText::from("first".to_string()), lm)
        .await
        .unwrap();

    // Structural edit on the same mark: move it after its sibling. It must
    // start a new group and never absorb the following text edit.
    svc.move_marks(lm, &[children[0]], Some(children[1]), lm)
        .await
        .unwrap();

    svc.update_mark_text(children[0], RichText::from("second".to_string()), lm)
        .await
        .unwrap();

    // One undo reverses only the later text update...
    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(mark_text(&svc, children[0]).await, "first");

    // ...leaving the structural move in place.
    let blocks = svc.project_landmark(lm, None).await.unwrap();
    let ids: Vec<domain::MarkID> = blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, vec![children[1], children[0]]);
}

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

#[rstest]
#[tokio::test]
async fn undo_reparent_restores_parent(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 3).await;

    svc.reparent_mark(children[2], children[1], None, lm)
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks[2].parent_id, Some(children[1]));

    svc.undo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[2].id, children[2]);
    assert_eq!(blocks[2].depth, 0);
}

#[rstest]
#[tokio::test]
async fn undo_redo_reparent_roundtrip(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 3).await;

    svc.reparent_mark(children[2], children[1], None, lm)
        .await
        .unwrap();

    svc.undo(lm).await.unwrap();
    svc.redo(lm).await.unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks[2].parent_id, Some(children[1]));
    assert_eq!(blocks[2].depth, 1);
}

#[rstest]
#[tokio::test]
async fn undo_update_mark_restores_type(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let original = svc.get_mark(children[0]).await.unwrap();
    let original_rt = match &original.data {
        MarkData::Content(c) => c.rich_text().unwrap().clone(),
        _ => panic!("expected content"),
    };

    svc.update_mark(children[0], Content::heading1(original_rt.clone()), lm)
        .await
        .unwrap();

    let fetched = svc.get_mark(children[0]).await.unwrap();
    assert!(matches!(
        fetched.data,
        MarkData::Content(Content::Heading1 { .. })
    ));

    svc.undo(lm).await.unwrap();

    let restored = svc.get_mark(children[0]).await.unwrap();
    assert!(matches!(
        restored.data,
        MarkData::Content(Content::Paragraph { .. })
    ));
}

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

async fn landmark_title(svc: &EditorService, id: domain::MarkID) -> String {
    let m = svc.get_mark(id).await.unwrap();
    match m.data {
        MarkData::Landmark(l) => l.title.plain_text().to_owned(),
        _ => panic!("expected landmark"),
    }
}

#[rstest]
#[tokio::test]
async fn undo_restores_previous_title(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, _) = seed_landmark(&db, 0).await;

    svc.update_mark_title(lm, RichText::from("Renamed".to_string()))
        .await
        .unwrap();
    assert_eq!(landmark_title(&svc, lm).await, "Renamed");

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(landmark_title(&svc, lm).await, "Test Page");

    svc.redo(lm).await.unwrap().unwrap();
    assert_eq!(landmark_title(&svc, lm).await, "Renamed");
}

#[rstest]
#[tokio::test]
async fn successive_title_edits_undo_one_at_a_time(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, _) = seed_landmark(&db, 0).await;

    svc.update_mark_title(lm, RichText::from("B".to_string()))
        .await
        .unwrap();
    svc.update_mark_title(lm, RichText::from("C".to_string()))
        .await
        .unwrap();

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(landmark_title(&svc, lm).await, "B");

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(landmark_title(&svc, lm).await, "Test Page");
}

#[rstest]
#[tokio::test]
async fn rejected_title_edit_pushes_nothing(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let result = svc
        .update_mark_title(children[0], RichText::from("nope".to_string()))
        .await;
    assert!(result.is_err());

    // Nothing was recorded — for the content mark's bucket or the landmark's.
    assert!(svc.undo(children[0]).await.unwrap().is_none());
    assert!(svc.undo(lm).await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn create_landmark_records_and_undoes(#[future] env: Env) {
    let (mut svc, _db) = env.await;

    let created = svc.create_landmark(ChartID::builtin("page")).await.unwrap();
    let id = created.id;
    assert!(matches!(created.data, MarkData::Landmark(_)));

    // Persisted, and its create op sits in the new landmark's own bucket.
    svc.get_mark(id).await.unwrap();

    // Undo soft-deletes the landmark.
    svc.undo(id).await.unwrap().unwrap();
    assert!(svc.get_mark(id).await.is_err());

    // Redo restores it with its original identity.
    svc.redo(id).await.unwrap().unwrap();
    let restored = svc.get_mark(id).await.unwrap();
    assert_eq!(restored.id, id);
    assert!(matches!(restored.data, MarkData::Landmark(_)));
}
