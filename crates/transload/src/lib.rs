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

pub(crate) mod bundle;
pub(crate) mod frontmatter;
pub(crate) mod identity;
pub(crate) mod refusal;

pub use bundle::{Bundle, Counts, ExportMeta, Manifest, layout};
pub use frontmatter::{Advisory, Frontmatter};
pub use identity::{Identity, Marker};
pub use refusal::{
    ChartConflict, DuplicateConflict, IngestError, Location, Malformed, MalformedKind, Refusal,
};
