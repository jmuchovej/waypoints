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

mod config;
mod database;
mod ddl;
mod durability;
mod error;
mod explain;
mod migrate;
mod pool;
mod query;
mod tx;
mod value;

// Engine surface (this crate).
pub use config::Config;
pub use database::Database;
pub use durability::FlushMode;
pub use error::BedrockError;
pub use explain::QueryPlan;
pub use migrate::MigrationError;
pub use tx::{ReadTx, Savepoint, WriteTx};

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
