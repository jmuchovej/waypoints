//! The Fragment grammar: source-keyed mirrors of the data model's entities.
//!
//! - every type round-trips through serde, and an SDK-written output is a
//!   fixed point of deserialize-then-serialize;
//! - `id` is refused while any other unknown field is ignored;
//! - each mirror is exhaustive over its original — the conversions below are
//!   non-wildcard matches, so a new data-model variant fails to compile here;
//! - the census counts what a set of Fragments uses, by serde tag.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use domain::{
    Content, FeatureKind, InlineMark, Reading, RichText, SelectGroup, SelectOption, TerrainKind,
    TextMarkKind, TextSegment,
};
use indexmap::IndexMap;
use serde_json::json;
use transload::{
    Census,
    abi::{Diagnostic, Envelope, FinalizeOutput, Init, Output, Summary},
    fragment::{
        ContentFragment, FRAGMENT_FORMAT, FeatureFragment, FeatureKindFragment, Fragments,
        LandmarkFragment, Protochart, ReadingFragment, SelectGroupFragment, SelectOptionFragment,
        SourceKey, TerrainFragment, TerrainKindFragment,
    },
};

fn key(text: &str) -> SourceKey {
    SourceKey::new(text).expect("non-empty")
}

fn paragraph(text: &str) -> Content {
    Content::Paragraph {
        rich_text: RichText::from(text.to_string()),
        theme: None,
    }
}

fn mention(text: &str, target: &str) -> Content {
    Content::Paragraph {
        rich_text: RichText::new(vec![TextSegment {
            text: text.to_string(),
            marks: vec![InlineMark {
                kind: TextMarkKind::Mention,
                data: Some(json!(target)),
            }],
        }]),
        theme: None,
    }
}

fn sample() -> Fragments {
    let mut status = Protochart::new(key("db:tasks"), "Tasks");
    let mut options = IndexMap::new();
    options.insert(
        key("opt:todo"),
        SelectOptionFragment {
            name: "To do".into(),
            description: None,
            icon: None,
            color: Default::default(),
        },
    );
    status.features.insert(
        key("prop:status"),
        FeatureFragment {
            name: "Status".into(),
            description: Some("Where it stands".into()),
            required: false,
            kind: FeatureKindFragment::Select {
                options,
                default_option: Some(key("opt:todo")),
            },
        },
    );

    let mut landmark = LandmarkFragment::new(key("page:plan"), "Plan".to_string());
    landmark.protochart = Some(key("db:tasks"));
    landmark
        .readings
        .insert(key("prop:status"), ReadingFragment::Select(key("opt:todo")));
    landmark.created =
        NaiveDate::from_ymd_opt(2026, 8, 2).and_then(|day| day.and_hms_milli_opt(9, 41, 0, 7));
    let mut heading = ContentFragment::new(
        key("block:plan:0"),
        Content::Heading2 {
            rich_text: RichText::from("Goals".to_string()),
            theme: None,
        },
    );
    heading.children.push(ContentFragment::new(
        key("block:plan:1"),
        mention("see the retro", "page:retro"),
    ));
    landmark.body.push(heading);
    landmark.body.push(ContentFragment::new(
        key("block:plan:2"),
        paragraph("Ship it."),
    ));

    let mut edge = TerrainFragment::new(
        key("page:plan"),
        key("block:plan:2"),
        TerrainKindFragment::Child,
    );
    edge.order = Some(1);
    edge.extra = Some(json!({ "so.waypoints.test": true }));

    Fragments {
        landmarks: vec![landmark],
        protocharts: vec![status],
        terrain: vec![edge],
    }
}

#[test]
fn fragments_round_trip_and_an_sdk_written_output_is_a_fixed_point() {
    let output = Output {
        fragments: sample(),
        diagnostics: vec![Diagnostic::warn("skipped a table").at("pages/plan.md", Some(12))],
    };
    let bytes = serde_json::to_vec(&output).unwrap();
    let back: Output = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(back, output);
    assert_eq!(serde_json::to_vec(&back).unwrap(), bytes);

    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.contains("\"created\":\"2026-08-02T09:41:00.007Z\""),
        "{text}"
    );
    assert!(text.contains("\"type\":\"select\""), "{text}");
    assert!(!text.contains("\"id\""), "{text}");
}

#[test]
fn id_is_refused_and_other_unknown_fields_are_ignored() {
    let with_id = json!({
        "src": "block:1",
        "content": { "type": "paragraph", "rich_text": { "segments": [] } },
        "id": "018f3c2a-0000-0000-0000-000000000000"
    });
    let error = serde_json::from_value::<ContentFragment>(with_id).unwrap_err();
    assert!(
        error.to_string().contains("`id` is not a Fragment field"),
        "{error}"
    );

    let with_unknown = json!({
        "src": "block:1",
        "content": { "type": "paragraph", "rich_text": { "segments": [] } },
        "flavor": "mint"
    });
    let block: ContentFragment = serde_json::from_value(with_unknown).unwrap();
    assert_eq!(block.src, key("block:1"));

    let landmark_with_id = json!({ "src": "page:1", "title": { "segments": [] }, "id": "x" });
    assert!(serde_json::from_value::<LandmarkFragment>(landmark_with_id).is_err());
    let protochart_with_id = json!({ "src": "db:1", "name": "Tasks", "id": "x" });
    assert!(serde_json::from_value::<Protochart>(protochart_with_id).is_err());
    let edge_with_id = json!({ "src": "a", "dst": "b", "kind": { "type": "child" }, "id": "x" });
    assert!(serde_json::from_value::<TerrainFragment>(edge_with_id).is_err());
}

#[test]
fn an_empty_source_key_is_refused_everywhere() {
    let empty = json!({ "src": "", "title": { "segments": [] } });
    assert!(serde_json::from_value::<LandmarkFragment>(empty).is_err());
}

#[test]
fn the_census_counts_what_the_fragments_use() {
    let mut census = Census::default();
    sample().tally(&mut census);
    assert_eq!(
        census.content.iter().cloned().collect::<Vec<_>>(),
        ["heading-2", "paragraph"]
    );
    assert_eq!(
        census.readings.iter().cloned().collect::<Vec<_>>(),
        ["select"]
    );
    assert_eq!(
        census.feature_kinds.iter().cloned().collect::<Vec<_>>(),
        ["select"]
    );
    assert_eq!(
        census.terrain_kinds.iter().cloned().collect::<Vec<_>>(),
        ["child"]
    );
}

#[test]
fn the_finalize_output_is_stamped() {
    let mut census = Census::default();
    let fragments = sample();
    fragments.tally(&mut census);
    let summary = Summary {
        landmarks: 1,
        content: 3,
        protocharts: 1,
        terrain: 1,
    };
    let output = FinalizeOutput::stamp(
        Output {
            fragments,
            diagnostics: vec![],
        },
        census,
        summary,
    );
    assert_eq!(output.fragment_format, FRAGMENT_FORMAT);
    assert_eq!(output.domain_version, domain::VERSION);
    let text = serde_json::to_string(&output).unwrap();
    let back: FinalizeOutput = serde_json::from_str(&text).unwrap();
    assert_eq!(back, output);
}

#[test]
fn envelopes_are_ok_or_err() {
    let ok: Envelope<Init> = Ok(Init { state: 1 }).into();
    assert_eq!(serde_json::to_string(&ok).unwrap(), r#"{"ok":{"state":1}}"#);
    let err: Envelope<Init> = Err(Diagnostic::error("no")).into();
    assert_eq!(
        serde_json::to_string(&err).unwrap(),
        r#"{"err":{"level":"error","msg":"no"}}"#
    );
    let back: Envelope<Init> = serde_json::from_str(r#"{"ok":{"state":7,"later":true}}"#).unwrap();
    assert_eq!(back.into_result().unwrap().state, 7);
}

// ── Mirrors are exhaustive ───────────────────────────────────────────────────
//
// Each conversion below is a non-wildcard match (or a full destructuring), so a
// variant or field added to the data model without a Fragment counterpart is a
// compile error here. `TerrainKind::Reading` is the one documented exclusion.

fn mirror_reading(reading: &Reading) -> ReadingFragment {
    let keys = |glyphs: &[domain::GlyphID]| {
        glyphs
            .iter()
            .map(|glyph| key(&glyph.to_string()))
            .collect::<Vec<_>>()
    };
    match reading {
        Reading::Text(text) => ReadingFragment::Text(text.clone()),
        Reading::Number(n) => ReadingFragment::Number(*n),
        Reading::Bool(b) => ReadingFragment::Bool(*b),
        Reading::Date(at) => ReadingFragment::Date(*at),
        Reading::DateRange(from, to) => ReadingFragment::DateRange(*from, *to),
        Reading::Select(glyph) => ReadingFragment::Select(key(&glyph.to_string())),
        Reading::MultiSelect(glyphs) => ReadingFragment::MultiSelect(keys(glyphs)),
        Reading::GroupedSelect(groups) => ReadingFragment::GroupedSelect(
            groups
                .iter()
                .map(|(group, options)| (key(&group.to_string()), keys(options)))
                .collect::<BTreeMap<_, _>>(),
        ),
        Reading::References(marks) => {
            ReadingFragment::References(marks.iter().map(|m| key(&m.to_string())).collect())
        }
        Reading::URL(url) => ReadingFragment::URL(url.clone()),
        Reading::Email(email) => ReadingFragment::Email(email.clone()),
        Reading::Phone(a, b) => ReadingFragment::Phone(a.clone(), b.clone()),
        Reading::File(file) => ReadingFragment::File(file.clone()),
        Reading::Formula(formula) => ReadingFragment::Formula(formula.clone()),
        Reading::Null => ReadingFragment::Null,
    }
}

fn mirror_option(option: &SelectOption) -> (SourceKey, SelectOptionFragment) {
    let SelectOption {
        id,
        name,
        description,
        icon,
        color,
    } = option;
    (
        key(&id.to_string()),
        SelectOptionFragment {
            name: name.clone(),
            description: description.clone(),
            icon: icon.clone(),
            color: color.clone(),
        },
    )
}

fn mirror_group(group: &SelectGroup) -> (SourceKey, SelectGroupFragment) {
    let SelectGroup {
        id,
        name,
        description,
        options,
        max_selections,
        default_options,
    } = group;
    (
        key(&id.to_string()),
        SelectGroupFragment {
            name: name.clone(),
            description: description.clone(),
            options: options.values().map(mirror_option).collect(),
            max_selections: *max_selections,
            default_options: default_options
                .iter()
                .map(|g| key(&g.to_string()))
                .collect(),
        },
    )
}

fn mirror_feature_kind(kind: &FeatureKind) -> FeatureKindFragment {
    match kind {
        FeatureKind::Text => FeatureKindFragment::Text,
        FeatureKind::Number {
            min,
            max,
            step,
            precision,
            format,
        } => FeatureKindFragment::Number {
            min: *min,
            max: *max,
            step: *step,
            precision: *precision,
            format: format.clone(),
        },
        FeatureKind::Checkbox => FeatureKindFragment::Checkbox,
        FeatureKind::Date {
            min_date,
            min_time,
            max_date,
            max_time,
            timezone,
            granularity,
        } => FeatureKindFragment::Date {
            min_date: *min_date,
            min_time: *min_time,
            max_date: *max_date,
            max_time: *max_time,
            timezone: timezone.clone(),
            granularity: *granularity,
        },
        FeatureKind::Email => FeatureKindFragment::Email,
        FeatureKind::Phone => FeatureKindFragment::Phone,
        FeatureKind::URL => FeatureKindFragment::URL,
        FeatureKind::Select {
            options,
            default_option,
        } => FeatureKindFragment::Select {
            options: options.values().map(mirror_option).collect(),
            default_option: default_option.as_ref().map(|g| key(&g.to_string())),
        },
        FeatureKind::MultiSelect {
            options,
            max_selections,
            default_options,
        } => FeatureKindFragment::MultiSelect {
            options: options.values().map(mirror_option).collect(),
            max_selections: *max_selections,
            default_options: default_options
                .iter()
                .map(|g| key(&g.to_string()))
                .collect(),
        },
        FeatureKind::GroupedSelect {
            groups,
            default_option,
        } => FeatureKindFragment::GroupedSelect {
            groups: groups.values().map(mirror_group).collect(),
            default_option: default_option.as_ref().map(|g| key(&g.to_string())),
        },
        FeatureKind::GroupedMultiSelect {
            groups,
            max_selections,
            default_options,
        } => FeatureKindFragment::GroupedMultiSelect {
            groups: groups.values().map(mirror_group).collect(),
            max_selections: *max_selections,
            default_options: default_options
                .iter()
                .map(|(g, o)| (key(&g.to_string()), key(&o.to_string())))
                .collect(),
        },
        FeatureKind::Reference {
            allowed_types,
            max_references,
            display_field,
            display_mode,
            filter_criteria,
        } => FeatureKindFragment::Reference {
            allowed_types: allowed_types
                .as_ref()
                .map(|charts| charts.iter().map(|c| key(&c.0)).collect()),
            max_references: *max_references,
            display_field: display_field.clone(),
            display_mode: display_mode.clone(),
            filter_criteria: filter_criteria.clone(),
        },
        FeatureKind::Formula => FeatureKindFragment::Formula,
    }
}

/// `None` is the exclusion: a reading edge is a derived index the core builds.
fn mirror_terrain_kind(kind: &TerrainKind) -> Option<TerrainKindFragment> {
    match kind {
        TerrainKind::Child => Some(TerrainKindFragment::Child),
        TerrainKind::Reference => Some(TerrainKindFragment::Reference),
        TerrainKind::Custom => Some(TerrainKindFragment::Custom),
        TerrainKind::Reading(_, _) => None,
    }
}

#[test]
fn mirrors_cover_their_originals() {
    assert_eq!(mirror_reading(&Reading::Null), ReadingFragment::Null);
    assert_eq!(
        mirror_reading(&Reading::Select(domain::GlyphID::derived("todo"))),
        ReadingFragment::Select(key(&domain::GlyphID::derived("todo").to_string()))
    );
    assert_eq!(
        mirror_feature_kind(&FeatureKind::Text),
        FeatureKindFragment::Text
    );
    assert_eq!(
        mirror_terrain_kind(&TerrainKind::Child),
        Some(TerrainKindFragment::Child)
    );
    assert_eq!(
        mirror_terrain_kind(&TerrainKind::Reading(
            domain::ChartID::builtin("page"),
            domain::FeatureID::derived("title")
        )),
        None
    );
}

#[test]
fn mirrored_variants_serialize_under_the_same_tags() {
    // The serde tag of a mirrored variant equals its original's, so the census
    // vocabulary is shared with Bundles.
    let tag = |value: &serde_json::Value| value["type"].as_str().unwrap().to_string();
    assert_eq!(
        tag(&serde_json::to_value(ReadingFragment::Null).unwrap()),
        tag(&serde_json::to_value(Reading::Null).unwrap())
    );
    assert_eq!(
        tag(&serde_json::to_value(FeatureKindFragment::URL).unwrap()),
        tag(&serde_json::to_value(FeatureKind::URL).unwrap())
    );
    assert_eq!(
        tag(&serde_json::to_value(TerrainKindFragment::Custom).unwrap()),
        tag(&serde_json::to_value(TerrainKind::Custom).unwrap())
    );
}
