//! Outpost — the sync layer for Waypoints.
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

/// The transport layer, routed through the façade so neither an embedding
/// app nor this crate's own modules ever name `outpost_fabric` directly.
/// Mostly an internal layer; what an app actually needs is
/// [`fabric::EndpointAddr`] (the dialable Station address, which appears in
/// role-module signatures and in app config) and [`fabric::fleet`].
#[cfg(any(feature = "vessel", feature = "station"))]
pub use outpost_fabric as fabric;

/// The storage layer, routed for the same reason as [`fabric`]. An embedding
/// app names [`hold::TerritoryHold`] when opening a Territory; the rest is
/// the roles' business.
#[cfg(any(feature = "vessel", feature = "station"))]
pub use outpost_hold as hold;

#[cfg(any(feature = "vessel", feature = "station"))]
pub mod replica;

#[cfg(feature = "vessel")]
pub mod vessel;

#[cfg(feature = "station")]
pub mod station;
