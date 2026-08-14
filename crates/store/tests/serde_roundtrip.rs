//! Serde roundtrip tests — guards the Rust↔TS boundary.
//!
//! Every MarkData variant must survive serialize→deserialize through JSON,
//! both in isolation and through the ORM layer.

mod fixtures;

use domain::MathEngine;
use domain::{ChartID, FeatureID};
use domain::{
    Content, InlineMark, Landmark, MarkData, Reading, RichText, TextMarkKind, TextSegment,
};
use fixtures::*;
use rstest::rstest;
use store::Store;
use store::query::mark;

// ---------------------------------------------------------------------------
// Pure JSON roundtrips — no database, parametrized over every Content variant
// ---------------------------------------------------------------------------

#[rstest]
#[case::paragraph(paragraph("hello"))]
#[case::heading1(heading("Title"))]
#[case::blockquote(MarkData::Content(Content::blockquote()))]
#[case::divider(MarkData::Content(Content::divider()))]
#[case::code_block(MarkData::Content(Content::code("rust".to_string(), vec!["fn main() {}".to_string()], None, None)))]
#[case::callout(MarkData::Content(Content::callout("💡".to_string())))]
#[case::table(MarkData::Content(Content::Table {
    cells: vec![
        vec![RichText::from("A".to_string()), RichText::from("B".to_string())],
        vec![RichText::from("1".to_string()), RichText::from("2".to_string())],
    ],
    col_size: None,
    row_size: None,
    col_header: Some(true),
    row_header: None,
    fill_width: None,
    col_colors: None,
    row_colors: None,
    theme: None,
}))]
#[case::math_typst(MarkData::Content(Content::math("x^2 + y^2 = z^2".to_string(), MathEngine::Typst)))]
#[case::math_latex(MarkData::Content(Content::math(r"\int_0^1 x dx".to_string(), MathEngine::LaTeX)))]
#[case::list_item(MarkData::Content(Content::list_item(RichText::from("bullet".to_string()))))]
#[case::enum_item(MarkData::Content(Content::enum_item(RichText::from("numbered".to_string()))))]
#[case::landmark_page(landmark("page", "Test Page"))]
#[case::landmark_note(landmark("note", "My Note"))]
fn json_roundtrip(#[case] data: MarkData) {
    let json = serde_json::to_value(&data).unwrap();
    let back: MarkData = serde_json::from_value(json).unwrap();
    assert_eq!(data, back);
}

// ---------------------------------------------------------------------------
// Inline mark roundtrips — richer structures that deserve individual attention
// ---------------------------------------------------------------------------

#[rstest]
#[case::bold(TextMarkKind::Bold, None)]
#[case::italic(TextMarkKind::Italic, None)]
#[case::underline(TextMarkKind::Underline, None)]
#[case::strikethrough(TextMarkKind::Strikethrough, None)]
#[case::code(TextMarkKind::Code, None)]
#[case::superscript(TextMarkKind::Superscript, None)]
#[case::subscript(TextMarkKind::Subscript, None)]
#[case::spoiler(TextMarkKind::Spoiler, None)]
#[case::link(TextMarkKind::Link, Some(serde_json::json!("https://example.com")))]
fn inline_mark_roundtrip(#[case] kind: TextMarkKind, #[case] data: Option<serde_json::Value>) {
    let mark_data = MarkData::Content(Content::paragraph(RichText::new(vec![TextSegment {
        text: "decorated".to_string(),
        marks: vec![InlineMark { kind, data }],
    }])));
    let json = serde_json::to_value(&mark_data).unwrap();
    let back: MarkData = serde_json::from_value(json).unwrap();
    assert_eq!(mark_data, back);
}

#[test]
fn multi_segment_with_mixed_marks_roundtrip() {
    let data = MarkData::Content(Content::paragraph(RichText::new(vec![
        TextSegment {
            text: "bold".to_string(),
            marks: vec![InlineMark {
                kind: TextMarkKind::Bold,
                data: None,
            }],
        },
        TextSegment {
            text: " plain ".to_string(),
            marks: vec![],
        },
        TextSegment {
            text: "italic+code".to_string(),
            marks: vec![
                InlineMark {
                    kind: TextMarkKind::Italic,
                    data: None,
                },
                InlineMark {
                    kind: TextMarkKind::Code,
                    data: None,
                },
            ],
        },
    ])));
    let json = serde_json::to_value(&data).unwrap();
    let back: MarkData = serde_json::from_value(json).unwrap();
    assert_eq!(data, back);
}

#[test]
fn landmark_with_readings_roundtrip() {
    let feature_id = FeatureID::new();
    let data = MarkData::Landmark(
        Landmark::builder()
            .with_chart(ChartID::builtin("note"))
            .with_title("My Note".to_string())
            .with_reading(feature_id, Reading::Text("some value".to_string().into()))
            .build(),
    );
    let json = serde_json::to_value(&data).unwrap();
    let back: MarkData = serde_json::from_value(json).unwrap();
    assert_eq!(data, back);
}

// ---------------------------------------------------------------------------
// Database roundtrips — data survives the full bedrock path (insert → fetch)
// ---------------------------------------------------------------------------

#[rstest]
#[case::paragraph(paragraph("database roundtrip"))]
#[case::heading(heading("DB Title"))]
#[case::code(MarkData::Content(Content::code("python".to_string(), vec!["def greet():".to_string(), "    print('hello')".to_string()], None, None)))]
#[case::landmark(landmark("page", "Persisted Page"))]
#[case::inline_marks(MarkData::Content(Content::paragraph(RichText::new(vec![
    TextSegment {
        text: "bold".to_string(),
        marks: vec![InlineMark { kind: TextMarkKind::Bold, data: None }],
    },
    TextSegment {
        text: " then ".to_string(),
        marks: vec![],
    },
    TextSegment {
        text: "strikethrough".to_string(),
        marks: vec![InlineMark { kind: TextMarkKind::Strikethrough, data: None }],
    },
]))))]
#[tokio::test]
async fn database_roundtrip(#[future] db: Store, #[case] data: MarkData) {
    let db = db.await;
    let id = new_id();
    mark::insert(&db, id, &data, now()).await.unwrap();

    let fetched = mark::get(&db, id).await.unwrap().unwrap();
    assert_eq!(fetched.data, data);
}
