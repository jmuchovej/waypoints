//! Publication: queued op batches become authored `waypoints.page.v1`
//! Notices.
//!
//! Each outbox row publishes as one Notice (batching per Notice) and is
//! cleared only on confirmed authorship — [`Vessel::author`] returns after
//! the Notice is signed and ingested into the local hold, so a crash at any
//! earlier point leaves the row queued for the next pass. Re-publication is
//! harmless: recipients resolve each target by arbitration key, and a
//! re-published batch under a fresh Notice folds to the same state.

use crate::sync::fold::{FoldError, PAGE_V1};
use domain::MarkID;
use outpost::protocol::{CryptoRNG, EpochID, NoticeID, TrackID};
use outpost::vessel::Vessel;
use store::{Store, query::sync as sync_store};

/// The deterministic Track choice for an operation concerning several
/// Landmarks: the smallest candidate `MarkID` wins. The candidates come
/// from the operation's content (the Landmarks it touches), so every device
/// evaluating the same operation picks the same Landmark — and therefore
/// the same Track. Partitioning quality is all that rides on the choice;
/// the fold is Territory-wide and never consults Tracks.
pub fn choose_track_landmark(candidates: impl IntoIterator<Item = MarkID>) -> Option<MarkID> {
    candidates.into_iter().min()
}

/// Publish every queued op batch as a Notice, oldest first. Returns how
/// many Notices were authored.
///
/// Parents come from the write-side `MarkID → TrackID` index: a known
/// Landmark Track contributes its heads, keeping that Landmark's history in
/// one Track. An unknown (or stale — heads no longer present) Track roots a
/// fresh one and re-indexes it: degraded partitioning, correct state.
pub async fn publish_queued(
    db: &Store,
    vessel: &Vessel,
    rng: &mut impl CryptoRNG,
) -> Result<usize, FoldError> {
    // The newest epoch this Vessel can read is the one it authors under.
    let epoch = vessel
        .readable_epochs()
        .into_iter()
        .max()
        .unwrap_or(EpochID::ZERO);

    let queue = sync_store::publish_queue(db).await?;
    let mut published = 0;
    for row in queue {
        let parents = match sync_store::track_of(db, &row.landmark).await? {
            Some(track) => vessel
                .replica()
                .hold()
                .track_heads(track)
                .await?
                .into_iter()
                .map(|stored| stored.id)
                .collect(),
            None => Vec::new(),
        };
        let rooting = parents.is_empty();

        let notice = vessel
            .author(rng, parents, epoch, PAGE_V1, &row.payload)
            .await?;

        if rooting && !row.landmark.is_empty() {
            // This Notice rooted a new Track; future batches for the same
            // Landmark extend it.
            let id = NoticeID::of(&notice).map_err(outpost::hold::HoldError::from)?;
            sync_store::set_track(db, &row.landmark, TrackID::from(id)).await?;
        }

        sync_store::clear_published(db, row.seq).await?;
        published += 1;
    }
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::fold::{encode_page_v1, fold_new_notices};
    use crate::{CommandHistory, Maneuver};
    use domain::{ChartID, MarkData, MarkID, RichText};
    use outpost::protocol::KeyRoot;
    use outpost::vessel::Vessel;
    use rand::{SeedableRng, rngs::StdRng};
    use store::query::mark;

    fn rich(text: &str) -> RichText {
        RichText::from(text.to_string())
    }

    fn landmark(title: &str) -> MarkData {
        MarkData::Landmark(
            domain::Landmark::builder()
                .with_chart(ChartID::builtin("page"))
                .with_title(title.to_string())
                .build(),
        )
    }

    async fn test_vessel() -> Vessel {
        let root = KeyRoot::from_seed([5u8; 64]);
        let mut rng = StdRng::seed_from_u64(0x9AB);
        Vessel::create_territory(&root.identity(), &root.device(0), ":memory:", &mut rng)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn mutations_enqueue_and_a_crash_before_publication_keeps_them_queued() {
        // A real on-disk store, so "crash" (drop) and "restart" (reopen) are
        // honest.
        let dir = tempfile::tempdir().unwrap().keep();
        let path = dir.join("waypoints.db");
        let db = store::open(&path).await.unwrap();

        // Apply a mutation through the editor's mutation layer: local apply
        // and durable enqueue are one unit.
        let mut histories = std::collections::HashMap::new();
        let created =
            crate::editor::mutation::create_landmark(&db, &mut histories, ChartID::builtin("page"))
                .await
                .unwrap();
        assert_eq!(
            store::query::sync::publish_queue(&db).await.unwrap().len(),
            1
        );

        // Crash before publication; restart.
        drop(db);
        let db = store::open(&path).await.unwrap();
        let queue = store::query::sync::publish_queue(&db).await.unwrap();
        assert_eq!(queue.len(), 1, "the edit must still be queued on restart");

        // The queued op then publishes and clears.
        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0xC4A);
        assert_eq!(publish_queued(&db, &vessel, &mut rng).await.unwrap(), 1);
        assert!(
            store::query::sync::publish_queue(&db)
                .await
                .unwrap()
                .is_empty()
        );

        // And folds back into a recipient's graph.
        let recipient = store::test_utils::temp_db().await;
        fold_new_notices(&recipient, &vessel).await.unwrap();
        assert!(mark::get(&recipient, created.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn republication_leaves_recipients_in_the_same_state() {
        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0x1D3);
        let db = store::test_utils::temp_db().await;

        let page = MarkID::new();
        let ops = vec![
            Maneuver::create_mark(page, landmark("Once")),
            Maneuver::update_landmark_title(page, rich(""), rich("Only state")),
        ];
        let payload = encode_page_v1(&ops).unwrap();

        // First publication succeeded but the process died before the queue
        // row cleared — the classic re-publication window.
        vessel
            .author(&mut rng, vec![], EpochID::ZERO, PAGE_V1, &payload)
            .await
            .unwrap();
        store::query::sync::enqueue_publish(&db, payload, page.to_string())
            .await
            .unwrap();
        assert_eq!(publish_queued(&db, &vessel, &mut rng).await.unwrap(), 1);

        // A recipient folding both Notices lands in the intended state…
        let recipient = store::test_utils::temp_db().await;
        fold_new_notices(&recipient, &vessel).await.unwrap();
        let title = match mark::get(&recipient, page).await.unwrap().unwrap().data {
            MarkData::Landmark(lm) => lm.title.plain_text().to_owned(),
            other => panic!("expected landmark, got {other:?}"),
        };
        assert_eq!(title, "Only state");

        // …and re-folding (re-delivery) changes nothing.
        fold_new_notices(&recipient, &vessel).await.unwrap();
        let again = match mark::get(&recipient, page).await.unwrap().unwrap().data {
            MarkData::Landmark(lm) => lm.title.plain_text().to_owned(),
            other => panic!("expected landmark, got {other:?}"),
        };
        assert_eq!(again, "Only state");
    }

    #[tokio::test]
    async fn undo_publishes_its_inverse_ops() {
        let db = store::test_utils::temp_db().await;
        let mut history = CommandHistory::new();
        let mut histories = std::collections::HashMap::new();

        let created =
            crate::editor::mutation::create_landmark(&db, &mut histories, ChartID::builtin("page"))
                .await
                .unwrap();
        crate::editor::mutation::update_mark_title(&db, &mut history, created.id, rich("Renamed"))
            .await
            .unwrap();
        let before = store::query::sync::publish_queue(&db).await.unwrap().len();

        // Undo through the history (as EditorService does), then enqueue the
        // applied inverses the same way the service does.
        let (_, inverses) = history.undo(&db).await.unwrap().unwrap();
        crate::editor::mutation::enqueue_for_publish(&db, &inverses, created.id)
            .await
            .unwrap();

        let after = store::query::sync::publish_queue(&db).await.unwrap().len();
        assert_eq!(after, before + 1, "an undo must enqueue its inverse ops");
    }

    /// `SyncService::sync_state` surfaces the store-backed counters — the
    /// crate-level test behind the thin `sync_state` Tauri command.
    #[tokio::test]
    async fn sync_state_reflects_pending_and_deferred_counts() {
        let db = store::test_utils::temp_db().await;
        let shared: crate::Database =
            std::sync::Arc::new(tokio::sync::Mutex::new(Some(db.clone())));
        let dir = tempfile::tempdir().unwrap().keep();
        let svc = crate::SyncService::new(dir).with_app_db(shared);

        let initial = svc.sync_state().await.unwrap();
        assert_eq!(initial.pending_publications, 0);
        assert_eq!(initial.deferred_ops, 0);
        assert!(!initial.live_connected);
        assert_eq!(initial.last_sync_ms, None);

        // One queued publication and one deferred op become visible.
        let page = MarkID::new();
        let ops = vec![Maneuver::create_mark(page, landmark("Stateful"))];
        store::query::sync::enqueue_publish(&db, encode_page_v1(&ops).unwrap(), page.to_string())
            .await
            .unwrap();
        store::query::sync::defer_op(
            &db,
            &MarkID::new().to_string(),
            store::query::sync::StoredKey {
                hlc: outpost::protocol::HLC(1),
                author: [1; 32],
                notice: outpost::protocol::NoticeID::from_bytes([1; 32]),
            },
            0,
            b"{}".to_vec(),
        )
        .await
        .unwrap();

        let state = svc.sync_state().await.unwrap();
        assert_eq!(state.pending_publications, 1);
        assert_eq!(state.deferred_ops, 1);
    }

    /// Enqueue a single-op batch that retitles `page`, tagged with `page`
    /// as the concerned Landmark.
    async fn enqueue_title(db: &store::Store, page: MarkID, title: &str) {
        let ops = vec![Maneuver::update_landmark_title(page, rich(""), rich(title))];
        store::query::sync::enqueue_publish(db, encode_page_v1(&ops).unwrap(), page.to_string())
            .await
            .unwrap();
    }

    /// All `waypoints.page.v1` Notices in the vessel's hold, in topo order.
    async fn page_notices(vessel: &Vessel) -> Vec<outpost::hold::StoredNotice> {
        vessel
            .replica()
            .hold()
            .notices_after(-1)
            .await
            .unwrap()
            .into_iter()
            .filter(|row| {
                row.notice
                    .parse_and_verify()
                    .map(|e| e.header.payload_type == PAGE_V1)
                    .unwrap_or(false)
            })
            .collect()
    }

    #[tokio::test]
    async fn successive_batches_for_one_landmark_share_a_track() {
        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0x77AC);
        let db = store::test_utils::temp_db().await;

        let page = MarkID::new();
        let ops = vec![Maneuver::create_mark(page, landmark("Tracked"))];
        store::query::sync::enqueue_publish(&db, encode_page_v1(&ops).unwrap(), page.to_string())
            .await
            .unwrap();
        enqueue_title(&db, page, "Renamed once").await;
        enqueue_title(&db, page, "Renamed twice").await;

        assert_eq!(publish_queued(&db, &vessel, &mut rng).await.unwrap(), 3);

        let notices = page_notices(&vessel).await;
        assert_eq!(notices.len(), 3);
        assert!(
            notices.iter().all(|n| n.track == notices[0].track),
            "one Landmark's history must occupy one Track"
        );
        // Each Notice extends the previous head.
        let second = notices[1].notice.parse_and_verify().unwrap();
        assert_eq!(second.header.parents, vec![notices[0].id]);
        let third = notices[2].notice.parse_and_verify().unwrap();
        assert_eq!(third.header.parents, vec![notices[1].id]);
    }

    #[test]
    fn cross_landmark_track_choice_agrees_on_every_device() {
        let a = MarkID::new();
        let b = MarkID::new();
        // Two devices deriving candidates from the same op content, in
        // whatever order their local evaluation produced them.
        let device_one = choose_track_landmark([a, b]);
        let device_two = choose_track_landmark([b, a]);
        assert_eq!(device_one, device_two);
        assert_eq!(device_one, Some(a.min(b)));
        assert_eq!(choose_track_landmark([]), None);
    }

    #[tokio::test]
    async fn fold_never_reads_the_track_index() {
        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0xF01D);
        let db = store::test_utils::temp_db().await;

        let page = MarkID::new();
        let ops = vec![Maneuver::create_mark(page, landmark("Indexed"))];
        store::query::sync::enqueue_publish(&db, encode_page_v1(&ops).unwrap(), page.to_string())
            .await
            .unwrap();
        enqueue_title(&db, page, "Final title").await;
        publish_queued(&db, &vessel, &mut rng).await.unwrap();

        let title_in = |db: store::Store| async move {
            match mark::get(&db, page).await.unwrap().unwrap().data {
                MarkData::Landmark(lm) => lm.title.plain_text().to_owned(),
                other => panic!("expected landmark, got {other:?}"),
            }
        };

        // Index absent.
        let absent = store::test_utils::temp_db().await;
        fold_new_notices(&absent, &vessel).await.unwrap();

        // Index present but empty-of-this-landmark (an unrelated entry).
        let empty = store::test_utils::temp_db().await;
        store::query::sync::set_track(
            &empty,
            &MarkID::new().to_string(),
            outpost::protocol::TrackID::from(NoticeID::from_bytes([1; 32])),
        )
        .await
        .unwrap();
        fold_new_notices(&empty, &vessel).await.unwrap();

        // Index deliberately corrupted for this very landmark.
        let corrupt = store::test_utils::temp_db().await;
        store::query::sync::set_track(
            &corrupt,
            &page.to_string(),
            outpost::protocol::TrackID::from(NoticeID::from_bytes([0xBA; 32])),
        )
        .await
        .unwrap();
        fold_new_notices(&corrupt, &vessel).await.unwrap();

        assert_eq!(title_in(absent).await, "Final title");
        assert_eq!(title_in(empty).await, "Final title");
        assert_eq!(title_in(corrupt).await, "Final title");
    }

    #[tokio::test]
    async fn stale_index_fragments_partitioning_but_state_stays_correct() {
        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0x57A1);
        let db = store::test_utils::temp_db().await;

        let page = MarkID::new();
        let ops = vec![Maneuver::create_mark(page, landmark("Fragmented"))];
        store::query::sync::enqueue_publish(&db, encode_page_v1(&ops).unwrap(), page.to_string())
            .await
            .unwrap();
        publish_queued(&db, &vessel, &mut rng).await.unwrap();

        // The index goes stale: it names a Track the hold has no heads for.
        store::query::sync::set_track(
            &db,
            &page.to_string(),
            outpost::protocol::TrackID::from(NoticeID::from_bytes([0xDE; 32])),
        )
        .await
        .unwrap();
        enqueue_title(&db, page, "Still correct").await;
        publish_queued(&db, &vessel, &mut rng).await.unwrap();

        // Degraded partitioning: the history fragments across two Tracks…
        let notices = page_notices(&vessel).await;
        assert_eq!(notices.len(), 2);
        assert_ne!(
            notices[0].track, notices[1].track,
            "a stale index must fragment, not corrupt"
        );

        // …while the folded state stays correct everywhere.
        let recipient = store::test_utils::temp_db().await;
        fold_new_notices(&recipient, &vessel).await.unwrap();
        match mark::get(&recipient, page).await.unwrap().unwrap().data {
            MarkData::Landmark(lm) => assert_eq!(lm.title.plain_text(), "Still correct"),
            other => panic!("expected landmark, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn divergence_resupplies_only_the_diverged_landmarks_track() {
        use outpost::protocol::TallyDigest;
        use outpost::replica::Replica;

        let vessel = test_vessel().await;
        let mut rng = StdRng::seed_from_u64(0xD1E);
        let db = store::test_utils::temp_db().await;

        // Two Landmarks, each publishing into its own Track.
        let l1 = MarkID::new();
        let l2 = MarkID::new();
        for (id, title) in [(l1, "One"), (l2, "Two")] {
            let ops = vec![Maneuver::create_mark(id, landmark(title))];
            store::query::sync::enqueue_publish(&db, encode_page_v1(&ops).unwrap(), id.to_string())
                .await
                .unwrap();
        }
        publish_queued(&db, &vessel, &mut rng).await.unwrap();

        // A second device holding everything so far.
        let acl = vessel.replica().hold().acl_records().await.unwrap();
        let replica_b = Replica::bootstrap(":memory:", &acl).await.unwrap();
        let all: Vec<_> = vessel
            .replica()
            .hold()
            .notices_after(-1)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.notice)
            .collect();
        replica_b.ingest(all).await.unwrap();

        // Only L1 diverges.
        enqueue_title(&db, l1, "One, edited").await;
        publish_queued(&db, &vessel, &mut rng).await.unwrap();

        let heads_a = vessel.replica().hold().all_heads().await.unwrap();
        let heads_b = replica_b.hold().all_heads().await.unwrap();
        let divergent = TallyDigest::of_heads(&heads_a).divergent(&TallyDigest::of_heads(&heads_b));

        let l1_track = store::query::sync::track_of(&db, &l1.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            divergent.into_iter().collect::<Vec<_>>(),
            vec![l1_track],
            "only the diverged Landmark's Track may resupply"
        );
    }
}
