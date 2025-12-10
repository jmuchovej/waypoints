use super::ChartIDMap;
use crate::utils::{MigrationError, MigrationResult};
use sea_orm::{ActiveModelTrait, ActiveValue::Unchanged, EntityTrait, TryIntoModel};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use waypoints_domain::{
    chart::ChartID,
    mark::{Content, Landmark, Mark, MarkID, RichText},
};
use waypoints_entities::prelude::{ActiveMark, MarkModel, Marks};

pub type MarkIDMap = HashMap<String, MarkID>;

/// Seed marks - currently only Page landmarks and basic content
pub(crate) async fn seed_marks(
    manager: &SchemaManager<'_>,
    chart_ids: &ChartIDMap,
) -> MigrationResult<MarkIDMap> {
    let mut marks = Vec::new();

    // Add Pages only (other chart types deferred)
    let page_id = try_get_chart(chart_ids, "Page")?;
    marks.extend(seed_pages(page_id).await?);

    // Add basic content marks
    marks.extend(seed_content_marks().await?);

    let db = manager.get_connection();
    let mut markmap = MarkIDMap::new();

    for mark in marks {
        let Mark { id, .. } = mark;
        let exists = Marks::find_by_id(id.inner()).one(db).await?;
        let mut active: ActiveMark = mark.into();

        let model: MarkModel = if exists.is_some() {
            active.id = Unchanged(id.inner());
            active.update(db).await?
        } else {
            active.insert(db).await?
        }
        .try_into_model()?;
        markmap.insert(model.id.to_string(), model.id.into());
    }

    Ok(markmap)
}

fn try_get_chart(chart_ids: &ChartIDMap, key: &str) -> MigrationResult<ChartID> {
    chart_ids
        .get(key)
        .ok_or(MigrationError::ChartNotFound(key.into()))
        .cloned()
}

async fn seed_pages(chart_id: ChartID) -> MigrationResult<Vec<Mark>> {
    Ok(vec![
        // Sample Page 1: Welcome Page
        Mark::builder()
            .with_landmark(
                Landmark::builder()
                    .with_chart(chart_id.clone())
                    .with_title("Welcome to Waypoints".to_string())
                    .build(),
            )
            .build(),
        // Sample Page 2: Getting Started
        Mark::builder()
            .with_landmark(
                Landmark::builder()
                    .with_chart(chart_id.clone())
                    .with_title("Getting Started".to_string())
                    .build(),
            )
            .build(),
        // Sample Page 3: Example Project
        Mark::builder()
            .with_landmark(
                Landmark::builder()
                    .with_chart(chart_id)
                    .with_title("Example Project".to_string())
                    .build(),
            )
            .build(),
    ])
}

async fn seed_content_marks() -> MigrationResult<Vec<Mark>> {
    Ok(vec![
        // Basic paragraph content
        Mark::builder()
            .with_content(Content::Paragraph {
                rich_text: RichText::from("This is a sample paragraph in Waypoints.".to_string()),
            })
            .build(),
        // Heading content
        Mark::builder()
            .with_content(Content::Heading1 {
                rich_text: RichText::from("Sample Heading".to_string()),
            })
            .build(),
        // List item content
        Mark::builder()
            .with_content(Content::ListItem {
                rich_text: RichText::from("Sample list item".to_string()),
            })
            .build(),
    ])
}
