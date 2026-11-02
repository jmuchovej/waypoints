//! `bedrock-core`: the schema vocabulary at the heart of bedrock.
//!
//! This crate is pure and engine-free. It answers one question — *how does a
//! Rust type map into storage?* — and nothing else. No turso, no SQL, no I/O.
//!
//! The load-bearing piece is the [`Field`] trait: every field type declares its
//! [`FieldKind`] (a promotable [`ColumnType`] scalar, or a `jsonb`
//! [`Document`](FieldKind::Document)) and round-trips through a [`StoredValue`].

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "bedrock-core: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "bedrock-core: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "bedrock-core: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

mod collection;
mod field;
mod modify;
mod query;
mod spec;
mod value;
mod view;

pub use collection::{Collection, WriteOp};
pub use field::{DecodeError, EncodeError, Field, FieldKind};
pub use modify::{Assignment, Update};
pub use query::{CompareOp, Filter, Query, QueryError, ScalarValue, Sort};
pub use spec::{CollectionSpec, FieldSpec, IdSpec, IndexSpec, SpecError};
pub use value::{ColumnType, StoredValue};
pub use view::{Discriminant, ViewColumn, ViewScope, ViewSpec};
