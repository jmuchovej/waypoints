//! `bedrock-core`: the schema vocabulary at the heart of bedrock.
//!
//! This crate is pure and engine-free. It answers one question — *how does a
//! Rust type map into storage?* — and nothing else. No turso, no SQL, no I/O.
//!
//! The load-bearing piece is the [`Field`] trait: every field type declares its
//! [`FieldKind`] (a promotable [`ColumnType`] scalar, or a `jsonb`
//! [`Document`](FieldKind::Document)) and round-trips through a [`StoredValue`].

mod field;
mod spec;
mod value;

pub use field::{DecodeError, Field, FieldKind};
pub use spec::{CollectionSpec, FieldSpec, IdSpec, SpecError};
pub use value::{ColumnType, StoredValue};
