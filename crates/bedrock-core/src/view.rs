use serde::{Deserialize, Serialize};

use crate::value::ColumnType;

/// A materialized view over a collection: homogeneous, typed projections of
/// document paths, so those paths can be filtered and sorted (which the raw,
/// heterogeneous `jsonb` document cannot support directly).
///
/// Views are **runtime** artifacts — in Waypoints, user-created Charts drive
/// them — so a `ViewSpec` is built as data, not produced by a derive. It is
/// scoped to a collection at the call site (`declare_view::<C>`), while the
/// projection and discriminant are runtime values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewSpec {
    /// The view name, unique within the database.
    pub name: String,
    /// An optional discriminant filter scoping the view to one kind of row.
    pub scope: Option<ViewScope>,
    /// The projected columns.
    pub columns: Vec<ViewColumn>,
}

/// Scope a view to rows whose document path equals a value — the discriminant
/// (e.g. a chart id or a serde tag).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewScope {
    /// The document column to look inside.
    pub document: String,
    /// A dotted path within it, e.g. `"chart"`.
    pub path: String,
    /// The value the path must equal (compared as text).
    pub equals: String,
}

/// One projected column: a document path lifted into a typed view column.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewColumn {
    /// The output column name.
    pub name: String,
    /// The document column to extract from.
    pub document: String,
    /// A dotted path within it, e.g. `"readings.abc"`.
    pub path: String,
    /// The logical type of the projected value, used when reading it back.
    pub column_type: ColumnType,
}
