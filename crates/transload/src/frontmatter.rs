//! A Landmark's YAML frontmatter — the flattened root Mark.
//!
//! Emit is hand-rolled: no serde YAML library writes the advisory `#` comments,
//! and bytes must be stable. Parse uses `saphyr` (keys are dynamic
//! `FeatureID`s), which stays confined to this module.
//!
//! YAML cannot tell a `Select` from a one-element `MultiSelect`, so [`parse`]
//! is given the `FeatureKind` per `FeatureID`; Charts must be resolved first.

use std::{borrow::Cow, collections::HashMap};

use chrono::NaiveDateTime;
use domain::{
    Base36ID, ChartID, FeatureID, FeatureKind, GlyphID, Icon, MarkID, Reading, RichText,
    syntax::inline,
};
use indexmap::IndexMap;
use saphyr::{LoadableYamlNode, Mapping, Scalar, Yaml};

use crate::{bundle::render_timestamp, refusal::MalformedKind};

/// The frontmatter fence.
const FENCE: &str = "---";

/// One indentation step. YAML, so spaces.
const STEP: &str = "  ";

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
    /// Human names for features, written as `# Attendees` above their key.
    pub feature_names: HashMap<FeatureID, String>,
    /// Human names for referenced Marks, written as each ref's `label`.
    pub mark_titles: HashMap<MarkID, String>,
}

// ── Emit ─────────────────────────────────────────────────────────────────────

/// Render frontmatter, fences included.
pub fn emit(front: &Frontmatter, advisory: &Advisory) -> String {
    let mut out = String::new();
    out.push_str(FENCE);
    out.push('\n');

    push_block_scalar(&mut out, "title", &inline::emit(&front.title), 0);
    push_line(
        &mut out,
        0,
        &format!("chart: {}", plain(crate::layout::chart_id(&front.chart))),
    );
    if let Some(icon) = &front.icon {
        // JSON is a subset of YAML, so a compact flow mapping is valid and
        // deterministic without a recursive YAML writer.
        push_line(&mut out, 0, &format!("icon: {}", compact_json(icon)));
    }
    push_line(
        &mut out,
        0,
        &format!("created: {}", render_timestamp(front.created)),
    );
    push_line(
        &mut out,
        0,
        &format!("updated: {}", render_timestamp(front.updated)),
    );

    if !front.readings.is_empty() {
        push_line(&mut out, 0, "readings:");
        for (feature, reading) in &front.readings {
            if let Some(name) = advisory.feature_names.get(feature) {
                push_line(&mut out, 1, &format!("# {}", one_line(name)));
            }
            push_reading(&mut out, feature, reading, advisory);
        }
    }

    out.push_str(FENCE);
    out.push('\n');
    out
}

fn push_line(out: &mut String, depth: usize, line: &str) {
    for _ in 0..depth {
        out.push_str(STEP);
    }
    out.push_str(line);
    out.push('\n');
}

/// `key: |-` plus the indented body — the one spelling rich text takes.
///
/// A value opening with a space gets an explicit indentation indicator, or
/// YAML would read the space as indentation and drop it.
fn push_block_scalar(out: &mut String, key: &str, value: &str, depth: usize) {
    if value.is_empty() {
        push_line(out, depth, &format!("{key}: \"\""));
        return;
    }
    let indicator = if value.starts_with(' ') {
        STEP.len().to_string()
    } else {
        String::new()
    };
    push_line(out, depth, &format!("{key}: |{indicator}-"));
    for line in value.split('\n') {
        push_line(out, depth + 1, line);
    }
}

/// A base-36 id as a YAML scalar, quoted where YAML would re-type it:
/// `0001` reads as an integer, `12e3` as a float, `null` and `true` as such.
fn plain(id: &str) -> String {
    let retyped = id.starts_with(|c: char| c.is_ascii_digit())
        || matches!(id, "null" | "true" | "false" | "~");
    if retyped {
        format!("\"{id}\"")
    } else {
        id.to_string()
    }
}

fn push_reading(out: &mut String, feature: &FeatureID, reading: &Reading, advisory: &Advisory) {
    let key = plain(&feature.to_string());
    match reading {
        Reading::Null => push_line(out, 1, &format!("{key}: null")),
        Reading::Text(rich) => push_block_scalar(out, &key, &inline::emit(rich), 1),
        Reading::Number(number) => push_line(out, 1, &format!("{key}: {}", compact_json(number))),
        Reading::Bool(flag) => push_line(out, 1, &format!("{key}: {flag}")),
        Reading::Date(at) => push_line(out, 1, &format!("{key}: {}", render_timestamp(*at))),
        Reading::DateRange(from, to) => {
            push_line(out, 1, &format!("{key}:"));
            push_line(out, 2, &format!("from: {}", render_timestamp(*from)));
            push_line(out, 2, &format!("to: {}", render_timestamp(*to)));
        }
        Reading::Select(glyph) => {
            push_line(out, 1, &format!("{key}: {}", plain(&glyph.to_string())))
        }
        // An empty collection is written in flow form: `key:` alone reads as null.
        Reading::MultiSelect(glyphs) if glyphs.is_empty() => {
            push_line(out, 1, &format!("{key}: []"))
        }
        Reading::GroupedSelect(groups) if groups.is_empty() => {
            push_line(out, 1, &format!("{key}: {{}}"))
        }
        Reading::References(targets) if targets.is_empty() => {
            push_line(out, 1, &format!("{key}: []"))
        }
        Reading::MultiSelect(glyphs) => {
            push_line(out, 1, &format!("{key}:"));
            for glyph in glyphs {
                push_line(out, 2, &format!("- {}", plain(&glyph.to_string())));
            }
        }
        Reading::GroupedSelect(groups) => {
            push_line(out, 1, &format!("{key}:"));
            // A `HashMap`: sorted here, so bytes do not depend on the hash seed.
            let mut sorted: Vec<_> = groups.iter().collect();
            sorted.sort_by_key(|(group, _)| group.to_string());
            for (group, members) in sorted {
                if members.is_empty() {
                    push_line(out, 2, &format!("{}: []", plain(&group.to_string())));
                    continue;
                }
                push_line(out, 2, &format!("{}:", plain(&group.to_string())));
                for member in members {
                    push_line(out, 3, &format!("- {}", plain(&member.to_string())));
                }
            }
        }
        Reading::References(targets) => {
            push_line(out, 1, &format!("{key}:"));
            for target in targets {
                push_line(out, 2, &format!("- ref: {target}"));
                if let Some(title) = advisory.mark_titles.get(target) {
                    push_line(out, 3, &format!("label: {}", quote(&one_line(title))));
                }
            }
        }
        Reading::URL(value)
        | Reading::Email(value)
        | Reading::File(value)
        | Reading::Formula(value) => {
            push_line(out, 1, &format!("{key}: {}", quote(value)));
        }
        // The two strings' meaning is undefined in `FeatureKind::Phone`, so they
        // are carried positionally.
        Reading::Phone(first, second) => {
            push_line(
                out,
                1,
                &format!("{key}: [{}, {}]", quote(first), quote(second)),
            );
        }
    }
}

/// A double-quoted YAML scalar — the one spelling that never needs the emitter
/// to reason about whether a value looks like something else.
fn quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// Advisory text is one line by construction: a newline would break the comment.
fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn compact_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
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

/// Read frontmatter, interpreting each reading as its feature's kind.
pub fn parse(
    source: &str,
    kinds: &HashMap<FeatureID, FeatureKind>,
) -> Result<Frontmatter, MalformedKind> {
    let documents = Yaml::load_from_str(source).map_err(|error| MalformedKind::Grammar {
        detail: format!("frontmatter is not YAML: {error}"),
    })?;
    let Some(Yaml::Mapping(root)) = documents.into_iter().next() else {
        return Err(MalformedKind::Grammar {
            detail: "frontmatter is not a mapping".to_string(),
        });
    };

    let title = match get(&root, "title") {
        Some(node) => inline::parse(&as_text(node)?).map_err(grammar)?,
        None => RichText::default(),
    };
    let chart = match get(&root, "chart") {
        Some(node) => ChartID(as_text(node)?),
        None => {
            return Err(MalformedKind::Grammar {
                detail: "frontmatter with no `chart`".to_string(),
            });
        }
    };
    let icon = match get(&root, "icon") {
        Some(node) => Some(from_json_node(node)?),
        None => None,
    };
    let created = timestamp(&root, "created")?;
    let updated = timestamp(&root, "updated")?;

    let mut readings = IndexMap::new();
    if let Some(Yaml::Mapping(entries)) = get(&root, "readings") {
        for (key, value) in entries {
            let key = as_text(key)?;
            let feature = FeatureID(Base36ID(key.clone()));
            let Some(kind) = kinds.get(&feature) else {
                return Err(MalformedKind::Grammar {
                    detail: format!("reading `{key}` names no feature of this Chart"),
                });
            };
            readings.insert(feature, reading(value, kind, &key)?);
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

fn grammar(error: inline::GrammarError) -> MalformedKind {
    MalformedKind::Grammar {
        detail: error.to_string(),
    }
}

fn get<'a, 'i>(mapping: &'a Mapping<'i>, key: &str) -> Option<&'a Yaml<'i>> {
    // Owned, so the probe key does not borrow from the caller's `&str`.
    mapping.get(&Yaml::Value(Scalar::String(Cow::Owned(key.to_string()))))
}

/// A node's text; scalars arrive as a parsed [`Scalar`] or an unparsed
/// [`Yaml::Representation`] depending on style.
fn as_text(node: &Yaml<'_>) -> Result<String, MalformedKind> {
    match node {
        Yaml::Value(Scalar::String(text)) => Ok(text.to_string()),
        Yaml::Value(Scalar::Integer(number)) => Ok(number.to_string()),
        Yaml::Value(Scalar::FloatingPoint(number)) => Ok(number.to_string()),
        Yaml::Value(Scalar::Boolean(flag)) => Ok(flag.to_string()),
        Yaml::Value(Scalar::Null) => Ok(String::new()),
        Yaml::Representation(text, _, _) => Ok(text.to_string()),
        other => Err(MalformedKind::Grammar {
            detail: format!("expected a scalar, got {other:?}"),
        }),
    }
}

/// A `saphyr` node as a JSON value. Compact JSON written by the emitter comes
/// back as real mappings and sequences, so it is walked rather than re-parsed.
fn to_json(node: &Yaml<'_>) -> Result<domain::Json, MalformedKind> {
    Ok(match node {
        Yaml::Value(Scalar::Null) => domain::Json::Null,
        Yaml::Value(Scalar::Boolean(flag)) => domain::Json::Bool(*flag),
        Yaml::Value(Scalar::Integer(number)) => domain::Json::from(*number),
        Yaml::Value(Scalar::FloatingPoint(number)) => {
            serde_json::Number::from_f64(number.into_inner())
                .map(domain::Json::Number)
                .unwrap_or(domain::Json::Null)
        }
        Yaml::Value(Scalar::String(text)) => domain::Json::String(text.to_string()),
        Yaml::Representation(text, _, _) => domain::Json::String(text.to_string()),
        Yaml::Sequence(items) => {
            domain::Json::Array(items.iter().map(to_json).collect::<Result<Vec<_>, _>>()?)
        }
        Yaml::Mapping(entries) => {
            let mut object = serde_json::Map::new();
            for (key, value) in entries {
                object.insert(as_text(key)?, to_json(value)?);
            }
            domain::Json::Object(object)
        }
        other => {
            return Err(MalformedKind::Grammar {
                detail: format!("cannot read {other:?} as a value"),
            });
        }
    })
}

fn from_json_node<T: serde::de::DeserializeOwned>(node: &Yaml<'_>) -> Result<T, MalformedKind> {
    serde_json::from_value(to_json(node)?).map_err(|error| MalformedKind::Grammar {
        detail: format!("not a valid encoded value: {error}"),
    })
}

fn timestamp(root: &Mapping<'_>, key: &str) -> Result<NaiveDateTime, MalformedKind> {
    let Some(node) = get(root, key) else {
        return Err(MalformedKind::Grammar {
            detail: format!("frontmatter with no `{key}`"),
        });
    };
    parse_timestamp(&as_text(node)?)
}

fn parse_timestamp(text: &str) -> Result<NaiveDateTime, MalformedKind> {
    crate::bundle::parse_timestamp(text).map_err(|detail| MalformedKind::Grammar { detail })
}

fn glyph(node: &Yaml<'_>) -> Result<GlyphID, MalformedKind> {
    Ok(GlyphID(Base36ID(as_text(node)?)))
}

fn sequence<'a, 'b>(node: &'a Yaml<'b>, what: &str) -> Result<&'a [Yaml<'b>], MalformedKind> {
    match node {
        Yaml::Sequence(items) => Ok(items),
        other => Err(MalformedKind::Grammar {
            detail: format!("expected a list for {what}, got {other:?}"),
        }),
    }
}

/// One YAML value read as the kind its feature declares.
fn reading(node: &Yaml<'_>, kind: &FeatureKind, key: &str) -> Result<Reading, MalformedKind> {
    if matches!(node, Yaml::Value(Scalar::Null)) {
        return Ok(Reading::Null);
    }

    match kind {
        FeatureKind::Text => Ok(Reading::Text(
            inline::parse(&as_text(node)?).map_err(grammar)?,
        )),
        FeatureKind::Number { .. } => {
            let text = as_text(node)?;
            text.trim()
                .parse()
                .map(Reading::Number)
                .map_err(|_| MalformedKind::Grammar {
                    detail: format!("reading `{key}` is not a number: {text}"),
                })
        }
        FeatureKind::Checkbox => {
            let text = as_text(node)?;
            match text.trim() {
                "true" => Ok(Reading::Bool(true)),
                "false" => Ok(Reading::Bool(false)),
                other => Err(MalformedKind::Grammar {
                    detail: format!("reading `{key}` is not a boolean: {other}"),
                }),
            }
        }
        FeatureKind::Date { .. } => match node {
            Yaml::Mapping(range) => {
                let from = get(range, "from").ok_or_else(|| MalformedKind::Grammar {
                    detail: format!("reading `{key}` is a date range with no `from`"),
                })?;
                let to = get(range, "to").ok_or_else(|| MalformedKind::Grammar {
                    detail: format!("reading `{key}` is a date range with no `to`"),
                })?;
                Ok(Reading::DateRange(
                    parse_timestamp(&as_text(from)?)?,
                    parse_timestamp(&as_text(to)?)?,
                ))
            }
            scalar => Ok(Reading::Date(parse_timestamp(&as_text(scalar)?)?)),
        },
        FeatureKind::URL => Ok(Reading::URL(as_text(node)?)),
        FeatureKind::Email => Ok(Reading::Email(as_text(node)?)),
        FeatureKind::Formula => Ok(Reading::Formula(as_text(node)?)),
        FeatureKind::Phone => {
            let items = sequence(node, &format!("reading `{key}`"))?;
            if items.len() != 2 {
                return Err(MalformedKind::Grammar {
                    detail: format!("reading `{key}` is a phone number of {} parts", items.len()),
                });
            }
            Ok(Reading::Phone(as_text(&items[0])?, as_text(&items[1])?))
        }
        FeatureKind::Select { .. } => Ok(Reading::Select(glyph(node)?)),
        FeatureKind::MultiSelect { .. } => {
            let items = sequence(node, &format!("reading `{key}`"))?;
            Ok(Reading::MultiSelect(
                items.iter().map(glyph).collect::<Result<_, _>>()?,
            ))
        }
        FeatureKind::GroupedSelect { .. } | FeatureKind::GroupedMultiSelect { .. } => {
            let Yaml::Mapping(groups) = node else {
                return Err(MalformedKind::Grammar {
                    detail: format!("reading `{key}` is not a mapping of groups"),
                });
            };
            let mut out = std::collections::HashMap::new();
            for (group, members) in groups {
                let members = sequence(members, &format!("group in reading `{key}`"))?;
                out.insert(
                    glyph(group)?,
                    members.iter().map(glyph).collect::<Result<_, _>>()?,
                );
            }
            Ok(Reading::GroupedSelect(out))
        }
        FeatureKind::Reference { .. } => {
            let items = sequence(node, &format!("reading `{key}`"))?;
            let mut targets = Vec::new();
            for item in items {
                // `label` is advisory and deliberately not read: a Bundle whose
                // labels were hand-edited must ingest identically to one with
                // them stripped.
                let text = match item {
                    Yaml::Mapping(entry) => {
                        let node = get(entry, "ref").ok_or_else(|| MalformedKind::Grammar {
                            detail: format!("a reference in `{key}` has no `ref`"),
                        })?;
                        as_text(node)?
                    }
                    scalar => as_text(scalar)?,
                };
                let id: uuid::Uuid = text.trim().parse().map_err(|_| MalformedKind::Grammar {
                    detail: format!("`{text}` is not a Mark id"),
                })?;
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
        let source = emit(&front, &Advisory::default());
        let (inner, _) = split(&source).expect("fences");
        let parsed = parse(inner, &kinds).unwrap_or_else(|e| panic!("parse {source}: {e}"));
        assert_eq!(parsed, front, "via\n{source}");
    }

    #[test]
    fn the_shape_is_fenced_and_ordered() {
        let source = emit(&base(IndexMap::new()), &Advisory::default());
        assert_eq!(
            source,
            "---\ntitle: |-\n  Weekly sync\nchart: builtin.meeting\n\
             created: 2026-08-02T09:41:00.000Z\nupdated: 2026-08-02T09:41:00.000Z\n---\n"
        );
    }

    #[test]
    fn the_chart_keeps_its_builtin_prefix() {
        let source = emit(&base(IndexMap::new()), &Advisory::default());
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
        let source = emit(&base(readings), &advisory);
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
        );
        let without = emit(&base(readings), &Advisory::default());

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
        let source = emit(&front, &Advisory::default());
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

        let first = emit(&base(readings.clone()), &Advisory::default());
        let second = emit(&base(readings), &Advisory::default());
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
            emit(&front, &Advisory::default()),
            emit(&front, &Advisory::default())
        );
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
}
