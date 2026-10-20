//! `export(store) → Bundle`.
//!
//! A freeze-frame of live knowledge: soft-deleted Marks and edges do not
//! appear, because deletion in a freeze-frame is absence and sync — not the
//! Bundle — is the carrier of deletion history.
//!
//! The exporter reads through `bedrock::Database` generics rather than through
//! `store`, so this crate stays below it and seeding can ingest a corpus
//! without a dependency cycle.
//!
//! **It trusts the store.** Whatever the database hands back is serialized
//! faithfully; `transload` knows nothing about how Charts and Readings stay in
//! step, and declines to second-guess it. The one check it does make is on its
//! own output — a Bundle whose Landmark readings do not satisfy the
//! `_schema.json` the same export generated is internally inconsistent, and
//! saying so where it originates beats a baffling ingest refusal months later.
//! That is a **warning, never a refusal**: the covenant's promise is that you
//! can always get your data out, so an export must not fail because the store
//! is in a state the exporter did not expect.

use std::collections::{BTreeMap, BTreeSet};

use bedrock::{Database, Filter, Query};
use domain::{
    Chart, ChartID, Content, Mark, MarkData, MarkID, Reading, Terrain, TerrainKind,
    syntax::block::{self, Children},
};

use crate::{
    bundle::{Bundle, Census, Counts, ExportMeta, Manifest, layout},
    frontmatter::{self, Advisory, Frontmatter},
    identity::{self, Identity},
    schema,
};

/// Which carrier an edge kind travels in.
///
/// Every edge rides exactly one tier, and the ingester rebuilds Terrain as the
/// union of all three — so a kind assigned to no tier would be silently
/// dropped, which is what [`tier`]'s exhaustiveness prevents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Structure. Document nesting carries it; nothing is written for it.
    Structure,
    /// Reading-bound. A frontmatter `readings.*[].ref` carries it.
    ReadingBound,
    /// Constitutive. A `terrain.jsonl` line *is* the edge.
    Constitutive,
}

/// No wildcard arm: a new `TerrainKind` must be assigned a tier to compile.
pub fn tier(kind: &TerrainKind) -> Tier {
    match kind {
        TerrainKind::Child => Tier::Structure,
        TerrainKind::Reading(_, _) => Tier::ReadingBound,
        TerrainKind::Reference => Tier::Constitutive,
        TerrainKind::Custom => Tier::Constitutive,
    }
}

/// A Bundle, plus anything the exporter noticed on the way out.
#[derive(Clone, Debug)]
pub struct Exported {
    /// The Bundle itself.
    pub bundle: Bundle,
    /// Advisories about the store, not failures. An export never refuses.
    pub warnings: Vec<String>,
}

/// Why an export could not be produced at all.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The store could not be read.
    #[error("reading the store: {0}")]
    Store(#[from] bedrock::BedrockError),
}

/// Export every live entity in a store as a [`Bundle`].
pub async fn export(db: &Database, meta: &ExportMeta) -> Result<Exported, ExportError> {
    let live = Live::load(db).await?;
    let mut bundle = Bundle::new();
    let mut warnings = Vec::new();
    let mut census = Census::default();

    // Charts first: a Landmark's body cannot be rendered without the Chart's
    // feature names for its advisory comments.
    let mut schemas: BTreeMap<String, Chart> = BTreeMap::new();
    for landmark in live.landmarks() {
        let MarkData::Landmark(data) = &landmark.data else {
            continue;
        };
        if let Some(chart) = live.charts.get(layout::chart_id(&data.chart)) {
            schemas.insert(layout::chart_id(&data.chart).to_string(), chart.clone());
        } else {
            warnings.push(format!(
                "Landmark {} names Chart `{}`, which the store does not hold; \
                 its schema is absent from the Bundle",
                landmark.id,
                layout::chart_id(&data.chart)
            ));
        }
    }
    for chart in schemas.values() {
        for feature in chart.features.values() {
            census.feature_kinds.insert(variant_name(&feature.kind));
        }
        let document = schema::emit(chart);
        bundle.insert(layout::schema(&chart.id), pretty(&document));
    }

    // Landmark files.
    let mut counts = Counts::default();
    for landmark in live.landmarks() {
        let MarkData::Landmark(data) = &landmark.data else {
            continue;
        };
        counts.landmarks += 1;

        let advisory = live.advisory_for(data.chart.clone());
        let front = Frontmatter {
            title: data.title.clone(),
            chart: data.chart.clone(),
            icon: data.icon.clone(),
            created: landmark.created_at,
            updated: landmark.updated_at,
            readings: data.readings.clone(),
        };
        for reading in data.readings.values() {
            census.readings.insert(variant_name(reading));
        }

        let mut file = frontmatter::emit(&front, &advisory);
        let mut body = String::new();
        live.render_children(landmark.id, &mut body, &mut census);
        if !body.is_empty() {
            file.push('\n');
            file.push_str(&body);
        }
        bundle.insert(layout::landmark(&data.chart, &landmark.id), file);

        // The self-check: does this Landmark satisfy the schema this same
        // export just generated for its Chart?
        if let Some(chart) = schemas.get(layout::chart_id(&data.chart)) {
            let document = schema::emit(chart);
            let readings = readings_as_json(&front);
            if let Err(problems) = schema::validate_readings(&document, &readings) {
                for (feature, detail) in problems {
                    warnings.push(format!(
                        "Landmark {} has a reading the Chart's own schema rejects \
                         (`{feature}`): {detail}. It is in the Bundle, but will not \
                         re-ingest — the store and its Chart disagree.",
                        landmark.id
                    ));
                }
            }
        }
    }
    counts.charts = schemas.len() as u64;
    counts.content = live.content.len() as u64;

    // Sidecars.
    let (terrain_lines, terrain_count) = live.terrain_lines(&mut census);
    counts.terrains = terrain_count;
    if !terrain_lines.is_empty() {
        bundle.insert(layout::TERRAIN, terrain_lines);
    }
    if let Some(marks) = live.mark_lines() {
        bundle.insert(layout::MARKS, marks);
    }

    bundle.insert(
        layout::MANIFEST,
        pretty(
            &serde_json::to_value(Manifest::new(meta, counts, census))
                .unwrap_or(serde_json::Value::Null),
        ),
    );

    Ok(Exported { bundle, warnings })
}

/// Pretty JSON with a trailing newline — a Bundle's files are text files.
fn pretty(value: &serde_json::Value) -> String {
    let mut out = serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string());
    out.push('\n');
    out
}

/// A serde tag name, for the census — the spelling the Bundle itself uses.
fn variant_name<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map
            .get("type")
            .and_then(|tag| tag.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| "unknown".to_string()),
        Ok(serde_json::Value::String(tag)) => tag,
        _ => "unknown".to_string(),
    }
}

/// A Landmark's readings as the JSON the Draft-07 face validates.
fn readings_as_json(front: &Frontmatter) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for (feature, reading) in &front.readings {
        out.insert(feature.to_string(), reading_instance(reading));
    }
    serde_json::Value::Object(out)
}

/// One reading as the schema expects to see it — mirroring what frontmatter
/// writes, not `Reading`'s own tagged serialization.
fn reading_instance(reading: &Reading) -> serde_json::Value {
    use serde_json::json;
    match reading {
        Reading::Null => json!(null),
        Reading::Text(rich) => json!(domain::syntax::inline::emit(rich)),
        Reading::Number(number) => json!(number),
        Reading::Bool(flag) => json!(flag),
        Reading::Date(at) => json!(crate::bundle::render_timestamp(*at)),
        Reading::DateRange(from, to) => json!({
            "from": crate::bundle::render_timestamp(*from),
            "to": crate::bundle::render_timestamp(*to),
        }),
        Reading::Select(glyph) => json!(glyph.to_string()),
        Reading::MultiSelect(glyphs) => {
            json!(glyphs.iter().map(ToString::to_string).collect::<Vec<_>>())
        }
        Reading::GroupedSelect(groups) => {
            let mut out = serde_json::Map::new();
            let mut sorted: Vec<_> = groups.iter().collect();
            sorted.sort_by_key(|(group, _)| group.to_string());
            for (group, members) in sorted {
                out.insert(
                    group.to_string(),
                    json!(members.iter().map(ToString::to_string).collect::<Vec<_>>()),
                );
            }
            serde_json::Value::Object(out)
        }
        Reading::References(targets) => json!(
            targets
                .iter()
                .map(|target| json!({ "ref": target.to_string() }))
                .collect::<Vec<_>>()
        ),
        Reading::URL(value)
        | Reading::Email(value)
        | Reading::File(value)
        | Reading::Formula(value) => json!(value),
        Reading::Phone(first, second) => json!([first, second]),
    }
}

/// Everything live in the store, indexed the ways the exporter walks it.
struct Live {
    marks: BTreeMap<MarkID, Mark>,
    content: BTreeSet<MarkID>,
    roots: Vec<MarkID>,
    /// Parent → children, already in order.
    children: BTreeMap<MarkID, Vec<MarkID>>,
    edges: Vec<Terrain>,
    charts: BTreeMap<String, Chart>,
}

impl Live {
    async fn load(db: &Database) -> Result<Self, ExportError> {
        let alive = Query::new().filter(Filter::missing("deleted_at"));
        let marks: Vec<Mark> = db.query::<Mark>(&alive).await?;
        let edges: Vec<Terrain> = db.query::<Terrain>(&alive).await?;
        let charts: Vec<Chart> = db.query::<Chart>(&Query::new()).await?;

        let mut by_id = BTreeMap::new();
        let mut content = BTreeSet::new();
        let mut roots = Vec::new();
        for mark in marks {
            if mark.is_landmark() {
                roots.push(mark.id);
            } else {
                content.insert(mark.id);
            }
            by_id.insert(mark.id, mark);
        }
        // Sorted, so file order and iteration order are the Bundle's, not the
        // engine's.
        roots.sort();

        // Child edges, ordered by their runtime fractional key. The key itself
        // never reaches a Bundle — only the order it implies.
        let mut children: BTreeMap<MarkID, Vec<(Option<String>, MarkID)>> = BTreeMap::new();
        for edge in &edges {
            if tier(&edge.kind) == Tier::Structure {
                children
                    .entry(edge.src_id)
                    .or_default()
                    .push((edge.order.clone(), edge.dst_id));
            }
        }
        let children = children
            .into_iter()
            .map(|(parent, mut kids)| {
                // `None` sorts last: an edge with no key has no claim to a place.
                kids.sort_by(|left, right| match (&left.0, &right.0) {
                    (Some(a), Some(b)) => a.cmp(b).then(left.1.cmp(&right.1)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => left.1.cmp(&right.1),
                });
                (parent, kids.into_iter().map(|(_, id)| id).collect())
            })
            .collect();

        Ok(Self {
            marks: by_id,
            content,
            roots,
            children,
            edges,
            charts: charts
                .into_iter()
                .map(|chart| (chart.id.0.clone(), chart))
                .collect(),
        })
    }

    fn landmarks(&self) -> impl Iterator<Item = &Mark> {
        self.roots.iter().filter_map(|id| self.marks.get(id))
    }

    /// Feature names and reference labels for one Chart — regenerated every
    /// export, never read back.
    fn advisory_for(&self, chart: ChartID) -> Advisory {
        let mut advisory = Advisory::default();
        if let Some(chart) = self.charts.get(&chart.0) {
            for (id, feature) in &chart.features {
                advisory
                    .feature_names
                    .insert(id.clone(), feature.name.clone());
            }
        }
        for (id, mark) in &self.marks {
            if let MarkData::Landmark(data) = &mark.data {
                advisory
                    .mark_titles
                    .insert(*id, data.title.plain_text().to_string());
            }
        }
        advisory
    }

    /// Render a Mark's children beneath it, recursively.
    ///
    /// A Mark shared by several Landmarks is rendered identically into each —
    /// the bytes come from the same code path with the same inputs, so
    /// divergence within one export is not possible.
    fn render_children(&self, parent: MarkID, out: &mut String, census: &mut Census) {
        let Some(kids) = self.children.get(&parent) else {
            return;
        };
        for child in kids {
            let Some(mark) = self.marks.get(child) else {
                continue;
            };
            let MarkData::Content(content) = &mark.data else {
                // A Landmark nested under another is its own file; the edge to
                // it is structure, and the child's file carries its content.
                continue;
            };
            self.render_block(mark.id, content, out, census);
        }
    }

    fn render_block(&self, id: MarkID, content: &Content, out: &mut String, census: &mut Census) {
        census.content.insert(variant_name(content));
        let rendered = block::emit(content);
        let has_children = self
            .children
            .get(&id)
            .map(|kids| !kids.is_empty())
            .unwrap_or(false);

        out.push_str(&identity::emit_open(&Identity::new(id)));
        out.push('\n');
        for line in &rendered.lines {
            out.push_str(line);
            out.push('\n');
        }

        if has_children {
            let mut inner = String::new();
            self.render_children(id, &mut inner, census);
            match rendered.children {
                Children::None => out.push_str(&inner),
                Children::Prefixed(prefix) => {
                    for line in inner.lines() {
                        out.push_str(prefix.trim_end());
                        if !line.is_empty() {
                            out.push(' ');
                            out.push_str(line);
                        }
                        out.push('\n');
                    }
                }
                Children::Indented(width) => {
                    for line in inner.lines() {
                        if !line.is_empty() {
                            out.push_str(&" ".repeat(width));
                            out.push_str(line);
                        }
                        out.push('\n');
                    }
                }
                Children::Closed(closer) => {
                    out.push_str(&inner);
                    out.push_str(closer);
                    out.push('\n');
                }
            }
            // Paired only when there are children to enclose: a closer with
            // nothing between it and its opener says nothing.
            out.push_str(&identity::emit_close());
            out.push('\n');
        } else if let Children::Closed(closer) = rendered.children {
            // A childless component still has to close its own syntax.
            out.push_str(closer);
            out.push('\n');
        }
    }

    /// `terrain.jsonl`, plus how many live edges the Bundle accounts for.
    ///
    /// Constitutive kinds get a line that *is* the edge. Structure and
    /// reading-bound kinds are already carried by the projection, so they get a
    /// line only to hang `extra` on — addressed by natural key, never by id.
    fn terrain_lines(&self, census: &mut Census) -> (String, u64) {
        let mut ordinals: BTreeMap<(MarkID, MarkID, String), u32> = BTreeMap::new();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        let mut counted = 0;

        // Sorted by natural key so the file is byte-stable and ordinals are
        // assigned the same way every time.
        let mut edges: Vec<&Terrain> = self.edges.iter().collect();
        edges.sort_by(|left, right| {
            (
                left.src_id,
                left.dst_id,
                left.kind.to_storage(),
                left.order.clone(),
            )
                .cmp(&(
                    right.src_id,
                    right.dst_id,
                    right.kind.to_storage(),
                    right.order.clone(),
                ))
        });

        for edge in edges {
            counted += 1;
            census.terrain_kinds.insert(variant_name(&edge.kind));
            let kind = edge.kind.to_storage();
            let slot = ordinals
                .entry((edge.src_id, edge.dst_id, kind.clone()))
                .or_insert(0);
            let ordinal = *slot;
            *slot += 1;

            let constitutive = tier(&edge.kind) == Tier::Constitutive;
            if !constitutive && edge.extra.is_none() {
                // Fully carried by the projection; nothing to say.
                continue;
            }

            let mut row = serde_json::Map::new();
            row.insert("src".into(), serde_json::json!(edge.src_id.to_string()));
            row.insert("dst".into(), serde_json::json!(edge.dst_id.to_string()));
            row.insert("kind".into(), serde_json::json!(kind));
            if ordinal > 0 || !constitutive {
                // An annotative line must always say which parallel edge it
                // annotates; a constitutive one only when it is not the first.
                row.insert("order".into(), serde_json::json!(ordinal));
            }
            if let Some(extra) = &edge.extra {
                row.insert("extra".into(), extra.clone());
            }
            rows.push(serde_json::Value::Object(row));
        }

        let mut out = String::new();
        for row in rows {
            out.push_str(&serde_json::to_string(&row).unwrap_or_default());
            out.push('\n');
        }
        (out, counted)
    }

    /// `marks.jsonl` — one `{id, extra}` line per extra-bearing Mark.
    fn mark_lines(&self) -> Option<String> {
        let mut out = String::new();
        for (id, mark) in &self.marks {
            let Some(extra) = &mark.extra else {
                continue;
            };
            let row = serde_json::json!({ "id": id.to_string(), "extra": extra });
            out.push_str(&serde_json::to_string(&row).unwrap_or_default());
            out.push('\n');
        }
        (!out.is_empty()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terrain_kind_has_a_tier() {
        // The exhaustive match in `tier` is the real guarantee; this pins the
        // assignments so a silent re-tiering shows up as a test failure.
        assert_eq!(tier(&TerrainKind::Child), Tier::Structure);
        assert_eq!(tier(&TerrainKind::Reference), Tier::Constitutive);
        assert_eq!(tier(&TerrainKind::Custom), Tier::Constitutive);
        assert_eq!(
            tier(&TerrainKind::Reading(
                ChartID::builtin("meeting"),
                domain::FeatureID(domain::Base36ID("a1b2".into()))
            )),
            Tier::ReadingBound
        );
    }

    #[test]
    fn variant_names_are_the_serde_tags() {
        assert_eq!(variant_name(&TerrainKind::Child), "child");
        assert_eq!(variant_name(&Content::Divider { theme: None }), "divider");
        assert_eq!(variant_name(&Reading::Null), "null");
    }
}
