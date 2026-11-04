// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

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

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "outpost-hold: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "outpost-hold: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "outpost-hold: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

mod acl_index;
mod error;
mod hold;
mod models;
mod station_hold;
pub(crate) use outpost_protocol as protocol;

pub use acl_index::ACLIndex;
pub use error::HoldError;
pub use hold::{NoticeIndex, StoredNotice, TerritoryHold};
pub use models::{ACLs, Backlog, Meta, NoticeKey, Notices, RecordKey};
pub use station_hold::{RegistrationRow, SightingRow, StationHold};
