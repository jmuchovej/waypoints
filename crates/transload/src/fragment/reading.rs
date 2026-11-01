use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use domain::RichText;
use serde::{Deserialize, Serialize};

use super::key::SourceKey;

/// A Reading as an importer emits it: the data model's `Reading` with a
/// source key wherever it carries a `GlyphID` or a `MarkID`, and every other
/// variant identical. Serialized in the same tagged form.
///
/// Grouped selections are a sorted map rather than the data model's hash
/// map, so the serialized form is canonical by construction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum ReadingFragment {
    Text(RichText),
    Number(f64),
    Bool(bool),
    Date(NaiveDateTime),
    DateRange(NaiveDateTime, NaiveDateTime),
    Select(SourceKey),
    MultiSelect(Vec<SourceKey>),
    GroupedSelect(BTreeMap<SourceKey, Vec<SourceKey>>),
    References(Vec<SourceKey>),
    #[serde(rename = "url")]
    URL(String),
    Email(String),
    Phone(String, String),
    File(String),
    Formula(String),
    Null,
}

impl ReadingFragment {
    /// Every key this reading names in its protochart's feature — select
    /// options and groups — for the finalize-time consistency check.
    pub fn option_keys(&self) -> Vec<&SourceKey> {
        match self {
            ReadingFragment::Select(key) => vec![key],
            ReadingFragment::MultiSelect(keys) => keys.iter().collect(),
            ReadingFragment::GroupedSelect(groups) => groups
                .iter()
                .flat_map(|(group, options)| std::iter::once(group).chain(options.iter()))
                .collect(),
            _ => Vec::new(),
        }
    }
}
