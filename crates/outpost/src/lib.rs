//! Outpost — the Waypoints sync layer façade.
//!
//! This is the crate embedding code depends on; it reaches nothing deeper.
//! The protocol vocabulary is always available; the two roles are
//! feature-gated, and the feature is pure dependency selection — `vessel`
//! compiles in the [`vessel`] module, `station` the [`station`] module, and
//! either pulls in the shared [`replica`] engine plus the fabric/hold deps
//! the roles need. Waypoints enables `vessel`; a self-host binary both.
//!
//! Layering below this façade (each a crate, so those boundaries stay
//! compiler-enforced): `outpost-protocol` (pure types, codec, validation,
//! ACL fold, key schedule, sync reconciliation), `outpost-fabric` (iroh
//! transport + framed sessions), `outpost-hold` (bedrock-backed storage).
//! The roles themselves are thin enough to live here as modules rather than
//! their own crates: a Station adds the orderer and serve loop; a Vessel
//! adds origination, the read-key cache, and decryption; both share the
//! [`replica`] engine (validate + store + answer sync).
//!
//! One boundary that was compiler-enforced when the roles were separate
//! crates is now convention: a Station holds no key or decryption code.
//! The [`station`] module keeps that discipline in code review, not in the
//! type system — the trade John accepted for collapsing three thin crates.

/// The protocol vocabulary — always available.
pub use outpost_protocol as protocol;

/// The one fabric type an embedding app must name: the dialable Station
/// address (it appears in role-module signatures and in app config). A
/// deliberate single re-export, not a barrel — the rest of the fabric stays
/// an internal layer behind this façade.
#[cfg(any(feature = "vessel", feature = "station"))]
pub use outpost_fabric::EndpointAddr;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(any(feature = "vessel", feature = "station"))]
pub mod replica;

#[cfg(feature = "vessel")]
pub mod vessel;

#[cfg(feature = "station")]
pub mod station;
