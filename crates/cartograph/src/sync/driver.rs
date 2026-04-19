//! Pull-based sync: drain the outbox up, then page the peer down.
//!
//! Deliberately quick-and-dirty per the guide's Phase 6 allowance — a
//! poll loop, no push, no channels. Everything durable rides on
//! `sync_log`: uploads are at-least-once (the peer dedupes on
//! group_id), and the fetch cursor only persists after the envelopes it
//! covers are applied, so a crash mid-page re-fetches into idempotent
//! application rather than losing anything.
//!
//! The peer seam is outpost's (raw envelopes on the wire); this driver
//! owns the typed boundary — `Envelope`s are serialized on the way up
//! and parsed on the way down, here and nowhere below.

use domain::sync::HLC;
use outpost::sync::peers::peer::{Peer, PeerError};
use outpost::sync::peers::wire::EnvelopeFace;
use serde_json::value::RawValue;
use store::DatabaseConnection;
use store::query::{relay_cursor, sync_log};

use crate::editor::{EditorError, EditorResult};

use super::coordinator::{ApplyOutcome, SyncCoordinator};
use super::envelope::Envelope;
use super::resolver::ConflictResolver;

const UPLOAD_BATCH: u64 = 100;
const FETCH_LIMIT: usize = 200;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncOutcome {
    pub uploaded: usize,
    pub fetched: usize,
    pub applied: usize,
    pub dominated: usize,
    pub merged: usize,
    pub skipped: usize,
}

/// One full sync cycle against one peer. Call it on an interval, after
/// bursts of local edits, or on a push nudge — it is safe to call at any
/// time, from any state, repeatedly.
pub async fn sync_once<P, C>(
    db: &DatabaseConnection,
    coordinator: &mut SyncCoordinator<C>,
    peer: &P,
    peer_id: &str,
) -> EditorResult<SyncOutcome>
where
    P: Peer,
    C: ConflictResolver,
{
    let realm = coordinator.realm_id();
    let device = coordinator.device_id();
    let mut outcome = SyncOutcome::default();

    // ── Up: drain the outbox ────────────────────────────────────────
    loop {
        let batch = sync_log::pending_upload(db, realm, UPLOAD_BATCH).await?;
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        let envelopes = batch
            .into_iter()
            .map(envelope_from_row)
            .collect::<EditorResult<Vec<_>>>()?;
        let raw = envelopes
            .iter()
            .map(raw_envelope)
            .collect::<EditorResult<Vec<_>>>()?;
        let response = peer.upload(realm, raw).await.map_err(peer_error)?;
        if response.results.is_empty() {
            // Offline (NoopPeer) or a peer that acked nothing: leave
            // the outbox pending and stop rather than spin.
            break;
        }
        let ids: Vec<_> = response.results.iter().map(|ack| ack.group_id).collect();
        sync_log::mark_uploaded(db, &ids).await?;
        outcome.uploaded += ids.len();
        if batch_len < UPLOAD_BATCH as usize {
            break;
        }
    }

    // ── Down: page from the per-(peer, realm) cursor ────────────────
    loop {
        let cursor = relay_cursor::get(db, peer_id, realm).await?;
        let page = peer
            .fetch_since(realm, cursor, Some(device), FETCH_LIMIT)
            .await
            .map_err(peer_error)?;
        let made_progress = page.next_since > cursor;

        for sequenced in page.envelopes {
            outcome.fetched += 1;
            let envelope: Envelope = serde_json::from_str(sequenced.envelope.get())
                .map_err(|e| EditorError::Relay(format!("malformed envelope from peer: {e}")))?;
            match coordinator.apply_remote(db, envelope).await? {
                ApplyOutcome::Applied {
                    applied,
                    dominated,
                    merged,
                } => {
                    outcome.applied += applied;
                    outcome.dominated += dominated;
                    outcome.merged += merged;
                }
                ApplyOutcome::Skipped(_) => outcome.skipped += 1,
            }
        }

        if made_progress {
            relay_cursor::set(db, peer_id, realm, page.next_since).await?;
        }
        if !page.has_more || !made_progress {
            break;
        }
    }

    Ok(outcome)
}

/// Rebuild the wire envelope from its persisted `sync_log` row.
fn envelope_from_row(row: store::entities::SyncLogModel) -> EditorResult<Envelope> {
    Ok(Envelope {
        face: EnvelopeFace {
            group_id: row.group_id.into(),
            account_id: row.account_id.into(),
            realm_id: row.realm_id.into(),
            device_id: row.device_id.into(),
            hlc: HLC(row.hlc),
        },
        checkpoints: serde_json::from_value(row.ops)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
        touched_marks: serde_json::from_value(row.touched_marks)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
    })
}

/// Wire form of a typed envelope: serialize once, ship verbatim.
fn raw_envelope(envelope: &Envelope) -> EditorResult<Box<RawValue>> {
    serde_json::to_string(envelope)
        .and_then(RawValue::from_string)
        .map_err(|e| EditorError::Serialization(e.to_string()))
}

fn peer_error(e: PeerError) -> EditorError {
    EditorError::Relay(e.to_string())
}

#[cfg(test)]
mod tests {
    use domain::mark::{Content, MarkData, MarkID, RichText};
    use domain::sync::{AccountID, DeviceID, RealmID};
    use outpost::sync::peers::peer::{LocalPeer, NoopPeer};
    use store::query::{device, mark as mark_q, sync_log};
    use store::test_utils::temp_db;

    use super::*;
    use crate::editor::Maneuver;
    use crate::sync::touched_marks;

    const PEER: &str = "test-peer";

    fn paragraph(text: &str) -> MarkData {
        MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
    }

    /// A device already joined to `realm` (the realm-adoption that real
    /// pairing will do via the accounts DB).
    async fn joined_device(
        account: AccountID,
        realm: RealmID,
        name: &str,
    ) -> (DatabaseConnection, SyncCoordinator) {
        let db = temp_db().await;
        device::insert(&db, DeviceID::new(), account, realm, name)
            .await
            .unwrap();
        let coordinator = SyncCoordinator::load(&db).await.unwrap();
        assert_eq!(coordinator.realm_id(), realm);
        (db, coordinator)
    }

    async fn create_mark(
        db: &DatabaseConnection,
        coordinator: &mut SyncCoordinator,
        text: &str,
    ) -> MarkID {
        let id = MarkID::new();
        mark_q::insert(db, id, &paragraph(text), chrono::Local::now().naive_utc())
            .await
            .unwrap();
        let ops = vec![Maneuver::CreateMark {
            mark_id: id,
            data: paragraph(text),
        }];
        let marks = touched_marks(&ops);
        coordinator.log_local(db, ops, marks).await.unwrap();
        id
    }

    async fn update_mark(
        db: &DatabaseConnection,
        coordinator: &mut SyncCoordinator,
        id: MarkID,
        old: &str,
        new: &str,
    ) {
        mark_q::update_data(db, id, &paragraph(new)).await.unwrap();
        let ops = vec![Maneuver::UpdateMarkData {
            mark_id: id,
            old: paragraph(old),
            new: paragraph(new),
        }];
        let marks = touched_marks(&ops);
        coordinator.log_local(db, ops, marks).await.unwrap();
    }

    #[tokio::test]
    async fn two_devices_round_trip() {
        let (account, realm) = (AccountID::new(), RealmID::new());
        let peer = LocalPeer::in_memory();
        let (db_a, mut a) = joined_device(account, realm, "a").await;
        let (db_b, mut b) = joined_device(account, realm, "b").await;

        let id = create_mark(&db_a, &mut a, "hello from A").await;
        let up = sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        assert_eq!(up.uploaded, 1);

        let down = sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();
        assert_eq!((down.fetched, down.applied), (1, 1));
        let on_b = mark_q::get(&db_b, id).await.unwrap().unwrap();
        assert_eq!(on_b.data, paragraph("hello from A"));

        // And back: B edits, A converges.
        update_mark(&db_b, &mut b, id, "hello from A", "edited on B").await;
        sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();
        let back = sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        assert_eq!(back.applied, 1);
        let on_a = mark_q::get(&db_a, id).await.unwrap().unwrap();
        assert_eq!(on_a.data, paragraph("edited on B"));
    }

    #[tokio::test]
    async fn resync_is_idempotent() {
        let (account, realm) = (AccountID::new(), RealmID::new());
        let peer = LocalPeer::in_memory();
        let (db_a, mut a) = joined_device(account, realm, "a").await;
        let (db_b, mut b) = joined_device(account, realm, "b").await;

        create_mark(&db_a, &mut a, "once").await;
        sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();

        // Nothing new anywhere: everything is a no-op.
        let again_a = sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        let again_b = sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();
        assert_eq!(again_a, SyncOutcome::default());
        assert_eq!(again_b, SyncOutcome::default());

        // Rewinding B's cursor re-fetches but idempotency skips.
        relay_cursor::set(&db_b, PEER, realm, 0).await.unwrap();
        let rewound = sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();
        assert_eq!(rewound.fetched, 1);
        assert_eq!(rewound.applied, 0);
        assert_eq!(rewound.skipped, 1);
        assert_eq!(
            sync_log::pending_upload(&db_b, realm, 10)
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[tokio::test]
    async fn realms_are_isolated() {
        let peer = LocalPeer::in_memory();
        let (db_a, mut a) = joined_device(AccountID::new(), RealmID::new(), "a").await;
        let (db_c, mut c) = joined_device(AccountID::new(), RealmID::new(), "c").await;

        create_mark(&db_a, &mut a, "realm A only").await;
        sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();

        let outcome = sync_once(&db_c, &mut c, &peer, PEER).await.unwrap();
        assert_eq!(outcome.fetched, 0);
    }

    #[tokio::test]
    async fn cursor_advances_past_own_envelopes() {
        let (account, realm) = (AccountID::new(), RealmID::new());
        let peer = LocalPeer::in_memory();
        let (db_a, mut a) = joined_device(account, realm, "a").await;

        create_mark(&db_a, &mut a, "one").await;
        create_mark(&db_a, &mut a, "two").await;
        let outcome = sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        assert_eq!(outcome.uploaded, 2);
        assert_eq!(outcome.fetched, 0, "own envelopes are excluded");
        assert_eq!(
            relay_cursor::get(&db_a, PEER, realm).await.unwrap(),
            2,
            "cursor still advances past the excluded window"
        );
    }

    #[tokio::test]
    async fn concurrent_edits_converge() {
        let (account, realm) = (AccountID::new(), RealmID::new());
        let peer = LocalPeer::in_memory();
        let (db_a, mut a) = joined_device(account, realm, "a").await;
        let (db_b, mut b) = joined_device(account, realm, "b").await;

        // Shared baseline.
        let id = create_mark(&db_a, &mut a, "base").await;
        sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
        sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();

        // Both edit while "offline".
        update_mark(&db_a, &mut a, id, "base", "A's edit").await;
        update_mark(&db_b, &mut b, id, "base", "B's edit").await;

        // Everyone syncs until quiescent.
        for _ in 0..2 {
            sync_once(&db_a, &mut a, &peer, PEER).await.unwrap();
            sync_once(&db_b, &mut b, &peer, PEER).await.unwrap();
        }

        let on_a = mark_q::get(&db_a, id).await.unwrap().unwrap();
        let on_b = mark_q::get(&db_b, id).await.unwrap().unwrap();
        assert_eq!(on_a.data, on_b.data, "LWW must converge identically");
        assert!(
            on_a.data == paragraph("A's edit") || on_a.data == paragraph("B's edit"),
            "winner is one of the two edits"
        );
    }

    #[tokio::test]
    async fn noop_peer_leaves_outbox_pending() {
        let (account, realm) = (AccountID::new(), RealmID::new());
        let (db_a, mut a) = joined_device(account, realm, "a").await;
        create_mark(&db_a, &mut a, "offline edit").await;

        let outcome = sync_once(&db_a, &mut a, &NoopPeer, PEER).await.unwrap();
        assert_eq!(outcome, SyncOutcome::default());
        assert_eq!(
            sync_log::pending_upload(&db_a, realm, 10)
                .await
                .unwrap()
                .len(),
            1,
            "envelope stays queued for a real peer"
        );
    }
}
