use chrono::{NaiveDate, NaiveTime};
use domain::{Color, DateGranularity, Icon, Json, ReferenceDisplayMode};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::key::{NoId, SourceKey, refuse_id};

/// The structure an importer discovered in its source — a Notion database's
/// columns, a Logseq page's properties — described in the data model's
/// feature vocabulary but keyed by source key throughout.
///
/// A protochart is what a person is asked to map onto a Chart at import; the
/// `name`, `description`, and `icon` are the labels they recognize it by.
/// A plugin never names a Chart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Protochart {
    pub src: SourceKey,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub features: IndexMap<SourceKey, FeatureFragment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Json>,
    #[serde(
        default,
        rename = "id",
        deserialize_with = "refuse_id",
        skip_serializing
    )]
    pub(crate) no_id: NoId,
}

/// A feature of a protochart: the data model's `Feature` minus its id, which
/// is the map key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureFragment {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    pub kind: FeatureKindFragment,
}

/// The data model's `FeatureKind` with a source key wherever it carries a
/// `GlyphID` or a `ChartID`; every other variant's payload is the same, and
/// the tagged serialization matches.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FeatureKindFragment {
    #[default]
    Text,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
        precision: Option<u8>,
        format: Option<String>,
    },
    Checkbox,
    Date {
        min_date: Option<NaiveDate>,
        min_time: Option<NaiveTime>,
        max_date: Option<NaiveDate>,
        max_time: Option<NaiveTime>,
        timezone: Option<String>,
        #[serde(default)]
        granularity: DateGranularity,
    },
    Email,
    Phone,
    #[serde(rename = "url")]
    URL,
    Select {
        options: IndexMap<SourceKey, SelectOptionFragment>,
        default_option: Option<SourceKey>,
    },
    MultiSelect {
        options: IndexMap<SourceKey, SelectOptionFragment>,
        max_selections: Option<u32>,
        #[serde(default)]
        default_options: Vec<SourceKey>,
    },
    GroupedSelect {
        groups: IndexMap<SourceKey, SelectGroupFragment>,
        default_option: Option<SourceKey>,
    },
    GroupedMultiSelect {
        groups: IndexMap<SourceKey, SelectGroupFragment>,
        max_selections: Option<u32>,
        default_options: IndexMap<SourceKey, SourceKey>,
    },
    /// `allowed_types` names protocharts, not Charts.
    Reference {
        allowed_types: Option<Vec<SourceKey>>,
        max_references: Option<u32>,
        #[serde(default = "default_display_field")]
        display_field: String,
        #[serde(rename = "as", default)]
        display_mode: ReferenceDisplayMode,
        filter_criteria: Option<Json>,
    },
    Formula,
}

fn default_display_field() -> String {
    "title".to_string()
}

/// The data model's `SelectOption` minus its id, which is the map key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectOptionFragment {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(default)]
    pub color: Color,
}

/// The data model's `SelectGroup` minus its id, with its options keyed by
/// source key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectGroupFragment {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub options: IndexMap<SourceKey, SelectOptionFragment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_options: Vec<SourceKey>,
}

impl Protochart {
    /// A protochart with no features.
    pub fn new(src: SourceKey, name: impl Into<String>) -> Self {
        Self {
            src,
            name: name.into(),
            description: None,
            icon: None,
            features: IndexMap::new(),
            extra: None,
            no_id: NoId,
        }
    }
}

impl FeatureKindFragment {
    /// Every option and group key this kind defines, for the finalize-time
    /// consistency check of readings against their feature.
    pub fn option_keys(&self) -> Vec<&SourceKey> {
        match self {
            FeatureKindFragment::Select { options, .. }
            | FeatureKindFragment::MultiSelect { options, .. } => options.keys().collect(),
            FeatureKindFragment::GroupedSelect { groups, .. }
            | FeatureKindFragment::GroupedMultiSelect { groups, .. } => groups
                .iter()
                .flat_map(|(group, def)| std::iter::once(group).chain(def.options.keys()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The protocharts a reference feature allows, when it restricts them.
    pub fn allowed_protocharts(&self) -> &[SourceKey] {
        match self {
            FeatureKindFragment::Reference {
                allowed_types: Some(keys),
                ..
            } => keys,
            _ => &[],
        }
    }
}
