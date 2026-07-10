use bedrock::{BedrockError, Database, MigrationError};
use bedrock_core::{CollectionSpec, ColumnType, FieldKind, FieldSpec, IdSpec};

fn column(name: &str, ty: ColumnType, nullable: bool) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        kind: FieldKind::Column(ty),
        nullable,
    }
}

fn widget_v1() -> CollectionSpec {
    CollectionSpec {
        name: "widget".into(),
        id: IdSpec {
            field: "id".into(),
            column_type: ColumnType::Uuid,
        },
        fields: vec![
            column("id", ColumnType::Uuid, false),
            column("kind", ColumnType::Text, false),
        ],
        indexes: vec![],
    }
}

#[tokio::test]
async fn migrations_apply_safe_and_refuse_destructive() {
    let db = Database::open(":memory:").await.expect("open");

    // First ensure creates the table + records the spec; a second is a no-op.
    db.ensure_spec(&widget_v1()).await.expect("create v1");
    db.ensure_spec(&widget_v1()).await.expect("idempotent v1");

    // v2 adds a nullable column — a safe, auto-applied migration.
    let mut v2 = widget_v1();
    v2.fields.push(column("note", ColumnType::Text, true));
    db.ensure_spec(&v2).await.expect("migrate to v2");
    // The stored spec is now v2, so re-ensuring v2 is a no-op...
    db.ensure_spec(&v2).await.expect("idempotent v2");

    // ...and going back to v1 now reads as dropping "note" — destructive, refused.
    let err = db.ensure_spec(&widget_v1()).await.unwrap_err();
    assert!(matches!(
        err,
        BedrockError::Migration(MigrationError::ColumnDropped { .. })
    ));

    // Changing a column's type is also refused.
    let mut retyped = v2.clone();
    retyped.fields[1].kind = FieldKind::Column(ColumnType::Integer);
    let err = db.ensure_spec(&retyped).await.unwrap_err();
    assert!(matches!(
        err,
        BedrockError::Migration(MigrationError::ColumnChanged { .. })
    ));
}
