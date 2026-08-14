//! The domain types map into bedrock storage: IDs as scalar columns, MarkData /
//! Icon as jsonb documents. Round-tripped through `bedrock::Field`.

use bedrock::{ColumnType, Field, FieldKind, StoredValue};
use domain::ChartID;
use domain::Icon;
use domain::TerrainID;
use domain::{Content, MarkData, MarkID};
use uuid::Uuid;

#[test]
fn ids_are_scalar_columns_and_roundtrip() {
    assert_eq!(MarkID::KIND, FieldKind::Column(ColumnType::Uuid));
    assert_eq!(TerrainID::KIND, FieldKind::Column(ColumnType::Uuid));
    assert_eq!(ChartID::KIND, FieldKind::Column(ColumnType::Text));

    let m = MarkID(Uuid::from_u128(1));
    assert_eq!(MarkID::decode(m.encode().unwrap()).unwrap(), m);
    let t = TerrainID(Uuid::from_u128(2));
    assert_eq!(TerrainID::decode(t.encode().unwrap()).unwrap(), t);
    // The raw ChartID (incl. any `builtin.` prefix) round-trips as text.
    let c = ChartID("builtin.page".to_string());
    assert_eq!(ChartID::decode(c.encode().unwrap()).unwrap(), c);
}

#[test]
fn markdata_and_icon_are_documents_and_roundtrip() {
    assert_eq!(MarkData::KIND, FieldKind::Document);
    assert_eq!(Icon::KIND, FieldKind::Document);

    let data = MarkData::Content(Content::default());
    let enc = data.encode().unwrap();
    assert!(matches!(enc, StoredValue::Json(_)));
    assert_eq!(MarkData::decode(enc).unwrap(), data);

    let icon = Icon {
        identifier: "star".into(),
        alt_text: None,
        colorable: None,
        metadata: None,
    };
    assert_eq!(Icon::decode(icon.encode().unwrap()).unwrap(), icon);
}
