use crate::field::{DecodeError, EncodeError, Field};
use crate::spec::CollectionSpec;
use crate::value::StoredValue;

/// Which write is about to happen, passed to [`Collection::before_save`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteOp {
    /// A row is being inserted (`insert`, `insert_all`, or `upsert`).
    Insert,
    /// An existing row is being replaced (`update`).
    Update,
}

/// An entity that maps to one bedrock collection (table).
///
/// This is the contract the `#[derive(Collection)]` macro implements. It is
/// pure — no engine, no SQL — so it can be exercised without a database: it
/// yields the collection's [`CollectionSpec`] and round-trips a value through a
/// positional row of [`StoredValue`]s.
///
/// The row is aligned to [`CollectionSpec::fields`] order. The derive owns both
/// the spec and the split/merge, so `to_row`, `from_row`, and `spec().fields`
/// share one column order by construction.
pub trait Collection: Sized {
    /// The primary-key field's type, so callers can look up and delete by id.
    type Id: Field;

    /// The collection's schema.
    fn spec() -> CollectionSpec;

    /// Split a value into a positional row of stored values, in `spec().fields` order.
    fn to_row(&self) -> Result<Vec<StoredValue>, EncodeError>;

    /// Reassemble a value from a positional row, in `spec().fields` order.
    fn from_row(row: Vec<StoredValue>) -> Result<Self, DecodeError>;

    /// Hook run just before a whole-entity write, returning the value to store.
    ///
    /// Consume-and-return (rather than `&mut`) so the transformation is
    /// explicit: the engine stores — and hands back — exactly what this returns.
    /// Use it to stamp derived or audit columns (a discriminant recomputed from a
    /// document, an `updated_at` timestamp). The default is the identity.
    ///
    /// `#[derive(Collection)]` wires this to a `#[bedrock(before_save = path)]`
    /// function when present; otherwise this default applies. Field-level updates
    /// (`update_where`) do not run it.
    fn before_save(self, op: WriteOp) -> Self {
        let _ = op;
        self
    }
}
