//! The Bundle: its path layout, its manifest, and the in-memory form both
//! directions pass through.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDateTime;
use domain::{ChartID, MarkID};
use serde::{Deserialize, Serialize};

/// The Bundle format version this build writes and ingests. `1.0.0` is frozen
/// at app v1.0, so the exporter writes `0.x` until then.
pub const BUNDLE_FORMAT: &str = "0.1.0";

/// Path layout. Every name a Bundle may contain is spelled exactly once here.
pub mod layout {
    use domain::{ChartID, MarkID};

    /// The manifest, read before anything else.
    pub const MANIFEST: &str = "manifest.json";
    /// Terrain edges that the projection cannot carry, one JSON object per line.
    pub const TERRAIN: &str = "terrain.jsonl";
    /// Mark `extra`, one `{id, extra}` object per line.
    pub const MARKS: &str = "marks.jsonl";
    /// The Landmark tree, grouped by Chart.
    pub const LIBRARY: &str = "library";
    /// The per-Chart schema document, inside that Chart's directory.
    pub const SCHEMA: &str = "_schema.json";

    /// Directory names a Bundle reserves and never emits. An ingester treats
    /// either as unknown content rather than guessing at it.
    pub const RESERVED: [&str; 2] = ["assets", "beacons"];

    /// A Chart id as a Bundle spells it, `builtin.` prefix included. Not
    /// `Display`, which strips the prefix and would collapse `builtin.page`
    /// onto a custom Chart named `page`.
    pub fn chart_id(chart: &ChartID) -> &str {
        chart.0.as_str()
    }

    /// `library/<ChartID>/<MarkID>.md`
    pub fn landmark(chart: &ChartID, mark: &MarkID) -> String {
        format!("{LIBRARY}/{}/{mark}.md", chart_id(chart))
    }

    /// `library/<ChartID>/_schema.json`
    pub fn schema(chart: &ChartID) -> String {
        format!("{LIBRARY}/{}/{SCHEMA}", chart_id(chart))
    }

    /// `library/<ChartID>`
    pub fn chart_dir(chart: &ChartID) -> String {
        format!("{LIBRARY}/{}", chart_id(chart))
    }
}

/// What the caller knows and the exporter cannot: which application wrote this
/// Bundle, when, and from which territory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportMeta {
    /// The exporting application's version.
    pub app_version: String,
    /// When the export ran (naive UTC, millisecond-truncated, as the engine
    /// stores timestamps).
    pub exported_at: NaiveDateTime,
    /// The territory this store belongs to, when the caller knows it.
    pub territory: Option<String>,
}

/// `manifest.json` — the version gate, and a census of what the Bundle holds.
///
/// Per-file checksums are OPTIONAL in the format and unimplemented in 0.1.0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The Bundle format's own version. An ingester refuses what it does not
    /// support before reading any other file.
    pub bundle_format: String,
    /// The exporting application's version.
    pub app_version: String,
    /// When the export ran: ISO-8601 UTC with an explicit `Z` and millisecond
    /// precision, the same spelling frontmatter timestamps use.
    pub exported_at: String,
    /// What the Bundle holds, so a reader can tell at a glance.
    pub counts: Counts,
    /// The territory this store belonged to, when the exporter was told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub territory: Option<String>,
}

/// A census of the exported store. Computed, never passed in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// Landmark Marks — one file each.
    pub landmarks: u64,
    /// Content Marks, counted once even where shared across Landmarks.
    pub content: u64,
    /// Live Terrain edges, across all three tiers.
    pub terrains: u64,
    /// Charts with a `_schema.json`, builtins included.
    pub charts: u64,
}

/// A Bundle as content: every UTF-8 file keyed by its Bundle-relative path.
/// A `BTreeMap` keeps iteration order deterministic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bundle {
    files: BTreeMap<String, String>,
}

impl Bundle {
    /// An empty Bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Place a file, replacing any file already at that path.
    pub fn insert(&mut self, path: impl Into<String>, contents: impl Into<String>) {
        self.files.insert(path.into(), contents.into());
    }

    /// One file's contents.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    /// Every file, in path order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.files
            .iter()
            .map(|(path, contents)| (path.as_str(), contents.as_str()))
    }

    /// Every path, in order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// How many files the Bundle holds.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Does the Bundle hold no files?
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Every Chart directory under `library/`, in order — the Charts this
    /// Bundle describes.
    pub fn chart_dirs(&self) -> BTreeSet<&str> {
        let prefix = format!("{}/", layout::LIBRARY);
        self.files
            .keys()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter_map(|rest| rest.split('/').next())
            .collect()
    }
}

/// Render a naive UTC timestamp the one way a Bundle ever spells one:
/// ISO-8601 with an explicit `Z` and millisecond precision.
///
/// The engine's `timestamp` columns hold millisecond precision, so this is
/// lossless in both directions.
pub(crate) fn render_timestamp(at: NaiveDateTime) -> String {
    at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

impl Manifest {
    /// The manifest for an export: the caller's metadata, this build's format
    /// version, and a census the exporter counted.
    pub fn new(meta: &ExportMeta, counts: Counts) -> Self {
        Self {
            bundle_format: BUNDLE_FORMAT.to_string(),
            app_version: meta.app_version.clone(),
            exported_at: render_timestamp(meta.exported_at),
            counts,
            territory: meta.territory.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use uuid::Uuid;

    fn at() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 2)
            .unwrap()
            .and_hms_milli_opt(9, 41, 0, 0)
            .unwrap()
    }

    #[test]
    fn landmark_paths_group_by_chart() {
        let chart = ChartID("c4d8".into());
        let mark = MarkID(Uuid::from_u128(0x018f));
        assert_eq!(
            layout::landmark(&chart, &mark),
            format!("library/c4d8/{mark}.md")
        );
        assert_eq!(layout::schema(&chart), "library/c4d8/_schema.json");
    }

    #[test]
    fn builtin_charts_keep_their_dotted_name() {
        let chart = ChartID::builtin("page");
        assert_eq!(layout::chart_dir(&chart), "library/builtin.page");
    }

    #[test]
    fn timestamps_render_with_an_explicit_z_and_milliseconds() {
        assert_eq!(render_timestamp(at()), "2026-08-02T09:41:00.000Z");
    }

    #[test]
    fn chart_dirs_are_the_library_subdirectories() {
        let mut bundle = Bundle::new();
        bundle.insert(layout::MANIFEST, "{}");
        bundle.insert("library/c4d8/_schema.json", "{}");
        bundle.insert("library/c4d8/a.md", "");
        bundle.insert("library/builtin.page/_schema.json", "{}");

        let dirs: Vec<_> = bundle.chart_dirs().into_iter().collect();
        assert_eq!(dirs, ["builtin.page", "c4d8"]);
    }

    #[test]
    fn a_manifest_carries_the_pre_freeze_format() {
        let meta = ExportMeta {
            app_version: "0.3.1".into(),
            exported_at: at(),
            territory: None,
        };
        let manifest = Manifest::new(&meta, Counts::default());
        assert_eq!(manifest.bundle_format, "0.1.0");
        assert_ne!(manifest.bundle_format, "1.0.0", "1.0.0 is the covenant's");

        // Territory is absent, not null: it is optional while 0.x holds.
        let json = serde_json::to_string(&manifest).expect("serialize");
        assert!(!json.contains("territory"), "got {json}");
    }
}
