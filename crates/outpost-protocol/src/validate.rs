//! The validation function (§5.4). This function *is* the protocol: every
//! peer — Vessel or Station, identically (§3 rule 5) — runs it on every
//! Notice received, from anyone. Validity travels with the data, never
//! with the connection.
//!
//! Layering notes:
//! - Validation never decrypts. The AAD binding (§7.4) protects payload
//!   integrity for key holders; everything here reads plaintext structure.
//! - The §8.3 *ingest quarantine* (drop fresh Notices from since-removed
//!   authors) is deliberately not here: it consults the peer's *current*
//!   ACL state and arrival time, which are service-level facts. This
//!   function is pure — same inputs, same verdict, on every peer forever.
//! - "Roots reference themselves" (§6.3) is implemented with the
//!   [`NoticeID::SELF`] sentinel: a content address cannot literally
//!   contain itself.

use crate::acl::{ACLReject, ACLState};
use crate::envelope::{Envelope, SignedNotice};
use crate::error::WireError;
use crate::id::{NoticeID, RecordID};

impl NoticeID {
    /// Sentinel meaning "this Notice itself" in `snapshot_base`, used by
    /// track roots (§6.3). All-zero is unreachable as a real BLAKE3 output.
    pub const SELF: Self = Self::from_bytes([0u8; 32]);
}

/// Where a peer resolves `acl_head` pins: already-validated ACL records
/// and the states their fold produced (§8). Implementations also expose
/// each record's log position so the §8.3 monotonicity rule can compare
/// pins without refolding.
pub trait ACLStateProvider {
    fn state_at(&self, record: RecordID) -> Option<&ACLState>;
    /// Position of the record in the (totally ordered) ACL log.
    fn position_of(&self, record: RecordID) -> Option<u64>;
}

/// What validation needs from local Notice storage: presence, and the ACL
/// pin of already-accepted Notices (for monotonicity against parents).
pub trait NoticeStore {
    fn contains(&self, notice: NoticeID) -> bool;
    /// `acl_head` of an already-accepted Notice. `None` iff unknown.
    fn acl_head_of(&self, notice: NoticeID) -> Option<RecordID>;
}

/// A passed verdict (§5.4). `Pending` is a fetch obligation, not an
/// acceptance: the Notice waits in the orphan queue and re-enters
/// validation when its parents arrive.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Validated {
    Ready {
        id: NoticeID,
        envelope: Box<Envelope>,
    },
    Pending {
        id: NoticeID,
        missing: Vec<NoticeID>,
    },
}

/// Rejection is terminal and non-gossiped (§5.4): an invalid Notice is
/// simply dropped, never forwarded, never retried.
#[derive(Debug, thiserror::Error)]
pub enum RejectReason {
    /// Undecodable bytes or a signature that does not verify.
    #[error("malformed notice: {0}")]
    Malformed(#[from] WireError),

    /// The pinned `acl_head` is not a validated record we know (§5.4 step
    /// 3). Distinct from `Pending`: parents are fetchable data, an unknown
    /// ACL pin means the *authority claim itself* cannot be evaluated.
    #[error("unknown ACL head")]
    UnknownACLHead,

    /// The author may not write at the pinned ACL state (§5.4 step 4).
    #[error("author not permitted at pinned ACL state: {0}")]
    Forbidden(#[from] ACLReject),

    /// The pinned epoch is newer than the pinned ACL state allows (§5.4
    /// step 5).
    #[error("epoch ahead of pinned ACL state")]
    EpochAhead,

    /// §8.3 monotonicity: a Notice's `acl_head` must be at or after each
    /// parent's in log order. This is what structurally confines a removed
    /// member's forks to history they could already read.
    #[error("ACL pin regresses behind a parent's pin")]
    ACLRegression,

    /// A track root must use the [`NoticeID::SELF`] snapshot sentinel, and
    /// only a track root may.
    #[error("malformed snapshot base")]
    BadSnapshotBase,
}

/// §5.4, steps in order. Every peer runs exactly this on every received
/// Notice; Vessels additionally decrypt and merge afterwards, Stations
/// never do.
pub fn validate(
    notice: &SignedNotice,
    acl: &impl ACLStateProvider,
    store: &impl NoticeStore,
) -> Result<Validated, RejectReason> {
    // 1–2. Well-formed, signed over the raw bytes by the embedded author.
    let envelope = notice.parse_and_verify()?;
    let header = &envelope.header;
    let id = NoticeID::of(notice)?;

    // 3. The pinned ACL record exists and is validated.
    let state = acl
        .state_at(header.acl_head)
        .ok_or(RejectReason::UnknownACLHead)?;

    // 4. The author may write *at that state* — this plus step 6 is what
    // defeats backdated writes by removed members (§8.3).
    state.require_writer(&header.author)?;

    // 5. The epoch is one the pinned state has reached.
    state
        .require_epoch_current_or_older(header.epoch)
        .map_err(|_| RejectReason::EpochAhead)?;

    // Roots (and only roots) use the self-snapshot sentinel.
    let is_root = header.parents.is_empty();
    if is_root != (header.snapshot_base == NoticeID::SELF) {
        return Err(RejectReason::BadSnapshotBase);
    }

    // 6. Parents: monotonicity against the ones we have, pending on the
    // ones we don't.
    let own_position = acl
        .position_of(header.acl_head)
        .ok_or(RejectReason::UnknownACLHead)?;
    let mut missing = Vec::new();
    for parent in &header.parents {
        match store.acl_head_of(*parent) {
            Some(parent_pin) => {
                // A parent was accepted, so its pin resolves; a gap here is
                // local inconsistency, not a property of this Notice.
                let parent_position = acl
                    .position_of(parent_pin)
                    .ok_or(RejectReason::UnknownACLHead)?;
                if own_position < parent_position {
                    return Err(RejectReason::ACLRegression);
                }
            }
            None => missing.push(*parent),
        }
    }

    if missing.is_empty() {
        Ok(Validated::Ready {
            id,
            envelope: Box::new(envelope),
        })
    } else {
        Ok(Validated::Pending { id, missing })
    }
}
