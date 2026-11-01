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

// The grammar builds with no features; `export` and `ingest` need a store.
pub(crate) mod bundle;
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
