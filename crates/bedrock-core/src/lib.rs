//! `bedrock-core`: the schema vocabulary at the heart of bedrock.
//!
//! This crate is pure and engine-free. It answers one question — *how does a
//! Rust type map into storage?* — and nothing else. No turso, no SQL, no I/O.
//!
//! The load-bearing piece is the [`Field`] trait: every field type declares its
//! [`FieldKind`] (a promotable [`ColumnType`] scalar, or a `jsonb`
//! [`Document`](FieldKind::Document)) and round-trips through a [`StoredValue`].

mod collection;
mod field;
mod spec;
mod value;

pub use collection::Collection;
pub use field::{DecodeError, EncodeError, Field, FieldKind};
pub use spec::{CollectionSpec, FieldSpec, IdSpec, IndexSpec, SpecError};
pub use value::{ColumnType, StoredValue};
