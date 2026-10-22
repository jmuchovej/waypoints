//! `export(store) → Bundle`: a freeze-frame of live knowledge. Soft-deleted
//! Marks and edges are absent.
//!
//! Anything that will not survive re-ingest (an orphaned Mark, a cycle, a
//! reading its Chart rejects) is a warning, never a refusal.

use std::collections::{BTreeMap, BTreeSet};

use bedrock::{Database, Filter, Query};
use domain::{
    Chart, Content, InlineMark, Mark, MarkData, MarkID, RichText, Terrain, TerrainKind,
    TextMarkKind, TextSegment,
    syntax::{
        block::{self, Children},
        inline,
    },
};

use crate::{
    bundle::{Bundle, Census, Counts, ExportMeta, Manifest, layout},
    frontmatter::{self, Advisory, Frontmatter},
    identity::{self, Identity},
    schema,
};

/// The carrier an edge kind travels in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Document nesting carries it; a sidecar line only hangs `extra` on it.
    Structure,
    /// A `terrain.jsonl` line is the edge.
    Constitutive,
}

/// No wildcard arm: a new `TerrainKind` must be assigned a tier to compile.
/// `Reading` edges are constitutive because the app does not keep them in step
/// with reading refs.
pub fn tier(kind: &TerrainKind) -> Tier {
    match kind {
        TerrainKind::Child => Tier::Structure,
        TerrainKind::Reference | TerrainKind::Custom | TerrainKind::Reading(_, _) => {
            Tier::Constitutive
        }
    }
}

/// A Bundle, plus what the exporter noticed on the way out.
#[derive(Clone, Debug)]
pub struct Exported {
    pub bundle: Bundle,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("reading the store: {0}")]
    Store(#[from] bedrock::BedrockError),
}

pub async fn export(db: &Database, meta: &ExportMeta) -> Result<Exported, ExportError> {
    let live = Live::load(db).await?;
    let mut out = Renderer {
        live: &live,
        bundle: Bundle::new(),
        census: Census::default(),
        warnings: Vec::new(),
        reached: BTreeSet::new(),
        expanded: BTreeSet::new(),
        projected: BTreeMap::new(),
    };

    let mut charts: BTreeMap<String, &Chart> = BTreeMap::new();
    for root in live.landmarks() {
        let MarkData::Landmark(data) = &root.data else {
            continue;
        };
        let key = layout::chart_id(&data.chart);
        match live.charts.get(key) {
            Some(chart) => {
                charts.insert(key.to_string(), chart);
            }
            None => out.warnings.push(format!(
                "Landmark {} names Chart `{key}`, which the store does not hold",
                root.id
            )),
        }
    }
    for chart in charts.values() {
        for feature in chart.features.values() {
            out.census.feature_kinds.insert(variant_name(&feature.kind));
        }
        out.bundle
            .insert(layout::schema(&chart.id), pretty(&schema::emit(chart)));
    }

    for root in live.landmarks() {
        out.landmark(
            root,
            charts.get(layout::chart_id(&landmark_chart(root))).copied(),
        );
    }

    for id in &live.content {
        if !out.reached.contains(id) {
            out.warnings.push(format!(
                "Content Mark {id} is not reachable from any Landmark and is not in the Bundle"
            ));
        }
    }

    let (terrain, terrains) = out.terrain_lines();
    if !terrain.is_empty() {
        out.bundle.insert(layout::TERRAIN, terrain);
    }
    if let Some(marks) = live.mark_lines(&out.reached) {
        out.bundle.insert(layout::MARKS, marks);
    }

    let counts = Counts {
        landmarks: live.roots.len() as u64,
        content: out.reached.len() as u64,
        terrains,
        charts: charts.len() as u64,
    };
    let manifest = Manifest::new(meta, counts, out.census);
    out.bundle.insert(
        layout::MANIFEST,
        pretty(&serde_json::to_value(manifest).unwrap_or_default()),
    );

    Ok(Exported {
        bundle: out.bundle,
        warnings: out.warnings,
    })
}

fn landmark_chart(mark: &Mark) -> domain::ChartID {
    match &mark.data {
        MarkData::Landmark(data) => data.chart.clone(),
        MarkData::Content(_) => domain::ChartID(String::new()),
    }
}

fn pretty(value: &serde_json::Value) -> String {
    let mut out = serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into());
    out.push('\n');
    out
}

/// A value's serde tag — the spelling the Bundle itself uses.
pub(crate) fn variant_name<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map
            .get("type")
            .and_then(|tag| tag.as_str())
            .unwrap_or("unknown")
            .to_string(),
        Ok(serde_json::Value::String(tag)) => tag,
        _ => "unknown".into(),
    }
}

/// Everything live, indexed the ways the exporter walks it.
struct Live {
    marks: BTreeMap<MarkID, Mark>,
    content: BTreeSet<MarkID>,
    roots: Vec<MarkID>,
    /// Parent → children, in fractional-key order.
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
        roots.sort();

        // An edge to a soft-deleted Mark is dead weight, not live structure.
        let edges: Vec<Terrain> = edges
            .into_iter()
            .filter(|edge| by_id.contains_key(&edge.src_id) && by_id.contains_key(&edge.dst_id))
            .collect();

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

    fn title(&self, id: &MarkID) -> Option<String> {
        match &self.marks.get(id)?.data {
            MarkData::Landmark(data) => Some(data.title.plain_text().to_string()),
            MarkData::Content(_) => None,
        }
    }

    fn mark_lines(&self, reached: &BTreeSet<MarkID>) -> Option<String> {
        let mut out = String::new();
        for (id, mark) in &self.marks {
            let Some(extra) = &mark.extra else {
                continue;
            };
            if !mark.is_landmark() && !reached.contains(id) {
                continue;
            }
            let row = serde_json::json!({ "id": id.to_string(), "extra": extra });
            out.push_str(&serde_json::to_string(&row).unwrap_or_default());
            out.push('\n');
        }
        (!out.is_empty()).then_some(out)
    }
}

struct Renderer<'a> {
    live: &'a Live,
    bundle: Bundle,
    census: Census,
    warnings: Vec<String>,
    /// Content Marks that made it into some body.
    reached: BTreeSet<MarkID>,
    /// Marks whose child edges have been counted, so a shared Mark rendered
    /// into several files counts its edges once.
    expanded: BTreeSet<MarkID>,
    /// How many `(parent, child)` structure edges the bodies carry.
    projected: BTreeMap<(MarkID, MarkID), u32>,
}

impl Renderer<'_> {
    fn landmark(&mut self, root: &Mark, chart: Option<&Chart>) {
        let MarkData::Landmark(data) = &root.data else {
            return;
        };
        let mut advisory = Advisory::default();
        if let Some(chart) = chart {
            for (id, feature) in &chart.features {
                advisory
                    .feature_names
                    .insert(id.clone(), feature.name.clone());
            }
        }
        for target in self.live.roots.iter() {
            if let Some(title) = self.live.title(target) {
                advisory.mark_titles.insert(*target, title);
            }
        }
        for reading in data.readings.values() {
            self.census.readings.insert(variant_name(reading));
        }

        let front = Frontmatter {
            title: data.title.clone(),
            chart: data.chart.clone(),
            icon: data.icon.clone(),
            created: root.created_at,
            updated: root.updated_at,
            readings: data.readings.clone(),
        };
        let mut file = frontmatter::emit(&front, &advisory);
        let mut body = String::new();
        let mut path = vec![root.id];
        self.children(root.id, &mut path, &mut body);
        if !body.is_empty() {
            file.push('\n');
            file.push_str(&body);
        }
        self.bundle
            .insert(layout::landmark(&data.chart, &root.id), file);

        if let Some(chart) = chart
            && let Err(problems) =
                schema::validate_readings(&schema::emit(chart), &schema::instance(&data.readings))
        {
            for (feature, detail) in problems {
                self.warnings.push(format!(
                    "Landmark {}: reading `{feature}` fails its Chart's schema and will not \
                     re-ingest: {detail}",
                    root.id
                ));
            }
        }
    }

    fn children(&mut self, parent: MarkID, path: &mut Vec<MarkID>, out: &mut String) {
        let Some(kids) = self.live.children.get(&parent).cloned() else {
            return;
        };
        let first_expansion = self.expanded.insert(parent);
        for child in kids {
            if path.contains(&child) {
                self.warnings.push(format!(
                    "the edge {parent} → {child} closes a cycle and is not in the Bundle"
                ));
                continue;
            }
            let Some(mark) = self.live.marks.get(&child) else {
                continue;
            };
            if first_expansion {
                *self.projected.entry((parent, child)).or_default() += 1;
            }
            match &mark.data {
                // A nested Landmark is its own file; here it is only a position.
                MarkData::Landmark(data) => {
                    out.push_str(&identity::emit_open(&Identity::new(child)));
                    out.push('\n');
                    out.push_str(&mention_line(child, data.title.plain_text()));
                    out.push('\n');
                }
                MarkData::Content(content) => {
                    self.reached.insert(child);
                    path.push(child);
                    self.block(mark, content, path, out);
                    path.pop();
                }
            }
        }
    }

    fn block(&mut self, mark: &Mark, content: &Content, path: &mut Vec<MarkID>, out: &mut String) {
        self.census.content.insert(variant_name(content));
        let rendered = block::emit(content);
        let identity = Identity::stamped(mark.id, mark.created_at, mark.updated_at);
        out.push_str(&identity::emit_open(&identity));
        out.push('\n');
        for line in &rendered.lines {
            out.push_str(line);
            out.push('\n');
        }

        let mut inner = String::new();
        self.children(mark.id, path, &mut inner);
        if inner.is_empty() {
            // A block with no line of its own and nothing beneath it still
            // needs one line, or its comment would stand over nothing.
            if rendered.lines.is_empty() {
                out.push_str(&rendered.opener());
                out.push('\n');
            }
            if let Children::Closed(closer) = rendered.children {
                out.push_str(closer);
                out.push('\n');
            }
            return;
        }
        match rendered.children {
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
        out.push_str(&identity::emit_close());
        out.push('\n');
    }

    /// `terrain.jsonl`, sorted by natural key so ordinals and bytes are stable,
    /// and the number of live edges the Bundle accounts for.
    fn terrain_lines(&mut self) -> (String, u64) {
        let mut edges: Vec<&Terrain> = self.live.edges.iter().collect();
        edges.sort_by_key(|edge| {
            (
                edge.src_id,
                edge.dst_id,
                edge.kind.to_storage(),
                edge.order.clone(),
            )
        });

        let mut ordinals: BTreeMap<(MarkID, MarkID, String), u32> = BTreeMap::new();
        let mut out = String::new();
        let mut carried = 0;
        for edge in edges {
            let kind = edge.kind.to_storage();
            let ordinal = {
                let slot = ordinals
                    .entry((edge.src_id, edge.dst_id, kind.clone()))
                    .or_insert(0);
                *slot += 1;
                *slot - 1
            };
            let constitutive = tier(&edge.kind) == Tier::Constitutive;
            if !constitutive {
                let projected = self
                    .projected
                    .get(&(edge.src_id, edge.dst_id))
                    .copied()
                    .unwrap_or(0);
                if ordinal >= projected {
                    // Unreachable or cycle-breaking structure: already warned.
                    continue;
                }
            }
            carried += 1;
            self.census.terrain_kinds.insert(variant_name(&edge.kind));
            if !constitutive && edge.extra.is_none() {
                continue;
            }

            let mut row = serde_json::Map::new();
            row.insert("src".into(), edge.src_id.to_string().into());
            row.insert("dst".into(), edge.dst_id.to_string().into());
            row.insert("kind".into(), kind.into());
            if ordinal > 0 || !constitutive {
                row.insert("order".into(), ordinal.into());
            }
            if let Some(extra) = &edge.extra {
                row.insert("extra".into(), extra.clone());
            }
            out.push_str(&serde_json::Value::Object(row).to_string());
            out.push('\n');
        }
        (out, carried)
    }
}

/// `[[id|Title]]` — a nested Landmark's place in its parent's body.
fn mention_line(id: MarkID, title: &str) -> String {
    inline::emit(&RichText::new(vec![TextSegment {
        text: title.to_string(),
        marks: vec![InlineMark {
            kind: TextMarkKind::Mention,
            data: Some(serde_json::Value::String(id.to_string())),
        }],
    }]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_terrain_kind_has_a_tier() {
        assert_eq!(tier(&TerrainKind::Child), Tier::Structure);
        assert_eq!(tier(&TerrainKind::Reference), Tier::Constitutive);
        assert_eq!(tier(&TerrainKind::Custom), Tier::Constitutive);
        assert_eq!(
            tier(&TerrainKind::Reading(
                domain::ChartID::builtin("meeting"),
                domain::FeatureID::derived("attendees")
            )),
            Tier::Constitutive
        );
    }

    #[test]
    fn variant_names_are_the_serde_tags() {
        assert_eq!(variant_name(&TerrainKind::Child), "child");
        assert_eq!(variant_name(&Content::Divider { theme: None }), "divider");
        assert_eq!(variant_name(&domain::Reading::Null), "null");
    }
}
