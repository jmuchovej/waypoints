//! A Landmark's YAML frontmatter — the flattened root Mark.
//!
//! [`yaml`] is the subset of YAML it is written in, one `Node` tree both ways;
//! this module says what each entry means. YAML cannot tell a `Select` from a
//! one-element `MultiSelect`, so [`parse`] is given the `FeatureKind` per
//! `FeatureID`; Charts must be resolved first.

use std::collections::HashMap;

use chrono::NaiveDateTime;
use domain::{
    Base36ID, ChartID, FeatureID, FeatureKind, GlyphID, Icon, MarkID, Reading, RichText,
    syntax::inline::{self, Links},
};
use indexmap::IndexMap;

use crate::{bundle::render_timestamp, names::KeepLabels, refusal::MalformedKind};

#[cfg(test)]
pub(crate) mod legacy;
mod yaml;

use yaml::{Entry, Node};

/// The frontmatter fence.
const FENCE: &str = "---";

/// A Landmark's frontmatter: the root Mark, flattened.
#[derive(Clone, Debug, PartialEq)]
pub struct Frontmatter {
    /// The Landmark's title.
    pub title: RichText,
    /// The Chart it conforms to.
    pub chart: ChartID,
    /// Its icon, when it has one.
    pub icon: Option<Icon>,
    /// When the Mark was created.
    pub created: NaiveDateTime,
    /// When the Mark was last written.
    pub updated: NaiveDateTime,
    /// Readings, keyed by `FeatureID`, in Chart order.
    pub readings: IndexMap<FeatureID, Reading>,
}

/// Text the emitter writes as comments and labels, and the parser ignores.
#[derive(Clone, Debug, Default)]
pub struct Advisory {
    /// Human names for features, written as `# Attendees` after their key.
    pub feature_names: HashMap<FeatureID, String>,
    /// Human names for referenced Marks, written as each ref's `label`.
    pub mark_titles: HashMap<MarkID, String>,
}

// ── Emit ─────────────────────────────────────────────────────────────────────

/// Render frontmatter, fences included.
pub fn emit(front: &Frontmatter, advisory: &Advisory, links: &dyn Links) -> String {
    let title = inline::emit_with(&front.title, &KeepLabels(links));
    let mut entries = vec![
        Entry::new("title", Node::text(title)),
        Entry::new("chart", Node::id(crate::layout::chart_id(&front.chart))),
    ];
    if let Some(icon) = &front.icon {
        let icon = serde_json::to_value(icon).unwrap_or_default();
        entries.push(Entry::new("icon", Node::Json(icon)));
    }
    // `marks.jsonl` is normative for timestamps; these are for the reader.
    for (key, at) in [("created", front.created), ("updated", front.updated)] {
        entries.push(
            Entry::new(key, Node::Plain(render_timestamp(at))).noted(Some("advisory".into())),
        );
    }
    if !front.readings.is_empty() {
        let readings = front
            .readings
            .iter()
            .map(|(feature, reading)| {
                let name = advisory
                    .feature_names
                    .get(feature)
                    .map(|name| one_line(name));
                Entry::new(feature.to_string(), value(reading, advisory, links)).noted(name)
            })
            .collect();
        entries.push(Entry::new("readings", Node::Map(readings)));
    }
    format!("{FENCE}\n{}{FENCE}\n", yaml::write(&entries))
}

fn value(reading: &Reading, advisory: &Advisory, links: &dyn Links) -> Node {
    let ids = |ids: &[GlyphID]| Node::Seq(ids.iter().map(|id| Node::id(&id.to_string())).collect());
    match reading {
        Reading::Null => Node::Plain("null".into()),
        Reading::Text(rich) => Node::text(inline::emit_with(rich, links)),
        Reading::Number(number) => {
            Node::Plain(serde_json::to_string(number).unwrap_or_else(|_| "null".into()))
        }
        Reading::Bool(flag) => Node::Plain(flag.to_string()),
        Reading::Date(at) => Node::Plain(render_timestamp(*at)),
        Reading::DateRange(from, to) => Node::Map(vec![
            Entry::new("from", Node::Plain(render_timestamp(*from))),
            Entry::new("to", Node::Plain(render_timestamp(*to))),
        ]),
        Reading::Select(glyph) => Node::id(&glyph.to_string()),
        Reading::MultiSelect(glyphs) => ids(glyphs),
        Reading::GroupedSelect(groups) => {
            // A `HashMap`: sorted here, so bytes do not depend on the hash seed.
            let mut sorted: Vec<_> = groups.iter().collect();
            sorted.sort_by_key(|(group, _)| group.to_string());
            Node::Map(
                sorted
                    .into_iter()
                    .map(|(group, members)| Entry::new(group.to_string(), ids(members)))
                    .collect(),
            )
        }
        Reading::References(targets) => Node::Seq(
            targets
                .iter()
                .map(|target| {
                    let mut entries = vec![Entry::new("ref", Node::Plain(target.to_string()))];
                    if let Some(title) = advisory.mark_titles.get(target) {
                        entries.push(Entry::new("label", Node::Quoted(one_line(title))));
                    }
                    Node::Map(entries)
                })
                .collect(),
        ),
        Reading::URL(value)
        | Reading::Email(value)
        | Reading::File(value)
        | Reading::Formula(value) => Node::Quoted(value.clone()),
        // The two strings' meaning is undefined in `FeatureKind::Phone`, so they
        // are carried positionally.
        Reading::Phone(first, second) => Node::Flow(vec![
            Node::Quoted(first.clone()),
            Node::Quoted(second.clone()),
        ]),
    }
}

/// Advisory text is one line by construction: a newline would break the comment.
fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

// ── Parse ────────────────────────────────────────────────────────────────────

/// Split a Landmark file into its frontmatter source and its body.
///
/// `None` when the file does not open with a fence — a Landmark that does not
/// start with frontmatter is malformed, but saying so is the caller's job.
pub fn split(file: &str) -> Option<(&str, &str)> {
    let rest = file.strip_prefix(FENCE)?.strip_prefix('\n')?;
    let end = rest.find("\n---")?;
    let (front, after) = rest.split_at(end);
    let body = after
        .strip_prefix("\n---")
        .and_then(|rest| rest.strip_prefix('\n'))
        .unwrap_or("");
    Some((front, body))
}

/// A refusal of frontmatter: its 1-indexed line within the frontmatter source,
/// and what is wrong there.
pub type Refused = (usize, MalformedKind);

/// Read frontmatter, interpreting each reading as its feature's kind.
pub fn parse(
    source: &str,
    kinds: &HashMap<FeatureID, FeatureKind>,
    links: &dyn Links,
) -> Result<Frontmatter, Refused> {
    let line = |at: usize| source[..at.min(source.len())].matches('\n').count() + 1;
    let entries =
        yaml::read(source).map_err(|(at, detail)| (line(at), MalformedKind::Grammar { detail }))?;
    let get = |key: &str| entries.iter().find(|entry| entry.key == key);
    let at = |entry: &Entry| {
        let at = line(entry.at);
        move |detail: String| (at, MalformedKind::Grammar { detail })
    };
    let missing = |key: &str| (1, grammar(format!("frontmatter with no `{key}`")));

    let title = match get("title") {
        Some(entry) => inline::parse_with(
            scalar(&entry.value, "title").map_err(at(entry))?,
            &KeepLabels(links),
        )
        .map_err(|error| at(entry)(error.to_string()))?,
        None => RichText::default(),
    };
    let chart = match get("chart") {
        Some(entry) => ChartID(
            scalar(&entry.value, "chart")
                .map_err(at(entry))?
                .to_string(),
        ),
        None => return Err(missing("chart")),
    };
    let icon = match get("icon") {
        Some(entry) => match &entry.value {
            Node::Json(icon) => Some(
                serde_json::from_value(icon.clone())
                    .map_err(|error| at(entry)(format!("not a valid encoded value: {error}")))?,
            ),
            _ => return Err(at(entry)("an icon is one line of JSON".into())),
        },
        None => None,
    };
    let mut stamp = |key: &str| match get(key) {
        Some(entry) => scalar(&entry.value, key)
            .and_then(timestamp)
            .map_err(at(entry)),
        None => Err(missing(key)),
    };
    let created = stamp("created")?;
    let updated = stamp("updated")?;

    let mut readings = IndexMap::new();
    if let Some(Node::Map(entries)) = get("readings").map(|entry| &entry.value) {
        for entry in entries {
            let feature = FeatureID(Base36ID(entry.key.clone()));
            let Some(kind) = kinds.get(&feature) else {
                return Err(at(entry)(format!(
                    "reading `{}` names no feature of this Chart",
                    entry.key
                )));
            };
            let read = reading(&entry.value, kind, &entry.key, links).map_err(at(entry))?;
            readings.insert(feature, read);
        }
    }

    Ok(Frontmatter {
        title,
        chart,
        icon,
        created,
        updated,
        readings,
    })
}

fn grammar(detail: String) -> MalformedKind {
    MalformedKind::Grammar { detail }
}

fn scalar<'a>(node: &'a Node, what: &str) -> Result<&'a str, String> {
    node.scalar()
        .ok_or_else(|| format!("expected a scalar for `{what}`"))
}

fn items<'a>(node: &'a Node, what: &str) -> Result<&'a [Node], String> {
    node.items()
        .ok_or_else(|| format!("expected a list for {what}"))
}

fn timestamp(text: &str) -> Result<NaiveDateTime, String> {
    crate::bundle::parse_timestamp(text)
}

fn glyph(text: &str) -> GlyphID {
    GlyphID(Base36ID(text.to_string()))
}

fn glyphs(node: &Node, what: &str) -> Result<Vec<GlyphID>, String> {
    items(node, what)?
        .iter()
        .map(|item| scalar(item, what).map(glyph))
        .collect()
}

/// One value read as the kind its feature declares.
fn reading(
    node: &Node,
    kind: &FeatureKind,
    key: &str,
    links: &dyn Links,
) -> Result<Reading, String> {
    if matches!(node, Node::Plain(text) if matches!(text.as_str(), "null" | "Null" | "NULL" | "~"))
    {
        return Ok(Reading::Null);
    }
    let text = || scalar(node, &format!("reading `{key}`"));
    let what = format!("reading `{key}`");

    match kind {
        FeatureKind::Text => inline::parse_with(text()?, links)
            .map(Reading::Text)
            .map_err(|error| error.to_string()),
        FeatureKind::Number { .. } => {
            let text = text()?;
            text.trim()
                .parse()
                .map(Reading::Number)
                .map_err(|_| format!("reading `{key}` is not a number: {text}"))
        }
        FeatureKind::Checkbox => match text()?.trim() {
            "true" => Ok(Reading::Bool(true)),
            "false" => Ok(Reading::Bool(false)),
            other => Err(format!("reading `{key}` is not a boolean: {other}")),
        },
        FeatureKind::Date { .. } => match node {
            Node::Map(range) => {
                let end = |name: &str| {
                    range
                        .iter()
                        .find(|entry| entry.key == name)
                        .ok_or_else(|| format!("reading `{key}` is a date range with no `{name}`"))
                        .and_then(|entry| scalar(&entry.value, name).and_then(timestamp))
                };
                Ok(Reading::DateRange(end("from")?, end("to")?))
            }
            _ => Ok(Reading::Date(timestamp(text()?)?)),
        },
        FeatureKind::URL => Ok(Reading::URL(text()?.to_string())),
        FeatureKind::Email => Ok(Reading::Email(text()?.to_string())),
        FeatureKind::Formula => Ok(Reading::Formula(text()?.to_string())),
        FeatureKind::Phone => match items(node, &what)? {
            [first, second] => Ok(Reading::Phone(
                scalar(first, &what)?.to_string(),
                scalar(second, &what)?.to_string(),
            )),
            parts => Err(format!(
                "reading `{key}` is a phone number of {} parts",
                parts.len()
            )),
        },
        FeatureKind::Select { .. } => Ok(Reading::Select(glyph(text()?))),
        FeatureKind::MultiSelect { .. } => Ok(Reading::MultiSelect(glyphs(node, &what)?)),
        FeatureKind::GroupedSelect { .. } | FeatureKind::GroupedMultiSelect { .. } => {
            let Node::Map(groups) = node else {
                return Err(format!("reading `{key}` is not a mapping of groups"));
            };
            let mut out = HashMap::new();
            for group in groups {
                let what = format!("group in reading `{key}`");
                out.insert(glyph(&group.key), glyphs(&group.value, &what)?);
            }
            Ok(Reading::GroupedSelect(out))
        }
        FeatureKind::Reference { .. } => {
            let mut targets = Vec::new();
            for item in items(node, &what)? {
                // `label` is advisory and deliberately not read: a Bundle whose
                // labels were hand-edited must ingest identically to one with
                // them stripped.
                let text = match item {
                    Node::Map(entries) => entries
                        .iter()
                        .find(|entry| entry.key == "ref")
                        .ok_or_else(|| format!("a reference in `{key}` has no `ref`"))
                        .and_then(|entry| scalar(&entry.value, "ref"))?,
                    scalar => self::scalar(scalar, &what)?,
                };
                let id: uuid::Uuid = text
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{text}` is not a Mark id"))?;
                targets.push(MarkID(id));
            }
            Ok(Reading::References(targets))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::ReferenceDisplayMode;
    use domain::syntax::inline::Verbatim;

    fn parse(
        source: &str,
        kinds: &HashMap<FeatureID, FeatureKind>,
    ) -> Result<Frontmatter, MalformedKind> {
        super::parse(source, kinds, &Verbatim).map_err(|(_, kind)| kind)
    }

    /// The constraint-free shapes of the struct variants, so a fixture can name
    /// a kind without restating five fields it does not care about.
    fn number_kind() -> FeatureKind {
        FeatureKind::Number {
            min: None,
            max: None,
            step: None,
            precision: None,
            format: None,
        }
    }

    fn date_kind() -> FeatureKind {
        FeatureKind::Date {
            min_date: None,
            min_time: None,
            max_date: None,
            max_time: None,
            timezone: None,
            granularity: Default::default(),
        }
    }

    fn reference_kind() -> FeatureKind {
        FeatureKind::Reference {
            allowed_types: None,
            max_references: None,
            display_field: "title".to_string(),
            display_mode: ReferenceDisplayMode::default(),
            filter_criteria: None,
        }
    }

    fn select_kind() -> FeatureKind {
        FeatureKind::Select {
            options: IndexMap::new(),
            default_option: None,
        }
    }

    fn at(milli: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 2)
            .unwrap()
            .and_hms_milli_opt(9, 41, 0, milli)
            .unwrap()
    }

    fn feature(id: &str) -> FeatureID {
        FeatureID(Base36ID(id.to_string()))
    }

    fn glyph_id(id: &str) -> GlyphID {
        GlyphID(Base36ID(id.to_string()))
    }

    fn mark(n: u128) -> MarkID {
        MarkID(uuid::Uuid::from_u128(n))
    }

    fn base(readings: IndexMap<FeatureID, Reading>) -> Frontmatter {
        Frontmatter {
            title: RichText::from("Weekly sync".to_string()),
            chart: ChartID::builtin("meeting"),
            icon: None,
            created: at(0),
            updated: at(0),
            readings,
        }
    }

    /// Emit, parse with the given kinds, and demand the same frontmatter back.
    fn survives(front: Frontmatter, kinds: HashMap<FeatureID, FeatureKind>) {
        let source = emit(&front, &Advisory::default(), &Verbatim);
        let (inner, _) = split(&source).expect("fences");
        let parsed = parse(inner, &kinds).unwrap_or_else(|e| panic!("parse {source}: {e}"));
        assert_eq!(parsed, front, "via\n{source}");
    }

    #[test]
    fn the_shape_is_fenced_and_ordered() {
        let source = emit(&base(IndexMap::new()), &Advisory::default(), &Verbatim);
        assert_eq!(
            source,
            "---\ntitle: |-\n  Weekly sync\nchart: builtin.meeting\n\
             created: 2026-08-02T09:41:00.000Z # advisory\n\
             updated: 2026-08-02T09:41:00.000Z # advisory\n---\n"
        );
    }

    #[test]
    fn the_chart_keeps_its_builtin_prefix() {
        let source = emit(&base(IndexMap::new()), &Advisory::default(), &Verbatim);
        assert!(source.contains("chart: builtin.meeting"), "{source}");
    }

    #[test]
    fn a_feature_name_rides_only_as_a_comment() {
        let mut readings = IndexMap::new();
        readings.insert(feature("a1b2"), Reading::Bool(true));
        let advisory = Advisory {
            feature_names: HashMap::from([(feature("a1b2"), "Attendees".to_string())]),
            ..Advisory::default()
        };
        let source = emit(&base(readings), &advisory, &Verbatim);
        assert!(source.contains("# Attendees"), "{source}");
        assert!(source.contains("a1b2: true"), "{source}");
    }

    #[test]
    fn hand_mangled_advisory_content_changes_nothing() {
        let mut readings = IndexMap::new();
        readings.insert(feature("a1b2"), Reading::References(vec![mark(7)]));
        let kinds = HashMap::from([(feature("a1b2"), reference_kind())]);

        let with_advice = emit(
            &base(readings.clone()),
            &Advisory {
                feature_names: HashMap::from([(feature("a1b2"), "NONSENSE".to_string())]),
                mark_titles: HashMap::from([(mark(7), "also nonsense".to_string())]),
            },
            &Verbatim,
        );
        let without = emit(&base(readings), &Advisory::default(), &Verbatim);

        let parsed_with = parse(split(&with_advice).expect("fences").0, &kinds).expect("parse");
        let parsed_without = parse(split(&without).expect("fences").0, &kinds).expect("parse");
        assert_eq!(
            parsed_with, parsed_without,
            "advisory content is not normative"
        );
    }

    #[test]
    fn timestamps_keep_their_instant() {
        let mut front = base(IndexMap::new());
        front.updated = at(123);
        let source = emit(&front, &Advisory::default(), &Verbatim);
        assert!(
            source.contains("updated: 2026-08-02T09:41:00.123Z"),
            "{source}"
        );
        survives(front, HashMap::new());
    }

    #[test]
    fn grouped_selects_are_sorted_at_the_boundary() {
        let mut groups = std::collections::HashMap::new();
        groups.insert(glyph_id("zzzz"), vec![glyph_id("0001")]);
        groups.insert(glyph_id("aaaa"), vec![glyph_id("0002")]);
        let mut readings = IndexMap::new();
        readings.insert(feature("a1b2"), Reading::GroupedSelect(groups));

        let first = emit(&base(readings.clone()), &Advisory::default(), &Verbatim);
        let second = emit(&base(readings), &Advisory::default(), &Verbatim);
        assert_eq!(first, second, "a HashMap must not leak its iteration order");
        let aaaa = first.find("aaaa").expect("aaaa");
        let zzzz = first.find("zzzz").expect("zzzz");
        assert!(aaaa < zzzz, "sorted, not hashed:\n{first}");
    }

    #[test]
    fn every_reading_kind_survives() {
        let cases: Vec<(FeatureKind, Reading)> = vec![
            (
                FeatureKind::Text,
                Reading::Text(RichText::from("some *bold* text".to_string())),
            ),
            (number_kind(), Reading::Number(42.5)),
            (FeatureKind::Checkbox, Reading::Bool(true)),
            (FeatureKind::Checkbox, Reading::Bool(false)),
            (date_kind(), Reading::Date(at(0))),
            (date_kind(), Reading::DateRange(at(0), at(500))),
            (
                FeatureKind::URL,
                Reading::URL("https://waypoints.so".into()),
            ),
            (FeatureKind::Email, Reading::Email("a@b.co".into())),
            (
                FeatureKind::Phone,
                Reading::Phone("+1".into(), "555-0100".into()),
            ),
            (FeatureKind::Formula, Reading::Formula("1 + 1".into())),
            (
                reference_kind(),
                Reading::References(vec![mark(1), mark(2)]),
            ),
            (FeatureKind::Text, Reading::Null),
        ];

        for (kind, reading) in cases {
            let mut readings = IndexMap::new();
            readings.insert(feature("a1b2"), reading.clone());
            survives(
                base(readings),
                HashMap::from([(feature("a1b2"), kind.clone())]),
            );
        }
    }

    #[test]
    fn an_icon_survives_even_though_the_spec_forgot_it() {
        let mut front = base(IndexMap::new());
        front.icon = Some(Icon {
            identifier: "star".into(),
            alt_text: Some("a star".into()),
            colorable: None,
            metadata: None,
        });
        survives(front, HashMap::new());
    }

    #[test]
    fn a_title_carrying_dialect_syntax_survives() {
        let mut front = base(IndexMap::new());
        front.title = inline::parse("the *weekly* sync").expect("parse");
        survives(front, HashMap::new());
    }

    #[test]
    fn splitting_finds_the_body() {
        let file = "---\ntitle: |-\n  Hi\nchart: builtin.page\n\
                    created: 2026-08-02T09:41:00.000Z\n\
                    updated: 2026-08-02T09:41:00.000Z\n---\nthe body\n";
        let (front, body) = split(file).expect("fences");
        assert!(front.contains("chart: builtin.page"));
        assert_eq!(body, "the body\n");
    }

    #[test]
    fn a_file_with_no_frontmatter_does_not_split() {
        assert!(split("just a body\n").is_none());
        assert!(split("---\nunterminated\n").is_none());
    }

    #[test]
    fn malformed_frontmatter_is_refused() {
        let kinds = HashMap::new();
        for source in [
            "title: |-\n  Hi\n",     // no chart
            "chart: builtin.page\n", // no created
            "chart: builtin.page\ncreated: yesterday\nupdated: yesterday\n",
            "- not\n- a\n- mapping\n",
        ] {
            assert!(parse(source, &kinds).is_err(), "{source} should be refused");
        }
    }

    #[test]
    fn a_reading_naming_no_feature_is_refused() {
        let source = "title: |-\n  Hi\nchart: builtin.page\n\
                      created: 2026-08-02T09:41:00.000Z\n\
                      updated: 2026-08-02T09:41:00.000Z\nreadings:\n  zzzz: true\n";
        assert!(parse(source, &HashMap::new()).is_err());
    }

    #[test]
    fn a_reference_list_ignores_its_labels() {
        let kinds = HashMap::from([(feature("a1b2"), reference_kind())]);
        let with_label = format!(
            "title: |-\n  Hi\nchart: builtin.page\n\
             created: 2026-08-02T09:41:00.000Z\nupdated: 2026-08-02T09:41:00.000Z\n\
             readings:\n  a1b2:\n    - ref: {}\n      label: \"whatever\"\n",
            mark(7)
        );
        let parsed = parse(&with_label, &kinds).expect("parse");
        assert_eq!(
            parsed.readings[&feature("a1b2")],
            Reading::References(vec![mark(7)])
        );
    }

    #[test]
    fn emitting_twice_is_byte_identical() {
        let mut readings = IndexMap::new();
        readings.insert(
            feature("a1b2"),
            Reading::MultiSelect(vec![glyph_id("0001"), glyph_id("0002")]),
        );
        let front = base(readings);
        assert_eq!(
            emit(&front, &Advisory::default(), &Verbatim),
            emit(&front, &Advisory::default(), &Verbatim)
        );
    }

    fn parity_cases() -> Vec<(FeatureKind, Reading)> {
        let multi_kind = FeatureKind::MultiSelect {
            options: IndexMap::new(),
            max_selections: None,
            default_options: vec![],
        };
        let grouped_kind = FeatureKind::GroupedSelect {
            groups: IndexMap::new(),
            default_option: None,
        };
        let text = |text: &str| Reading::Text(inline::parse(text).expect("dialect"));
        let ids = |ids: &[&str]| ids.iter().map(|id| glyph_id(id)).collect::<Vec<_>>();
        vec![
            (FeatureKind::Text, text("some *bold* text")),
            (FeatureKind::Text, Reading::Text(RichText::default())),
            (FeatureKind::Text, text("line one\nline two")),
            (FeatureKind::Text, text(" opens with a space")),
            (FeatureKind::Text, text("true")),
            (FeatureKind::Text, Reading::Null),
            (number_kind(), Reading::Number(42.5)),
            (number_kind(), Reading::Number(-3.0)),
            (number_kind(), Reading::Number(0.0)),
            (FeatureKind::Checkbox, Reading::Bool(true)),
            (FeatureKind::Checkbox, Reading::Bool(false)),
            (date_kind(), Reading::Date(at(0))),
            (date_kind(), Reading::DateRange(at(0), at(500))),
            (select_kind(), Reading::Select(glyph_id("abcd"))),
            (select_kind(), Reading::Select(glyph_id("0001"))),
            (select_kind(), Reading::Select(glyph_id("12e3"))),
            (select_kind(), Reading::Select(glyph_id("null"))),
            (select_kind(), Reading::Select(glyph_id("true"))),
            (select_kind(), Reading::Select(glyph_id("false"))),
            (multi_kind.clone(), Reading::MultiSelect(Vec::new())),
            (
                multi_kind,
                Reading::MultiSelect(ids(&["0001", "true", "abcd"])),
            ),
            (
                grouped_kind.clone(),
                Reading::GroupedSelect(std::collections::HashMap::new()),
            ),
            (
                grouped_kind,
                Reading::GroupedSelect(std::collections::HashMap::from([
                    (glyph_id("g1"), ids(&["o1", "0002"])),
                    (glyph_id("0003"), Vec::new()),
                ])),
            ),
            (reference_kind(), Reading::References(Vec::new())),
            (
                reference_kind(),
                Reading::References(vec![mark(1), mark(2)]),
            ),
            (
                FeatureKind::URL,
                Reading::URL("https://waypoints.so".into()),
            ),
            (FeatureKind::URL, Reading::URL(String::new())),
            (FeatureKind::Email, Reading::Email("a@b.co".into())),
            (
                FeatureKind::Formula,
                Reading::Formula("a \"b\" \\ c\nd: # e".into()),
            ),
            (
                FeatureKind::Phone,
                Reading::Phone("+1".into(), "555-0100".into()),
            ),
            (
                FeatureKind::Phone,
                Reading::Phone(String::new(), String::new()),
            ),
        ]
    }

    fn noted() -> Advisory {
        Advisory {
            feature_names: HashMap::from([
                (feature("a1b2"), "Attendees\nand # guests".to_string()),
                (feature("0001"), "Retyped".to_string()),
            ]),
            mark_titles: HashMap::from([(mark(1), "A \"quoted\"\ntitle".to_string())]),
        }
    }

    /// Every frontmatter of the parity table, with the kinds to read it with.
    fn parity_fronts() -> Vec<(Frontmatter, HashMap<FeatureID, FeatureKind>)> {
        let mut out = Vec::new();
        for (kind, reading) in parity_cases() {
            for key in ["a1b2", "0001"] {
                let mut readings = IndexMap::new();
                readings.insert(feature(key), reading.clone());
                out.push((
                    base(readings),
                    HashMap::from([(feature(key), kind.clone())]),
                ));
            }
        }
        for title in ["", " leading", "two\nlines", "the *weekly* sync"] {
            let mut front = base(IndexMap::new());
            front.title = inline::parse(title).expect("dialect");
            front.icon = Some(Icon {
                identifier: "iconify:mdi:home".into(),
                alt_text: Some("a \"home\"".into()),
                colorable: Some(true),
                metadata: None,
            });
            out.push((front, HashMap::new()));
        }
        out
    }

    /// The emitter's branches as a checklist — every reading kind empty and
    /// not, every re-typed id, the icon, the `|2-` indicator, with and without
    /// advisory notes — each read back to the frontmatter it came from.
    #[test]
    fn every_frontmatter_the_emitter_writes_reads_back() {
        for advisory in [Advisory::default(), noted()] {
            for (front, kinds) in parity_fronts() {
                survives_with(front, &advisory, kinds);
            }
        }
    }

    fn survives_with(
        front: Frontmatter,
        advisory: &Advisory,
        kinds: HashMap<FeatureID, FeatureKind>,
    ) {
        let source = emit(&front, advisory, &Verbatim);
        let (inner, _) = split(&source).expect("fences");
        let parsed = parse(inner, &kinds).unwrap_or_else(|e| panic!("parse {source}: {e}"));
        assert_eq!(parsed, front, "via\n{source}");
    }

    /// A Feature exists only to carry a kind here; the schema module owns the
    /// rest of it.
    #[test]
    fn feature_kinds_come_from_the_chart_not_the_yaml() {
        // The same YAML scalar reads as a Select or as Text depending on kind,
        // which is exactly why `parse` is given the kinds.
        let source = "title: |-\n  Hi\nchart: builtin.page\n\
                      created: 2026-08-02T09:41:00.000Z\n\
                      updated: 2026-08-02T09:41:00.000Z\nreadings:\n  a1b2: abcd\n";
        let as_select =
            parse(source, &HashMap::from([(feature("a1b2"), select_kind())])).expect("as select");
        assert!(matches!(
            as_select.readings[&feature("a1b2")],
            Reading::Select(_)
        ));

        let as_text = parse(
            source,
            &HashMap::from([(feature("a1b2"), FeatureKind::Text)]),
        )
        .expect("as text");
        assert!(matches!(
            as_text.readings[&feature("a1b2")],
            Reading::Text(_)
        ));
    }

    /// The committed Bundle's frontmatters, each with its Chart's kinds.
    fn committed() -> Vec<(String, HashMap<FeatureID, FeatureKind>)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../store/src/collections/getting-started");
        let bundle = crate::Bundle::read_dir(&root).expect("the committed Bundle");
        let mut out = Vec::new();
        for (path, file) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
            let dir = path.rsplit_once('/').expect("filed").0;
            let schema = bundle
                .get(&format!("{dir}/{}", crate::layout::SCHEMA))
                .expect("schema");
            let chart =
                crate::schema::read(&serde_json::from_str(schema).expect("JSON")).expect("schema");
            let kinds = chart
                .features
                .iter()
                .map(|(id, feature)| (id.clone(), feature.kind.clone()))
                .collect();
            out.push((split(file).expect("fences").0.to_string(), kinds));
        }
        assert!(!out.is_empty());
        out
    }

    #[test]
    fn emit_writes_the_bytes_the_hand_emitter_did() {
        for advisory in [Advisory::default(), noted()] {
            for (front, _) in parity_fronts() {
                assert_eq!(
                    emit(&front, &advisory, &Verbatim),
                    legacy::emit(&front, &advisory, &Verbatim)
                );
            }
        }
        for (source, kinds) in committed() {
            let front = parse(&source, &kinds).expect("the committed Bundle parses");
            assert_eq!(
                emit(&front, &noted(), &Verbatim),
                legacy::emit(&front, &noted(), &Verbatim)
            );
        }
    }

    #[test]
    fn the_subset_reads_what_saphyr_read() {
        let mut sources = committed();
        for advisory in [Advisory::default(), noted()] {
            for (front, kinds) in parity_fronts() {
                let source = emit(&front, &advisory, &Verbatim);
                sources.push((split(&source).expect("fences").0.to_string(), kinds));
            }
        }
        for (source, kinds) in sources {
            assert_eq!(
                parse(&source, &kinds).expect("the subset"),
                legacy::parse(&source, &kinds, &Verbatim).expect("saphyr"),
                "{source}"
            );
        }
    }

    #[tokio::test]
    async fn generated_frontmatters_read_as_saphyr_read_them() {
        for seed in crate::common::generate::seeds() {
            let db = crate::common::open().await;
            crate::common::generate::generate(&db, seed).await;
            let bundle = crate::export(&db, &crate::common::meta())
                .await
                .expect("export")
                .bundle;
            let chart = crate::common::generate::wide_chart();
            let kinds: HashMap<_, _> = chart
                .features
                .iter()
                .map(|(id, feature)| (id.clone(), feature.kind.clone()))
                .collect();
            let names = crate::names::Lenient;
            for (_, file) in bundle.iter().filter(|(path, _)| path.ends_with(".md")) {
                let (source, _) = split(file).expect("fences");
                assert_eq!(
                    super::parse(source, &kinds, &names).expect("the subset"),
                    legacy::parse(source, &kinds, &names).expect("saphyr"),
                    "seed {seed}:\n{source}"
                );
            }
        }
    }

    /// A frontmatter outside the subset is refused at the line of the construct.
    #[test]
    fn constructs_outside_the_subset_refuse_naming_their_line() {
        let head = "title: |-\n  Hi\nchart: builtin.page\ncreated: 2026-08-02T09:41:00.000Z\n\
                    updated: 2026-08-02T09:41:00.000Z\nreadings:\n";
        let kinds = HashMap::from([
            (feature("a1b2"), FeatureKind::Text),
            (feature("c3d4"), select_kind()),
        ]);
        for (tail, line, says) in [
            ("  a1b2: !!str x\n", 7, "tag"),
            ("  a1b2: &anchor x\n", 7, "anchor"),
            ("  a1b2: *alias\n", 7, "alias"),
            ("  a1b2: 'single'\n", 7, "single-quoted"),
            ("  a1b2: [a,\n    b]\n", 7, "flow sequence"),
            ("  a1b2: \"no close\n", 7, "quoted"),
            ("  a1b2: >-\n    folded\n", 7, "folded"),
            ("  c3d4: x\n   a1b2: y\n", 8, "expected"),
            ("\ta1b2: x\n", 7, "expected"),
            (
                "  a1b2:\n    x:\n      y:\n        z:\n          w:\n            v: 1\n",
                11,
                "nests deeper",
            ),
        ] {
            let source = format!("{head}{tail}");
            let (at, kind) = super::parse(&source, &kinds, &Verbatim).expect_err(&source);
            let MalformedKind::Grammar { detail } = kind else {
                panic!("{source}: {kind:?}");
            };
            assert_eq!(at, line, "{source}: {detail}");
            assert!(detail.contains(says), "{source}: {detail}");
        }
    }

    #[test]
    fn interpretation_refusals_cite_their_entry() {
        let source = "title: |-\n  Hi\nchart: builtin.page\ncreated: 2026-08-02T09:41:00.000Z\n\
                      updated: 2026-08-02T09:41:00.000Z\nreadings:\n  zzzz: true\n";
        let (line, _) = super::parse(source, &HashMap::new(), &Verbatim).expect_err("no feature");
        assert_eq!(line, 7);
    }

    /// CRLF, trailing whitespace, blank lines and own-line comments change
    /// nothing the subset reads.
    #[test]
    fn tolerances_read_to_the_same_frontmatter() {
        for (source, kinds) in committed() {
            let untouched = parse(&source, &kinds).expect("untouched");
            // Trailing whitespace inside a literal block is its text's, so only
            // the lines outside one carry it here.
            let trailing: String = source
                .lines()
                .map(|line| match line.starts_with(' ') {
                    true => format!("{line}\n"),
                    false => format!("{line}  \t\n"),
                })
                .collect();
            let tolerated = [
                source.replace('\n', "\r\n"),
                trailing,
                source.replace("\nchart:", "\n\nchart:"),
                source.replacen("readings:\n", "readings:\n\n", 1),
                source.replace("readings:", "# what it says\nreadings:"),
                source.replace("chart: ", "chart:   "),
            ];
            for variant in tolerated {
                assert_eq!(
                    parse(&variant, &kinds).unwrap_or_else(|e| panic!("{variant}\n{e}")),
                    untouched,
                    "{variant}"
                );
            }
        }
    }
}
