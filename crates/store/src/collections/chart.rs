use domain::{
    Icon,
    chart::{Chart, ChartID, Feature, FeatureID},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Newtype wrapper around `IndexMap<FeatureID, Feature>` so it can implement
/// `bedrock::Field` (i.e., be stored as a jsonb document column). `IndexMap` is
/// a foreign type, so the orphan rule prevents a direct impl — this newtype is
/// the canonical way to get there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bedrock::Document)]
#[serde(transparent)]
pub struct ChartFeatures(pub IndexMap<FeatureID, Feature>);

/// Bedrock-backed storage row for a [`Chart`].
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
pub struct ChartRow {
    #[bedrock(id)]
    pub id: ChartID,

    pub name: String,
    pub description: Option<String>,

    /// Typed icon, stored as jsonb. `Icon` derives `bedrock::Document`.
    pub icon: Option<Icon>,

    /// Feature map, stored as jsonb via the `ChartFeatures` newtype.
    pub features: ChartFeatures,
}

impl From<ChartRow> for Chart {
    fn from(row: ChartRow) -> Self {
        let mut builder = Chart::builder().with_name(row.name);

        if let Some(description) = row.description {
            builder = builder.with_description(description);
        }

        if let Some(icon) = row.icon {
            builder = builder.with_icon(icon);
        }

        for (_id, feature) in &row.features.0 {
            builder = builder.add_feature(feature.clone());
        }

        let chart = builder.build();
        Chart {
            id: row.id,
            ..chart
        }
    }
}

impl ChartRow {
    pub fn from_chart(chart: &Chart) -> Self {
        Self {
            id: chart.id.clone(),
            name: chart.name.clone(),
            description: chart.description.clone(),
            icon: chart.icon.clone(),
            features: ChartFeatures(chart.features.clone()),
        }
    }
}
