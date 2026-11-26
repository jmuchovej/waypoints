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

//! The Outpost sync protocol: pure data model, codecs, and (in later
//! slices) validation, the ACL fold, key schedule, and sync state machines.
//!
//! This crate is deliberately I/O-free: no async runtime, no
//! network, no storage engine. Everything here is testable with byte
//! vectors, and the crate boundary keeps it that way.

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "outpost-protocol: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "outpost-protocol: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "outpost-protocol: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

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
