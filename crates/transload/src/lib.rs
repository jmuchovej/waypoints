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

//! The Bundle format — Waypoints' exit.
//!
//! A Bundle is a plain directory of human-legible markdown that Waypoints can
//! re-ingest without loss. `export` and `ingest` are one pair with three
//! guarantees:
//!
//! - **\[L\] legibility** — strip every comment and what remains is plain text.
//! - **\[F\] fidelity** — `ingest(export(store)) == store`.
//! - **\[I\] idempotency** — exporting twice yields byte-identical Bundles.
//!
//! The markdown dialect lives in `domain`; Bundle-level grammar (frontmatter,
//! identity comments, manifest) lives here. A Bundle is never partially
//! ingested: see [`Refusal`].

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "transload: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "transload: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "transload: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

pub(crate) mod bundle;
pub(crate) mod document;
pub(crate) mod export;
pub(crate) mod frontmatter;
pub(crate) mod identity;
pub(crate) mod ingest;
pub(crate) mod names;
pub(crate) mod refusal;
pub(crate) mod schema;
pub(crate) mod sidecar;

pub use bundle::{
    BUNDLE_FORMAT, Bundle, Census, Counts, ExportMeta, Manifest, SUPPORTED_FORMATS, layout,
};
pub use export::{ExportError, Exported, Tier, export, tier};
pub use frontmatter::{Advisory, Frontmatter};
pub use identity::{Identity, Marker};
pub use ingest::{Ingested, ingest};
pub use refusal::{
    ChartConflict, DuplicateConflict, IngestError, Location, Malformed, MalformedKind, Refusal,
};
