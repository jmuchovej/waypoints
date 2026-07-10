use bedrock_core::{Collection, ColumnType, Field, FieldKind, StoredValue};
use bedrock_derive::{Collection, Document, Scalar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Scalar)]
struct MarkId(Uuid);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Scalar)]
#[serde(rename_all = "kebab-case")]
enum TerrainKind {
    ChildOf,
    ReferencesTo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct ThingData {
    title: String,
    // `u64` is fine *inside* a document (serde handles it); it just can't be a
    // promoted scalar column.
    count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Document)]
struct Extra {
    note: String,
}

#[derive(Debug, Clone, PartialEq, Collection)]
struct Thing {
    #[bedrock(id)]
    id: MarkId,
    kind: TerrainKind,
    data: ThingData,
    extra: Option<Extra>,
}

#[test]
fn scalar_newtype_forwards_to_inner() {
    assert_eq!(MarkId::KIND, FieldKind::Column(ColumnType::Uuid));

    let id = MarkId(Uuid::from_u128(0x2a));
    let encoded = id.encode().unwrap();
    assert_eq!(MarkId::decode(encoded).unwrap(), id);
}

#[test]
fn scalar_enum_uses_serde_names() {
    assert_eq!(TerrainKind::KIND, FieldKind::Column(ColumnType::Text));

    assert_eq!(
        TerrainKind::ChildOf.encode().unwrap(),
        StoredValue::Text("child-of".into())
    );
    assert_eq!(
        TerrainKind::decode(StoredValue::Text("references-to".into())).unwrap(),
        TerrainKind::ReferencesTo
    );
    assert!(TerrainKind::decode(StoredValue::Text("nope".into())).is_err());
    assert!(TerrainKind::decode(StoredValue::Integer(1)).is_err());
}

#[test]
fn document_roundtrips_through_json() {
    assert_eq!(ThingData::KIND, FieldKind::Document);

    let data = ThingData {
        title: "hello".into(),
        count: 3,
    };
    let encoded = data.encode().unwrap();
    assert!(matches!(encoded, StoredValue::Json(_)));
    assert_eq!(ThingData::decode(encoded).unwrap(), data);
}

#[test]
fn collection_spec_reflects_the_struct() {
    let spec = Thing::spec();
    spec.validate().expect("derived spec is valid");

    assert_eq!(spec.name, "thing");
    assert_eq!(spec.id.field, "id");
    assert_eq!(spec.id.column_type, ColumnType::Uuid);

    let columns: Vec<_> = spec.columns().map(|f| f.name.as_str()).collect();
    let documents: Vec<_> = spec.documents().map(|f| f.name.as_str()).collect();
    assert_eq!(columns, ["id", "kind"]);
    assert_eq!(documents, ["data", "extra"]);

    assert!(spec.field("extra").unwrap().nullable);
    assert!(!spec.field("data").unwrap().nullable);
}

#[test]
fn collection_row_roundtrips() {
    let thing = Thing {
        id: MarkId(Uuid::from_u128(42)),
        kind: TerrainKind::ChildOf,
        data: ThingData {
            title: "t".into(),
            count: 1,
        },
        extra: None,
    };

    let row = thing.to_row().unwrap();
    assert_eq!(row.len(), 4);
    // extra is None → NULL in the wire form.
    assert_eq!(row[3], StoredValue::Null);

    assert_eq!(Thing::from_row(row).unwrap(), thing);
}

#[test]
fn from_row_rejects_wrong_column_count() {
    let err = Thing::from_row(vec![StoredValue::Null]).unwrap_err();
    assert!(matches!(
        err,
        bedrock_core::DecodeError::ColumnCount {
            expected: 4,
            found: 1,
            ..
        }
    ));
}
