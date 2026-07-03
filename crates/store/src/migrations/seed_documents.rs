//! Document seed migration (mock feature only).
//!
//! Imports 34 embedded markdown files via [`WaypointsMarkdown::import`] and
//! inserts the resulting marks + terrains into the database. Idempotency
//! anchor is the stable UUID embedded in `demo.md`'s YAML frontmatter; if it's
//! already present, the seed is skipped.
//!
//! All inserts run inside a single transaction with `INSERT OR FAIL`
//! semantics — partial-seed states roll back cleanly. Per the spec, terrains
//! are only inserted after all marks across all documents succeed: any mark
//! failure aborts the transaction before any terrain is touched.

use chrono::Utc;
use domain::chart::{ChartID, Feature, FeatureID};
use domain::mark::{Mark, MarkData};
use domain::terrain::Terrain;
use importer::markdown::{MarkdownImporter, WaypointsMarkdown};
use indexmap::IndexMap;
use std::collections::HashMap;
use turso::Connection;
use uuid::Uuid;

use crate::entities::terrain_kind_to_string;
use crate::error::StoreResult;

/// UUID of the first seeded mark (from `data/demo.md` frontmatter).
/// Used as the idempotency anchor for the entire seed.
const SEED_ANCHOR_UUID: Uuid = Uuid::from_u128(0x45f453a4_b942_41ed_b144_57207eb74ce5);

/// Format datetime values to match sqlx-sqlite's NaiveDateTime encoding
/// (`%Y-%m-%d %H:%M:%S%.f`, space-separated, no timezone). The SeaORM
/// query layer reads with this format — diverge and rows fail to decode.
fn datetime_now() -> String {
    Utc::now()
        .naive_utc()
        .format("%Y-%m-%d %H:%M:%S%.f")
        .to_string()
}

/// sqlx-sqlite encodes `Uuid` as the 16 raw bytes (BLOB). Seeds must match.
fn uuid_to_blob(u: &Uuid) -> Vec<u8> {
    u.as_bytes().to_vec()
}

pub async fn run(conn: &Connection) -> StoreResult<()> {
    if probe_mark(conn, &SEED_ANCHOR_UUID).await? {
        tracing::debug!("seed_documents: already applied, skipping");
        return Ok(());
    }

    let chart_features = load_chart_features(conn).await?;

    conn.execute("BEGIN", ()).await?;
    let result = insert_all(conn, &chart_features).await;

    match result {
        Ok((mark_count, terrain_count)) => {
            conn.execute("COMMIT", ()).await?;
            tracing::info!(
                marks = mark_count,
                terrains = terrain_count,
                "seed_documents: inserted documents"
            );
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(e)
        }
    }
}

async fn probe_mark(conn: &Connection, id: &Uuid) -> StoreResult<bool> {
    let mut rows = conn
        .query("SELECT 1 FROM marks WHERE id = ?", [uuid_to_blob(id)])
        .await?;
    Ok(rows.next().await?.is_some())
}

/// Load `(ChartID, IndexMap<FeatureID, Feature>)` per row from the `charts`
/// table. Used to resolve reading-feature names back to their IDs.
async fn load_chart_features(
    conn: &Connection,
) -> StoreResult<HashMap<ChartID, IndexMap<FeatureID, Feature>>> {
    let mut out = HashMap::new();
    let mut rows = conn.query("SELECT id, features FROM charts", ()).await?;
    while let Some(row) = rows.next().await? {
        let id_str: String = row.get(0)?;
        let features_json: String = row.get(1)?;
        let Ok(chart_id) = id_str.parse::<ChartID>() else {
            continue;
        };
        let Ok(features) = serde_json::from_str::<IndexMap<FeatureID, Feature>>(&features_json)
        else {
            continue;
        };
        out.insert(chart_id, features);
    }
    Ok(out)
}

fn resolve_feature(features: &IndexMap<FeatureID, Feature>, name: &str) -> Option<FeatureID> {
    features
        .iter()
        .find(|(_, f)| f.name == name)
        .map(|(id, _)| id.clone())
}

async fn insert_all(
    conn: &Connection,
    chart_features: &HashMap<ChartID, IndexMap<FeatureID, Feature>>,
) -> StoreResult<(usize, usize)> {
    let mut mark_count = 0usize;
    let mut terrain_count = 0usize;

    for source in documents() {
        let result = WaypointsMarkdown::import(source);

        // Attach readings to the first mark if it's a Landmark.
        let mut marks = result.marks;
        if let Some(first) = marks.first_mut() {
            if !result.readings.is_empty() {
                if let MarkData::Landmark(landmark) = &mut first.data {
                    if let Some(features) = chart_features.get(&landmark.chart) {
                        for (name, reading) in &result.readings {
                            if let Some(feature_id) = resolve_feature(features, name) {
                                landmark.readings.insert(feature_id, reading.clone());
                            }
                        }
                    }
                }
            }
        }

        for mark in &marks {
            insert_mark(conn, mark).await?;
            mark_count += 1;
        }
        for terrain in &result.terrains {
            insert_terrain(conn, terrain).await?;
            terrain_count += 1;
        }
    }

    Ok((mark_count, terrain_count))
}

async fn insert_mark(conn: &Connection, mark: &Mark) -> StoreResult<()> {
    const SQL: &str = r#"INSERT OR FAIL INTO marks
        (id, mark, data, extra,
         created_at, updated_at,
         created_by, updated_by,
         deleted_at, archived_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;

    let now = datetime_now();
    let id = uuid_to_blob(&mark.id.inner());
    let mark_type = mark.type_name();
    let data_json = serde_json::to_string(&mark.data)?;
    let extra_json: Option<String> = mark.extra.as_ref().map(serde_json::to_string).transpose()?;

    conn.execute(
        SQL,
        (
            id,
            mark_type,
            data_json,
            extra_json,
            now.clone(),
            now,
            None::<Vec<u8>>,
            None::<Vec<u8>>,
            None::<String>,
            None::<String>,
        ),
    )
    .await?;
    Ok(())
}

async fn insert_terrain(conn: &Connection, terrain: &Terrain) -> StoreResult<()> {
    const SQL: &str = r#"INSERT OR FAIL INTO terrains
        (id, src_id, dst_id, kind, sequence, extra)
        VALUES (?, ?, ?, ?, ?, ?)"#;

    let id = uuid_to_blob(&terrain.id.inner());
    let src_id = uuid_to_blob(&terrain.src_id.inner());
    let dst_id = uuid_to_blob(&terrain.dst_id.inner());
    let kind = terrain_kind_to_string(&terrain.kind);
    let extra_json: Option<String> = terrain
        .extra
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    conn.execute(
        SQL,
        (id, src_id, dst_id, kind, terrain.order.clone(), extra_json),
    )
    .await?;
    Ok(())
}

/// All seed document sources, embedded at compile time.
fn documents() -> Vec<&'static str> {
    vec![
        // ── Pages ──────────────────────────────────────────────────
        include_str!("data/demo.md"),
        include_str!("data/japan-trip.md"),
        include_str!("data/tokyo-restaurants.md"),
        include_str!("data/travel-gear.md"),
        include_str!("data/reading-list.md"),
        // ── People ─────────────────────────────────────────────────
        include_str!("data/people/carlo-rovelli.md"),
        include_str!("data/people/douglas-hofstadter.md"),
        include_str!("data/people/richard-dawkins.md"),
        include_str!("data/people/melanie-mitchell.md"),
        include_str!("data/people/richard-feynman.md"),
        include_str!("data/people/thomas-kuhn.md"),
        include_str!("data/people/ernest-nagel.md"),
        include_str!("data/people/james-newman.md"),
        include_str!("data/people/brian-greene.md"),
        include_str!("data/people/yuval-noah-harari.md"),
        include_str!("data/people/roger-penrose.md"),
        include_str!("data/people/carl-sagan.md"),
        // ── Books ──────────────────────────────────────────────────
        include_str!("data/books/order-of-time.md"),
        include_str!("data/books/godel-escher-bach.md"),
        include_str!("data/books/selfish-gene.md"),
        include_str!("data/books/complexity.md"),
        include_str!("data/books/surely-joking.md"),
        include_str!("data/books/structure-revolutions.md"),
        include_str!("data/books/godels-proof.md"),
        include_str!("data/books/elegant-universe.md"),
        include_str!("data/books/sapiens.md"),
        include_str!("data/books/emperors-new-mind.md"),
        include_str!("data/books/cosmos.md"),
    ]
}
