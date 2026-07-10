//! `bedrock`: the turso-backed engine for the bedrock schema vocabulary.
//!
//! `bedrock-core` says how a Rust type maps into storage; this crate makes that
//! real against a turso database. It generates a table from a
//! [`CollectionSpec`](bedrock_core::CollectionSpec), splits an entity into a row
//! to insert it, and reassembles entities read back out.
//!
//! turso 0.6.1 exposes only SQLite's four storage classes — there are no custom
//! `uuid`/`jsonb` column types yet — so uuids and the chrono types ride in TEXT,
//! booleans in INTEGER, and documents as JSON text. All engine-specific
//! behavior lives behind this crate's boundary; nothing leaks into the core
//! vocabulary.

mod database;
mod ddl;
mod error;
mod value;

pub use database::Database;
pub use error::BedrockError;
