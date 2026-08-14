use super::*;
use rstest::rstest;
use store::query::{mark, terrain};

#[rstest]
#[tokio::test]
async fn empty_landmark_projects_to_no_blocks(#[future] env: Env) {
    let (svc, db) = env.await;
    let lm = new_id();
    mark::insert(&db, lm, &landmark("page", "Empty"), now())
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert!(blocks.is_empty());
}

#[rstest]
#[tokio::test]
async fn projects_children_in_order(#[future] env: Env) {
    let (svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 3).await;

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 3);
    for (block, expected_id) in blocks.iter().zip(children.iter()) {
        assert_eq!(block.id, *expected_id);
        assert_eq!(block.depth, 0);
        assert_eq!(block.parent_id, Some(lm));
    }
}

#[rstest]
#[tokio::test]
async fn nested_container_tracks_depth(#[future] env: Env) {
    let (svc, db) = env.await;
    let (lm, children) = seed_landmark(&db, 1).await;

    let bq = new_id();
    mark::insert(&db, bq, &blockquote(), now()).await.unwrap();
    terrain::insert_child_edge(&db, lm, bq, "z000")
        .await
        .unwrap();

    let inner = new_id();
    mark::insert(&db, inner, &paragraph("nested"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, bq, inner, "i000")
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].id, children[0]);
    assert_eq!(blocks[0].depth, 0);
    assert_eq!(blocks[1].id, bq);
    assert_eq!(blocks[1].depth, 0);
    assert_eq!(blocks[2].id, inner);
    assert_eq!(blocks[2].depth, 1);
    assert_eq!(blocks[2].parent_id, Some(bq));
}

#[rstest]
#[tokio::test]
async fn nested_landmark_is_leaf(#[future] env: Env) {
    let (svc, db) = env.await;
    let lm = new_id();
    mark::insert(&db, lm, &landmark("page", "Parent"), now())
        .await
        .unwrap();

    let nested_lm = new_id();
    mark::insert(&db, nested_lm, &landmark("note", "Nested Note"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, lm, nested_lm, "i000")
        .await
        .unwrap();

    let inner = new_id();
    mark::insert(&db, inner, &paragraph("hidden"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, nested_lm, inner, "i000")
        .await
        .unwrap();

    let blocks = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id, nested_lm);
    assert!(blocks[0].is_leaf_landmark);
}

#[rstest]
#[tokio::test]
async fn max_depth_limits_recursion(#[future] env: Env) {
    let (svc, db) = env.await;
    let lm = new_id();
    mark::insert(&db, lm, &landmark("page", "Deep"), now())
        .await
        .unwrap();

    let bq1 = new_id();
    mark::insert(&db, bq1, &blockquote(), now()).await.unwrap();
    terrain::insert_child_edge(&db, lm, bq1, "i000")
        .await
        .unwrap();

    let bq2 = new_id();
    mark::insert(&db, bq2, &blockquote(), now()).await.unwrap();
    terrain::insert_child_edge(&db, bq1, bq2, "i000")
        .await
        .unwrap();

    let deep = new_id();
    mark::insert(&db, deep, &paragraph("deep"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, bq2, deep, "i000")
        .await
        .unwrap();

    let all = svc.project_landmark(lm, None).await.unwrap();
    assert_eq!(all.len(), 3);

    let shallow = svc.project_landmark(lm, Some(1)).await.unwrap();
    assert_eq!(shallow.len(), 1);
    assert_eq!(shallow[0].id, bq1);
}
