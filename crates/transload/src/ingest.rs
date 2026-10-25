//! `ingest(bundle) → store`.
//!
//! Three phases: the manifest gate (read before any other file), staging
//! (parse and check everything, collecting every refusal), and one write
//! transaction. A refused Bundle never reaches the transaction, so the store is
//! untouched.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bedrock::Database;
use chrono::NaiveDateTime;
use domain::{
    Chart, ChartID, Content, FeatureID, FeatureKind, Landmark, Mark, MarkData, MarkID, Reading,
    RichText, Terrain, TerrainID, TerrainKind, TextMarkKind, fractional_index,
    syntax::{
        block,
        inline::{self, Links},
    },
};

use crate::{
    bundle::{BUNDLE_FORMAT, Bundle, Manifest, layout},
    export::{Tier, tier},
    frontmatter,
    identity::{self, Identity, Marker},
    names::{Lenient, Names},
    refusal::{
        ChartConflict, DuplicateConflict, IngestError, Location, Malformed, MalformedKind, Refusal,
    },
    schema,
    sidecar::{self, MarkLine, TerrainLine},
};

/// What an ingest wrote, and anything worth telling a person about it.
#[derive(Clone, Debug, Default)]
pub struct Ingested {
    pub marks: usize,
    pub terrains: usize,
    pub charts: usize,
    pub advisories: Vec<String>,
}

/// Ingest a Bundle into a store that does not already hold its Marks.
pub async fn ingest(db: &Database, bundle: &Bundle) -> Result<Ingested, IngestError> {
    let manifest = gate(bundle)?;
    let staged = Stager::new(db, bundle).run().await?;

    let mut advisories = Vec::new();
    if manifest.grammar_may_be_newer() {
        advisories.push(format!(
            "this Bundle was written by Waypoints {} (domain {}), newer than this build's \
             domain {}; it uses {:?}",
            manifest.app_version,
            manifest.domain_version,
            domain::VERSION,
            manifest.uses
        ));
    }

    let tx = db.begin_write().await?;
    for chart in &staged.charts {
        tx.upsert(chart.clone()).await?;
    }
    for mark in staged.marks.values() {
        tx.insert(mark.clone()).await?;
    }
    for edge in &staged.edges {
        tx.insert(edge.clone()).await?;
    }
    tx.commit().await?;

    Ok(Ingested {
        marks: staged.marks.len(),
        terrains: staged.edges.len(),
        charts: staged.charts.len(),
        advisories,
    })
}

fn gate(bundle: &Bundle) -> Result<Manifest, IngestError> {
    let refuse = |detail: String| {
        Refusal {
            malformed: vec![Malformed {
                at: Location::file(layout::MANIFEST),
                kind: MalformedKind::Grammar { detail },
            }],
            ..Refusal::default()
        }
        .into_result()
        .expect_err("a non-empty refusal")
    };
    let Some(text) = bundle.get(layout::MANIFEST) else {
        return Err(refuse("a Bundle has no manifest".into()));
    };
    let manifest: Manifest = serde_json::from_str(text)
        .map_err(|error| refuse(format!("the manifest is not valid: {error}")))?;
    if !manifest.format_supported() {
        return Err(IngestError::UnsupportedFormat {
            declared: manifest.bundle_format,
            supported: BUNDLE_FORMAT.into(),
        });
    }
    Ok(manifest)
}

struct Staged {
    marks: BTreeMap<MarkID, Mark>,
    edges: Vec<Terrain>,
    charts: Vec<Chart>,
}

/// One body node: a Mark, or (with no content) a nested Landmark's position.
struct Node {
    identity: Identity,
    content: Option<Content>,
    children: Vec<Node>,
    line: usize,
}

struct Stager<'a> {
    db: &'a Database,
    bundle: &'a Bundle,
    refusal: Refusal,
    /// Charts by directory name, as the Bundle describes them.
    charts: BTreeMap<String, Chart>,
    upserts: Vec<Chart>,
    landmarks: BTreeSet<MarkID>,
    marks: BTreeMap<MarkID, Mark>,
    /// The first file to carry each Content Mark, and what it said.
    seen: BTreeMap<MarkID, (String, (Content, Vec<MarkID>))>,
    /// Child edges in sibling order, so ordinals and order keys follow it.
    children: Vec<(MarkID, Vec<MarkID>)>,
    extra_edges: Vec<Terrain>,
    names: Names,
    /// Marks whose timestamps `marks.jsonl` supplied.
    stamped: BTreeSet<MarkID>,
}

impl<'a> Stager<'a> {
    fn new(db: &'a Database, bundle: &'a Bundle) -> Self {
        Self {
            db,
            bundle,
            refusal: Refusal::default(),
            charts: BTreeMap::new(),
            upserts: Vec::new(),
            landmarks: BTreeSet::new(),
            marks: BTreeMap::new(),
            seen: BTreeMap::new(),
            children: Vec::new(),
            extra_edges: Vec::new(),
            names: Names::default(),
            stamped: BTreeSet::new(),
        }
    }

    fn malformed(&mut self, at: Location, kind: MalformedKind) {
        self.refusal.malformed.push(Malformed { at, kind });
    }

    async fn run(mut self) -> Result<Staged, IngestError> {
        let mut landmark_files = Vec::new();
        for path in self.bundle.paths() {
            match classify(path) {
                Some(Kind::Landmark(dir, id)) => {
                    self.landmarks.insert(id);
                    landmark_files.push((path.to_string(), dir, id));
                }
                Some(Kind::Schema(_) | Kind::Sidecar) => {}
                None => self.malformed(
                    Location::file(path),
                    MalformedKind::UnknownContent {
                        detail: format!("`{path}` has no place in a Bundle"),
                    },
                ),
            }
        }

        for dir in self
            .bundle
            .chart_dirs()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        {
            self.chart(&dir).await?;
        }
        // Names need every title and block before any mention is read.
        let mut titles = BTreeMap::new();
        let mut files = BTreeMap::new();
        for (path, dir, id) in &landmark_files {
            if let Some((title, blocks)) = self.outline(path, dir) {
                titles.insert(*id, title);
                files.insert(*id, blocks);
            }
        }
        self.names = Names::new(&titles, &files);

        for (path, dir, id) in landmark_files {
            self.landmark(&path, &dir, id);
        }
        let mut edges = self.child_edges();
        self.mark_sidecar();
        let unstamped: Vec<MarkID> = self
            .marks
            .values()
            .filter(|mark| !mark.is_landmark() && !self.stamped.contains(&mark.id))
            .map(|mark| mark.id)
            .collect();
        for id in unstamped {
            self.malformed(
                Location::file(layout::MARKS),
                MalformedKind::Grammar {
                    detail: format!("Content Mark {id} has no `marks.jsonl` line"),
                },
            );
        }
        self.terrain_sidecar(&mut edges);
        self.fill_bare_mentions();

        self.refusal.into_result()?;
        edges.append(&mut self.extra_edges);
        Ok(Staged {
            marks: self.marks,
            edges,
            charts: self.upserts,
        })
    }

    async fn chart(&mut self, dir: &str) -> Result<(), IngestError> {
        let path = format!("{}/{dir}/{}", layout::LIBRARY, layout::SCHEMA);
        let at = Location::file(&path);
        let Some(text) = self.bundle.get(&path) else {
            self.malformed(
                at,
                MalformedKind::Grammar {
                    detail: format!("Chart directory `{dir}` has no schema"),
                },
            );
            return Ok(());
        };
        let document = match serde_json::from_str(text) {
            Ok(document) => document,
            Err(error) => {
                self.malformed(
                    at,
                    MalformedKind::Grammar {
                        detail: format!("not JSON: {error}"),
                    },
                );
                return Ok(());
            }
        };
        if let Err(kind) = schema::check_consistent(&document) {
            self.malformed(at, kind);
            return Ok(());
        }
        let chart = schema::read(&document).expect("consistent, so readable");
        if layout::chart_id(&chart.id) != dir {
            self.malformed(
                at,
                MalformedKind::Grammar {
                    detail: format!("the schema in `{dir}` describes Chart `{}`", chart.id.0),
                },
            );
            return Ok(());
        }

        if chart.is_builtin() {
            // A builtin resolves to the app's own, never overwritten.
            let ours = self.db.get::<Chart>(&chart.id).await?;
            let missing: Vec<FeatureID> = chart
                .features
                .keys()
                .filter(|id| {
                    ours.as_ref()
                        .map(|ours| !ours.features.contains_key(*id))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            if !missing.is_empty() {
                self.refusal.charts.push(ChartConflict {
                    chart: chart.id.clone(),
                    missing_features: missing,
                });
            }
        } else {
            self.upserts.push(chart.clone());
        }
        self.charts.insert(dir.to_string(), chart);
        Ok(())
    }

    fn landmark(&mut self, path: &str, dir: &str, id: MarkID) {
        let at = || Location::file(path);
        let Some(chart) = self.charts.get(dir).cloned() else {
            return; // its Chart already refused
        };
        let file = self.bundle.get(path).unwrap_or_default();
        let Some((front, _)) = frontmatter::split(file) else {
            self.malformed(
                at(),
                MalformedKind::Grammar {
                    detail: "a Landmark file must open with frontmatter".into(),
                },
            );
            return;
        };
        let front = match frontmatter::parse(front, &kinds(&chart), &self.names) {
            Ok(front) => front,
            Err(kind) => return self.malformed(at(), kind),
        };
        if layout::chart_id(&front.chart) != dir {
            return self.malformed(
                at(),
                MalformedKind::Grammar {
                    detail: format!("filed under `{dir}` but names Chart `{}`", front.chart.0),
                },
            );
        }
        if let Err(problems) =
            schema::validate_readings(&schema::emit(&chart), &schema::instance(&front.readings))
        {
            for (feature, detail) in problems {
                self.malformed(at(), MalformedKind::SchemaViolation { feature, detail });
            }
        }
        if self.marks.contains_key(&id) {
            self.refusal.duplicates.push(DuplicateConflict {
                id,
                files: vec![path.to_string()],
            });
            return;
        }
        self.marks.insert(
            id,
            Mark {
                id,
                data: MarkData::Landmark(Landmark {
                    chart: front.chart,
                    title: front.title,
                    icon: front.icon,
                    readings: front.readings,
                }),
                extra: None,
                created_at: front.created,
                updated_at: front.updated,
                deleted_at: None,
            },
        );

        match parse_region(&body_lines(file), &self.names) {
            Ok(nodes) => self.stage(id, nodes, path),
            Err((line, kind)) => self.malformed(Location::line(path, line), kind),
        }
    }

    /// A Landmark file's plain title and the Content Marks its body writes,
    /// read with every mention left unresolved. `None` when the file is
    /// malformed, which its full read reports.
    fn outline(&self, path: &str, dir: &str) -> Option<(String, Vec<MarkID>)> {
        let chart = self.charts.get(dir)?;
        let file = self.bundle.get(path)?;
        let (front, _) = frontmatter::split(file)?;
        let front = frontmatter::parse(front, &kinds(chart), &Lenient).ok()?;
        let nodes = parse_region(&body_lines(file), &Lenient).ok()?;

        fn collect(nodes: &[Node], landmarks: &BTreeSet<MarkID>, out: &mut Vec<MarkID>) {
            for node in nodes {
                if node.content.is_some() && !landmarks.contains(&node.identity.id) {
                    out.push(node.identity.id);
                }
                collect(&node.children, landmarks, out);
            }
        }
        let mut blocks = Vec::new();
        collect(&nodes, &self.landmarks, &mut blocks);
        Some((front.title.plain_text().to_string(), blocks))
    }

    fn stage(&mut self, parent: MarkID, nodes: Vec<Node>, path: &str) {
        self.children
            .push((parent, nodes.iter().map(|node| node.identity.id).collect()));
        for node in nodes {
            let id = node.identity.id;
            let Some(content) = node.content else {
                if !self.landmarks.contains(&id) {
                    self.malformed(
                        Location::line(path, node.line),
                        MalformedKind::Grammar {
                            detail: format!("names Landmark {id}, which the Bundle does not carry"),
                        },
                    );
                }
                continue;
            };
            let signature = (
                content.clone(),
                node.children
                    .iter()
                    .map(|child| child.identity.id)
                    .collect(),
            );
            if let Some((first, earlier)) = self.seen.get(&id) {
                // A shared Mark is byte-copied into every file; anything else is surgery.
                if *earlier != signature {
                    let files = vec![first.clone(), path.to_string()];
                    match self.refusal.duplicates.iter_mut().find(|dup| dup.id == id) {
                        Some(dup) => dup.files.push(path.to_string()),
                        None => self
                            .refusal
                            .duplicates
                            .push(DuplicateConflict { id, files }),
                    }
                }
                continue;
            }
            if self.landmarks.contains(&id) {
                self.refusal.duplicates.push(DuplicateConflict {
                    id,
                    files: vec![path.to_string()],
                });
                continue;
            }
            self.seen.insert(id, (path.to_string(), signature));
            self.marks.insert(
                id,
                Mark {
                    id,
                    data: MarkData::Content(content),
                    extra: None,
                    // `marks.jsonl` supplies these, or the Mark is refused.
                    created_at: NaiveDateTime::default(),
                    updated_at: NaiveDateTime::default(),
                    deleted_at: None,
                },
            );
            self.stage(id, node.children, path);
        }
    }

    fn child_edges(&self) -> Vec<Terrain> {
        let mut edges = Vec::new();
        for (parent, kids) in &self.children {
            let keys = fractional_index::generate_n(kids.len());
            for (child, key) in kids.iter().zip(keys) {
                edges.push(Terrain {
                    id: TerrainID::new(),
                    src_id: *parent,
                    dst_id: *child,
                    kind: TerrainKind::Child,
                    order: Some(key),
                    extra: None,
                    deleted_at: None,
                });
            }
        }
        edges
    }

    fn mark_sidecar(&mut self) {
        let Some(text) = self.bundle.get(layout::MARKS) else {
            return;
        };
        for (line, row) in sidecar::lines::<MarkLine>(text) {
            let at = Location::line(layout::MARKS, line);
            let row = match row {
                Ok(row) => row,
                Err(detail) => {
                    self.malformed(at, MalformedKind::Grammar { detail });
                    continue;
                }
            };
            match self.marks.get_mut(&row.id) {
                Some(mark) => {
                    mark.extra = row.extra;
                    mark.created_at = row.created;
                    mark.updated_at = row.updated;
                    self.stamped.insert(mark.id);
                }
                None => self.malformed(
                    at,
                    MalformedKind::Grammar {
                        detail: "a `marks.jsonl` line names no Mark in the Bundle".into(),
                    },
                ),
            }
        }
    }

    fn terrain_sidecar(&mut self, edges: &mut [Terrain]) {
        let Some(text) = self.bundle.get(layout::TERRAIN) else {
            return;
        };
        // (src, dst, ordinal) → the reconstructed child edge it annotates.
        let mut projected: BTreeMap<(MarkID, MarkID, u32), usize> = BTreeMap::new();
        let mut counts: BTreeMap<(MarkID, MarkID), u32> = BTreeMap::new();
        for (index, edge) in edges.iter().enumerate() {
            let count = counts.entry((edge.src_id, edge.dst_id)).or_default();
            projected.insert((edge.src_id, edge.dst_id, *count), index);
            *count += 1;
        }

        for (line, row) in sidecar::lines::<TerrainLine>(text) {
            let at = Location::line(layout::TERRAIN, line);
            let grammar = |detail: String| MalformedKind::Grammar { detail };
            let TerrainLine {
                src,
                dst,
                kind: raw_kind,
                order,
                extra,
            } = match row {
                Ok(row) => row,
                Err(detail) => {
                    self.malformed(at, grammar(detail));
                    continue;
                }
            };
            let Some(kind) = strict_kind(&raw_kind) else {
                self.malformed(at, grammar(format!("`{raw_kind}` is not an edge kind")));
                continue;
            };
            if !self.marks.contains_key(&src) || !self.marks.contains_key(&dst) {
                self.malformed(
                    at,
                    grammar("a terrain line names a Mark the Bundle does not carry".into()),
                );
                continue;
            }
            match tier(&kind) {
                Tier::Structure => match projected.get(&(src, dst, order.unwrap_or(0))) {
                    Some(&slot) => edges[slot].extra = extra,
                    None => self.malformed(
                        at,
                        MalformedKind::OrphanAnnotativeEdge {
                            src,
                            dst,
                            kind: raw_kind,
                            order,
                        },
                    ),
                },
                Tier::Constitutive => self.extra_edges.push(Terrain {
                    id: TerrainID::new(),
                    src_id: src,
                    dst_id: dst,
                    kind,
                    order: None,
                    extra,
                    deleted_at: None,
                }),
            }
        }
    }

    /// A bare `[[id]]` has no anchor text; it gets its target's title.
    fn fill_bare_mentions(&mut self) {
        let titles: BTreeMap<MarkID, String> = self
            .marks
            .values()
            .filter_map(|mark| match &mark.data {
                MarkData::Landmark(data) => Some((mark.id, data.title.plain_text().to_string())),
                MarkData::Content(_) => None,
            })
            .collect();
        for mark in self.marks.values_mut() {
            match &mut mark.data {
                MarkData::Landmark(data) => {
                    fill(&mut data.title, &titles);
                    for reading in data.readings.values_mut() {
                        if let Reading::Text(rich) = reading {
                            fill(rich, &titles);
                        }
                    }
                }
                MarkData::Content(content) => {
                    if let Content::Table { cells, .. } = content {
                        for cell in cells.iter_mut().flatten() {
                            fill(cell, &titles);
                        }
                    } else if let Some(rich) = rich_text_mut(content) {
                        fill(rich, &titles);
                    }
                }
            }
        }
    }
}

fn kinds(chart: &Chart) -> HashMap<FeatureID, FeatureKind> {
    chart
        .features
        .iter()
        .map(|(id, feature)| (id.clone(), feature.kind.clone()))
        .collect()
}

/// A Landmark file's lines after its frontmatter, numbered from 1.
fn body_lines(file: &str) -> Vec<(usize, String)> {
    let first_body_line = file
        .lines()
        .enumerate()
        .filter(|(_, line)| *line == "---")
        .nth(1)
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    file.lines()
        .enumerate()
        .skip(first_body_line)
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect()
}

fn rich_text_mut(content: &mut Content) -> Option<&mut RichText> {
    match content {
        Content::Paragraph { rich_text, .. }
        | Content::Heading1 { rich_text, .. }
        | Content::Heading2 { rich_text, .. }
        | Content::Heading3 { rich_text, .. }
        | Content::ListItem { rich_text, .. }
        | Content::EnumItem { rich_text, .. } => Some(rich_text),
        _ => None,
    }
}

fn fill(rich: &mut RichText, titles: &BTreeMap<MarkID, String>) {
    let mut changed = false;
    let mut segments = rich.segments().to_vec();
    for segment in &mut segments {
        if !segment.text.is_empty() {
            continue;
        }
        let target = segment
            .marks
            .iter()
            .find(|mark| mark.kind == TextMarkKind::Mention)
            .and_then(|mark| mark.data.as_ref()?.as_str()?.parse::<uuid::Uuid>().ok());
        if let Some(title) = target.and_then(|id| titles.get(&MarkID(id))) {
            segment.text = title.clone();
            changed = true;
        }
    }
    if changed {
        *rich = RichText::new(segments);
    }
}

/// Parse a `kind` exactly — never the lossy storage fallback, which degrades
/// anything unknown to `custom`.
fn strict_kind(raw: &str) -> Option<TerrainKind> {
    let kind = match raw {
        "child" => TerrainKind::Child,
        "reference" => TerrainKind::Reference,
        "custom" => TerrainKind::Custom,
        other => {
            let (chart, feature) = other.strip_prefix("reading::")?.split_once("::")?;
            TerrainKind::Reading(
                chart.parse::<ChartID>().ok()?,
                feature.parse::<FeatureID>().ok()?,
            )
        }
    };
    (kind.to_storage() == raw).then_some(kind)
}

enum Kind {
    Landmark(String, MarkID),
    Schema(String),
    Sidecar,
}

fn classify(path: &str) -> Option<Kind> {
    if [layout::MANIFEST, layout::TERRAIN, layout::MARKS].contains(&path) {
        return Some(Kind::Sidecar);
    }
    let rest = path.strip_prefix(layout::LIBRARY)?.strip_prefix('/')?;
    let (dir, file) = rest.split_once('/')?;
    if file == layout::SCHEMA {
        return Some(Kind::Schema(dir.to_string()));
    }
    let id = file.strip_suffix(".md")?.parse::<uuid::Uuid>().ok()?;
    Some(Kind::Landmark(dir.to_string(), MarkID(id)))
}

type Region = [(usize, String)];
type Parsed<T> = Result<T, (usize, MalformedKind)>;

fn grammar(line: usize, detail: impl Into<String>) -> (usize, MalformedKind) {
    (
        line,
        MalformedKind::Grammar {
            detail: detail.into(),
        },
    )
}

/// Parse a sequence of sibling nodes.
fn parse_region(lines: &Region, links: &dyn Links) -> Parsed<Vec<Node>> {
    let mut nodes = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        if lines[at].1.trim().is_empty() {
            at += 1;
            continue;
        }
        let (node, next) = parse_node(lines, at, links)?;
        nodes.push(node);
        at = next;
    }
    Ok(nodes)
}

fn parse_node(lines: &Region, start: usize, links: &dyn Links) -> Parsed<(Node, usize)> {
    let (line_no, line) = &lines[start];
    let identity = match identity::parse(line).map_err(|kind| (*line_no, kind))? {
        Some(Marker::Open(identity)) => identity,
        _ => return Err(grammar(*line_no, "expected a `wp:mark` comment")),
    };
    let mut at = start + 1;
    let Some((first_no, first)) = lines.get(at) else {
        return Err(grammar(
            *line_no,
            "a `wp:mark` comment with no block after it",
        ));
    };
    let (first_no, first) = (*first_no, first.clone());

    // A nested Landmark: its own comment over a link to itself.
    if is_self_link(&first, identity.id, links) {
        let node = Node {
            identity,
            content: None,
            children: Vec::new(),
            line: *line_no,
        };
        return Ok((node, at + 1));
    }

    let take_until = |from: usize, end: &str| -> Parsed<usize> {
        (from + 1..lines.len())
            .find(|&index| lines[index].1 == end)
            .ok_or_else(|| grammar(first_no, format!("unclosed block, expected `{end}`")))
    };

    let mut children: Vec<(usize, String)> = Vec::new();
    let mut indented_children = true;
    let own: String;
    if first.starts_with("```") {
        let fence = &first[..first.len() - first.trim_start_matches('`').len()];
        let end = take_until(at, fence)?;
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if first.starts_with("$$") {
        let end = take_until(at, "$$")?;
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if let Some(name) = first.strip_prefix("::") {
        let end = matching_closer(lines, at)
            .ok_or_else(|| grammar(first_no, "unclosed `::` component"))?;
        let name = name.split('{').next().unwrap_or_default();
        if block::CONTAINER_COMPONENTS.contains(&name) {
            own = first.clone();
            children = lines[at + 1..end].to_vec();
            indented_children = false;
        } else {
            own = join(&lines[at..=end]);
        }
        at = end + 1;
    } else if first.starts_with(block::TABLE_ROW) {
        let end = (at..lines.len())
            .take_while(|&index| lines[index].1.starts_with(block::TABLE_ROW))
            .last()
            .unwrap_or(at);
        own = join(&lines[at..=end]);
        at = end + 1;
    } else if first.starts_with('>') {
        indented_children = false;
        if first.starts_with(">{") {
            own = first.clone();
            at += 1;
        } else {
            own = ">".into();
        }
        while let Some((number, line)) = lines.get(at) {
            let stripped = if line == ">" {
                Some("")
            } else {
                line.strip_prefix("> ")
            };
            let Some(stripped) = stripped else { break };
            children.push((*number, stripped.to_string()));
            at += 1;
        }
    } else {
        own = first.clone();
        at += 1;
    }

    if indented_children {
        let indent = " ".repeat(block::INDENT);
        while let Some((number, line)) = lines.get(at) {
            let continues = line.starts_with(&indent)
                || (line.is_empty()
                    && lines[at + 1..]
                        .iter()
                        .find(|(_, next)| !next.is_empty())
                        .map(|(_, next)| next.starts_with(&indent))
                        .unwrap_or(false));
            if !continues {
                break;
            }
            children.push((
                *number,
                line.strip_prefix(&indent).unwrap_or("").to_string(),
            ));
            at += 1;
        }
    }

    let content =
        block::parse_with(&own, links).map_err(|error| grammar(first_no, error.to_string()))?;
    let children = parse_region(&children, links)?;
    if !children.is_empty() {
        match lines.get(at) {
            Some((_, line)) if matches!(identity::parse(line), Ok(Some(Marker::Close))) => at += 1,
            _ => return Err(grammar(*line_no, "a container's comment is never closed")),
        }
    }
    Ok((
        Node {
            identity,
            content: Some(content),
            children,
            line: *line_no,
        },
        at,
    ))
}

fn join(lines: &Region) -> String {
    lines
        .iter()
        .map(|(_, line)| line.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `::` closing the component opened at `start`, counting nested ones and
/// skipping fenced code and math, whose lines are verbatim.
fn matching_closer(lines: &Region, start: usize) -> Option<usize> {
    let mut depth = 0;
    let mut fence: Option<&str> = None;
    for (index, (_, line)) in lines.iter().enumerate().skip(start) {
        if let Some(open) = fence {
            if line == open {
                fence = None;
            }
            continue;
        }
        if line.starts_with("```") {
            fence = Some(&line[..line.len() - line.trim_start_matches('`').len()]);
        } else if line.starts_with("$$") {
            fence = Some("$$");
        } else if line == "::" {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        } else if line.starts_with("::") {
            depth += 1;
        }
    }
    None
}

fn is_self_link(line: &str, id: MarkID, links: &dyn Links) -> bool {
    let Ok(rich) = inline::parse_with(line, links) else {
        return false;
    };
    match rich.segments() {
        [segment] => segment.marks.iter().any(|mark| {
            mark.kind == TextMarkKind::Mention
                && mark.data.as_ref().and_then(|data| data.as_str()) == Some(&id.to_string())
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kinds_parse_strictly() {
        assert_eq!(strict_kind("child"), Some(TerrainKind::Child));
        assert!(matches!(
            strict_kind("reading::builtin.meeting::a1b2"),
            Some(TerrainKind::Reading(_, _))
        ));
        for bad in [
            "",
            "children",
            "reading::c4d8",
            "reading::not a chart::a1b2",
        ] {
            assert_eq!(strict_kind(bad), None, "{bad}");
        }
    }

    #[test]
    fn paths_classify_by_the_layout() {
        assert!(matches!(classify("manifest.json"), Some(Kind::Sidecar)));
        assert!(matches!(
            classify("library/c4d8/_schema.json"),
            Some(Kind::Schema(_))
        ));
        assert!(matches!(
            classify("library/c4d8/00000000-0000-0000-0000-000000000001.md"),
            Some(Kind::Landmark(_, _))
        ));
        for unknown in [
            "assets/x.png",
            "beacons/a",
            "library/c4d8/notes.txt",
            "README.md",
        ] {
            assert!(classify(unknown).is_none(), "{unknown}");
        }
    }
}
