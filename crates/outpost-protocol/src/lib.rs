//! The Outpost sync protocol: pure data model, codecs, and (in later
//! slices) validation, the ACL fold, key schedule, and sync state machines.
//!
//! This crate is deliberately I/O-free: no async runtime, no
//! network, no storage engine. Everything here is testable with byte
//! vectors, and the crate boundary keeps it that way.

pub(crate) mod acl;
pub(crate) mod auth;
pub(crate) mod crypto;
mod envelope;
mod error;
mod hlc;
mod id;
pub(crate) mod identity;
pub(crate) mod sync;
mod validate;

pub use acl::{
    ACLPayload, ACLRecord, ACLReject, ACLState, InviteKey, OrderError, OrderProof, OrderedRecord,
    Permissions, RECORD_VERSION, RecordBody, join_binding,
};
pub use auth::{DEVICE_CERT_CONTEXT, DeviceCert, cert_message};
pub use crypto::{
    CryptoRNG, KEM_CIPHERTEXT_LEN, KEM_PUBLIC_LEN, KEMPublic, KEMSecret, NONCE_LEN, ReadKey,
    open_payload, open_sealed_key, seal_key_to_member, seal_payload, unwrap_prev_key,
    wrap_prev_key,
};
pub use envelope::{ENVELOPE_VERSION, Envelope, EnvelopeHeader, SignedNotice};
pub use error::{CryptoError, WireError};
pub use hlc::{Clock, HLC, SystemClock};
pub use id::{BlobHash, EpochID, MemberID, NoticeID, RecordID, TerritoryID, TrackID};
pub use identity::{DeviceKey, Identity, IdentityError, KeyRoot};
pub use sync::{DeviceSighting, SyncMessage, TallyDigest, wanted};
pub use validate::{ACLStateProvider, NoticeStore, RejectReason, Validated, validate};
