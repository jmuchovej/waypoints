use crate::field::{DecodeError, EncodeError, Field};
use crate::spec::CollectionSpec;
use crate::value::StoredValue;

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
}
