//! `bedrock`: the turso-backed engine for the bedrock schema vocabulary.
//!
//! `bedrock-core` says how a Rust type maps into storage; this crate makes that
//! real against a turso database (a pinned pre-release). It generates STRICT
//! tables with custom column types from a
//! [`CollectionSpec`](bedrock_core::CollectionSpec), stores documents as `jsonb`,
//! does CRUD, provisions materialized views, and runs queries. All
//! engine-specific behavior lives behind this crate's boundary; nothing leaks
//! into the core vocabulary.

mod database;
mod ddl;
mod error;
mod query;
mod value;

pub use database::Database;
pub use error::BedrockError;
