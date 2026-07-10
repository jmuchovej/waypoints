use crate::{builtin, collections::chart::ChartRow, error::StoreResult};

/// Seed the builtin charts into the bedrock database.
///
/// Uses `upsert` so this is idempotent on every boot — existing rows are
/// replaced in-place, new ones are inserted. The bedrock `upsert` takes the
/// entity by value and returns it; we discard the returned value here.
pub async fn run(db: &bedrock::Database) -> StoreResult<()> {
    let charts = [
        builtin::page::chart(),
        builtin::objective::chart(),
        builtin::note::chart(),
        builtin::person::chart(),
        builtin::campaign::chart(),
        builtin::meeting::chart(),
        builtin::bookmark::chart(),
        builtin::journal::chart(),
        builtin::book::chart(),
    ];

    let count = charts.len();
    for chart in charts {
        let row = ChartRow::from_chart(&chart);
        db.upsert::<ChartRow>(row).await?;
    }

    tracing::info!(count, "seed: upserted builtin charts");
    Ok(())
}
