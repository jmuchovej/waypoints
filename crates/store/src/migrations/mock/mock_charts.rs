use crate::builtin::builtin_charts;
use crate::entities::{ActiveChart, ChartModel, Charts};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{Set, Unchanged},
    EntityTrait, TryIntoModel,
};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use waypoints_domain::chart::{Chart, ChartID};

use crate::migrations::utils::MigrationResult;

pub type ChartIDMap = HashMap<String, ChartID>;

pub(crate) async fn seed_charts(manager: &SchemaManager<'_>) -> MigrationResult<ChartIDMap> {
    let db = manager.get_connection();

    let mut chartmap = ChartIDMap::new();
    for chart in builtin_charts() {
        let Chart {
            id,
            name,
            description,
            icon,
            features,
        } = chart;
        let mut active = ActiveChart {
            id: Set(id.0.clone()),
            name: Set(name.clone()),
            description: Set(description.clone()),
            icon: Set(icon.map(|i| serde_json::to_value(i).unwrap())),
            features: Set(serde_json::to_value(features).unwrap()),
        };

        let exists = Charts::find_by_id(&id.0).one(db).await?;

        let model: ChartModel = if exists.is_some() {
            active.id = Unchanged(id.0.clone());
            active.update(db).await?
        } else {
            active.insert(db).await?
        }
        .try_into_model()?;

        chartmap.insert(model.name, model.id.try_into()?);
    }

    Ok(chartmap)
}
