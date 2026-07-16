//! One sync session (§10), driven over a framed byte stream. Transport-
//! generic: `send`/`recv` are any split async halves — iroh's
//! `SendStream`/`RecvStream` in production, a split tokio duplex pipe in
//! tests. The peer's storage is reached through the [`SyncPeer`] seam so
//! this crate depends on no storage engine (design doc §12); the role
//! crates implement [`SyncPeer`] over a hold.
//!
//! The exchange is the §10 two-round-trip flow, initiator ⇄ responder:
//!
//! ```text
//! initiate ──TallyRequest(digest)──▶ serve
//! initiate ◀──TallyResponse{offer}── serve      (offer = ids per divergent track)
//! initiate ──ResupplyRequest{want,push}──▶ serve (bidirectional transfer)
//! initiate ◀──ResupplyResponse{notices}── serve
//! ```
//!
//! Both sides converge in one pairing; a receiver applies pulled and pushed
//! Notices through the same [`SyncPeer::ingest`], which runs `validate`
//! and orphan-queues as needed — live pushes over an open session are
//! identical (§10.2).

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use outpost_protocol::sync::{SyncMessage, TallyDigest, wanted};
use outpost_protocol::{NoticeID, SignedNotice, TrackID};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::framing::{FrameError, read_frame, write_frame};

/// A peer's replicated state, as a session needs to see it. The role crates
/// implement this over a [`TerritoryHold`](outpost_hold::TerritoryHold); the
/// implementation decides accept/pending/drop inside `ingest` (a Station
/// and a Vessel ingest identically, §3 rule 5).
#[async_trait]
pub trait SyncPeer {
    /// This peer's `(Track → heads)` summary (§10.1).
    async fn digest(&self) -> TallyDigest;
    /// Every Notice this peer holds for `track` (its whole DAG).
    async fn track_notices(&self, track: TrackID) -> Vec<SignedNotice>;
    /// Every Notice id this peer holds — the `have` set for `wanted`.
    async fn held_ids(&self) -> BTreeSet<NoticeID>;
    /// Apply received Notices (validate → append, orphan-queue as needed).
    async fn ingest(&self, notices: Vec<SignedNotice>);
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("framing: {0}")]
    Frame(#[from] FrameError),
    #[error("unexpected message in {expected} phase")]
    Unexpected { expected: &'static str },
}

/// The initiating half of a session (a Vessel arriving at a Station, or a
/// gossiping peer). Reconciles both directions and returns.
pub async fn initiate<P, S, R>(peer: &P, send: &mut S, recv: &mut R) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    // 1. Announce our state.
    write_frame(send, &SyncMessage::TallyRequest(peer.digest().await)).await?;

    // 2. Receive the responder's per-track offer for divergent tracks.
    let offer = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::TallyResponse { offer } => offer,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "tally-response",
            });
        }
    };

    // 3. Compute what to pull and what to push, then send both.
    let have = peer.held_ids().await;
    let offered: BTreeSet<NoticeID> = offer.values().flatten().copied().collect();
    let mut want = Vec::new();
    let mut push = Vec::new();
    for (track, ids) in &offer {
        want.extend(wanted(&have, ids));
        for notice in peer.track_notices(*track).await {
            if !offered.contains(&NoticeID::of(&notice).unwrap_or(NoticeID::SELF)) {
                push.push(notice);
            }
        }
    }
    write_frame(send, &SyncMessage::ResupplyRequest { want, push }).await?;

    // 4. Apply the pulled Notices.
    let notices = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::ResupplyResponse { notices } => notices,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "resupply-response",
            });
        }
    };
    peer.ingest(notices).await;
    Ok(())
}

/// The responding half of a session (a Station serving, or a peer being
/// gossiped to).
pub async fn serve<P, S, R>(peer: &P, send: &mut S, recv: &mut R) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    // 1. Receive the initiator's digest, find divergent tracks.
    let init_digest = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::TallyRequest(digest) => digest,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "tally-request",
            });
        }
    };
    let divergent = peer.digest().await.divergent(&init_digest);

    // 2. Offer our ids for each divergent track (empty list ⇒ we hold none
    //    of it, which tells the initiator to push us its whole track).
    let mut offer: BTreeMap<TrackID, Vec<NoticeID>> = BTreeMap::new();
    for track in &divergent {
        let ids = peer
            .track_notices(*track)
            .await
            .iter()
            .filter_map(|n| NoticeID::of(n).ok())
            .collect();
        offer.insert(*track, ids);
    }
    write_frame(send, &SyncMessage::TallyResponse { offer }).await?;

    // 3. Apply the pushed Notices; answer the pull request.
    let (want, push) = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::ResupplyRequest { want, push } => (want, push),
        _ => {
            return Err(SessionError::Unexpected {
                expected: "resupply-request",
            });
        }
    };
    peer.ingest(push).await;

    let want_set: BTreeSet<NoticeID> = want.into_iter().collect();
    let mut notices = Vec::new();
    for track in &divergent {
        for notice in peer.track_notices(*track).await {
            if NoticeID::of(&notice).is_ok_and(|id| want_set.contains(&id)) {
                notices.push(notice);
            }
        }
    }
    write_frame(send, &SyncMessage::ResupplyResponse { notices }).await?;
    Ok(())
}
