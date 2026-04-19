//! Pull-based gossip (PROTOCOL.md §1 "Gossip: relays replicating from
//! relays"): fetch a peer's log since our cursor and append it verbatim.
//! The pulling relay's obligations live here — realm guard, idempotent
//! append (group_id), fresh local seqs, cursor advanced only after the
//! append succeeds. No drift quarantine: that happens at apply time, on
//! devices.

use tracing::warn;

use super::client::{PeerClient, PeerError};
use super::store::{GossipStore, StoreError, rfc3339_now};
use super::wire::{DEFAULT_LIMIT, EnvelopeHead};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PumpOutcome {
    pub fetched: usize,
    /// Acknowledged by the local store — includes idempotent replays.
    pub stored: usize,
    /// Malformed or realm-mismatched: logged and skipped; the cursor
    /// still advances past them (the divergence is the peer's fault and
    /// retrying won't fix it).
    pub dropped: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PumpError {
    #[error(transparent)]
    Peer(#[from] PeerError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Drain one (peer, realm) edge to quiescence. Safe to call at any time,
/// from any state, repeatedly — same contract as the app's `sync_once`.
pub async fn pump_once<S>(
    store: &S,
    peer: &PeerClient,
    realm_id: &str,
) -> Result<PumpOutcome, PumpError>
where
    S: GossipStore + ?Sized,
{
    let mut outcome = PumpOutcome::default();
    loop {
        let cursor = store.peer_cursor(peer.peer_id(), realm_id).await?;
        let page = peer
            .fetch_since(realm_id, cursor, None, DEFAULT_LIMIT)
            .await?;
        let made_progress = page.next_since > cursor;

        let mut heads = Vec::with_capacity(page.envelopes.len());
        let mut payloads = Vec::with_capacity(page.envelopes.len());
        for sequenced in &page.envelopes {
            outcome.fetched += 1;
            let raw = sequenced.envelope.get();
            match EnvelopeHead::parse(raw) {
                Ok(head) if head.realm_id == realm_id => {
                    heads.push(head);
                    payloads.push(raw);
                }
                Ok(head) => {
                    warn!(
                        group_id = %head.group_id,
                        got_realm = %head.realm_id,
                        want_realm = %realm_id,
                        peer = %peer.peer_id(),
                        "dropping realm-mismatched envelope"
                    );
                    outcome.dropped += 1;
                }
                Err(e) => {
                    warn!(error = %e, peer = %peer.peer_id(), "dropping malformed envelope");
                    outcome.dropped += 1;
                }
            }
        }

        if !heads.is_empty() {
            let acks = store.append(&heads, &payloads, &rfc3339_now()).await?;
            outcome.stored += acks.len();
        }
        if made_progress {
            store
                .set_peer_cursor(peer.peer_id(), realm_id, page.next_since)
                .await?;
        }
        if !page.has_more || !made_progress {
            return Ok(outcome);
        }
    }
}

/// One gossip round: every configured peer × every relevant realm
/// (locally-known realms plus configured extras). A failing edge is
/// logged and skipped — one dead peer must not stall the others.
pub async fn pump_all<S>(store: &S, peers: &[PeerClient], extra_realms: &[String]) -> PumpOutcome
where
    S: GossipStore + ?Sized,
{
    let mut realms = match store.known_realms().await {
        Ok(realms) => realms,
        Err(e) => {
            warn!(error = %e, "gossip: could not list local realms");
            Vec::new()
        }
    };
    for realm in extra_realms {
        if !realms.contains(realm) {
            realms.push(realm.clone());
        }
    }

    let mut total = PumpOutcome::default();
    for peer in peers {
        for realm in &realms {
            match pump_once(store, peer, realm).await {
                Ok(outcome) => {
                    total.fetched += outcome.fetched;
                    total.stored += outcome.stored;
                    total.dropped += outcome.dropped;
                }
                Err(e) => {
                    warn!(peer = %peer.peer_id(), realm = %realm, error = %e, "gossip edge failed");
                }
            }
        }
    }
    total
}
