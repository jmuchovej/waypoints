//! Bedrock-backed storage for Outpost records. This is the storage
//! interface for *records* — Notices and ACL records — on both Station and
//! Vessel roles. Files/assets are out of scope here (they live behind a
//! separate `BlobStore` seam).
//!
//! Layering:
//! - One bedrock `Database` per Territory ([`TerritoryHold`]): one turso
//!   file, so GC / deletion / migration stay per-Territory mechanical.
//! - Notices are stored as their **raw `SignedNotice` bytes** plus
//!   *denormalized* header columns for indexing. The bytes column is
//!   the object of record; the typed columns exist only to make "heads of a
//!   track", "everything after topo key K", and "pending parents" index
//!   lookups. Reads that make a security decision re-derive everything from
//!   the bytes — the denormalized columns are never trusted for that.
//! - The store seams in `outpost-protocol` ([`NoticeStore`],
//!   [`ACLStateProvider`]) are synchronous and pure; bedrock is async. So a
//!   hold *materializes* an in-memory [`ACLIndex`] snapshot (the folded log)
//!   that implements those traits, and validation runs against the
//!   snapshot. Persistence is async; validation stays pure.

mod acl_index;
mod error;
mod hold;
mod models;
mod station_hold;
pub(crate) use outpost_protocol as protocol;

pub use acl_index::ACLIndex;
pub use error::HoldError;
pub use hold::{NoticeIndex, StoredNotice, TerritoryHold};
pub use models::{ACLs, Backlog, NoticeKey, Notices, RecordKey};
pub use station_hold::{RegistrationRow, SightingRow, StationHold};
