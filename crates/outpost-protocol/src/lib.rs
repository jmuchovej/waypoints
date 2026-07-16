//! The Outpost sync protocol: pure data model, codecs, and (in later
//! slices) validation, the ACL fold, key schedule, and sync state machines.
//!
//! Design doc: `.agents/outpost-design.md` — doc comments cite its
//! §numbers. This crate is deliberately I/O-free: no async runtime, no
//! network, no storage engine. Everything here is testable with byte
//! vectors, and the crate boundary keeps it that way (§12).

pub mod acl;
pub mod crypto;
mod envelope;
mod error;
mod id;

pub use envelope::{ENVELOPE_VERSION, Envelope, EnvelopeHeader, SignedNotice};
pub use error::{CryptoError, WireError};
pub use id::{BlobHash, EpochID, MemberID, NoticeID, RecordID, TerritoryID, TrackID};
