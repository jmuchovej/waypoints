use crate::{core::CollectionSpec, ddl};

/// A schema change bedrock cannot apply automatically.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MigrationError {
    #[error("collection {collection:?}: the primary key cannot change")]
    IdChanged { collection: String },

    #[error("collection {collection:?}: column {column:?} was dropped")]
    ColumnDropped { collection: String, column: String },

    #[error("collection {collection:?}: column {column:?} changed type or nullability")]
    ColumnChanged { collection: String, column: String },

    #[error(
        "collection {collection:?}: new column {column:?} is NOT NULL and cannot be added to a populated table"
    )]
    NonNullableColumnAdded { collection: String, column: String },
}

/// The DDL that brings `stored` up to `target`, or a [`MigrationError`] if the
/// change is destructive. An empty plan means the schemas already match.
///
/// Safe, auto-applied: new **nullable** columns, and index add/drop/change.
/// Everything else — dropped columns, a type or nullability change, a changed id,
/// or a new non-null column — is destructive and refused here.
pub(crate) fn plan(
    stored: &CollectionSpec,
    target: &CollectionSpec,
) -> Result<Vec<String>, MigrationError> {
    let collection = || target.name.clone();

    if stored.id != target.id {
        return Err(MigrationError::IdChanged {
            collection: collection(),
        });
    }

    // Dropped columns are destructive.
    for field in &stored.fields {
        if target.field(&field.name).is_none() {
            return Err(MigrationError::ColumnDropped {
                collection: collection(),
                column: field.name.clone(),
            });
        }
    }

    let mut statements = Vec::new();

    // New and changed columns.
    for field in &target.fields {
        match stored.field(&field.name) {
            None if !field.nullable => {
                return Err(MigrationError::NonNullableColumnAdded {
                    collection: collection(),
                    column: field.name.clone(),
                });
            }
            None => statements.push(ddl::alter_add_column(target, field)),
            Some(existing)
                if existing.kind != field.kind || existing.nullable != field.nullable =>
            {
                return Err(MigrationError::ColumnChanged {
                    collection: collection(),
                    column: field.name.clone(),
                });
            }
            Some(_) => {}
        }
    }

    // Indexes: drop those that are gone or changed, create those that are new or
    // changed. Drops are emitted before creates so a same-named redefinition works.
    for index in &stored.indexes {
        if !target.indexes.contains(index) {
            statements.push(ddl::drop_index(&index.name));
        }
    }
    for index in &target.indexes {
        if !stored.indexes.contains(index) {
            statements.push(ddl::create_index(target, index));
        }
    }

    Ok(statements)
}

#[cfg(test)]
mod tests {
    use bedrock_core::{ColumnType, FieldKind, FieldSpec, IdSpec, IndexSpec};

    use super::*;

    fn column(name: &str, ty: ColumnType, nullable: bool) -> FieldSpec {
        FieldSpec {
            name: name.into(),
            kind: FieldKind::Column(ty),
            nullable,
        }
    }

    fn spec(fields: Vec<FieldSpec>, indexes: Vec<IndexSpec>) -> CollectionSpec {
        CollectionSpec {
            name: "thing".into(),
            id: IdSpec {
                field: "id".into(),
                column_type: ColumnType::Uuid,
            },
            fields,
            indexes,
        }
    }

    fn base() -> CollectionSpec {
        spec(
            vec![
                column("id", ColumnType::Uuid, false),
                column("kind", ColumnType::Text, false),
            ],
            vec![],
        )
    }

    #[test]
    fn identical_specs_are_a_noop() {
        assert_eq!(plan(&base(), &base()), Ok(vec![]));
    }

    #[test]
    fn adding_a_nullable_column_is_safe() {
        let mut target = base();
        target.fields.push(column("note", ColumnType::Text, true));
        let statements = plan(&base(), &target).unwrap();
        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("ADD COLUMN"));
        assert!(statements[0].contains("\"note\""));
    }

    #[test]
    fn adding_a_non_null_column_is_refused() {
        let mut target = base();
        target.fields.push(column("note", ColumnType::Text, false));
        assert!(matches!(
            plan(&base(), &target),
            Err(MigrationError::NonNullableColumnAdded { .. })
        ));
    }

    #[test]
    fn dropping_a_column_is_refused() {
        let mut target = base();
        target.fields.pop();
        assert!(matches!(
            plan(&base(), &target),
            Err(MigrationError::ColumnDropped { .. })
        ));
    }

    #[test]
    fn changing_type_or_nullability_is_refused() {
        let mut retyped = base();
        retyped.fields[1].kind = FieldKind::Column(ColumnType::Integer);
        assert!(matches!(
            plan(&base(), &retyped),
            Err(MigrationError::ColumnChanged { .. })
        ));

        let mut widened = base();
        widened.fields[1].nullable = true;
        assert!(matches!(
            plan(&base(), &widened),
            Err(MigrationError::ColumnChanged { .. })
        ));
    }

    #[test]
    fn changing_the_id_is_refused() {
        let mut target = base();
        target.id.field = "kind".into();
        assert!(matches!(
            plan(&base(), &target),
            Err(MigrationError::IdChanged { .. })
        ));
    }

    #[test]
    fn adding_and_dropping_indexes_is_safe() {
        let index = IndexSpec {
            name: "thing_kind_idx".into(),
            columns: vec!["kind".into()],
            unique: false,
        };
        let mut with_index = base();
        with_index.indexes.push(index.clone());

        let create = plan(&base(), &with_index).unwrap();
        assert_eq!(create.len(), 1);
        assert!(create[0].contains("CREATE") && create[0].contains("INDEX"));

        let drop = plan(&with_index, &base()).unwrap();
        assert_eq!(drop.len(), 1);
        assert!(drop[0].contains("DROP INDEX"));
    }
}
