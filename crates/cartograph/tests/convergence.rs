//! Convergence acceptance: two Vessels, a real Station over QUIC, and the
//! full pipeline — apply + enqueue, publish as `waypoints.page.v1` Notices,
//! reconcile, fold — ending in identical Mark/Terrain graphs.

use cartograph::{Maneuver, PAGE_V1, encode_page_v1, fold_new_notices, publish_queued};
use domain::{ChartID, Landmark, MarkData, MarkID, RichText, TerrainID, TerrainKind};
use ed25519_dalek::SigningKey;
use outpost::{
    protocol::{EpochID, KeyRoot},
    replica::Replica,
    station::Station,
    vessel::Vessel,
};
use rand::{SeedableRng, rngs::StdRng};
use std::sync::Arc;
use store::{
    Store,
    query::{mark, sync as sync_store, terrain},
};

fn rich(text: &str) -> RichText {
    RichText::from(text.to_string())
}

fn landmark(title: &str) -> MarkData {
    MarkData::Landmark(
        Landmark::builder()
            .with_chart(ChartID::builtin("page"))
            .with_title(title.to_string())
            .build(),
    )
}

fn paragraph(text: &str) -> MarkData {
    MarkData::Content(domain::Content::paragraph(rich(text)))
}

/// One device: its Vessel and its app store.
struct Device {
    vessel: Vessel,
    db: Store,
    rng: StdRng,
}

impl Device {
    /// Apply a batch locally and durably enqueue it — the editor's
    /// apply+enqueue unit, driven through the same public seams.
    async fn edit(&mut self, landmark_id: MarkID, ops: Vec<Maneuver>) {
        for op in &ops {
            op.apply(&self.db).await.unwrap();
        }
        sync_store::enqueue_publish(
            &self.db,
            encode_page_v1(&ops).unwrap(),
            landmark_id.to_string(),
        )
        .await
        .unwrap();
    }

    /// One full sync: publish the outbox, reconcile with the Station, fold
    /// what arrived — the `sync_now` pipeline.
    async fn sync(&mut self, station: &outpost::fabric::EndpointAddr) {
        publish_queued(&self.db, &self.vessel, &mut self.rng)
            .await
            .unwrap();
        self.vessel.sync_with(station.clone()).await.unwrap();
        fold_new_notices(&self.db, &self.vessel).await.unwrap();
    }
}

/// Two devices of one identity plus a serving Station.
async fn rig(seed: u8) -> (Device, Device, outpost::fabric::EndpointAddr) {
    let root = KeyRoot::from_seed([seed; 64]);
    let identity = root.identity();
    let mut rng_a = StdRng::seed_from_u64(seed as u64);
    let rng_b = StdRng::seed_from_u64(seed as u64 + 1000);

    let vessel_a = Vessel::create_territory(&identity, &root.device(0), ":memory:", &mut rng_a)
        .await
        .unwrap();
    let acl = vessel_a.replica().hold().acl_records().await.unwrap();

    let station = Arc::new(
        Station::new(
            &SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &acl).await.unwrap(),
        )
        .await
        .unwrap(),
    );
    let addr = station.endpoint().direct_addr();
    let serving = Arc::clone(&station);
    tokio::spawn(async move { serving.serve_loop().await });

    let vessel_b = Vessel::from_identity(
        &identity,
        &root.device(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();

    let a = Device {
        vessel: vessel_a,
        db: store::test_utils::temp_db().await,
        rng: rng_a,
    };
    let b = Device {
        vessel: vessel_b,
        db: store::test_utils::temp_db().await,
        rng: rng_b,
    };
    (a, b, addr)
}

/// Everything the scenario observes about a graph, per device.
async fn graph(db: &Store, marks: &[MarkID], edges: &[TerrainID]) -> String {
    let mut out = String::new();
    for &id in marks {
        let row = mark::get(db, id).await.unwrap().map(|m| match m.data {
            MarkData::Landmark(lm) => format!("landmark:{}", lm.title.plain_text()),
            MarkData::Content(c) => format!(
                "content:{}",
                c.rich_text().map(|r| r.plain_text()).unwrap_or_default()
            ),
        });
        out.push_str(&format!("{id}={row:?};"));
    }
    for &id in edges {
        let row = terrain::get_edge_by_id(db, id)
            .await
            .unwrap()
            .map(|e| format!("{}->{} {:?} {:?}", e.src_id, e.dst_id, e.kind, e.order));
        out.push_str(&format!("{id}={row:?};"));
    }
    out
}

#[tokio::test]
async fn two_vessels_converge_through_a_station() {
    let (mut a, mut b, addr) = rig(21).await;

    let page = MarkID::new();
    let block = MarkID::new();
    let edge = TerrainID::new();
    a.edit(
        page,
        vec![
            Maneuver::create_mark(page, landmark("Converged page")),
            Maneuver::create_mark(block, paragraph("hello from A")),
            Maneuver::CreateTerrain {
                edge_id: edge,
                src: page,
                dst: block,
                kind: TerrainKind::Child,
                order: Some("a0".into()),
                extra: None,
            },
        ],
    )
    .await;

    a.sync(&addr).await;
    b.sync(&addr).await;

    // B edits on top; another round trip converges both.
    b.edit(
        page,
        vec![Maneuver::update_landmark_title(
            page,
            rich(""),
            rich("Renamed on B"),
        )],
    )
    .await;
    b.sync(&addr).await;
    a.sync(&addr).await;

    let marks = [page, block];
    let edges = [edge];
    let ga = graph(&a.db, &marks, &edges).await;
    let gb = graph(&b.db, &marks, &edges).await;
    assert_eq!(ga, gb, "both devices must hold an identical graph");
    assert!(ga.contains("Renamed on B"));
    assert!(ga.contains("hello from A"));
}

#[tokio::test]
async fn offline_divergence_reconverges() {
    let (mut a, mut b, addr) = rig(22).await;

    // Shared base: one page.
    let page = MarkID::new();
    a.edit(page, vec![Maneuver::create_mark(page, landmark("Base"))])
        .await;
    a.sync(&addr).await;
    b.sync(&addr).await;

    // Both edit while unable to reach each other: A retitles, B adds a
    // block. Distinct targets — both must survive; the title races only
    // with itself.
    let block_b = MarkID::new();
    let edge_b = TerrainID::new();
    a.edit(
        page,
        vec![Maneuver::update_landmark_title(
            page,
            rich(""),
            rich("Offline A title"),
        )],
    )
    .await;
    b.edit(
        page,
        vec![
            Maneuver::create_mark(block_b, paragraph("offline B block")),
            Maneuver::CreateTerrain {
                edge_id: edge_b,
                src: page,
                dst: block_b,
                kind: TerrainKind::Child,
                order: Some("a0".into()),
                extra: None,
            },
        ],
    )
    .await;

    // Then both sync; an extra round lets each pull the other's edits.
    a.sync(&addr).await;
    b.sync(&addr).await;
    a.sync(&addr).await;

    let marks = [page, block_b];
    let edges = [edge_b];
    let ga = graph(&a.db, &marks, &edges).await;
    let gb = graph(&b.db, &marks, &edges).await;
    assert_eq!(ga, gb, "offline divergence must reconverge");
    assert!(ga.contains("Offline A title"));
    assert!(ga.contains("offline B block"));
}

#[tokio::test]
async fn permuted_delivery_converges_end_to_end() {
    let (mut a, _b, addr) = rig(23).await;

    // A authors three Notices through the real pipeline.
    let page = MarkID::new();
    a.edit(
        page,
        vec![Maneuver::create_mark(page, landmark("Permuted"))],
    )
    .await;
    a.edit(
        page,
        vec![Maneuver::update_landmark_title(
            page,
            rich(""),
            rich("First rename"),
        )],
    )
    .await;
    a.edit(
        page,
        vec![Maneuver::update_landmark_title(
            page,
            rich(""),
            rich("Final rename"),
        )],
    )
    .await;
    a.sync(&addr).await;

    // One recipient receives them via the Station (its delivery order); the
    // other ingests the same set reversed — end-to-end order independence,
    // not just the pure fold.
    let root = KeyRoot::from_seed([23; 64]);
    let identity = root.identity();
    let acl = a.vessel.replica().hold().acl_records().await.unwrap();

    let mut via_station = Vessel::from_identity(
        &identity,
        &root.device(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    via_station.sync_with(addr.clone()).await.unwrap();
    let db_station = store::test_utils::temp_db().await;
    fold_new_notices(&db_station, &via_station).await.unwrap();

    let reversed = Vessel::from_identity(
        &identity,
        &root.device(3),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let mut notices: Vec<_> = a
        .vessel
        .replica()
        .hold()
        .notices_after(-1)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.notice)
        .collect();
    notices.reverse();
    reversed.replica().ingest(notices).await.unwrap();
    let db_reversed = store::test_utils::temp_db().await;
    fold_new_notices(&db_reversed, &reversed).await.unwrap();

    let marks = [page];
    let gs = graph(&db_station, &marks, &[]).await;
    let gr = graph(&db_reversed, &marks, &[]).await;
    assert_eq!(gs, gr, "delivery order must not matter end to end");
    assert!(gs.contains("Final rename"));
}

#[tokio::test]
async fn cross_landmark_move_lands_on_both_devices() {
    let (mut a, mut b, addr) = rig(24).await;

    // Two pages; a block under page one.
    let page_one = MarkID::new();
    let page_two = MarkID::new();
    let block = MarkID::new();
    let edge = TerrainID::new();
    a.edit(
        page_one,
        vec![
            Maneuver::create_mark(page_one, landmark("Page one")),
            Maneuver::create_mark(block, paragraph("movable block")),
            Maneuver::CreateTerrain {
                edge_id: edge,
                src: page_one,
                dst: block,
                kind: TerrainKind::Child,
                order: Some("a0".into()),
                extra: None,
            },
        ],
    )
    .await;
    a.edit(
        page_two,
        vec![Maneuver::create_mark(page_two, landmark("Page two"))],
    )
    .await;
    a.sync(&addr).await;
    b.sync(&addr).await;

    // Move the block from page one to page two — one UpdateTerrain
    // concerning two Landmarks, authored on A.
    a.edit(
        page_one,
        vec![Maneuver::UpdateTerrain {
            edge_id: edge,
            old_src: page_one,
            old_order: Some("a0".into()),
            new_src: page_two,
            new_order: Some("a0".into()),
        }],
    )
    .await;
    a.sync(&addr).await;
    b.sync(&addr).await;

    for (name, db) in [("A", &a.db), ("B", &b.db)] {
        let moved = terrain::get_edge_by_id(db, edge).await.unwrap().unwrap();
        assert_eq!(
            moved.src_id, page_two,
            "device {name}: block moved to page two"
        );
        let one_children = terrain::get_child_edges(db, page_one).await.unwrap();
        assert!(
            one_children.is_empty(),
            "device {name}: page one no longer contains the block"
        );
        let two_children = terrain::get_child_edges(db, page_two).await.unwrap();
        assert_eq!(two_children.len(), 1, "device {name}: page two contains it");
        assert_eq!(two_children[0].dst_id, block);
    }
}
