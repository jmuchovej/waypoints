use crate::Json;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(from = "RichTextDTO")]
pub struct RichText {
    plain_text: String,
    segments: Vec<TextSegment>,
}

/// Deserialization DTO that routes through `RichText::new()`.
#[derive(Deserialize)]
struct RichTextDTO {
    segments: Vec<TextSegment>,
}

impl From<RichTextDTO> for RichText {
    fn from(dto: RichTextDTO) -> Self {
        RichText::new(dto.segments)
    }
}

impl RichText {
    pub fn new(segments: Vec<TextSegment>) -> Self {
        let plain_text = segments.iter().map(|s| s.text.as_str()).collect();
        Self {
            plain_text,
            segments,
        }
    }

    pub fn plain_text(&self) -> &str {
        &self.plain_text
    }

    pub fn segments(&self) -> &[TextSegment] {
        &self.segments
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl Default for RichText {
    fn default() -> Self {
        RichText::new(Vec::new())
    }
}

impl From<String> for RichText {
    fn from(text: String) -> Self {
        if text.is_empty() {
            return RichText::default();
        }
        RichText::new(vec![TextSegment {
            text,
            marks: Vec::new(),
        }])
    }
}

/// A contiguous run of text sharing the same set of inline marks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct TextSegment {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<InlineMark>,
}

/// An inline formatting mark (bold, italic, link, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct InlineMark {
    pub kind: TextMarkKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
}

/// Types of text formatting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum TextMarkKind {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Superscript,
    Subscript,
    Code,
    Link,
    Mention,
    Highlight,
    Color,
    BackgroundColor,
    FontSize,
    FontFamily,
    Math,
    Spoiler,
    Comment,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_creation() {
        let seg = TextSegment {
            text: "hello".to_string(),
            marks: vec![InlineMark {
                kind: TextMarkKind::Bold,
                data: None,
            }],
        };
        assert_eq!(seg.text, "hello");
        assert_eq!(seg.marks.len(), 1);
        assert_eq!(seg.marks[0].kind, TextMarkKind::Bold);
    }

    #[test]
    fn test_inline_mark_with_data() {
        let m = InlineMark {
            kind: TextMarkKind::Link,
            data: Some(serde_json::json!("https://example.com")),
        };
        assert_eq!(m.kind, TextMarkKind::Link);
        assert_eq!(m.data, Some(serde_json::json!("https://example.com")));
    }

    #[test]
    fn test_rich_text_creation() {
        let rt = RichText::new(vec![TextSegment {
            text: "Hello, world!".to_string(),
            marks: vec![],
        }]);
        assert_eq!(rt.plain_text(), "Hello, world!");
        assert_eq!(rt.segments().len(), 1);
    }

    #[test]
    fn test_rich_text_with_marks() {
        let rt = RichText::new(vec![
            TextSegment {
                text: "Hello, ".to_string(),
                marks: vec![],
            },
            TextSegment {
                text: "bold".to_string(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                }],
            },
            TextSegment {
                text: " world!".to_string(),
                marks: vec![],
            },
        ]);
        assert_eq!(rt.plain_text(), "Hello, bold world!");
        assert_eq!(rt.segments().len(), 3);
        assert_eq!(rt.segments()[1].marks[0].kind, TextMarkKind::Bold);
    }

    #[test]
    fn test_from_string() {
        let rt = RichText::from("Hello".to_string());
        assert_eq!(rt.plain_text(), "Hello");
        assert_eq!(rt.segments().len(), 1);
        assert_eq!(rt.segments()[0].text, "Hello");
        assert!(rt.segments()[0].marks.is_empty());
    }

    #[test]
    fn test_from_empty_string() {
        let rt = RichText::from(String::new());
        assert!(rt.is_empty());
        assert_eq!(rt.segments().len(), 0);
    }

    #[test]
    fn test_rich_text_serialization() {
        let rt = RichText::new(vec![
            TextSegment {
                text: "Click ".to_string(),
                marks: vec![],
            },
            TextSegment {
                text: "here".to_string(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Link,
                    data: Some(serde_json::json!("https://example.com")),
                }],
            },
            TextSegment {
                text: " for more".to_string(),
                marks: vec![],
            },
        ]);

        let json = serde_json::to_string(&rt).unwrap();
        let deserialized: RichText = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, deserialized);
    }

    #[test]
    fn test_mark_type_serialization() {
        let mark_types = vec![
            (TextMarkKind::Bold, "\"bold\""),
            (TextMarkKind::Italic, "\"italic\""),
            (TextMarkKind::Link, "\"link\""),
            (TextMarkKind::Code, "\"code\""),
            (TextMarkKind::Highlight, "\"highlight\""),
        ];

        for (mark_type, expected_json) in mark_types {
            let json = serde_json::to_string(&mark_type).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: TextMarkKind = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, mark_type);
        }
    }

    #[test]
    fn test_all_mark_types() {
        use TextMarkKind::*;
        let mark_types = vec![
            Bold,
            Italic,
            Underline,
            Strikethrough,
            Superscript,
            Subscript,
            Code,
            Link,
            Mention,
            Highlight,
            Color,
            BackgroundColor,
            FontSize,
            FontFamily,
            Spoiler,
            Comment,
        ];

        for mark_type in mark_types {
            let m = InlineMark {
                kind: mark_type.clone(),
                data: None,
            };
            assert_eq!(m.kind, mark_type);
        }
    }

    #[test]
    fn test_empty_marks_skipped_in_serialization() {
        let seg = TextSegment {
            text: "plain".to_string(),
            marks: vec![],
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(!json.contains("marks"));

        let seg_with_marks = TextSegment {
            text: "bold".to_string(),
            marks: vec![InlineMark {
                kind: TextMarkKind::Bold,
                data: None,
            }],
        };
        let json = serde_json::to_string(&seg_with_marks).unwrap();
        assert!(json.contains("marks"));
    }

    #[test]
    fn test_nested_marks_on_segment() {
        let seg = TextSegment {
            text: "bold italic".to_string(),
            marks: vec![
                InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                },
                InlineMark {
                    kind: TextMarkKind::Italic,
                    data: None,
                },
            ],
        };
        assert_eq!(seg.marks.len(), 2);
    }

    #[test]
    fn test_deserialization_computes_plain_text() {
        let json = r#"{"segments":[{"text":"hello"},{"text":" world","marks":[{"kind":"bold"}]}]}"#;
        let rt: RichText = serde_json::from_str(json).unwrap();
        assert_eq!(rt.plain_text(), "hello world");
    }
}
