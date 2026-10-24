//! `_schema.json` — a real Draft-07 document that also carries its Chart whole.
//!
//! The Draft-07 face serves tooling and validates a Landmark's readings. The
//! canonical Chart rides in the reserved top-level `x-waypoints` member, since
//! Draft-07 cannot express `FeatureKind` (unknown keywords are legal).
//!
//! The face is generated from the canonical form; [`check_consistent`]
//! regenerates and compares on ingest, so drift is a refusal.
//!
//! Every referenced Chart emits one, builtins included.

use domain::{Chart, Feature, FeatureID, FeatureKind, GlyphID, Json, Reading, syntax::inline};
use indexmap::IndexMap;
use serde_json::{Map, json};

use crate::{bundle::render_timestamp, refusal::MalformedKind};

/// The reserved member carrying the canonical Chart.
pub const X_WAYPOINTS: &str = "x-waypoints";

/// The Draft-07 meta-schema's identifier.
const DRAFT_07: &str = "http://json-schema.org/draft-07/schema#";

// ── Emit ─────────────────────────────────────────────────────────────────────

/// The `_schema.json` document for one Chart.
pub fn emit(chart: &Chart) -> Json {
    let mut document = Map::new();
    document.insert("$schema".into(), json!(DRAFT_07));
    document.insert("title".into(), json!(chart.name));
    if let Some(description) = &chart.description {
        document.insert("description".into(), json!(description));
    }
    document.insert("type".into(), json!("object"));
    document.insert("properties".into(), properties(chart));

    let required: Vec<String> = chart
        .features
        .values()
        .filter(|feature| feature.required)
        .map(|feature| feature.id.to_string())
        .collect();
    if !required.is_empty() {
        document.insert("required".into(), json!(required));
    }
    document.insert("additionalProperties".into(), json!(false));

    // Last, so a reader meets the schema before the payload.
    document.insert(X_WAYPOINTS.into(), canonical(chart));
    Json::Object(document)
}

/// `properties`, in Chart feature order — the order is the Chart's, so the
/// document is byte-stable without sorting.
fn properties(chart: &Chart) -> Json {
    let mut out = Map::new();
    for (id, feature) in &chart.features {
        out.insert(id.to_string(), property(feature));
    }
    Json::Object(out)
}

/// One feature's schema: its kind's face, widened to admit null (every kind
/// accepts `Reading::Null`), described.
fn property(feature: &Feature) -> Json {
    let mut schema = match face(&feature.kind) {
        Json::Object(mut object) => match object.get("type") {
            Some(Json::String(kind)) => {
                let kind = kind.clone();
                object.insert("type".into(), json!([kind, "null"]));
                if let Some(Json::Array(allowed)) = object.get_mut("enum") {
                    allowed.push(Json::Null);
                }
                Json::Object(object)
            }
            _ => json!({ "anyOf": [Json::Object(object), { "type": "null" }] }),
        },
        other => json!({ "anyOf": [other, { "type": "null" }] }),
    };

    if let (Json::Object(object), Some(description)) = (&mut schema, &feature.description) {
        object.insert("description".into(), json!(description));
    }
    schema
}

/// The Draft-07 face for one kind; mirrors `frontmatter::push_reading`.
fn face(kind: &FeatureKind) -> Json {
    match kind {
        // Rich text reaches YAML as dialect inline source.
        FeatureKind::Text | FeatureKind::Formula => json!({ "type": "string" }),
        FeatureKind::Number {
            min,
            max,
            step,
            precision: _,
            format: _,
        } => {
            let mut schema = Map::new();
            schema.insert("type".into(), json!("number"));
            if let Some(min) = min {
                schema.insert("minimum".into(), json!(min));
            }
            if let Some(max) = max {
                schema.insert("maximum".into(), json!(max));
            }
            if let Some(step) = step
                && *step > 0.0
            {
                schema.insert("multipleOf".into(), json!(step));
            }
            Json::Object(schema)
        }
        FeatureKind::Checkbox => json!({ "type": "boolean" }),
        // A single instant, or a `{from, to}` range — both are legal for one
        // date feature, which is why this is the one kind with a `oneOf`.
        FeatureKind::Date { .. } => json!({
            "oneOf": [
                { "type": "string", "format": "date-time" },
                {
                    "type": "object",
                    "properties": {
                        "from": { "type": "string", "format": "date-time" },
                        "to": { "type": "string", "format": "date-time" },
                    },
                    "required": ["from", "to"],
                    "additionalProperties": false,
                },
            ]
        }),
        FeatureKind::Email => json!({ "type": "string", "format": "email" }),
        FeatureKind::URL => json!({ "type": "string", "format": "uri" }),
        // Two strings whose meaning is undefined in-tree, so carried
        // positionally — the schema says only that there are two.
        FeatureKind::Phone => json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 2,
            "maxItems": 2,
        }),
        FeatureKind::Select { options, .. } => json!({
            "type": "string",
            "enum": glyph_names(options.keys()),
        }),
        FeatureKind::MultiSelect {
            options,
            max_selections,
            ..
        } => {
            let mut schema = Map::new();
            schema.insert("type".into(), json!("array"));
            schema.insert(
                "items".into(),
                json!({ "type": "string", "enum": glyph_names(options.keys()) }),
            );
            if let Some(max) = max_selections {
                schema.insert("maxItems".into(), json!(max));
            }
            Json::Object(schema)
        }
        FeatureKind::GroupedSelect { groups, .. }
        | FeatureKind::GroupedMultiSelect { groups, .. } => json!({
            "type": "object",
            "propertyNames": { "enum": glyph_names(groups.keys()) },
            "additionalProperties": {
                "type": "array",
                "items": { "type": "string" },
            },
        }),
        // `{ref, label?}` — `ref` is normative, `label` advisory.
        FeatureKind::Reference { max_references, .. } => {
            let mut schema = Map::new();
            schema.insert("type".into(), json!("array"));
            schema.insert(
                "items".into(),
                json!({
                    "type": "object",
                    "properties": {
                        "ref": { "type": "string", "format": "uuid" },
                        "label": { "type": "string" },
                    },
                    "required": ["ref"],
                    "additionalProperties": false,
                }),
            );
            if let Some(max) = max_references {
                schema.insert("maxItems".into(), json!(max));
            }
            Json::Object(schema)
        }
    }
}

fn glyph_names<'a>(keys: impl Iterator<Item = &'a domain::GlyphID>) -> Vec<String> {
    keys.map(ToString::to_string).collect()
}

/// The Chart, serialized whole, for `x-waypoints`. The id keeps its `builtin.`
/// prefix because serde is transparent over the raw string.
fn canonical(chart: &Chart) -> Json {
    serde_json::to_value(chart).unwrap_or(Json::Null)
}

// ── Read ─────────────────────────────────────────────────────────────────────

/// The canonical Chart a `_schema.json` carries.
pub fn read(document: &Json) -> Result<Chart, MalformedKind> {
    let Some(member) = document.get(X_WAYPOINTS) else {
        return Err(MalformedKind::Grammar {
            detail: format!("a schema document with no `{X_WAYPOINTS}` member"),
        });
    };
    serde_json::from_value(member.clone()).map_err(|error| MalformedKind::Grammar {
        detail: format!("`{X_WAYPOINTS}` is not a Chart: {error}"),
    })
}

/// Refuse a document whose Draft-07 face disagrees with the Chart it carries.
pub fn check_consistent(document: &Json) -> Result<(), MalformedKind> {
    let chart = read(document)?;
    let regenerated = emit(&chart);
    if &regenerated == document {
        return Ok(());
    }
    Err(MalformedKind::Grammar {
        detail: format!("the Draft-07 face disagrees with the `{X_WAYPOINTS}` Chart it carries",),
    })
}

/// Check a Landmark's readings against the Chart's Draft-07 face.
///
/// `readings` is the frontmatter's `readings` object as JSON. Errors come back
/// per feature, so a refusal can name the file *and* the feature.
pub fn validate_readings(document: &Json, readings: &Json) -> Result<(), Vec<(FeatureID, String)>> {
    let validator = match jsonschema::draft7::new(document) {
        Ok(validator) => validator,
        Err(error) => {
            return Err(vec![(
                FeatureID(domain::Base36ID(String::new())),
                format!("the Chart's schema is not valid Draft-07: {error}"),
            )]);
        }
    };

    let mut problems = Vec::new();
    for error in validator.iter_errors(readings) {
        // The instance path's first segment is the feature, when there is one.
        let feature = error
            .instance_path()
            .into_iter()
            .next()
            .map(|segment| segment.to_string())
            .unwrap_or_default();
        problems.push((FeatureID(domain::Base36ID(feature)), error.to_string()));
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

/// Does this document satisfy the Draft-07 meta-schema?
pub fn check_meta_schema(document: &Json) -> Result<(), MalformedKind> {
    jsonschema::draft7::meta::validate(document).map_err(|error| MalformedKind::Grammar {
        detail: format!("not a valid Draft-07 schema: {error}"),
    })
}

/// Readings as the instance the face validates — the shape frontmatter writes,
/// not `Reading`'s own tagged serialization.
pub(crate) fn instance(readings: &IndexMap<FeatureID, Reading>) -> Json {
    let mut out = Map::new();
    for (feature, reading) in readings {
        out.insert(feature.to_string(), reading_instance(reading));
    }
    Json::Object(out)
}

fn reading_instance(reading: &Reading) -> Json {
    let names = |glyphs: &[GlyphID]| glyphs.iter().map(ToString::to_string).collect::<Vec<_>>();
    match reading {
        Reading::Null => json!(null),
        Reading::Text(rich) => json!(inline::emit(rich)),
        Reading::Number(number) => json!(number),
        Reading::Bool(flag) => json!(flag),
        Reading::Date(at) => json!(render_timestamp(*at)),
        Reading::DateRange(from, to) => json!({
            "from": render_timestamp(*from),
            "to": render_timestamp(*to),
        }),
        Reading::Select(glyph) => json!(glyph.to_string()),
        Reading::MultiSelect(glyphs) => json!(names(glyphs)),
        Reading::GroupedSelect(groups) => {
            let mut sorted: Vec<_> = groups.iter().collect();
            sorted.sort_by_key(|(group, _)| group.to_string());
            Json::Object(
                sorted
                    .into_iter()
                    .map(|(group, members)| (group.to_string(), json!(names(members))))
                    .collect(),
            )
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
        | Reading::Formula(value) => {
            json!(value)
        }
        Reading::Phone(first, second) => json!([first, second]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Base36ID, ChartID, GlyphID, Icon, ReferenceDisplayMode, SelectOption};
    use indexmap::IndexMap;

    fn feature_id(id: &str) -> FeatureID {
        FeatureID(Base36ID(id.to_string()))
    }

    fn glyph(id: &str) -> GlyphID {
        GlyphID(Base36ID(id.to_string()))
    }

    fn feature(id: &str, name: &str, kind: FeatureKind) -> Feature {
        Feature {
            id: feature_id(id),
            name: name.to_string(),
            kind,
            description: None,
            required: false,
        }
    }

    fn chart_with(features: Vec<Feature>) -> Chart {
        let mut map = IndexMap::new();
        for feature in features {
            map.insert(feature.id.clone(), feature);
        }
        Chart {
            id: ChartID("c4d8".into()),
            name: "Meeting".into(),
            description: Some("A meeting".into()),
            icon: None,
            features: map,
            extra: None,
        }
    }

    fn option(id: &str, name: &str) -> SelectOption {
        SelectOption {
            id: glyph(id),
            ..SelectOption::builder().with_name(name).build()
        }
    }

    fn select_kind() -> FeatureKind {
        let mut options = IndexMap::new();
        options.insert(glyph("0001"), option("0001", "Open"));
        options.insert(glyph("0002"), option("0002", "Done"));
        FeatureKind::Select {
            options,
            default_option: None,
        }
    }

    fn reference_kind() -> FeatureKind {
        FeatureKind::Reference {
            allowed_types: None,
            max_references: None,
            display_field: "title".into(),
            display_mode: ReferenceDisplayMode::default(),
            filter_criteria: None,
        }
    }

    #[test]
    fn a_schema_document_is_valid_draft_07() {
        let chart = chart_with(vec![
            feature("a1b2", "Notes", FeatureKind::Text),
            feature("c3d4", "Status", select_kind()),
            feature("e5f6", "Attendees", reference_kind()),
        ]);
        let document = emit(&chart);
        check_meta_schema(&document).expect("the meta-schema must accept it");
    }

    #[test]
    fn the_chart_rides_whole_under_x_waypoints() {
        let mut chart = chart_with(vec![feature("a1b2", "Notes", FeatureKind::Text)]);
        chart.icon = Some(Icon {
            identifier: "star".into(),
            alt_text: None,
            colorable: None,
            metadata: None,
        });
        chart.extra = Some(serde_json::json!({
            "so.waypoints.importer.logseq": { "page": "Meetings" },
            "a1b2": { "unit": "minutes" },
        }));

        let document = emit(&chart);
        let read_back = read(&document).expect("read");
        assert_eq!(read_back, chart, "the Chart must survive whole");
    }

    #[test]
    fn a_builtin_chart_keeps_its_prefix_in_the_canonical_form() {
        let mut chart = chart_with(vec![]);
        chart.id = ChartID::builtin("meeting");
        let document = emit(&chart);
        assert_eq!(
            read(&document).expect("read").id,
            ChartID::builtin("meeting")
        );
        assert!(
            serde_json::to_string(&document)
                .expect("serialize")
                .contains("builtin.meeting"),
            "the raw id, not the display name"
        );
    }

    #[test]
    fn the_face_describes_every_kind() {
        let kinds: Vec<FeatureKind> = vec![
            FeatureKind::Text,
            FeatureKind::Number {
                min: Some(0.0),
                max: Some(10.0),
                step: Some(0.5),
                precision: None,
                format: None,
            },
            FeatureKind::Checkbox,
            FeatureKind::Date {
                min_date: None,
                min_time: None,
                max_date: None,
                max_time: None,
                timezone: None,
                granularity: Default::default(),
            },
            FeatureKind::Email,
            FeatureKind::URL,
            FeatureKind::Phone,
            select_kind(),
            reference_kind(),
            FeatureKind::Formula,
        ];
        for kind in kinds {
            let chart = chart_with(vec![feature("a1b2", "A feature", kind.clone())]);
            let document = emit(&chart);
            check_meta_schema(&document).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            // Every property admits null, because every kind accepts a null
            // reading.
            let property = &document["properties"]["a1b2"];
            let admits_null = property["type"]
                .as_array()
                .map(|types| types.contains(&serde_json::json!("null")))
                .unwrap_or(false)
                || property["anyOf"]
                    .as_array()
                    .map(|branches| branches.contains(&serde_json::json!({ "type": "null" })))
                    .unwrap_or(false);
            assert!(admits_null, "{kind:?} must admit null: {property}");
        }
    }

    #[test]
    fn a_select_enumerates_its_glyphs() {
        let chart = chart_with(vec![feature("a1b2", "Status", select_kind())]);
        let document = emit(&chart);
        let enumerated = &document["properties"]["a1b2"]["enum"];
        assert_eq!(enumerated, &serde_json::json!(["0001", "0002", null]));
    }

    #[test]
    fn required_features_are_listed() {
        let mut required = feature("a1b2", "Notes", FeatureKind::Text);
        required.required = true;
        let chart = chart_with(vec![
            required,
            feature("c3d4", "Optional", FeatureKind::Text),
        ]);
        let document = emit(&chart);
        assert_eq!(document["required"], serde_json::json!(["a1b2"]));
    }

    #[test]
    fn a_description_reaches_its_property() {
        let mut described = feature("a1b2", "Notes", FeatureKind::Text);
        described.description = Some("What was said".into());
        let chart = chart_with(vec![described]);
        let document = emit(&chart);
        assert_eq!(
            document["properties"]["a1b2"]["description"],
            serde_json::json!("What was said")
        );
    }

    #[test]
    fn the_face_and_the_chart_cannot_drift() {
        let chart = chart_with(vec![feature("a1b2", "Status", select_kind())]);
        let document = emit(&chart);
        check_consistent(&document).expect("freshly generated, so consistent");

        // Hand-edit the face without touching the Chart: refused.
        let mut tampered = document.clone();
        tampered["properties"]["a1b2"]["enum"] = serde_json::json!(["9999"]);
        assert!(
            check_consistent(&tampered).is_err(),
            "a face that disagrees with its Chart is a refusal"
        );

        // And the other way: edit the Chart, leave the face.
        let mut tampered = document.clone();
        tampered[X_WAYPOINTS]["name"] = serde_json::json!("Renamed");
        assert!(check_consistent(&tampered).is_err());
    }

    #[test]
    fn a_document_without_the_member_is_refused() {
        let mut document = emit(&chart_with(vec![]));
        document
            .as_object_mut()
            .expect("object")
            .remove(X_WAYPOINTS);
        assert!(read(&document).is_err());
        assert!(check_consistent(&document).is_err());
    }

    #[test]
    fn readings_are_validated_against_the_face() {
        let chart = chart_with(vec![
            feature("a1b2", "Notes", FeatureKind::Text),
            feature("c3d4", "Status", select_kind()),
        ]);
        let document = emit(&chart);

        let good = serde_json::json!({ "a1b2": "some notes", "c3d4": "0001" });
        validate_readings(&document, &good).expect("valid readings");

        // A null reading is always allowed.
        let null = serde_json::json!({ "a1b2": null });
        validate_readings(&document, &null).expect("null is a reading");

        // A select outside its options is not.
        let bad = serde_json::json!({ "c3d4": "9999" });
        let problems = validate_readings(&document, &bad).expect_err("must refuse");
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].0, feature_id("c3d4"), "names the feature");

        // Neither is a reading for a feature the Chart does not have.
        let unknown = serde_json::json!({ "zzzz": "surprise" });
        assert!(validate_readings(&document, &unknown).is_err());
    }

    #[test]
    fn emitting_twice_is_byte_identical() {
        let chart = chart_with(vec![
            feature("a1b2", "Notes", FeatureKind::Text),
            feature("c3d4", "Status", select_kind()),
        ]);
        let first = serde_json::to_string_pretty(&emit(&chart)).expect("serialize");
        let second = serde_json::to_string_pretty(&emit(&chart)).expect("serialize");
        assert_eq!(first, second);
    }

    #[test]
    fn properties_follow_chart_order_not_sorted_order() {
        // The Chart's features are an IndexMap, so their order is the Chart's
        // and needs no sorting to be stable.
        let chart = chart_with(vec![
            feature("zzzz", "Last", FeatureKind::Text),
            feature("aaaa", "First", FeatureKind::Text),
        ]);
        let document = emit(&chart);
        let keys: Vec<&String> = document["properties"]
            .as_object()
            .expect("object")
            .keys()
            .collect();
        assert_eq!(keys, vec!["zzzz", "aaaa"]);
    }
}
