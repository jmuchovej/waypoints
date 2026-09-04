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
use outpost::protocol::{CryptoRNG, EpochID};
use outpost::vessel::Vessel;
use store::{Store, query::sync as sync_store};

/// Publish every queued op batch as a Notice, oldest first. Returns how
/// many Notices were authored.
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
        // Parents stay empty until Track selection (section 7 of the
        // sync-mark-graph change) chooses a Landmark Track's heads.
        vessel
            .author(rng, vec![], epoch, PAGE_V1, &row.payload)
            .await?;
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
        store::query::sync::enqueue_publish(&db, payload)
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
        crate::editor::mutation::enqueue_for_publish(&db, &inverses)
            .await
            .unwrap();

        let after = store::query::sync::publish_queue(&db).await.unwrap().len();
        assert_eq!(after, before + 1, "an undo must enqueue its inverse ops");
    }
}
