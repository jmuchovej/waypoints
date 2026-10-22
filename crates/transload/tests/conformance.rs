//! Properties over generated stores. Each store comes from a seed, so a failure
//! names a reproducible case; `TRANSLOAD_SEEDS` raises the count for a deep run.

mod common;

use bedrock::{Database, Query};
use common::*;
use domain::{
    BlockTheme, Chart, ChartID, Color, Content, Feature, FeatureID, FeatureKind, GlyphID,
    InlineMark, Landmark, Mark, MarkData, MarkID, MathEngine, Reading, ReferenceDisplayMode,
    RichText, SelectGroup, SelectOption, Size, Terrain, TerrainKind, TextMarkKind, TextSegment,
    syntax::inline,
};
use indexmap::IndexMap;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use std::collections::{BTreeMap, HashMap};
use transload::{Bundle, export, ingest, layout};

/// Generated stores per property; `TRANSLOAD_SEEDS` raises it for a deep run.
fn seeds() -> std::ops::Range<u64> {
    let count = std::env::var("TRANSLOAD_SEEDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(64);
    0..count
}

const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'X', ' ', '*', '_', '[', ']', '$', '^', '~', '=', '|', '{', '}', '\\', '`', '#',
    '>', '-', '+', ':', '<', '!', '(', ')', '.', '"', '\'',
];

const STYLES: [TextMarkKind; 10] = [
    TextMarkKind::Bold,
    TextMarkKind::Italic,
    TextMarkKind::Underline,
    TextMarkKind::Strikethrough,
    TextMarkKind::Superscript,
    TextMarkKind::Subscript,
    TextMarkKind::Code,
    TextMarkKind::Highlight,
    TextMarkKind::Spoiler,
    TextMarkKind::Math,
];

struct Gen {
    rng: StdRng,
    targets: Vec<MarkID>,
}

impl Gen {
    fn chance(&mut self, p: f64) -> bool {
        self.rng.random_bool(p)
    }

    fn pick<T: Clone>(&mut self, items: &[T]) -> T {
        items[self.rng.random_range(0..items.len())].clone()
    }

    fn text(&mut self) -> String {
        let length = self.rng.random_range(1..8);
        (0..length).map(|_| self.pick(ALPHABET)).collect()
    }

    fn rich(&mut self) -> RichText {
        let mut segments = Vec::new();
        for _ in 0..self.rng.random_range(1..4) {
            let mut marks = Vec::new();
            if self.chance(0.15) && !self.targets.is_empty() {
                let target = self.pick(&self.targets.clone());
                marks.push(mark(TextMarkKind::Mention, Some(target.to_string())));
            } else if self.chance(0.15) {
                marks.push(mark(
                    TextMarkKind::Link,
                    Some("https://waypoints.so/a b".into()),
                ));
            }
            for style in STYLES {
                if self.chance(0.12) {
                    let data = (style == TextMarkKind::Highlight && self.chance(0.5))
                        .then(|| "yellow".to_string());
                    marks.push(mark(style, data));
                }
            }
            for (kind, value) in [
                (TextMarkKind::Color, "blue"),
                (TextMarkKind::FontSize, "sm"),
                (TextMarkKind::FontFamily, "mono"),
            ] {
                if self.chance(0.08) {
                    marks.push(mark(kind, Some(value.into())));
                }
            }
            segments.push(TextSegment {
                text: self.text(),
                marks,
            });
        }
        RichText::new(inline::coalesce(&segments))
    }

    fn theme(&mut self) -> Option<BlockTheme> {
        self.chance(0.2).then(|| BlockTheme {
            color: self.pick(&[Color::Blue, Color::Red, Color::Default]),
        })
    }

    fn content(&mut self) -> Content {
        let theme = self.theme();
        match self.rng.random_range(0..13) {
            0 => Content::Heading1 {
                rich_text: self.rich(),
                theme,
            },
            1 => Content::Heading2 {
                rich_text: self.rich(),
                theme,
            },
            2 => Content::Heading3 {
                rich_text: self.rich(),
                theme,
            },
            3 => Content::Blockquote { theme },
            4 => Content::ListItem {
                rich_text: self.rich(),
                theme,
            },
            5 => Content::EnumItem {
                rich_text: self.rich(),
                theme,
            },
            6 => Content::Callout {
                icon: self.pick(&["warning", "info"]).into(),
                theme,
            },
            7 => Content::Code {
                language: self.pick(&["rust", ""]).into(),
                lines: (0..self.rng.random_range(0..4))
                    .map(|_| {
                        self.pick(&["fn main() {}", "", "```", "  <!-- not a mark -->", "::"])
                            .into()
                    })
                    .collect(),
                filename: self.chance(0.5).then(|| "main.rs".into()),
                highlights: self.chance(0.3).then(|| vec![1, 3]),
                theme,
            },
            8 => {
                let (rows, cols) = (self.rng.random_range(1..4), self.rng.random_range(1..4));
                let cells = (0..rows)
                    .map(|_| (0..cols).map(|_| self.rich()).collect())
                    .collect();
                let wrapped = self.chance(0.5);
                Content::Table {
                    cells,
                    col_size: wrapped.then(|| {
                        (0..cols)
                            .map(|i| (i % 2 == 0).then_some(Size::Fixed(120)))
                            .collect()
                    }),
                    row_size: None,
                    col_header: self.pick(&[Some(true), None, Some(false)]),
                    row_header: wrapped.then_some(true),
                    fill_width: None,
                    col_colors: wrapped.then(|| vec![Some(Color::Green); cols]),
                    row_colors: None,
                    theme,
                }
            }
            9 => Content::Math {
                expression: self.pick(&["x + y", "\\frac{1}{2}\n+ 3", ""]).into(),
                engine: self.pick(&[MathEngine::Typst, MathEngine::LaTeX]),
                theme,
            },
            10 => Content::Divider { theme },
            _ => Content::Paragraph {
                rich_text: self.rich(),
                theme,
            },
        }
    }

    fn at(&mut self) -> chrono::NaiveDateTime {
        at(self.rng.random_range(0..999))
    }

    fn extra(&mut self) -> Option<serde_json::Value> {
        self.chance(0.2).then(
            || serde_json::json!({ "so.waypoints.test": { "n": self.rng.random_range(0..9) } }),
        )
    }
}

fn mark(kind: TextMarkKind, data: Option<String>) -> InlineMark {
    InlineMark {
        kind,
        data: data.map(serde_json::Value::String),
    }
}

fn glyphs(keys: &[&str]) -> IndexMap<GlyphID, SelectOption> {
    keys.iter()
        .map(|key| {
            let option = SelectOption::builder().with_name(*key).build().keyed(key);
            (option.id.clone(), option)
        })
        .collect()
}

/// One custom Chart whose features span every `FeatureKind`.
fn wide_chart() -> Chart {
    let group = |key: &str, options: &[&str]| {
        SelectGroup::builder()
            .with_name(key)
            .with_options(glyphs(options).into_values().collect())
            .build()
            .keyed(key)
    };
    let groups: IndexMap<GlyphID, SelectGroup> = [group("g1", &["o1", "o2"]), group("g2", &["o3"])]
        .into_iter()
        .map(|group| (group.id.clone(), group))
        .collect();
    let kinds: Vec<(&str, FeatureKind)> = vec![
        ("text", FeatureKind::Text),
        (
            "number",
            FeatureKind::Number {
                min: Some(0.0),
                max: Some(100.0),
                step: None,
                precision: None,
                format: None,
            },
        ),
        ("checkbox", FeatureKind::Checkbox),
        (
            "date",
            FeatureKind::Date {
                min_date: None,
                min_time: None,
                max_date: None,
                max_time: None,
                timezone: None,
                granularity: Default::default(),
            },
        ),
        ("email", FeatureKind::Email),
        ("phone", FeatureKind::Phone),
        ("url", FeatureKind::URL),
        (
            "select",
            FeatureKind::Select {
                options: glyphs(&["s1", "s2", "s3"]),
                default_option: None,
            },
        ),
        (
            "multi",
            FeatureKind::MultiSelect {
                options: glyphs(&["m1", "m2", "m3"]),
                max_selections: None,
                default_options: vec![],
            },
        ),
        (
            "grouped",
            FeatureKind::GroupedSelect {
                groups: groups.clone(),
                default_option: None,
            },
        ),
        (
            "grouped-multi",
            FeatureKind::GroupedMultiSelect {
                groups,
                max_selections: None,
                default_options: IndexMap::new(),
            },
        ),
        (
            "refs",
            FeatureKind::Reference {
                allowed_types: None,
                max_references: None,
                display_field: "title".into(),
                display_mode: ReferenceDisplayMode::default(),
                filter_criteria: None,
            },
        ),
        ("formula", FeatureKind::Formula),
    ];
    Chart {
        id: ChartID("wide".into()),
        name: "Everything".into(),
        description: Some("every kind".into()),
        icon: None,
        features: kinds
            .into_iter()
            .map(|(key, kind)| {
                let feature = Feature {
                    id: FeatureID::derived(key),
                    name: key.into(),
                    kind,
                    description: None,
                    required: false,
                };
                (feature.id.clone(), feature)
            })
            .collect(),
        extra: Some(serde_json::json!({
            "so.waypoints.test": true,
            FeatureID::derived("text").to_string(): { "unit": "words" },
        })),
    }
}

fn reading(g: &mut Gen, kind: &FeatureKind) -> Reading {
    if g.chance(0.1) {
        return Reading::Null;
    }
    let ids =
        |options: &IndexMap<GlyphID, SelectOption>| options.keys().cloned().collect::<Vec<_>>();
    match kind {
        FeatureKind::Text => Reading::Text(g.rich()),
        FeatureKind::Number { .. } => Reading::Number(g.rng.random_range(0..400) as f64 / 4.0),
        FeatureKind::Checkbox => Reading::Bool(g.chance(0.5)),
        FeatureKind::Date { .. } if g.chance(0.5) => Reading::DateRange(g.at(), g.at()),
        FeatureKind::Date { .. } => Reading::Date(g.at()),
        FeatureKind::Email => Reading::Email("a@waypoints.so".into()),
        FeatureKind::Phone => Reading::Phone("+1".into(), g.text()),
        FeatureKind::URL => Reading::URL("https://waypoints.so/x".into()),
        FeatureKind::Formula => Reading::Formula(g.text()),
        FeatureKind::Select { options, .. } => Reading::Select(g.pick(&ids(options))),
        FeatureKind::MultiSelect { options, .. } => {
            Reading::MultiSelect(ids(options).into_iter().filter(|_| g.chance(0.5)).collect())
        }
        FeatureKind::GroupedSelect { groups, .. }
        | FeatureKind::GroupedMultiSelect { groups, .. } => Reading::GroupedSelect(
            groups
                .iter()
                .filter(|_| g.chance(0.6))
                .map(|(id, group)| (id.clone(), ids(&group.options)))
                .collect::<HashMap<_, _>>(),
        ),
        FeatureKind::Reference { .. } => {
            let targets = g.targets.clone();
            Reading::References(targets.into_iter().filter(|_| g.chance(0.3)).collect())
        }
    }
}

async fn generate(db: &Database, seed: u64) {
    let mut g = Gen {
        rng: StdRng::seed_from_u64(seed),
        targets: Vec::new(),
    };
    let chart = wide_chart();
    db.insert(chart.clone()).await.expect("chart");

    let roots: Vec<MarkID> = (0..g.rng.random_range(1..4))
        .map(|_| MarkID::new())
        .collect();
    g.targets.extend(&roots);

    let mut marks: Vec<Mark> = Vec::new();
    let mut edges: Vec<Terrain> = Vec::new();
    let mut content_ids: Vec<MarkID> = Vec::new();
    for root in &roots {
        let mut readings = IndexMap::new();
        for (id, feature) in &chart.features {
            if g.chance(0.6) {
                readings.insert(id.clone(), reading(&mut g, &feature.kind));
            }
        }
        let (created, updated) = (g.at(), g.at());
        marks.push(Mark {
            id: *root,
            data: MarkData::Landmark(Landmark {
                chart: chart.id.clone(),
                title: g.rich(),
                icon: None,
                readings,
            }),
            extra: g.extra(),
            created_at: created,
            updated_at: updated,
            deleted_at: None,
        });
        tree(&mut g, *root, 0, &mut marks, &mut edges, &mut content_ids);
    }

    if roots.len() > 1 {
        if let Some(shared) = content_ids.first() {
            edges.push(child(roots[1], *shared, "zz"));
        }
        edges.push(child(roots[0], roots[1], "zzz"));
    }
    let everything: Vec<MarkID> = roots.iter().chain(&content_ids).copied().collect();
    for kind in [
        TerrainKind::Reference,
        TerrainKind::Custom,
        TerrainKind::Reading(chart.id.clone(), FeatureID::derived("refs")),
    ] {
        let (src, dst) = (g.pick(&everything), g.pick(&everything));
        let mut edge = edge(src, dst, kind, None);
        edge.extra = g.extra();
        edges.push(edge);
    }

    for mark in marks {
        db.insert(mark).await.expect("mark");
    }
    for edge in edges {
        db.insert(edge).await.expect("edge");
    }
}

fn tree(
    g: &mut Gen,
    parent: MarkID,
    depth: usize,
    marks: &mut Vec<Mark>,
    edges: &mut Vec<Terrain>,
    content_ids: &mut Vec<MarkID>,
) {
    let width = if depth == 0 {
        g.rng.random_range(1..5)
    } else {
        g.rng.random_range(0..3)
    };
    for index in 0..width {
        let id = MarkID::new();
        let content = g.content();
        let (created, updated) = (g.at(), g.at());
        marks.push(Mark {
            id,
            data: MarkData::Content(content),
            extra: g.extra(),
            created_at: created,
            updated_at: updated,
            deleted_at: None,
        });
        let mut link = child(parent, id, &format!("{index:02}"));
        link.extra = g.extra();
        edges.push(link);
        content_ids.push(id);
        if depth < 2 {
            tree(g, id, depth + 1, marks, edges, content_ids);
        }
    }
}

async fn live_marks(db: &Database) -> Vec<Mark> {
    let mut marks = db.query::<Mark>(&Query::new()).await.expect("marks");
    marks.sort_by_key(|mark| mark.id);
    marks
}

/// Edges without their regenerated machinery (id and order key), with each
/// parent's children in order.
async fn structure(db: &Database) -> (Vec<String>, BTreeMap<MarkID, Vec<MarkID>>) {
    let edges = db.query::<Terrain>(&Query::new()).await.expect("edges");
    let mut flat: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{} {} {} {:?}",
                e.src_id,
                e.dst_id,
                e.kind.to_storage(),
                e.extra
            )
        })
        .collect();
    flat.sort();
    let mut children: BTreeMap<MarkID, Vec<(String, MarkID)>> = BTreeMap::new();
    for edge in edges.iter().filter(|e| e.kind == TerrainKind::Child) {
        children
            .entry(edge.src_id)
            .or_default()
            .push((edge.order.clone().unwrap_or_default(), edge.dst_id));
    }
    let ordered = children
        .into_iter()
        .map(|(parent, mut kids)| {
            kids.sort();
            (parent, kids.into_iter().map(|(_, id)| id).collect())
        })
        .collect();
    (flat, ordered)
}

/// A refusal with each located problem's source line beside it.
fn explain(bundle: &Bundle, error: transload::IngestError) -> String {
    let transload::IngestError::Refused(refusal) = error else {
        return error.to_string();
    };
    let mut out = String::new();
    for malformed in refusal
        .malformed
        .iter()
        .filter(|m| m.at.file.ends_with(".md"))
    {
        let context: String = malformed
            .at
            .line
            .and_then(|n| {
                let lines = bundle.get(&malformed.at.file)?.lines();
                Some(
                    lines
                        .skip(n - 1)
                        .take(8)
                        .map(|l| format!("\n    | {l}"))
                        .collect(),
                )
            })
            .unwrap_or_default();
        out.push_str(&format!("\n{}: {}{context}", malformed.at, malformed.kind));
    }
    out
}

async fn exported(db: &Database, seed: u64) -> Bundle {
    let exported = export(db, &meta()).await.expect("export");
    assert!(
        exported.warnings.is_empty(),
        "seed {seed}: {:?}",
        exported.warnings
    );
    exported.bundle
}

#[tokio::test]
async fn ingest_of_export_reproduces_every_generated_store() {
    for seed in seeds() {
        let (source, target) = (open().await, open().await);
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        if let Err(error) = ingest(&target, &bundle).await {
            panic!("seed {seed}: {}", explain(&bundle, error));
        }

        let (before, after) = (live_marks(&source).await, live_marks(&target).await);
        assert_eq!(before.len(), after.len(), "seed {seed}: Mark count");
        for (left, right) in before.iter().zip(&after) {
            assert_eq!(left, right, "seed {seed}: Mark {}", left.id);
        }
        assert_eq!(
            structure(&source).await,
            structure(&target).await,
            "seed {seed}: Terrain"
        );
        assert_eq!(
            target
                .get::<Chart>(&ChartID("wide".into()))
                .await
                .expect("get"),
            Some(wide_chart()),
            "seed {seed}: Chart"
        );
    }
}

#[tokio::test]
async fn export_is_byte_stable_and_a_fixed_point_of_ingest() {
    for seed in seeds() {
        let (source, target) = (open().await, open().await);
        generate(&source, seed).await;
        let first = exported(&source, seed).await;
        assert_eq!(first, exported(&source, seed).await, "seed {seed}: twice");

        ingest(&target, &first).await.expect("ingest");
        let again = exported(&target, seed).await;
        for (path, contents) in first.iter() {
            assert_eq!(again.get(path), Some(contents), "seed {seed}: {path}");
        }
        assert_eq!(first, again, "seed {seed}: export(ingest(export(s)))");
    }
}

/// Strip every comment and what remains is text: no `wp:` marker, and a UUID
/// only as a link target — and the text still parses as the dialect.
#[tokio::test]
async fn the_1980_test() {
    // A label may itself hold `[[`, so test what opens each UUID.
    let stray_uuid = |line: &str| {
        line.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
            .any(|token| {
                let at = token.as_ptr() as usize - line.as_ptr() as usize;
                token.len() == 36
                    && token.parse::<uuid::Uuid>().is_ok()
                    && !line[..at].ends_with("[[")
            })
    };
    for seed in seeds() {
        let source = open().await;
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        for (path, contents) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
            let stripped = strip_comments(contents);
            assert!(!stripped.contains("wp:"), "seed {seed} {path}:\n{stripped}");

            let body = stripped
                .splitn(3, "---\n")
                .nth(2)
                .expect("frontmatter fences");
            let mut fence: Option<String> = None;
            for line in body.lines() {
                assert!(
                    !stray_uuid(line),
                    "seed {seed} {path}: a UUID outside a link: {line}"
                );

                // Each line outside a fenced or tabular region is one block.
                let bare = line.trim_start_matches(['>', ' ']);
                if let Some(open) = &fence {
                    if bare == open {
                        fence = None;
                    }
                    continue;
                }
                if bare.starts_with("```") || bare.starts_with("$$") {
                    let marker: String = bare
                        .chars()
                        .take_while(|c| *c == '`' || *c == '$')
                        .collect();
                    fence = Some(if marker.starts_with('$') {
                        "$$".into()
                    } else {
                        marker
                    });
                    continue;
                }
                if !bare.is_empty() && !bare.starts_with(['|', ':']) {
                    domain::syntax::block::parse(bare)
                        .unwrap_or_else(|error| panic!("seed {seed} {path}: `{line}`: {error}"));
                }
            }
        }
    }
}

fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out.lines()
        .filter(|line| !line.trim().is_empty() || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn every_exported_schema_is_valid_draft_07() {
    for seed in seeds() {
        let source = open().await;
        generate(&source, seed).await;
        let bundle = exported(&source, seed).await;
        let schemas: Vec<_> = bundle
            .iter()
            .filter(|(path, _)| path.ends_with(layout::SCHEMA))
            .collect();
        assert!(!schemas.is_empty(), "seed {seed}");
        for (path, contents) in schemas {
            let document: serde_json::Value = serde_json::from_str(contents).expect("JSON");
            jsonschema::draft7::meta::validate(&document)
                .unwrap_or_else(|error| panic!("seed {seed} {path}: {error}"));
        }
    }
}
