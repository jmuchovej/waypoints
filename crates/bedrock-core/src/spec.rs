use serde::{Deserialize, Serialize};

use crate::field::FieldKind;
use crate::value::ColumnType;

/// One field of a collection — a named slot with a storage classification.
///
/// A `FieldSpec` carries the same facts as a [`Field`](crate::Field) impl's
/// `KIND` and `NULLABLE` constants, reified as runtime data: schemas can be
/// inspected, compared, and serialized without the Rust types in hand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSpec {
    /// The field's name in the virtual document (after any serde rename).
    pub name: String,
    /// Whether the field promotes to a native column or stays a `jsonb` document.
    pub kind: FieldKind,
    /// Whether the field admits `NULL` — i.e. the Rust type is `Option<_>`.
    pub nullable: bool,
}

impl FieldSpec {
    /// The logical column type, if this field promotes to a native column.
    pub fn column_type(&self) -> Option<ColumnType> {
        match self.kind {
            FieldKind::Column(ty) => Some(ty),
            FieldKind::Document => None,
        }
    }

    /// Whether this field is stored as a `jsonb` document rather than a column.
    pub fn is_document(&self) -> bool {
        matches!(self.kind, FieldKind::Document)
    }
}

/// The primary key of a collection.
///
/// The id is always a scalar column — you cannot key a document. `column_type`
/// is denormalized from the id field's [`FieldSpec`] for the convenience of
/// consumers that only hold the [`IdSpec`], and is kept honest by
/// [`CollectionSpec::validate`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdSpec {
    /// The name of the primary-key field — a member of [`CollectionSpec::fields`].
    pub field: String,
    /// The logical type of the id column.
    pub column_type: ColumnType,
}

/// A secondary index on one or more of a collection's scalar columns.
///
/// Indexes cover base *columns* only — a `jsonb` document has nothing
/// indexable here; document paths become queryable by projection into a
/// materialized view. A multi-column entry is a composite index.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSpec {
    /// The index name, unique within the database.
    pub name: String,
    /// The indexed columns, in order.
    pub columns: Vec<String>,
    /// Whether the index enforces uniqueness.
    pub unique: bool,
}

/// A collection's schema, expressed as data.
///
/// Pure data: no engine, no SQL. The `fields` are in virtual-document order and
/// include the id field — the id is just the column that happens to be the PK.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSpec {
    /// The collection (table) name.
    pub name: String,
    /// The primary key.
    pub id: IdSpec,
    /// Every field, in virtual-document order, the id field included.
    pub fields: Vec<FieldSpec>,
    /// Secondary indexes on scalar columns (the PK is implicitly indexed).
    #[serde(default)]
    pub indexes: Vec<IndexSpec>,
}

impl CollectionSpec {
    /// The field with the given name, if any.
    pub fn field(&self, name: &str) -> Option<&FieldSpec> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The [`FieldSpec`] of the primary-key field, if present.
    pub fn id_field(&self) -> Option<&FieldSpec> {
        self.field(&self.id.field)
    }

    /// The fields that promote to native columns (the id field among them).
    pub fn columns(&self) -> impl Iterator<Item = &FieldSpec> {
        self.fields.iter().filter(|f| !f.is_document())
    }

    /// The fields stored as `jsonb` documents.
    pub fn documents(&self) -> impl Iterator<Item = &FieldSpec> {
        self.fields.iter().filter(|f| f.is_document())
    }

    /// Check that the spec is internally consistent.
    ///
    /// An inconsistent spec — hand-built or generated — should fail loudly
    /// here rather than later as a confusing SQL error.
    pub fn validate(&self) -> Result<(), SpecError> {
        if self.name.is_empty() {
            return Err(SpecError::EmptyName);
        }
        if self.fields.is_empty() {
            return Err(SpecError::NoFields);
        }

        for (i, field) in self.fields.iter().enumerate() {
            if self.fields[..i]
                .iter()
                .any(|earlier| earlier.name == field.name)
            {
                return Err(SpecError::DuplicateField {
                    name: field.name.clone(),
                });
            }
        }

        let id = self.id_field().ok_or_else(|| SpecError::UnknownIdField {
            field: self.id.field.clone(),
        })?;
        match id.column_type() {
            None => {
                return Err(SpecError::IdMustBeColumn {
                    field: id.name.clone(),
                });
            }
            Some(actual) if actual != self.id.column_type => {
                return Err(SpecError::IdTypeMismatch {
                    field: id.name.clone(),
                    declared: self.id.column_type,
                    actual,
                });
            }
            Some(_) => {}
        }
        if id.nullable {
            return Err(SpecError::NullableId {
                field: id.name.clone(),
            });
        }

        for index in &self.indexes {
            if index.columns.is_empty() {
                return Err(SpecError::EmptyIndex {
                    name: index.name.clone(),
                });
            }
            for column in &index.columns {
                match self.field(column) {
                    None => {
                        return Err(SpecError::UnknownIndexColumn {
                            index: index.name.clone(),
                            column: column.clone(),
                        });
                    }
                    Some(field) if field.is_document() => {
                        return Err(SpecError::IndexOnDocument {
                            index: index.name.clone(),
                            column: column.clone(),
                        });
                    }
                    Some(_) => {}
                }
            }
        }

        Ok(())
    }
}

/// A collection schema that does not hold together.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpecError {
    #[error("collection name is empty")]
    EmptyName,

    #[error("collection has no fields")]
    NoFields,

    #[error("duplicate field name {name:?}")]
    DuplicateField { name: String },

    #[error("id field {field:?} is not among the collection's fields")]
    UnknownIdField { field: String },

    #[error("id field {field:?} must be a scalar column, not a document")]
    IdMustBeColumn { field: String },

    #[error("id field {field:?} declared as {declared:?} but its type is {actual:?}")]
    IdTypeMismatch {
        field: String,
        declared: ColumnType,
        actual: ColumnType,
    },

    #[error("id field {field:?} must not be nullable")]
    NullableId { field: String },

    #[error("index {name:?} has no columns")]
    EmptyIndex { name: String },

    #[error("index {index:?} references unknown column {column:?}")]
    UnknownIndexColumn { index: String, column: String },

    #[error("index {index:?} column {column:?} is a document; index scalar columns only")]
    IndexOnDocument { index: String, column: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spec shaped like
    /// `struct Thing { id: Uuid, kind: String, data: <doc>, extra: Option<<doc>> }`.
    fn thing_spec() -> CollectionSpec {
        CollectionSpec {
            name: "things".into(),
            id: IdSpec {
                field: "id".into(),
                column_type: ColumnType::Uuid,
            },
            fields: vec![
                FieldSpec {
                    name: "id".into(),
                    kind: FieldKind::Column(ColumnType::Uuid),
                    nullable: false,
                },
                FieldSpec {
                    name: "kind".into(),
                    kind: FieldKind::Column(ColumnType::Text),
                    nullable: false,
                },
                FieldSpec {
                    name: "data".into(),
                    kind: FieldKind::Document,
                    nullable: false,
                },
                FieldSpec {
                    name: "extra".into(),
                    kind: FieldKind::Document,
                    nullable: true,
                },
            ],
            indexes: vec![IndexSpec {
                name: "things_kind_idx".into(),
                columns: vec!["kind".into()],
                unique: false,
            }],
        }
    }

    #[test]
    fn valid_spec_validates() {
        assert_eq!(thing_spec().validate(), Ok(()));
    }

    #[test]
    fn partitions_columns_and_documents() {
        let spec = thing_spec();
        let columns: Vec<_> = spec.columns().map(|f| f.name.as_str()).collect();
        let documents: Vec<_> = spec.documents().map(|f| f.name.as_str()).collect();
        assert_eq!(columns, ["id", "kind"]);
        assert_eq!(documents, ["data", "extra"]);
    }

    #[test]
    fn resolves_id_and_named_fields() {
        let spec = thing_spec();
        assert_eq!(spec.id_field().unwrap().name, "id");
        assert_eq!(
            spec.field("kind").unwrap().column_type(),
            Some(ColumnType::Text)
        );
        assert!(spec.field("missing").is_none());
    }

    #[test]
    fn rejects_empty_name_and_fields() {
        let mut spec = thing_spec();
        spec.name = String::new();
        assert_eq!(spec.validate(), Err(SpecError::EmptyName));

        let mut spec = thing_spec();
        spec.fields.clear();
        assert_eq!(spec.validate(), Err(SpecError::NoFields));
    }

    #[test]
    fn rejects_duplicate_field() {
        let mut spec = thing_spec();
        spec.fields.push(FieldSpec {
            name: "kind".into(),
            kind: FieldKind::Column(ColumnType::Text),
            nullable: false,
        });
        assert_eq!(
            spec.validate(),
            Err(SpecError::DuplicateField {
                name: "kind".into()
            })
        );
    }

    #[test]
    fn rejects_unknown_id_field() {
        let mut spec = thing_spec();
        spec.id.field = "nope".into();
        assert_eq!(
            spec.validate(),
            Err(SpecError::UnknownIdField {
                field: "nope".into()
            })
        );
    }

    #[test]
    fn rejects_document_id() {
        let mut spec = thing_spec();
        spec.id = IdSpec {
            field: "data".into(),
            column_type: ColumnType::Uuid,
        };
        assert_eq!(
            spec.validate(),
            Err(SpecError::IdMustBeColumn {
                field: "data".into()
            })
        );
    }

    #[test]
    fn rejects_id_type_mismatch() {
        let mut spec = thing_spec();
        spec.id.column_type = ColumnType::Text;
        assert_eq!(
            spec.validate(),
            Err(SpecError::IdTypeMismatch {
                field: "id".into(),
                declared: ColumnType::Text,
                actual: ColumnType::Uuid,
            })
        );
    }

    #[test]
    fn rejects_nullable_id() {
        let mut spec = thing_spec();
        spec.fields[0].nullable = true;
        assert_eq!(
            spec.validate(),
            Err(SpecError::NullableId { field: "id".into() })
        );
    }

    #[test]
    fn rejects_bad_indexes() {
        let mut spec = thing_spec();
        spec.indexes.push(IndexSpec {
            name: "bad".into(),
            columns: vec!["missing".into()],
            unique: false,
        });
        assert!(matches!(
            spec.validate(),
            Err(SpecError::UnknownIndexColumn { .. })
        ));

        let mut spec = thing_spec();
        spec.indexes.push(IndexSpec {
            name: "on_doc".into(),
            columns: vec!["data".into()],
            unique: false,
        });
        assert!(matches!(
            spec.validate(),
            Err(SpecError::IndexOnDocument { .. })
        ));
    }

    #[test]
    fn round_trips_through_json() {
        let spec = thing_spec();
        let json = serde_json::to_string(&spec).unwrap();
        let back: CollectionSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, back);
    }
}
