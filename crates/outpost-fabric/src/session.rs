//! One sync session, driven over a framed byte stream. Transport-
//! generic: `send`/`recv` are any split async halves — iroh's
//! `SendStream`/`RecvStream` in production, a split tokio duplex pipe in
//! tests. The peer's storage is reached through the [`SyncPeer`] seam so
//! this crate depends on no storage engine; the role
//! crates implement [`SyncPeer`] over a hold.
//!
//! The exchange is the two-round-trip flow, initiator ⇄ responder:
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
//! identical.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;

use async_trait::async_trait;
use outpost_protocol::acl::OrderedRecord;
use outpost_protocol::sync::{SyncMessage, TallyDigest, wanted};
use outpost_protocol::{NoticeID, SignedNotice, TrackID};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::framing::{FrameError, read_frame, write_frame};

/// A peer's replicated state, as a session needs to see it. The role crates
/// implement this over a [`TerritoryHold`](outpost_hold::TerritoryHold); the
/// implementation decides accept/pending/drop inside `ingest` (a Station
/// and a Vessel ingest identically).
#[async_trait]
pub trait SyncPeer {
    /// This peer's `(Track → heads)` summary.
    async fn digest(&self) -> TallyDigest;
    /// Every Notice this peer holds for `track` (its whole DAG).
    async fn track_notices(&self, track: TrackID) -> Vec<SignedNotice>;
    /// Every Notice id this peer holds — the `have` set for `wanted`.
    async fn held_ids(&self) -> BTreeSet<NoticeID>;
    /// Apply received Notices (validate → append, orphan-queue as needed).
    async fn ingest(&self, notices: Vec<SignedNotice>);

    /// Length of this peer's ACL log — its position in the linear log.
    async fn acl_len(&self) -> u64;
    /// The ordered ACL records at or after log position `from` (the suffix a
    /// peer that is behind needs), each with its order proof.
    async fn acl_suffix(&self, from: u64) -> Vec<OrderedRecord>;
    /// Apply received ordered ACL records (verify proof → fold → append).
    async fn ingest_ordered_acl(&self, records: Vec<OrderedRecord>);
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("framing: {0}")]
    Frame(#[from] FrameError),
    #[error("unexpected message in {expected} phase")]
    Unexpected { expected: &'static str },
    /// The responder ended the session after the ACL phase — admission
    /// refused the Notice phase (not a member). The ACL records exchanged up
    /// to that point are already ingested and stay.
    #[error("the responder refused the notice phase")]
    Refused,
}

/// The initiating half of a session (a Vessel arriving at a Station, or a
/// gossiping peer). Reconciles both directions and returns.
pub async fn initiate<P, S, R>(peer: &P, send: &mut S, recv: &mut R) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    // ACL phase first: the Notice validator needs the ACL heads Notices
    // pin, so reconcile the linear ACL log before the DAG.
    let my_acl_len = peer.acl_len().await;
    write_frame(send, &SyncMessage::AclRequest { len: my_acl_len }).await?;
    let (acl_records, responder_acl_len) = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::AclResponse { records, len } => (records, len),
        _ => {
            return Err(SessionError::Unexpected {
                expected: "acl-response",
            });
        }
    };
    peer.ingest_ordered_acl(acl_records).await;
    // Push our own suffix if we are ahead of the responder.
    let push_acl = if my_acl_len > responder_acl_len {
        peer.acl_suffix(responder_acl_len).await
    } else {
        Vec::new()
    };
    write_frame(send, &SyncMessage::AclPush { records: push_acl }).await?;

    // 1. Announce our state.
    write_frame(send, &SyncMessage::TallyRequest(peer.digest().await)).await?;

    // 2. Receive the responder's per-track offer for divergent tracks — or
    //    its refusal: an admission-gated responder that reconciled the ACL
    //    but will not serve Notices answers the tally with `Ack { false }`.
    let offer = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::TallyResponse { offer } => offer,
        SyncMessage::Ack { ok: false } => return Err(SessionError::Refused),
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
/// gossiped to). Reads the leading `AclRequest`, then reconciles.
pub async fn serve<P, S, R>(peer: &P, send: &mut S, recv: &mut R) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    serve_gated(peer, send, recv, || async { true }).await
}

/// [`serve`] with an **admission gate between the phases**: the ACL phase
/// runs unconditionally, then `gate` decides whether the Notice phase is
/// served. Called *after* the ACL reconcile, so a gate that consults the
/// peer's folded state sees the just-exchanged records — an invited joiner
/// whose approval arrived in this very session passes. On refusal the
/// initiator's tally is answered with `Ack { false }`
/// ([`SessionError::Refused`] on its side) and the session ends; the ACL
/// records already exchanged stay.
pub async fn serve_gated<P, S, R, G, Fut>(
    peer: &P,
    send: &mut S,
    recv: &mut R,
    gate: G,
) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
    G: FnOnce() -> Fut + Send,
    Fut: Future<Output = bool> + Send,
{
    let init_acl_len = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::AclRequest { len } => len,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "acl-request",
            });
        }
    };
    serve_sync_gated(peer, init_acl_len, send, recv, gate).await
}

/// The ungated reconciliation, given the initiator's ACL length (its leading
/// `AclRequest` already read). A Station dispatches the first frame itself —
/// an [`OrderRequest`](SyncMessage::OrderRequest) goes to the orderer, an
/// `AclRequest` here — so it calls this directly rather than [`serve`].
pub async fn serve_sync<P, S, R>(
    peer: &P,
    init_acl_len: u64,
    send: &mut S,
    recv: &mut R,
) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
{
    serve_sync_gated(peer, init_acl_len, send, recv, || async { true }).await
}

/// [`serve_sync`] with the [`serve_gated`] admission gate.
pub async fn serve_sync_gated<P, S, R, G, Fut>(
    peer: &P,
    init_acl_len: u64,
    send: &mut S,
    recv: &mut R,
    gate: G,
) -> Result<(), SessionError>
where
    P: SyncPeer + Sync,
    S: AsyncWrite + Unpin + Send,
    R: AsyncRead + Unpin + Send,
    G: FnOnce() -> Fut + Send,
    Fut: Future<Output = bool> + Send,
{
    // ACL phase first (mirrors `initiate`): answer with the records the
    // initiator lacks, then apply what it pushes.
    let my_acl_len = peer.acl_len().await;
    let records = if my_acl_len > init_acl_len {
        peer.acl_suffix(init_acl_len).await
    } else {
        Vec::new()
    };
    write_frame(
        send,
        &SyncMessage::AclResponse {
            records,
            len: my_acl_len,
        },
    )
    .await?;
    match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::AclPush { records } => peer.ingest_ordered_acl(records).await,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "acl-push",
            });
        }
    }

    // 1. Receive the initiator's digest — then the admission gate:
    //    the ACL phase above ran for anyone; the Notice phase below is
    //    served only if the gate (now looking at the post-reconcile fold)
    //    admits the caller.
    let init_digest = match read_frame::<SyncMessage>(recv).await? {
        SyncMessage::TallyRequest(digest) => digest,
        _ => {
            return Err(SessionError::Unexpected {
                expected: "tally-request",
            });
        }
    };
    if !gate().await {
        write_frame(send, &SyncMessage::Ack { ok: false }).await?;
        return Ok(());
    }
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
