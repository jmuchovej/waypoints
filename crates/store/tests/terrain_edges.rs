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

    terrain::delete_edges_by_dst(&db, child, None)
        .await
        .unwrap();

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

// ---------------------------------------------------------------------------
// Multigraph: same (src, dst) with multiple edges of different kinds
// ---------------------------------------------------------------------------

use domain::chart::{ChartID, FeatureID};
use domain::terrain::TerrainKind;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use std::str::FromStr;
use store::entities::{
    ActiveTerrain, TerrainColumn, Terrains, terrain_kind_to_string as kind_to_string,
};

/// Insert a terrain edge of arbitrary kind. Tests only — production code
/// should use the query-layer helpers as they get added per-kind.
async fn insert_edge(
    db: &sea_orm::DatabaseConnection,
    src: MarkID,
    dst: MarkID,
    kind: TerrainKind,
    order: Option<&str>,
) {
    ActiveTerrain {
        id: Set(uuid::Uuid::new_v4()),
        src_id: Set(src.inner()),
        dst_id: Set(dst.inner()),
        kind: Set(kind_to_string(&kind)),
        order: Set(order.map(str::to_string)),
        extra: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// A Child edge AND a Reference edge between the same (src, dst) pair must
/// coexist. Under the old `(src_id, dst_id)` primary key this was
/// impossible — the whole point of adding `id` as the PK is to enable this.
#[rstest]
#[tokio::test]
async fn child_and_reference_between_same_pair_coexist(#[future] db: DatabaseConnection) {
    let db = db.await;
    let src = new_id();
    let dst = new_id();
    mark::insert(&db, src, &landmark("page", "Src"), now())
        .await
        .unwrap();
    mark::insert(&db, dst, &landmark("page", "Dst"), now())
        .await
        .unwrap();

    insert_edge(&db, src, dst, TerrainKind::Child, Some("i000")).await;
    insert_edge(&db, src, dst, TerrainKind::Reference, None).await;

    let all = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(src.inner()))
        .filter(TerrainColumn::DstId.eq(dst.inner()))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(all.len(), 2, "both edges should coexist");

    let kinds: Vec<&str> = all.iter().map(|m| m.kind.as_str()).collect();
    assert!(kinds.contains(&"child"));
    assert!(kinds.contains(&"reference"));
}

/// Two Reading edges on the same (src, dst) pair but for different features
/// must coexist — this is the "Meeting.attendees AND Meeting.organizer both
/// point at the same Person" case.
#[rstest]
#[tokio::test]
async fn multiple_reading_edges_same_pair_different_features(#[future] db: DatabaseConnection) {
    let db = db.await;
    let meeting = new_id();
    let person = new_id();
    mark::insert(&db, meeting, &landmark("meeting", "M"), now())
        .await
        .unwrap();
    mark::insert(&db, person, &landmark("person", "P"), now())
        .await
        .unwrap();

    let chart = ChartID::builtin("meeting");
    let attendees = FeatureID::from_str("attn").unwrap();
    let organizer = FeatureID::from_str("orgz").unwrap();

    insert_edge(
        &db,
        meeting,
        person,
        TerrainKind::Reading(chart.clone(), attendees.clone()),
        None,
    )
    .await;
    insert_edge(
        &db,
        meeting,
        person,
        TerrainKind::Reading(chart.clone(), organizer.clone()),
        None,
    )
    .await;

    let all = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(meeting.inner()))
        .filter(TerrainColumn::DstId.eq(person.inner()))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(all.len(), 2, "both reading edges should coexist");

    // Distinct edge IDs — the PK disambiguates them.
    assert_ne!(all[0].id, all[1].id);
}

/// Reading edges are queryable by kind-string prefix — this is what makes
/// the backlink query fast (no JSON scanning, just an indexed string filter).
#[rstest]
#[tokio::test]
async fn reading_edges_queryable_by_kind_prefix(#[future] db: DatabaseConnection) {
    let db = db.await;
    let person = new_id();
    mark::insert(&db, person, &landmark("person", "Target"), now())
        .await
        .unwrap();

    // Create 3 sources, each with a Reading edge pointing at `person`.
    let chart = ChartID::builtin("meeting");
    let feature = FeatureID::from_str("attn").unwrap();
    for i in 0..3 {
        let src = new_id();
        mark::insert(&db, src, &landmark("meeting", &format!("M{i}")), now())
            .await
            .unwrap();
        insert_edge(
            &db,
            src,
            person,
            TerrainKind::Reading(chart.clone(), feature.clone()),
            None,
        )
        .await;
    }

    // Also insert a Child edge and a Reference edge from an unrelated source —
    // those should NOT show up in the reading-prefix query.
    let noise_src = new_id();
    mark::insert(&db, noise_src, &landmark("page", "Noise"), now())
        .await
        .unwrap();
    insert_edge(&db, noise_src, person, TerrainKind::Child, Some("i000")).await;
    insert_edge(&db, noise_src, person, TerrainKind::Reference, None).await;

    // Backlink query: who references this person via ANY reading feature?
    let readings = Terrains::find()
        .filter(TerrainColumn::DstId.eq(person.inner()))
        .filter(TerrainColumn::Kind.starts_with("reading::"))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(readings.len(), 3, "exactly three reading edges, no noise");
    for r in &readings {
        assert!(
            r.kind.starts_with("reading::"),
            "unexpected kind: {}",
            r.kind
        );
    }
}

/// Round-trip: insert a Reading edge, fetch it back, verify the enum
/// variant is reconstructed correctly via the `From<Model>` impl.
#[rstest]
#[tokio::test]
async fn reading_edge_db_roundtrip_preserves_enum_variant(#[future] db: DatabaseConnection) {
    let db = db.await;
    let src = new_id();
    let dst = new_id();
    mark::insert(&db, src, &landmark("meeting", "M"), now())
        .await
        .unwrap();
    mark::insert(&db, dst, &landmark("person", "P"), now())
        .await
        .unwrap();

    let chart = ChartID::builtin("meeting");
    let feature = FeatureID::from_str("attn").unwrap();
    insert_edge(
        &db,
        src,
        dst,
        TerrainKind::Reading(chart.clone(), feature.clone()),
        None,
    )
    .await;

    let model = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(src.inner()))
        .filter(TerrainColumn::DstId.eq(dst.inner()))
        .one(&db)
        .await
        .unwrap()
        .expect("edge should exist");

    // DB string should preserve the "builtin." prefix that ChartID::Display
    // would strip. This is the whole reason kind_to_string bypasses Display.
    assert_eq!(model.kind, "reading::builtin.meeting::attn");

    let terrain: domain::terrain::Terrain = model.into();
    match terrain.kind {
        TerrainKind::Reading(c, f) => {
            assert_eq!(c, chart);
            assert_eq!(f, feature);
        }
        other => panic!("expected Reading variant, got {other:?}"),
    }
}

/// Two Child edges with the same src/dst but different edge IDs can be
/// inserted at the schema level — this is the "block embeds / synced
/// blocks" case that the `id`-based PK unlocks. The editor's query layer
/// still assumes one-parent-per-child (see `find_parent_edge` comment),
/// so this isn't yet wired up in production, but the schema doesn't
/// prevent it.
#[rstest]
#[tokio::test]
async fn schema_allows_duplicate_child_edges_for_future_block_embeds(
    #[future] db: DatabaseConnection,
) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "P"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("c"), now())
        .await
        .unwrap();

    insert_edge(&db, parent, child, TerrainKind::Child, Some("a000")).await;
    insert_edge(&db, parent, child, TerrainKind::Child, Some("m000")).await;

    let all = Terrains::find()
        .filter(TerrainColumn::SrcId.eq(parent.inner()))
        .filter(TerrainColumn::DstId.eq(child.inner()))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(all.len(), 2, "schema must allow duplicate same-kind edges");
    assert_ne!(all[0].id, all[1].id, "edge IDs must be distinct");
}

// ── Multigraph by-ID helpers (sync v1, #129) ────────────────────────────

use domain::terrain::TerrainID;

/// `insert_edge` must store the caller-provided `TerrainID` exactly — sync
/// requires the same edge identity on every device.
#[rstest]
#[tokio::test]
async fn insert_edge_round_trips_caller_provided_id(#[future] db: DatabaseConnection) {
    let db = db.await;
    let src = new_id();
    let dst = new_id();
    mark::insert(&db, src, &landmark("page", "Src"), now())
        .await
        .unwrap();
    mark::insert(&db, dst, &paragraph("dst"), now())
        .await
        .unwrap();

    let edge_id = TerrainID::new();
    terrain::insert_edge(&db, edge_id, src, dst, &TerrainKind::Child, Some("i000"), None)
        .await
        .unwrap();

    let edge = terrain::get_edge_by_id(&db, edge_id).await.unwrap().unwrap();
    assert_eq!(edge.id, edge_id);
    assert_eq!(edge.src_id, src);
    assert_eq!(edge.dst_id, dst);
    assert_eq!(edge.order.as_deref(), Some("i000"));
}

/// `update_edge_by_id` must touch only the addressed edge, even when
/// multiple same-kind edges share the same `(src, dst)` pair.
#[rstest]
#[tokio::test]
async fn update_edge_by_id_is_multigraph_safe(#[future] db: DatabaseConnection) {
    let db = db.await;
    let src = new_id();
    let other = new_id();
    let dst = new_id();
    for (id, data) in [
        (src, landmark("page", "Src")),
        (other, landmark("page", "Other")),
        (dst, paragraph("dst")),
    ] {
        mark::insert(&db, id, &data, now()).await.unwrap();
    }

    let edge_a = TerrainID::new();
    let edge_b = TerrainID::new();
    terrain::insert_edge(&db, edge_a, src, dst, &TerrainKind::Child, Some("a000"), None)
        .await
        .unwrap();
    terrain::insert_edge(&db, edge_b, src, dst, &TerrainKind::Child, Some("b000"), None)
        .await
        .unwrap();

    terrain::update_edge_by_id(&db, edge_a, other, Some("c000"))
        .await
        .unwrap();

    let a = terrain::get_edge_by_id(&db, edge_a).await.unwrap().unwrap();
    let b = terrain::get_edge_by_id(&db, edge_b).await.unwrap().unwrap();
    assert_eq!(a.src_id, other);
    assert_eq!(a.order.as_deref(), Some("c000"));
    assert_eq!(b.src_id, src, "sibling edge must be untouched");
    assert_eq!(b.order.as_deref(), Some("b000"));
}

#[rstest]
#[tokio::test]
async fn update_edge_by_id_errors_on_missing_edge(#[future] db: DatabaseConnection) {
    let db = db.await;
    let missing = TerrainID::new();
    let result = terrain::update_edge_by_id(&db, missing, new_id(), Some("i000")).await;
    assert!(matches!(
        result,
        Err(store::StoreError::EdgeNotFound(id)) if id == missing
    ));
}

/// `delete_edge_by_id` removes only the addressed edge and is idempotent.
#[rstest]
#[tokio::test]
async fn delete_edge_by_id_deletes_one_edge_idempotently(#[future] db: DatabaseConnection) {
    let db = db.await;
    let src = new_id();
    let dst = new_id();
    mark::insert(&db, src, &landmark("page", "Src"), now())
        .await
        .unwrap();
    mark::insert(&db, dst, &paragraph("dst"), now())
        .await
        .unwrap();

    let edge_a = TerrainID::new();
    let edge_b = TerrainID::new();
    terrain::insert_edge(&db, edge_a, src, dst, &TerrainKind::Child, Some("a000"), None)
        .await
        .unwrap();
    terrain::insert_edge(&db, edge_b, src, dst, &TerrainKind::Child, Some("b000"), None)
        .await
        .unwrap();

    terrain::delete_edge_by_id(&db, edge_a).await.unwrap();
    assert!(terrain::get_edge_by_id(&db, edge_a).await.unwrap().is_none());
    assert!(terrain::get_edge_by_id(&db, edge_b).await.unwrap().is_some());

    // Second delete of the same edge succeeds silently.
    terrain::delete_edge_by_id(&db, edge_a).await.unwrap();
}

/// `insert_child_edge` returns the `TerrainID` it generated for the row.
#[rstest]
#[tokio::test]
async fn insert_child_edge_returns_generated_id(#[future] db: DatabaseConnection) {
    let db = db.await;
    let parent = new_id();
    let child = new_id();
    mark::insert(&db, parent, &landmark("page", "Parent"), now())
        .await
        .unwrap();
    mark::insert(&db, child, &paragraph("child"), now())
        .await
        .unwrap();

    let edge_id = terrain::insert_child_edge(&db, parent, child, "i000")
        .await
        .unwrap();

    let edge = terrain::get_edge_by_id(&db, edge_id).await.unwrap().unwrap();
    assert_eq!(edge.src_id, parent);
    assert_eq!(edge.dst_id, child);
}
