//! Access control as an ordered log (§8).
//!
//! Content edits tolerate concurrency; membership does not. Each
//! Territory's ACL is a strictly ordered, signed log — the only linearized
//! structure in the system. State is a **pure fold** over that log:
//! deterministic, side-effect free, identically recomputable by every peer
//! (Vessel and Station alike — §3 rule 5).
//!
//! Subordinate decisions (refining the design doc):
//! - ACL records get their own wire-version byte ([`RECORD_VERSION`]),
//!   mirroring the envelope discipline.
//! - The order proof (§8.4) lives *outside* the author-signed bytes and
//!   outside [`RecordID`]: the orderer countersigns after the fact, and
//!   folding either would make the hash circular / the ID unstable.
//! - Invites are single-use: a `Join` consumes its invite. Re-inviting is
//!   cheap; silent multi-use invites are a liability.
//! - A `Join`'s proof binds the invite key to the joiner's identity *and*
//!   their KEM key (domain-tagged), so an observed Join cannot be replayed
//!   by or for anyone else.

use std::collections::BTreeMap;

use bytes::Bytes;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::crypto::KemPublicKey;
use crate::error::WireError;
use crate::id::{EpochID, MemberID, RecordID, TerritoryID};

/// Wire-format version of the ACL record encoding.
pub const RECORD_VERSION: u8 = 1;

/// Domain tag for the `Join` invite proof.
const JOIN_PROOF_TAG: &[u8] = b"outpost.join.v1";
/// Domain tag for orderer countersignatures (§8.4).
const ORDER_PROOF_TAG: &[u8] = b"outpost.order.v1";

crate::id::byte_id! {
    /// An invite's Ed25519 public key (§8.2). The matching secret travels
    /// out-of-band inside the invite payload; whoever holds it can `Join`.
    InviteKey
}

/// Permission lattice: `Owner ⊃ Admin ⊃ Writer ⊃ Reader` (§8.2). The
/// derived order makes `>=` mean "at least".
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Permissions {
    Reader,
    Writer,
    Admin,
    Owner,
}

/// The membership operations (§8.2). Sealed boxes are opaque bytes here —
/// the fold checks their *coverage*, never their contents (Stations cannot
/// decrypt, and must not need to).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ACLPayload {
    /// Creates the Territory. Author becomes Owner; epoch 0's read key is
    /// sealed to their KEM key.
    Root {
        territory_seed: [u8; 32],
        owner_kem: KemPublicKey,
        epoch0_key_wrapped: Vec<u8>,
    },
    /// Author invites the holder of `invite_key`'s secret.
    Invite {
        invite_key: InviteKey,
        permissions: Permissions,
    },
    /// The joiner presents their identity + KEM key, proven with the
    /// invite secret. Author is the *joiner* — not yet a member.
    Join {
        invite_key: InviteKey,
        member_kem: KemPublicKey,
        invite_proof: Signature,
    },
    /// An admin admits a joined candidate; the current read key is sealed
    /// to the new member (no rotation on add — §7.2).
    Approve {
        member: MemberID,
        permissions: Permissions,
        current_key_wrapped: Vec<u8>,
    },
    ChangeRole {
        member: MemberID,
        permissions: Permissions,
    },
    /// Removal must rotate (§7.2): the next epoch's key sealed to each
    /// remaining member, plus the old key wrapped under the new one so the
    /// backwards chain stays walkable (§7.3).
    Remove {
        member: MemberID,
        new_epoch: EpochID,
        new_key_wrapped: Vec<(MemberID, Vec<u8>)>,
        prev_key_wrapped: Vec<u8>,
    },
    RevokeInvite {
        invite_key: InviteKey,
    },
}

/// What the author signs: chain position, identity, operation.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RecordBody {
    /// The record this one chains from; `None` only for `Root`.
    pub prev: Option<RecordID>,
    /// The record author's identity key. Authority is checked against the
    /// folded state, not against who handed us the bytes.
    pub author: VerifyingKey,
    pub payload: ACLPayload,
}

impl RecordBody {
    /// Canonical encoding: version byte, then the postcard body — the byte
    /// string the author signs. Same raw-bytes discipline as Notices.
    pub fn encode(&self) -> Result<Bytes, WireError> {
        let mut out = postcard::to_allocvec(self)?;
        out.insert(0, RECORD_VERSION);
        Ok(Bytes::from(out))
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, WireError> {
        let (&version, body) = bytes.split_first().ok_or(WireError::Truncated)?;
        if version != RECORD_VERSION {
            return Err(WireError::UnsupportedVersion(version));
        }
        Ok(postcard::from_bytes(body)?)
    }

    /// Encode and sign. `key` must be the identity key of `self.author`;
    /// signing with any other key produces a record every peer rejects.
    pub fn sign(&self, key: &SigningKey) -> Result<ACLRecord, WireError> {
        let bytes = self.encode()?;
        let signature = key.sign(&bytes);
        Ok(ACLRecord { bytes, signature })
    }
}

/// Signed, immutable ACL record. As with Notices, `bytes` is canonical.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ACLRecord {
    pub bytes: Bytes,
    pub signature: Signature,
}

impl ACLRecord {
    pub fn parse_and_verify(&self) -> Result<RecordBody, WireError> {
        let body = RecordBody::parse(&self.bytes)?;
        body.author.verify_strict(&self.bytes, &self.signature)?;
        Ok(body)
    }

    /// Canonical encoding — what [`RecordID`] hashes. The order proof is
    /// *not* part of it (see the module notes).
    pub fn encode(&self) -> Result<Bytes, WireError> {
        Ok(Bytes::from(postcard::to_allocvec(self)?))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        Ok(postcard::from_bytes(bytes)?)
    }
}

impl RecordID {
    /// Content address of an ACL record: BLAKE3 of its encoding.
    pub fn of(record: &ACLRecord) -> Result<Self, WireError> {
        Ok(Self::from_bytes(
            *blake3::hash(&record.encode()?).as_bytes(),
        ))
    }
}

impl TerritoryID {
    /// A Territory's identity is the hash of its root ACL record (§9) —
    /// the same bytes as the root's [`RecordID`], as a distinct type.
    pub fn of_root(root: &ACLRecord) -> Result<Self, WireError> {
        Ok(Self::from_bytes(*RecordID::of(root)?.as_bytes()))
    }
}

/// The message an invite secret signs to prove a `Join` (domain-tagged;
/// binds invite → joiner identity → joiner KEM key).
pub fn join_binding(
    invite_key: &InviteKey,
    joiner: &VerifyingKey,
    member_kem: &KemPublicKey,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(JOIN_PROOF_TAG.len() + 32 + 32 + member_kem.as_bytes().len());
    msg.extend_from_slice(JOIN_PROOF_TAG);
    msg.extend_from_slice(invite_key.as_bytes());
    msg.extend_from_slice(joiner.as_bytes());
    msg.extend_from_slice(member_kem.as_bytes());
    msg
}

/// The per-Fleet orderer's countersignature over `(record, position)`
/// (§8.4). Travels beside the record, outside its ID and author signature.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct OrderProof {
    pub position: u64,
    pub signature: Signature,
}

fn order_message(record: RecordID, position: u64) -> Vec<u8> {
    let mut msg = Vec::with_capacity(ORDER_PROOF_TAG.len() + 32 + 8);
    msg.extend_from_slice(ORDER_PROOF_TAG);
    msg.extend_from_slice(record.as_bytes());
    msg.extend_from_slice(&position.to_le_bytes());
    msg
}

impl OrderProof {
    /// Orderer side: countersign a record's position with the fleet key.
    pub fn sign(fleet_key: &SigningKey, record: RecordID, position: u64) -> Self {
        Self {
            position,
            signature: fleet_key.sign(&order_message(record, position)),
        }
    }

    /// Peer side: audit a proof against the fleet's public key. Divergent
    /// proofs for one position are cryptographic evidence of orderer
    /// equivocation (§8.4) — callers surface that loudly, never silently.
    pub fn verify(
        &self,
        fleet_key: &VerifyingKey,
        record: RecordID,
    ) -> Result<(), ed25519_dalek::SignatureError> {
        fleet_key.verify_strict(&order_message(record, self.position), &self.signature)
    }
}

/// Why the fold refused a record. Every variant is a distinct, tested
/// behavior; rejection is terminal (§5.4 note applies to ACL records too).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ACLReject {
    #[error("record bytes/signature invalid: {0}")]
    Wire(String),
    #[error("record does not chain from the current head")]
    BrokenChain,
    #[error("only the first record may be a Root")]
    RootMisplaced,
    #[error("author is not a member")]
    NotAMember,
    #[error("author lacks the authority for this operation")]
    InsufficientAuthority,
    #[error("no such pending invite")]
    UnknownInvite,
    #[error("invite proof does not verify")]
    BadInviteProof,
    #[error("author is already a member or pending")]
    AlreadyMember,
    #[error("no such member")]
    UnknownMember,
    #[error("no such pending join")]
    UnknownJoin,
    #[error("the Owner's role cannot be changed or removed")]
    OwnerImmutable,
    #[error("a Remove must rotate to exactly the next epoch")]
    WrongEpoch,
    #[error("rotation wraps must cover exactly the remaining members")]
    WrapCoverage,
}

impl From<WireError> for ACLReject {
    fn from(err: WireError) -> Self {
        Self::Wire(err.to_string())
    }
}

/// The folded membership state (§8.2). Values are cheap to clone; the fold
/// returns a fresh state so rejected records cannot half-apply.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ACLState {
    /// The record that produced this state.
    pub head: RecordID,
    pub territory: TerritoryID,
    pub members: BTreeMap<MemberID, Permissions>,
    /// KEM keys for everyone a future `Remove` must reseal to.
    pub member_kems: BTreeMap<MemberID, KemPublicKey>,
    /// Joined-but-not-yet-approved candidates and the permissions their
    /// invite promised.
    pub pending_joins: BTreeMap<MemberID, (KemPublicKey, Permissions)>,
    pub pending_invites: BTreeMap<InviteKey, Permissions>,
    pub epoch: EpochID,
}

impl ACLState {
    /// Fold step zero: validate and apply a `Root` record (§8.2).
    pub fn genesis(root: &ACLRecord) -> Result<Self, ACLReject> {
        let body = root.parse_and_verify()?;
        let ACLPayload::Root { owner_kem, .. } = &body.payload else {
            return Err(ACLReject::RootMisplaced);
        };
        if body.prev.is_some() {
            return Err(ACLReject::BrokenChain);
        }
        let owner = MemberID::from(&body.author);
        Ok(Self {
            head: RecordID::of(root)?,
            territory: TerritoryID::of_root(root)?,
            members: BTreeMap::from([(owner, Permissions::Owner)]),
            member_kems: BTreeMap::from([(owner, owner_kem.clone())]),
            pending_joins: BTreeMap::new(),
            pending_invites: BTreeMap::new(),
            epoch: EpochID::ZERO,
        })
    }

    /// One fold step (§8.2): enforce chaining, authority, and
    /// well-formedness, and produce the next state.
    pub fn apply(&self, record: &ACLRecord) -> Result<Self, ACLReject> {
        let body = record.parse_and_verify()?;
        if body.prev != Some(self.head) {
            return Err(ACLReject::BrokenChain);
        }
        let author = MemberID::from(&body.author);
        let mut next = self.clone();
        next.head = RecordID::of(record)?;

        match &body.payload {
            ACLPayload::Root { .. } => return Err(ACLReject::RootMisplaced),

            ACLPayload::Invite {
                invite_key,
                permissions,
            } => {
                self.require_granting_authority(author, *permissions)?;
                next.pending_invites.insert(*invite_key, *permissions);
            }

            ACLPayload::Join {
                invite_key,
                member_kem,
                invite_proof,
            } => {
                if self.members.contains_key(&author) || self.pending_joins.contains_key(&author) {
                    return Err(ACLReject::AlreadyMember);
                }
                let promised = *self
                    .pending_invites
                    .get(invite_key)
                    .ok_or(ACLReject::UnknownInvite)?;
                let invite_verifier = VerifyingKey::from_bytes(invite_key.as_bytes())
                    .map_err(|_| ACLReject::BadInviteProof)?;
                invite_verifier
                    .verify_strict(
                        &join_binding(invite_key, &body.author, member_kem),
                        invite_proof,
                    )
                    .map_err(|_| ACLReject::BadInviteProof)?;
                next.pending_invites.remove(invite_key);
                next.pending_joins
                    .insert(author, (member_kem.clone(), promised));
            }

            ACLPayload::Approve {
                member,
                permissions,
                current_key_wrapped: _,
            } => {
                self.require_granting_authority(author, *permissions)?;
                let (kem, _promised) = next
                    .pending_joins
                    .remove(member)
                    .ok_or(ACLReject::UnknownJoin)?;
                next.members.insert(*member, *permissions);
                next.member_kems.insert(*member, kem);
            }

            ACLPayload::ChangeRole {
                member,
                permissions,
            } => {
                let current = *self.members.get(member).ok_or(ACLReject::UnknownMember)?;
                if current == Permissions::Owner {
                    return Err(ACLReject::OwnerImmutable);
                }
                // Touching an Admin (either direction) is Owner-only.
                let ceiling = current.max(*permissions);
                self.require_granting_authority(author, ceiling)?;
                next.members.insert(*member, *permissions);
            }

            ACLPayload::Remove {
                member,
                new_epoch,
                new_key_wrapped,
                prev_key_wrapped: _,
            } => {
                let current = *self.members.get(member).ok_or(ACLReject::UnknownMember)?;
                if current == Permissions::Owner {
                    return Err(ACLReject::OwnerImmutable);
                }
                self.require_granting_authority(author, current)?;
                if *new_epoch != self.epoch.next() {
                    return Err(ACLReject::WrongEpoch);
                }
                // Wraps must cover exactly the members that remain (§8.2).
                let mut covered: Vec<MemberID> = new_key_wrapped.iter().map(|(m, _)| *m).collect();
                covered.sort_unstable();
                covered.dedup();
                let remaining: Vec<MemberID> = self
                    .members
                    .keys()
                    .copied()
                    .filter(|m| m != member)
                    .collect();
                if covered != remaining {
                    return Err(ACLReject::WrapCoverage);
                }
                next.members.remove(member);
                next.member_kems.remove(member);
                next.epoch = *new_epoch;
            }

            ACLPayload::RevokeInvite { invite_key } => {
                self.require_authority(author, Permissions::Admin)?;
                if next.pending_invites.remove(invite_key).is_none() {
                    return Err(ACLReject::UnknownInvite);
                }
            }
        }
        Ok(next)
    }

    /// §5.4 step 4: may `author` write content at this state?
    pub fn require_writer(&self, author: &VerifyingKey) -> Result<(), ACLReject> {
        self.require_authority(MemberID::from(author), Permissions::Writer)
    }

    /// §5.4 step 5: a Notice's epoch must be current or older — a writer
    /// cannot pin an epoch this state has not reached yet.
    pub fn require_epoch_current_or_older(&self, epoch: EpochID) -> Result<(), ACLReject> {
        if epoch > self.epoch {
            return Err(ACLReject::WrongEpoch);
        }
        Ok(())
    }

    fn require_authority(&self, author: MemberID, needed: Permissions) -> Result<(), ACLReject> {
        let held = *self.members.get(&author).ok_or(ACLReject::NotAMember)?;
        if held < needed {
            return Err(ACLReject::InsufficientAuthority);
        }
        Ok(())
    }

    /// Membership operations need Admin; operations that grant or touch
    /// Admin-or-above need Owner ("only Owner may change Admins", §8.2).
    fn require_granting_authority(
        &self,
        author: MemberID,
        granting: Permissions,
    ) -> Result<(), ACLReject> {
        let needed = if granting >= Permissions::Admin {
            Permissions::Owner
        } else {
            Permissions::Admin
        };
        self.require_authority(author, needed)
    }
}
