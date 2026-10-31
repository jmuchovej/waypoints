//! `bedrock`: the turso-backed engine for the bedrock schema vocabulary.
//!
//! `bedrock-core` says how a Rust type maps into storage; this crate makes that
//! real against a turso database (a pinned pre-release). It generates STRICT
//! tables with custom column types from a
//! [`CollectionSpec`], stores documents as `jsonb`,
//! does CRUD, provisions materialized views, and runs queries. All
//! engine-specific behavior lives behind this crate's boundary; nothing leaks
//! into the core vocabulary.
//!
//! Connections are pooled: one serialized writer, several lazily-created
//! readers (see [`Config`]). [`Database::flush`] exposes the durability
//! primitive; scheduling it is the caller's policy.
//!
//! # Façade
//!
//! `bedrock` is the single crate a consumer needs. It re-exports the whole
//! `bedrock-core` vocabulary and the [`Scalar`]/[`Document`]/[`Collection`]
//! derives, and the derives emit `::bedrock::…` paths — so a crate that depends
//! only on `bedrock` can define and store its types without ever naming
//! `bedrock-core` or `bedrock-derive`.

#[cfg(feature = "engine")]
mod engine {
    pub(crate) mod config;
    pub(crate) mod database;
    pub(crate) mod ddl;
    pub(crate) mod durability;
    pub(crate) mod error;
    pub(crate) mod explain;
    pub(crate) mod migrate;
    pub(crate) mod pool;
    pub(crate) mod query;
    pub(crate) mod tx;
    pub(crate) mod value;

    // The synchronous level is the engine's own type, re-exported: `Config` takes
    // it and the durability readback reports it. (The engine's `JournalMode` is
    // crate-private upstream, so that one is bedrock's — see `durability`.)
    pub use turso_core::SyncMode;
}

// Engine surface (this crate).
#[cfg(feature = "engine")]
pub use engine::{
    SyncMode,
    config::Config,
    database::Database,
    durability::{DurabilityFault, DurabilityState, ENGINE_VERSION, FlushMode, JournalMode},
    error::BedrockError,
    explain::QueryPlan,
    migrate::MigrationError,
    tx::{ReadTx, Savepoint, WriteTx},
};

pub(crate) use bedrock_core as core;

// Façade: the core vocabulary. Re-exported so consumers depend only on `bedrock`
// and so the derives' generated `::bedrock::…` paths resolve.
pub use core::{
    Assignment, Collection, CollectionSpec, ColumnType, CompareOp, DecodeError, Discriminant,
    EncodeError, Field, FieldKind, FieldSpec, Filter, IdSpec, IndexSpec, Query, QueryError,
    ScalarValue, Sort, SpecError, StoredValue, Update, ViewColumn, ViewScope, ViewSpec, WriteOp,
};

// Façade: the derives. `Collection` names both a trait (above) and this derive
// macro — they live in different namespaces, mirroring `serde::Serialize`.
pub use bedrock_derive::{Collection, Document, Scalar};
