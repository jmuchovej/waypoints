mod fixtures;

use domain::fractional_index;
use domain::mark::MarkID;
use fixtures::*;
use rstest::rstest;
use sea_orm::DatabaseConnection;
use store::query::{mark, terrain};

#[rstest]
#[tokio::test]
async fn insert_and_get_child_edges(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "Parent"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("child block"), now())
        .await
        .unwrap();

    terrain::insert_child_edge(&db, parent, child, "i000")
        .await
        .unwrap();

    let edges = terrain::get_child_edges(&db, parent).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].src_id, parent.into());
    assert_eq!(edges[0].dst_id, child.into());
}

#[rstest]
#[tokio::test]
async fn find_parent_edge(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "P"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("C"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, parent, child, "i000")
        .await
        .unwrap();

    let edge = terrain::find_parent_edge(&db, child).await.unwrap();
    assert!(edge.is_some());
    assert_eq!(edge.unwrap().src_id, parent.into());
}

#[rstest]
#[tokio::test]
async fn find_parent_edge_orphan_returns_none(#[future] db: DatabaseConnection) {
    let db = db.await;
    let orphan = new_id();
    mark::insert(&db, orphan, &paragraph("alone"), now())
        .await
        .unwrap();

    let edge = terrain::find_parent_edge(&db, orphan).await.unwrap();
    assert!(edge.is_none());
}

#[rstest]
#[tokio::test]
async fn child_edges_ordered_by_fractional_index(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    mark::insert(&db, parent, &landmark("page", "P"), now())
        .await
        .unwrap();

    let keys = fractional_index::generate_n(3);
    let mut children = Vec::new();
    for key in &keys {
        let child = new_id();
        mark::insert(&db, child, &paragraph("block"), now())
            .await
            .unwrap();
        terrain::insert_child_edge(&db, parent, child, key)
            .await
            .unwrap();
        children.push(child);
    }

    let edges = terrain::get_child_edges(&db, parent).await.unwrap();
    assert_eq!(edges.len(), 3);
    for (edge, expected_child) in edges.iter().zip(children.iter()) {
        assert_eq!(edge.dst_id, (*expected_child).into());
    }
}

#[rstest]
#[tokio::test]
async fn delete_edges_by_dst(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "P"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("C"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, parent, child, "i000")
        .await
        .unwrap();

    terrain::delete_edges_by_dst(&db, child).await.unwrap();

    let edges = terrain::get_child_edges(&db, parent).await.unwrap();
    assert!(edges.is_empty());
}

#[rstest]
#[tokio::test]
async fn reorder_within_same_parent(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "P"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("C"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, parent, child, "a000")
        .await
        .unwrap();

    terrain::update_child_edge(&db, child, None, "z000")
        .await
        .unwrap();

    let edges = terrain::get_child_edges(&db, parent).await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].order.as_deref(), Some("z000"));
}

#[rstest]
#[tokio::test]
async fn reparent_to_new_parent(#[future] db: DatabaseConnection) {
    let db = db.await;
    let old_parent = new_id();
    let new_parent = new_id();
    let child = new_id();
    mark::insert(&db, old_parent, &landmark("page", "Old"), now())
        .await
        .unwrap();
    mark::insert(&db, new_parent, &landmark("page", "New"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("moving"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, old_parent, child, "i000")
        .await
        .unwrap();

    terrain::update_child_edge(&db, child, Some(new_parent), "i000")
        .await
        .unwrap();

    let old_edges = terrain::get_child_edges(&db, old_parent).await.unwrap();
    assert!(old_edges.is_empty());

    let new_edges = terrain::get_child_edges(&db, new_parent).await.unwrap();
    assert_eq!(new_edges.len(), 1);
    assert_eq!(new_edges[0].dst_id, child.into());
}

#[rstest]
#[tokio::test]
async fn reparent_preserves_extra_metadata(#[future] db: DatabaseConnection) {
    let db = db.await;
    let old_parent = new_id();
    let new_parent = new_id();
    let child = new_id();
    mark::insert(&db, old_parent, &landmark("page", "Old"), now())
        .await
        .unwrap();
    mark::insert(&db, new_parent, &landmark("page", "New"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("C"), now())
        .await
        .unwrap();
    terrain::insert_child_edge(&db, old_parent, child, "i000")
        .await
        .unwrap();

    terrain::update_child_edge(&db, child, Some(new_parent), "m000")
        .await
        .unwrap();

    let edge = terrain::find_parent_edge(&db, child)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(edge.src_id, new_parent.into());
    assert_eq!(edge.order.as_deref(), Some("m000"));
}

// ---------------------------------------------------------------------------
// Multi-mark reorder: move a non-contiguous selection within the same parent
// ---------------------------------------------------------------------------

/// Given M1..M9, move {M2, M3, M5, M6} below M8.
/// Expected result: M1, M4, M7, M8, M2, M3, M5, M6, M9.
#[rstest]
#[tokio::test]
async fn multi_mark_reorder_noncontiguous(#[future] db: DatabaseConnection) {
    let db = db.await;
    let (parent, m, keys) = parent_with_children(&db, 9).await;

    // We're moving M2, M3, M5, M6 (indices 1,2,4,5) to after M8 (index 7).
    // Generate 4 keys between M8's key and M9's key.
    let moving = [m[1], m[2], m[4], m[5]];
    let after_key = &keys[7]; // M8
    let before_key = &keys[8]; // M9

    // Generate 4 evenly-spaced keys in the gap between M8 and M9
    let mut new_keys = Vec::new();
    let mut prev = after_key.as_str();
    for _ in 0..moving.len() {
        let key = fractional_index::key_between(Some(prev), Some(before_key));
        new_keys.push(key);
        prev = new_keys.last().unwrap();
    }

    for (child, key) in moving.iter().zip(new_keys.iter()) {
        terrain::update_child_edge(&db, *child, None, key)
            .await
            .unwrap();
    }

    let edges = terrain::get_child_edges(&db, parent).await.unwrap();
    let order: Vec<MarkID> = edges.iter().map(|e| e.dst_id).collect();

    // Expected: M1, M4, M7, M8, M2, M3, M5, M6, M9
    let expected = [m[0], m[3], m[6], m[7], m[1], m[2], m[4], m[5], m[8]];
    assert_eq!(order.len(), 9);
    for (got, want) in order.iter().zip(expected.iter()) {
        assert_eq!(*got, (*want).into(), "order mismatch");
    }
}

// ---------------------------------------------------------------------------
// Reparent into container: move marks from a top-level parent into a blockquote
// ---------------------------------------------------------------------------

/// Given a page with M1..M5, move {M2, M3} into a blockquote that sits
/// between M1 and M4. The blockquote is a child of the page; M2 and M3
/// become children of the blockquote.
#[rstest]
#[tokio::test]
async fn reparent_marks_into_container(#[future] db: DatabaseConnection) {
    let db = db.await;
    let (parent, m, keys) = parent_with_children(&db, 5).await;

    // Insert a blockquote as a child of the page, between M1 and M4's positions
    // (M2 and M3 will be removed from this level, so the blockquote goes where M2 was)
    let bq = new_id();
    mark::insert(&db, bq, &blockquote(), now()).await.unwrap();
    let bq_key = fractional_index::key_between(Some(&keys[0]), Some(&keys[3]));
    terrain::insert_child_edge(&db, parent, bq, &bq_key)
        .await
        .unwrap();

    // Reparent M2 and M3 into the blockquote
    let inner_keys = fractional_index::generate_n(2);
    terrain::update_child_edge(&db, m[1], Some(bq), &inner_keys[0])
        .await
        .unwrap();
    terrain::update_child_edge(&db, m[2], Some(bq), &inner_keys[1])
        .await
        .unwrap();

    // Page children should be: M1, BQ, M4, M5
    let page_edges = terrain::get_child_edges(&db, parent).await.unwrap();
    let page_order: Vec<MarkID> = page_edges.iter().map(|e| e.dst_id).collect();
    assert_eq!(page_order.len(), 4);
    assert_eq!(page_order[0], m[0].into());
    assert_eq!(page_order[1], bq.into());
    assert_eq!(page_order[2], m[3].into());
    assert_eq!(page_order[3], m[4].into());

    // Blockquote children should be: M2, M3 in order
    let bq_edges = terrain::get_child_edges(&db, bq).await.unwrap();
    let bq_order: Vec<MarkID> = bq_edges.iter().map(|e| e.dst_id).collect();
    assert_eq!(bq_order.len(), 2);
    assert_eq!(bq_order[0], m[1].into());
    assert_eq!(bq_order[1], m[2].into());
}

/// Same as above but with a non-contiguous selection: move {M2, M4} into
/// a blockquote, preserving their relative order inside it.
#[rstest]
#[tokio::test]
async fn reparent_noncontiguous_into_container(#[future] db: DatabaseConnection) {
    let db = db.await;
    let (parent, m, keys) = parent_with_children(&db, 5).await;

    let bq = new_id();
    mark::insert(&db, bq, &blockquote(), now()).await.unwrap();
    // Place blockquote after M5 (at the end)
    let bq_key = fractional_index::key_between(Some(&keys[4]), None);
    terrain::insert_child_edge(&db, parent, bq, &bq_key)
        .await
        .unwrap();

    // Reparent M2 (index 1) and M4 (index 3) into blockquote
    let inner_keys = fractional_index::generate_n(2);
    terrain::update_child_edge(&db, m[1], Some(bq), &inner_keys[0])
        .await
        .unwrap();
    terrain::update_child_edge(&db, m[3], Some(bq), &inner_keys[1])
        .await
        .unwrap();

    // Page: M1, M3, M5, BQ
    let page_edges = terrain::get_child_edges(&db, parent).await.unwrap();
    let page_order: Vec<MarkID> = page_edges.iter().map(|e| e.dst_id).collect();
    assert_eq!(page_order.len(), 4);
    assert_eq!(page_order[0], m[0].into());
    assert_eq!(page_order[1], m[2].into());
    assert_eq!(page_order[2], m[4].into());
    assert_eq!(page_order[3], bq.into());

    // BQ: M2, M4
    let bq_edges = terrain::get_child_edges(&db, bq).await.unwrap();
    let bq_order: Vec<MarkID> = bq_edges.iter().map(|e| e.dst_id).collect();
    assert_eq!(bq_order.len(), 2);
    assert_eq!(bq_order[0], m[1].into());
    assert_eq!(bq_order[1], m[3].into());
}

// ---------------------------------------------------------------------------
// TODO stubs — future tests that need implementation work first
// ---------------------------------------------------------------------------

#[ignore = "requires block type conversion (not yet implemented in store layer)"]
#[tokio::test]
async fn convert_paragraph_to_heading() {
    todo!("convert a paragraph mark to heading1 — update data, preserve edges and order")
}

#[ignore = "requires block type conversion (not yet implemented in store layer)"]
#[tokio::test]
async fn convert_paragraphs_to_list_items() {
    todo!("select multiple paragraphs, convert all to list items in a single operation")
}

#[ignore = "requires unwrap-container operation"]
#[tokio::test]
async fn unwrap_container_promotes_children() {
    todo!("delete a blockquote, reparent its children back to the grandparent in-place")
}
