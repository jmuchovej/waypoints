#![allow(dead_code)]

use bedrock::Database;
use chrono::{NaiveDate, NaiveDateTime};
use domain::{
    Chart, ChartID, Content, Feature, FeatureID, FeatureKind, Landmark, Mark, MarkData, MarkID,
    Reading, RichText, Terrain, TerrainID, TerrainKind,
};
use indexmap::IndexMap;
use transload::ExportMeta;

pub const CHART: &str = "c4d8";

pub fn at(milli: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 8, 2)
        .unwrap()
        .and_hms_milli_opt(9, 41, 0, milli)
        .unwrap()
}

pub fn meta() -> ExportMeta {
    ExportMeta {
        app_version: "0.3.1".into(),
        exported_at: at(0),
        territory: None,
    }
}

pub async fn open() -> Database {
    let db = Database::open(":memory:").await.expect("open");
    db.ensure_collection::<Mark>().await.expect("marks");
    db.ensure_collection::<Terrain>().await.expect("terrains");
    db.ensure_collection::<Chart>().await.expect("charts");
    db
}

pub fn chart_id() -> ChartID {
    ChartID(CHART.into())
}

pub fn chart() -> Chart {
    let notes = Feature {
        id: FeatureID::derived("notes"),
        name: "Notes".into(),
        kind: FeatureKind::Text,
        description: None,
        required: false,
    };
    let mut features = IndexMap::new();
    features.insert(notes.id.clone(), notes);
    Chart {
        id: chart_id(),
        name: "Meeting".into(),
        description: None,
        icon: None,
        features,
        extra: None,
    }
}

pub fn landmark(id: MarkID, title: &str) -> Mark {
    Mark {
        id,
        data: MarkData::Landmark(Landmark {
            chart: chart_id(),
            title: RichText::from(title.to_string()),
            icon: None,
            readings: IndexMap::new(),
        }),
        extra: None,
        created_at: at(0),
        updated_at: at(1),
        deleted_at: None,
    }
}

pub fn content(id: MarkID, content: Content) -> Mark {
    Mark {
        id,
        data: MarkData::Content(content),
        extra: None,
        created_at: at(2),
        updated_at: at(3),
        deleted_at: None,
    }
}

pub fn paragraph(id: MarkID, text: &str) -> Mark {
    content(
        id,
        Content::Paragraph {
            rich_text: RichText::from(text.to_string()),
            theme: None,
        },
    )
}

pub fn edge(src: MarkID, dst: MarkID, kind: TerrainKind, order: Option<&str>) -> Terrain {
    Terrain {
        id: TerrainID::new(),
        src_id: src,
        dst_id: dst,
        kind,
        order: order.map(str::to_string),
        extra: None,
        deleted_at: None,
    }
}

pub fn child(src: MarkID, dst: MarkID, order: &str) -> Terrain {
    edge(src, dst, TerrainKind::Child, Some(order))
}

pub fn text_reading(text: &str) -> (FeatureID, Reading) {
    (
        FeatureID::derived("notes"),
        Reading::Text(RichText::from(text.to_string())),
    )
}

pub async fn census(db: &Database) -> (u64, u64, u64) {
    (
        db.count::<Mark>().await.expect("marks"),
        db.count::<Terrain>().await.expect("terrains"),
        db.count::<Chart>().await.expect("charts"),
    )
}
