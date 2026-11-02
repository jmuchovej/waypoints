//! The hand-rolled frontmatter emitter and the `saphyr` reader, kept as the
//! oracles the `Node` writer and the subset grammar are held to.

use std::{borrow::Cow, collections::HashMap};

use chrono::NaiveDateTime;
use domain::{
    Base36ID, ChartID, FeatureID, FeatureKind, GlyphID, MarkID, Reading, RichText,
    syntax::inline::{self, Links},
};
use indexmap::IndexMap;
use saphyr::{LoadableYamlNode, Mapping, Scalar, Yaml};

use super::{Advisory, Frontmatter};
use crate::{bundle::render_timestamp, names::KeepLabels, refusal::MalformedKind};

const FENCE: &str = "---";
const STEP: &str = "  ";

pub(crate) fn emit(front: &Frontmatter, advisory: &Advisory, links: &dyn Links) -> String {
    let mut out = String::new();
    out.push_str(FENCE);
    out.push('\n');

    push_block_scalar(
        &mut out,
        "title",
        &inline::emit_with(&front.title, &KeepLabels(links)),
        0,
    );
    push_line(
        &mut out,
        0,
        &format!("chart: {}", plain(crate::layout::chart_id(&front.chart))),
    );
    if let Some(icon) = &front.icon {
        push_line(&mut out, 0, &format!("icon: {}", compact_json(icon)));
    }
    push_line(
        &mut out,
        0,
        &format!("created: {} # advisory", render_timestamp(front.created)),
    );
    push_line(
        &mut out,
        0,
        &format!("updated: {} # advisory", render_timestamp(front.updated)),
    );

    if !front.readings.is_empty() {
        push_line(&mut out, 0, "readings:");
        for (feature, reading) in &front.readings {
            let mut lines = String::new();
            push_reading(&mut lines, feature, reading, advisory, links);
            let (first, rest) = lines.split_once('\n').unwrap_or((&lines, ""));
            out.push_str(first);
            if let Some(name) = advisory.feature_names.get(feature) {
                out.push_str(" # ");
                out.push_str(&one_line(name));
            }
            out.push('\n');
            out.push_str(rest);
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

fn plain(id: &str) -> String {
    let retyped = id.starts_with(|c: char| c.is_ascii_digit())
        || matches!(id, "null" | "true" | "false" | "~");
    if retyped {
        format!("\"{id}\"")
    } else {
        id.to_string()
    }
}

fn push_reading(
    out: &mut String,
    feature: &FeatureID,
    reading: &Reading,
    advisory: &Advisory,
    links: &dyn Links,
) {
    let key = plain(&feature.to_string());
    match reading {
        Reading::Null => push_line(out, 1, &format!("{key}: null")),
        Reading::Text(rich) => push_block_scalar(out, &key, &inline::emit_with(rich, links), 1),
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
        Reading::Phone(first, second) => {
            push_line(
                out,
                1,
                &format!("{key}: [{}, {}]", quote(first), quote(second)),
            );
        }
    }
}

fn quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn one_line(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn compact_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

pub(crate) fn parse(
    source: &str,
    kinds: &HashMap<FeatureID, FeatureKind>,
    links: &dyn Links,
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
        Some(node) => inline::parse_with(&as_text(node)?, &KeepLabels(links)).map_err(grammar)?,
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
            readings.insert(feature, reading(value, kind, &key, links)?);
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
    mapping.get(&Yaml::Value(Scalar::String(Cow::Owned(key.to_string()))))
}

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

fn reading(
    node: &Yaml<'_>,
    kind: &FeatureKind,
    key: &str,
    links: &dyn Links,
) -> Result<Reading, MalformedKind> {
    if matches!(node, Yaml::Value(Scalar::Null)) {
        return Ok(Reading::Null);
    }

    match kind {
        FeatureKind::Text => Ok(Reading::Text(
            inline::parse_with(&as_text(node)?, links).map_err(grammar)?,
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
