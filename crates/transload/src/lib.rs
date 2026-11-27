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

// The grammar builds with no features; `export` and `ingest` need a store.
pub(crate) mod bundle;
pub(crate) mod document;
pub(crate) mod frontmatter;
pub(crate) mod identity;
pub(crate) mod names;
pub(crate) mod refusal;
pub(crate) mod schema;
pub(crate) mod sidecar;
#[cfg(feature = "store")]
pub(crate) mod store {
    pub(crate) mod export;
    pub(crate) mod ingest;
}

/// The Fragment grammar: what an importer emits. The data model's entities
/// with a source key wherever the core would mint an identifier, and nothing
/// the core owns. Builds for wasm32 with no engine and no entropy.
pub mod fragment {
    mod census;
    mod key;
    mod landmark;
    mod protochart;
    mod reading;
    mod terrain;
    pub(crate) mod timestamp;

    pub use census::Fragments;
    pub use key::{EmptySourceKey, SourceKey};
    pub use landmark::{ContentFragment, LandmarkFragment};
    pub use protochart::{
        FeatureFragment, FeatureKindFragment, Protochart, SelectGroupFragment, SelectOptionFragment,
    };
    pub use reading::ReadingFragment;
    pub use terrain::{TerrainFragment, TerrainKindFragment};

    /// The Fragment envelope's version: additive minor, breaking major, `0.x`
    /// while pre-freeze. Independent of the ABI major and of `bundle_format`.
    pub const FRAGMENT_FORMAT: &str = "0.1.0";
}

/// The importer ABI's payloads: what crosses the boundary between a host and
/// a plugin module, in both directions. The exports themselves are the SDK's.
pub mod abi {
    mod diagnostic;
    mod manifest;
    mod output;

    pub use diagnostic::{At, Diagnostic, Level};
    pub use manifest::{Authored, Manifest, Source, SourceKind};
    pub use output::{Envelope, FinalizeOutput, Init, Output, Summary};

    /// The ABI major: exports, signatures, and the buffer convention. Frozen.
    pub const ABI: u32 = 1;
}

/// The plugin side: the [`Importer`](sdk::Importer) trait, the `importer!`
/// macro that turns an implementation into the ABI's exports, the manifest
/// loader, and the options-schema generator. What a plugin links.
#[cfg(feature = "sdk")]
pub mod sdk {
    pub mod buffer;
    mod importer;
    mod macros;
    mod manifest;
    pub mod panic;
    mod run;
    mod schema;

    pub use importer::{Importer, Manifested};
    pub use manifest::{ManifestError, build as manifest, parse as parse_manifest};
    pub use run::Runtime;
    pub use schema::{DRAFT_07, options as options_schema};
}

/// The conformance harness: a plugin driven natively through the trait and
/// again from its compiled module, over a fixture directory in the ABI's
/// entry order, against committed goldens. Never in a plugin.
#[cfg(feature = "harness")]
pub mod harness {
    mod error;
    mod fixture;
    pub mod golden;
    pub mod module;
    pub mod native;
    mod run;

    pub use error::HarnessError;
    pub use fixture::entries;
    pub use run::{Difference, Entry, Run};
}

pub use bundle::{
    BUNDLE_FORMAT, Bundle, Census, Counts, ExportMeta, Manifest, SUPPORTED_FORMATS, layout,
};
pub use frontmatter::{Advisory, Frontmatter};
pub use identity::{Identity, Marker};
pub use refusal::{
    ChartConflict, DuplicateConflict, IngestError, Location, Malformed, MalformedKind, Refusal,
};

#[cfg(feature = "store")]
pub use store::{
    export::{ExportError, Exported, Tier, export, tier},
    ingest::{Ingested, ingest},
};
