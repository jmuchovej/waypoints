//! Builtin chart seed migration.
//!
//! Inserts the 9 builtin charts (page, objective, note, person, campaign,
//! meeting, bookmark, journal, book) using stable [`ChartID`s][ChartID].
//! Idempotency: probes for the first builtin chart's ID; if present, assumes
//! all are seeded and skips. Inserts are wrapped in a transaction with
//! `INSERT OR FAIL` semantics so a partial-seed state rolls back cleanly.
//!
//! [ChartID]: domain::chart::ChartID
use domain::chart::Chart;
use turso::Connection;

use crate::builtin;
use crate::error::StoreResult;

fn builtin_charts() -> Vec<Chart> {
    vec![
        builtin::page::chart(),
        builtin::objective::chart(),
        builtin::note::chart(),
        builtin::person::chart(),
        builtin::campaign::chart(),
        builtin::meeting::chart(),
        builtin::bookmark::chart(),
        builtin::journal::chart(),
        builtin::book::chart(),
    ]
}

pub async fn run(conn: &Connection) -> StoreResult<()> {
    let charts = builtin_charts();
    let Some(first) = charts.first() else {
        return Ok(());
    };

    if probe_chart(conn, &first.id.0).await? {
        tracing::debug!("seed_charts: already applied, skipping");
        return Ok(());
    }

    conn.execute("BEGIN", ()).await?;
    let result = insert_charts(conn, &charts).await;

    match result {
        Ok(()) => {
            conn.execute("COMMIT", ()).await?;
            tracing::info!(count = charts.len(), "seed_charts: inserted builtin charts");
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", ()).await;
            Err(e)
        }
    }
}

async fn probe_chart(conn: &Connection, id: &str) -> StoreResult<bool> {
    let mut rows = conn
        .query("SELECT 1 FROM charts WHERE id = ?", [id])
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn insert_charts(conn: &Connection, charts: &[Chart]) -> StoreResult<()> {
    const SQL: &str = r#"INSERT OR FAIL INTO charts (id, name, description, icon, features)
                        VALUES (?, ?, ?, ?, ?)"#;

    for chart in charts {
        let icon_json: Option<String> =
            chart.icon.as_ref().map(serde_json::to_string).transpose()?;
        let features_json = serde_json::to_string(&chart.features)?;

        conn.execute(
            SQL,
            (
                chart.id.0.clone(),
                chart.name.clone(),
                chart.description.clone(),
                icon_json,
                features_json,
            ),
        )
        .await?;
    }
    Ok(())
}
