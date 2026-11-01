use chrono::NaiveDateTime;
use domain::{Content, Icon, Json, RichText};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    key::{NoId, SourceKey, refuse_id},
    reading::ReadingFragment,
    timestamp,
};

/// A Landmark as an importer emits it: the data model's Landmark with a
/// source key for identity, a protochart key (or none) where a Bundle has a
/// `ChartID`, and readings keyed by the protochart's feature keys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LandmarkFragment {
    pub src: SourceKey,
    /// The protochart this Landmark's readings are read against; `None` is a
    /// plain page, which carries no readings and takes the host's fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protochart: Option<SourceKey>,
    pub title: RichText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub readings: IndexMap<SourceKey, ReadingFragment>,
    #[serde(default, with = "timestamp", skip_serializing_if = "Option::is_none")]
    pub created: Option<NaiveDateTime>,
    #[serde(default, with = "timestamp", skip_serializing_if = "Option::is_none")]
    pub updated: Option<NaiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Json>,
    /// The body: nesting is the child-edge carrier, as it is in a Bundle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<ContentFragment>,
    #[serde(
        default,
        rename = "id",
        deserialize_with = "refuse_id",
        skip_serializing
    )]
    pub(crate) no_id: NoId,
}

/// A Content Mark as an importer emits it: the data model's `Content`,
/// unchanged, under a source key, with its children nested beneath it.
///
/// A mention inside the content names its target by source key in the same
/// slot a Bundle fills with a `MarkID`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentFragment {
    pub src: SourceKey,
    pub content: Content,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ContentFragment>,
    #[serde(default, with = "timestamp", skip_serializing_if = "Option::is_none")]
    pub created: Option<NaiveDateTime>,
    #[serde(default, with = "timestamp", skip_serializing_if = "Option::is_none")]
    pub updated: Option<NaiveDateTime>,
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

impl LandmarkFragment {
    /// A Landmark with no protochart, no readings, and an empty body.
    pub fn new(src: SourceKey, title: impl Into<RichText>) -> Self {
        Self {
            src,
            protochart: None,
            title: title.into(),
            icon: None,
            readings: IndexMap::new(),
            created: None,
            updated: None,
            extra: None,
            body: Vec::new(),
            no_id: NoId,
        }
    }
}

impl ContentFragment {
    /// A leaf block.
    pub fn new(src: SourceKey, content: Content) -> Self {
        Self {
            src,
            content,
            children: Vec::new(),
            created: None,
            updated: None,
            extra: None,
            no_id: NoId,
        }
    }
}
