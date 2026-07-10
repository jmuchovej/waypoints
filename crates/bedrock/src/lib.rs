//! `bedrock`: the turso-backed engine for the bedrock schema vocabulary.
//!
//! `bedrock-core` says how a Rust type maps into storage; this crate makes that
//! real against a turso database (a pinned pre-release). It generates STRICT
//! tables with custom column types from a
//! [`CollectionSpec`](bedrock_core::CollectionSpec), stores documents as `jsonb`,
//! does CRUD, provisions materialized views, and runs queries. All
//! engine-specific behavior lives behind this crate's boundary; nothing leaks
//! into the core vocabulary.
//!
//! Connections are pooled: one serialized writer, several lazily-created
//! readers (see [`Config`]). [`Database::flush`] exposes the durability
//! primitive; scheduling it is the caller's policy.

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

pub use config::Config;
pub use database::Database;
pub use durability::FlushMode;
pub use error::BedrockError;
pub use explain::QueryPlan;
pub use migrate::MigrationError;
pub use tx::{ReadTx, Savepoint, WriteTx};
