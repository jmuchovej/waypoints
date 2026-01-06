use crate::Json;
use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
pub struct RichText {
    pub plain_text: String,
    #[serde(default)]
    pub marks: Vec<TextMark>,
}

impl RichText {
    #[frb(sync)]
    pub fn is_empty(&self) -> bool {
        self.plain_text.is_empty() && self.marks.is_empty()
    }
}

impl Default for RichText {
    #[frb(ignore)]
    fn default() -> Self {
        RichText {
            plain_text: String::new(),
            marks: Vec::new(),
        }
    }
}

impl From<String> for RichText {
    // TODO: implement markdown parsing?
    fn from(text: String) -> Self {
        RichText {
            plain_text: text,
            marks: Vec::new(),
        }
    }
}

/// Text formatting marks (bold, italic, links, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
pub struct TextMark {
    /// Type of formatting
    pub kind: TextMarkKind,

    /// Start position in the text (Unicode character offset)
    pub start: usize,

    /// End position in the text (Unicode character offset)
    pub end: usize,

    /// Additional data for the mark (e.g., URL for links)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
}

impl TextMark {
    /// Validate that the mark has valid start/end positions
    #[frb(sync)]
    pub fn is_valid(&self) -> bool {
        self.start <= self.end
    }

    /// Check if this mark overlaps with another mark
    #[frb(sync)]
    pub fn overlaps_with(&self, other: &TextMark) -> bool {
        !(self.end <= other.start || other.end <= self.start)
    }

    /// Get the length of this mark
    #[frb(sync)]
    pub fn length(&self) -> usize {
        if self.end >= self.start {
            self.end - self.start
        } else {
            0
        }
    }
}

/// Types of text formatting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[frb(non_opaque, unignore)]
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
    fn test_text_mark_creation() {
        let mark = TextMark {
            kind: TextMarkKind::Bold,
            start: 0,
            end: 5,
            data: None,
        };

        assert_eq!(mark.kind, TextMarkKind::Bold);
        assert_eq!(mark.start, 0);
        assert_eq!(mark.end, 5);
        assert_eq!(mark.length(), 5);
        assert!(mark.is_valid());
    }

    #[test]
    fn test_text_mark_with_data() {
        let link_data = serde_json::json!({"url": "https://example.com"});
        let link_mark = TextMark {
            kind: TextMarkKind::Link,
            start: 5,
            end: 15,
            data: Some(link_data.clone()),
        };

        assert_eq!(link_mark.kind, TextMarkKind::Link);
        assert_eq!(link_mark.data, Some(link_data));
        assert!(link_mark.is_valid());
    }

    #[test]
    fn test_text_mark_validation() {
        let valid_mark = TextMark {
            kind: TextMarkKind::Italic,
            start: 5,
            end: 10,
            data: None,
        };

        let invalid_mark = TextMark {
            kind: TextMarkKind::Bold,
            start: 10,
            end: 5, // end before start
            data: None,
        };

        let zero_length_mark = TextMark {
            kind: TextMarkKind::Code,
            start: 5,
            end: 5,
            data: None,
        };

        assert!(valid_mark.is_valid());
        assert!(!invalid_mark.is_valid());
        assert!(zero_length_mark.is_valid()); // Zero-length is valid (cursor position)

        assert_eq!(valid_mark.length(), 5);
        assert_eq!(invalid_mark.length(), 0); // Invalid marks return 0 length
        assert_eq!(zero_length_mark.length(), 0);
    }

    #[test]
    fn test_text_mark_overlap_detection() {
        let mark1 = TextMark {
            kind: TextMarkKind::Bold,
            start: 0,
            end: 10,
            data: None,
        };

        let mark2 = TextMark {
            kind: TextMarkKind::Italic,
            start: 5,
            end: 15, // Overlaps with mark1
            data: None,
        };

        let mark3 = TextMark {
            kind: TextMarkKind::Link,
            start: 20,
            end: 30, // No overlap
            data: None,
        };

        let mark4 = TextMark {
            kind: TextMarkKind::Code,
            start: 10,
            end: 20, // Adjacent, no overlap
            data: None,
        };

        assert!(mark1.overlaps_with(&mark2));
        assert!(mark2.overlaps_with(&mark1)); // Symmetric
        assert!(!mark1.overlaps_with(&mark3));
        assert!(!mark1.overlaps_with(&mark4)); // Adjacent is not overlapping
        assert!(!mark3.overlaps_with(&mark4));
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

        for (i, mark_type) in mark_types.into_iter().enumerate() {
            let mark = TextMark {
                kind: mark_type.clone(),
                start: i * 5,
                end: i * 5 + 4,
                data: None,
            };

            assert_eq!(mark.kind, mark_type);
            assert!(mark.is_valid());
        }
    }

    #[test]
    fn test_mark_serialization() {
        let mark = TextMark {
            kind: TextMarkKind::Link,
            start: 0,
            end: 10,
            data: Some(serde_json::json!({"url": "https://example.com", "title": "Example"})),
        };

        let json = serde_json::to_string(&mark).unwrap();
        let deserialized: TextMark = serde_json::from_str(&json).unwrap();

        assert_eq!(mark, deserialized);
    }

    #[test]
    fn test_rich_text_creation() {
        let rich_text = RichText {
            plain_text: "Hello, world!".to_string(),
            marks: vec![],
        };

        assert_eq!(rich_text.plain_text, "Hello, world!");
        assert!(rich_text.marks.is_empty());
    }

    #[test]
    fn test_rich_text_with_marks() {
        let rich_text = RichText {
            plain_text: "Hello, bold world!".to_string(),
            marks: vec![
                TextMark {
                    kind: TextMarkKind::Bold,
                    start: 7,
                    end: 11,
                    data: None,
                },
                TextMark {
                    kind: TextMarkKind::Italic,
                    start: 12,
                    end: 17,
                    data: None,
                },
            ],
        };

        assert_eq!(rich_text.plain_text, "Hello, bold world!");
        assert_eq!(rich_text.marks.len(), 2);
        assert_eq!(rich_text.marks[0].kind, TextMarkKind::Bold);
        assert_eq!(rich_text.marks[1].kind, TextMarkKind::Italic);
    }

    #[test]
    fn test_rich_text_serialization() {
        let rich_text = RichText {
            plain_text: "Click [here](https://example.com) for more info".to_string(),
            marks: vec![TextMark {
                kind: TextMarkKind::Link,
                start: 6,
                end: 34,
                data: Some(serde_json::json!({"url": "https://example.com"})),
            }],
        };

        let json = serde_json::to_string(&rich_text).unwrap();
        let deserialized: RichText = serde_json::from_str(&json).unwrap();

        assert_eq!(rich_text, deserialized);
        assert_eq!(
            deserialized.marks[0].data,
            Some(serde_json::json!({"url": "https://example.com"}))
        );
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
    fn test_complex_mark_data() {
        let mention_mark = TextMark {
            kind: TextMarkKind::Mention,
            start: 0,
            end: 10,
            data: Some(serde_json::json!({
                "user_id": "user123",
                "display_name": "@alice",
                "type": "user"
            })),
        };

        let color_mark = TextMark {
            kind: TextMarkKind::Color,
            start: 0,
            end: 5,
            data: Some(serde_json::json!({
                "color": "#FF0000"
            })),
        };

        assert_eq!(mention_mark.kind, TextMarkKind::Mention);
        assert!(mention_mark.data.is_some());

        assert_eq!(color_mark.kind, TextMarkKind::Color);
        assert!(color_mark.data.is_some());
    }

    #[test]
    fn test_nested_marks_validation() {
        // Test that we can have multiple overlapping marks
        let rich_text = RichText {
            plain_text: "This is bold and italic text".to_string(),
            marks: vec![
                TextMark {
                    kind: TextMarkKind::Bold,
                    start: 8,
                    end: 28, // "bold and italic text"
                    data: None,
                },
                TextMark {
                    kind: TextMarkKind::Italic,
                    start: 17,
                    end: 28, // "italic text"
                    data: None,
                },
            ],
        };

        assert_eq!(rich_text.marks.len(), 2);
        assert!(rich_text.marks[0].overlaps_with(&rich_text.marks[1]));
        assert!(rich_text.marks[1].overlaps_with(&rich_text.marks[0]));
    }

    #[test]
    fn test_mark_edge_cases() {
        // Zero-length mark (cursor position)
        let cursor_mark = TextMark {
            kind: TextMarkKind::Comment,
            start: 5,
            end: 5,
            data: Some(serde_json::json!({"comment": "Note: cursor position"})),
        };

        assert!(cursor_mark.is_valid());
        assert_eq!(cursor_mark.length(), 0);

        // Full text mark
        let full_text = "Hello world";
        let full_mark = TextMark {
            kind: TextMarkKind::Highlight,
            start: 0,
            end: full_text.len(),
            data: None,
        };

        assert!(full_mark.is_valid());
        assert_eq!(full_mark.length(), full_text.len());
    }
}
