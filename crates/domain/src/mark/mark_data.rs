use super::{MarkID, Reading, ReferenceDisplayMode, RichReference, RichText};
use crate::{ChartID, Color, FeatureID, Icon};
use derive_more::{IsVariant, Unwrap};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use typed_builder::TypedBuilder;

/// A wrapper for [`Content`] and [`Landmark`] data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IsVariant, Type, bedrock::Document)]
#[serde(tag = "mark", rename_all = "kebab-case")]
pub enum MarkData {
    /// [`Content`] marks. These hold most user-generated data (i.e., paragraphs, headings, etc.).
    Content(Content),

    /// [`Landmark`] marks. These are instances of the various kinds of [`Charts`][C] a user has.
    ///
    /// [C]: crate::chart::Chart
    Landmark(Landmark),
}

impl MarkData {
    /// Produces a `type_name` for a `MarkData`. Suppose that `MarkData` is a
    /// [`MarkData::Content`] with a [`Content::Paragraph`] variant: the `type_name`
    /// should be `"content.paragraph"`. Similarly, if `MarkData` is a
    /// [`MarkData::Landmark`] named "Person", then the `type_name` should be
    /// `"landmark.person"`.
    pub fn type_name(&self) -> String {
        use MarkData::*;
        match self {
            Content(c) => format!("content.{inner}", inner = c.type_name()),
            Landmark(l) => format!("landmark.{inner}", inner = l.type_name()),
        }
    }
}

impl Default for MarkData {
    fn default() -> Self {
        MarkData::Content(Content::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Type)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Size {
    /// Fixed size in pixels
    Fixed(u16),
    /// Fractional size of available space
    Ratio(f32),
    /// Auto-size based on content (and table width)
    #[default]
    Auto,
    Min,
    Max,
}

/// Visual theme for a content block. Applies both ink (foreground) and
/// surface (background) styling from the selected color. Independent
/// ink/surface control is handled at the inline mark level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct BlockTheme {
    pub color: Color,
}

/// Content blocks - the actual content pieces users work with.
/// These are "dumb" blocks with minimal built-in properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Content {
    /// The base paragraph (the most common form of content)
    Paragraph {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Level 1 Heading
    #[serde(rename = "heading-1")]
    Heading1 {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Level 2 Heading
    #[serde(rename = "heading-2")]
    Heading2 {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Level 3 Heading
    #[serde(rename = "heading-3")]
    Heading3 {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// A quote
    Blockquote {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// An unordered list item
    ListItem {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// An ordered list item
    EnumItem {
        rich_text: RichText,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Callout
    Callout {
        icon: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Code block with syntax highlighting
    Code {
        language: String,
        lines: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        highlights: Option<Vec<u32>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Inline table
    Table {
        cells: Vec<Vec<RichText>>,
        col_size: Option<Vec<Option<Size>>>,
        row_size: Option<Vec<Option<Size>>>,
        col_header: Option<bool>,
        row_header: Option<bool>,
        fill_width: Option<bool>,
        col_colors: Option<Vec<Option<Color>>>,
        row_colors: Option<Vec<Option<Color>>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Typst/LaTeX content
    Math {
        expression: String,
        engine: MathEngine,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },

    /// Horizontal divider
    Divider {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<BlockTheme>,
    },
}

impl Default for Content {
    fn default() -> Self {
        Content::Paragraph {
            rich_text: RichText::default(),
            theme: None,
        }
    }
}

/// Math rendering engines
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Type)]
#[serde(rename_all = "lowercase")]
pub enum MathEngine {
    #[default]
    Typst,
    LaTeX,
}

impl Content {
    // ── Constructors ─────────────────────────────────────────────────────

    pub fn paragraph(rich_text: RichText) -> Self {
        Self::Paragraph {
            rich_text,
            theme: None,
        }
    }
    pub fn heading1(rich_text: RichText) -> Self {
        Self::Heading1 {
            rich_text,
            theme: None,
        }
    }
    pub fn heading2(rich_text: RichText) -> Self {
        Self::Heading2 {
            rich_text,
            theme: None,
        }
    }
    pub fn heading3(rich_text: RichText) -> Self {
        Self::Heading3 {
            rich_text,
            theme: None,
        }
    }
    pub fn blockquote() -> Self {
        Self::Blockquote { theme: None }
    }
    pub fn list_item(rich_text: RichText) -> Self {
        Self::ListItem {
            rich_text,
            theme: None,
        }
    }
    pub fn enum_item(rich_text: RichText) -> Self {
        Self::EnumItem {
            rich_text,
            theme: None,
        }
    }
    pub fn callout(icon: String) -> Self {
        Self::Callout {
            icon,
            theme: Some(BlockTheme {
                color: Color::default(),
            }),
        }
    }
    pub fn code(
        language: String,
        lines: Vec<String>,
        filename: Option<String>,
        highlights: Option<Vec<u32>>,
    ) -> Self {
        Self::Code {
            language,
            lines,
            filename,
            highlights,
            theme: None,
        }
    }
    pub fn divider() -> Self {
        Self::Divider { theme: None }
    }
    pub fn math(expression: String, engine: MathEngine) -> Self {
        Self::Math {
            expression,
            engine,
            theme: None,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    /// Get a reference to the rich text field, if this variant has one.
    pub fn rich_text(&self) -> Option<&RichText> {
        match self {
            Content::Paragraph { rich_text, .. }
            | Content::Heading1 { rich_text, .. }
            | Content::Heading2 { rich_text, .. }
            | Content::Heading3 { rich_text, .. }
            | Content::ListItem { rich_text, .. }
            | Content::EnumItem { rich_text, .. } => Some(rich_text),
            _ => None,
        }
    }

    /// Get a mutable reference to the rich text field, if this variant has one.
    pub fn rich_text_mut(&mut self) -> Option<&mut RichText> {
        match self {
            Content::Paragraph { rich_text, .. }
            | Content::Heading1 { rich_text, .. }
            | Content::Heading2 { rich_text, .. }
            | Content::Heading3 { rich_text, .. }
            | Content::ListItem { rich_text, .. }
            | Content::EnumItem { rich_text, .. } => Some(rich_text),
            _ => None,
        }
    }

    /// Set the rich text field on variants that have one.
    /// Returns `false` if the variant has no rich text field.
    pub fn set_rich_text(&mut self, new_rt: RichText) -> bool {
        match self {
            Content::Paragraph { rich_text, .. }
            | Content::Heading1 { rich_text, .. }
            | Content::Heading2 { rich_text, .. }
            | Content::Heading3 { rich_text, .. }
            | Content::ListItem { rich_text, .. }
            | Content::EnumItem { rich_text, .. } => {
                *rich_text = new_rt;
                true
            }
            _ => false,
        }
    }

    /// Get the block theme, if set.
    pub fn theme(&self) -> Option<&BlockTheme> {
        match self {
            Content::Paragraph { theme, .. }
            | Content::Heading1 { theme, .. }
            | Content::Heading2 { theme, .. }
            | Content::Heading3 { theme, .. }
            | Content::Blockquote { theme, .. }
            | Content::ListItem { theme, .. }
            | Content::EnumItem { theme, .. }
            | Content::Callout { theme, .. }
            | Content::Code { theme, .. }
            | Content::Table { theme, .. }
            | Content::Math { theme, .. }
            | Content::Divider { theme, .. } => theme.as_ref(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Content::Paragraph { .. } => "paragraph",
            Content::Heading1 { .. } => "heading-1",
            Content::Heading2 { .. } => "heading-2",
            Content::Heading3 { .. } => "heading-3",
            Content::Blockquote { .. } => "blockquote",
            Content::ListItem { .. } => "list-item",
            Content::EnumItem { .. } => "enum-item",
            Content::Callout { .. } => "callout",
            Content::Code { .. } => "code",
            Content::Table { .. } => "table",
            Content::Math { .. } => "math",
            Content::Divider { .. } => "divider",
        }
    }
}

/// Landmark marks &ndash; semantic objects defined by [`Charts`][C].
///
/// Every Landmark has guaranteed top-level fields (`id`, `title`, `icon`, `children`)
/// plus user-defined fields (`readings`) keyed by [`FeatureID`] for stability.
///
/// [C]: crate::chart::Chart
#[derive(Debug, Clone, PartialEq, TypedBuilder, Serialize, Deserialize, Type)]
#[builder(field_defaults(default, setter(strip_option(ignore_invalid), prefix = "with_")))]
pub struct Landmark {
    /// The schema this landmark conforms to
    pub chart: ChartID,

    /// Title (always present, rich text capable)
    #[builder(
      via_mutators,
      mutators(
        pub fn with_title(mut self, title: impl Into<RichText>) {
          self.title = title.into();
        }
      )
    )]
    pub title: RichText,

    /// Typed icon with rendering properties (preferred)
    pub icon: Option<Icon>,

    /// Child block references with display information
    #[builder(
      via_mutators(init = Vec::new()),
      mutators(
        pub fn add_child(&self, child: RichReference) {
          self.children.push(child);
        }

        pub fn with_children(&self, children: Vec<RichReference>) {
          self.children = children;
        }
      )
    )]
    pub children: Vec<RichReference>,

    /// User-defined fields (preserves field order, direct JSON values)
    #[builder(
      via_mutators(init = IndexMap::new()),
      mutators(
        pub fn with_reading(&self, id: FeatureID, reading: Reading) {
          self.readings.insert(id, reading);
        }

        pub fn with_readings(&self, readings: IndexMap<FeatureID, Reading>) {
          self.readings = readings;
        }
      )
    )]
    pub readings: IndexMap<FeatureID, Reading>,
}

impl Landmark {
    /// Get the chart ID for this landmark.
    pub fn chart_id(&self) -> ChartID {
        self.chart.clone()
    }

    /// Get the type-name for this landmark.
    pub fn type_name(&self) -> &str {
        self.chart.name()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{InlineMark, TextMarkKind, TextSegment};
    use super::*;

    #[test]
    fn test_content_type_names() {
        assert_eq!(
            Content::Paragraph {
                rich_text: RichText::from("test".to_string()),
                theme: None,
            }
            .type_name(),
            "paragraph"
        );
        assert_eq!(
            Content::Code {
                language: "rust".to_string(),
                lines: vec!["fn main() {}".to_string()],
                filename: None,
                highlights: None,
                theme: None,
            }
            .type_name(),
            "code"
        );
        assert_eq!(
            Content::Table {
                cells: vec![],
                col_size: None,
                row_size: None,
                col_header: None,
                row_header: None,
                fill_width: None,
                col_colors: None,
                row_colors: None,
                theme: None,
            }
            .type_name(),
            "table"
        );
        assert_eq!(
            Content::Math {
                expression: "x^2".to_string(),
                engine: MathEngine::Typst,
                theme: None,
            }
            .type_name(),
            "math"
        );
        assert_eq!(Content::Divider { theme: None }.type_name(), "divider");
    }

    #[test]
    fn test_paragraph_content_creation() {
        let content = Content::Paragraph {
            theme: None,
            rich_text: RichText::new(vec![
                TextSegment {
                    text: "Hello".to_string(),
                    marks: vec![InlineMark {
                        kind: TextMarkKind::Bold,
                        data: None,
                    }],
                },
                TextSegment {
                    text: ", world!".to_string(),
                    marks: vec![],
                },
            ]),
        };

        assert_eq!(content.type_name(), "paragraph");
        if let Content::Paragraph { rich_text, .. } = content {
            assert_eq!(rich_text.plain_text(), "Hello, world!");
            assert_eq!(rich_text.segments().len(), 2);
            assert_eq!(rich_text.segments()[0].marks[0].kind, TextMarkKind::Bold);
        }
    }

    #[test]
    fn test_code_content_creation() {
        let content = Content::Code {
            theme: None,
            language: "rust".to_string(),
            lines: vec![
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
                "}".to_string(),
            ],
            filename: None,
            highlights: None,
        };

        assert_eq!(content.type_name(), "code");
        if let Content::Code {
            language, lines, ..
        } = content
        {
            assert_eq!(language, "rust");
            assert!(lines.join("\n").contains("fn main()"));
        }
    }

    #[test]
    fn test_table_content_creation() {
        let content = Content::Table {
            theme: None,
            cells: vec![
                vec![
                    RichText::from("Name".to_string()),
                    RichText::from("Age".to_string()),
                    RichText::from("City".to_string()),
                ],
                vec![
                    RichText::from("Alice".to_string()),
                    RichText::from("30".to_string()),
                    RichText::from("New York".to_string()),
                ],
                vec![
                    RichText::from("Bob".to_string()),
                    RichText::from("25".to_string()),
                    RichText::from("San Francisco".to_string()),
                ],
            ],
            col_size: None,
            row_size: None,
            col_header: Some(true),
            row_header: None,
            fill_width: None,
            col_colors: None,
            row_colors: None,
        };

        assert_eq!(content.type_name(), "table");
        if let Content::Table {
            cells, col_header, ..
        } = content
        {
            assert_eq!(cells.len(), 3); // 3 rows
            assert_eq!(cells[0].len(), 3); // 3 columns in first row
            assert_eq!(cells[0][0].plain_text(), "Name");
            assert_eq!(cells[1][0].plain_text(), "Alice");
            assert_eq!(col_header, Some(true));
        }
    }

    #[test]
    fn test_math_content_creation() {
        let typst_math = Content::Math {
            expression: "x^2 + y^2 = z^2".to_string(),
            engine: MathEngine::Typst,
            theme: None,
        };

        let latex_math = Content::Math {
            expression: "\\int_0^1 x dx = \\frac{1}{2}".to_string(),
            engine: MathEngine::LaTeX,
            theme: None,
        };

        assert_eq!(typst_math.type_name(), "math");
        assert_eq!(latex_math.type_name(), "math");

        if let Content::Math {
            expression, engine, ..
        } = typst_math
        {
            assert_eq!(expression, "x^2 + y^2 = z^2");
            assert_eq!(engine, MathEngine::Typst);
        }

        if let Content::Math {
            expression, engine, ..
        } = latex_math
        {
            assert!(expression.contains("\\int"));
            assert_eq!(engine, MathEngine::LaTeX);
        }
    }

    #[test]
    fn test_divider_creation() {
        let content = Content::Divider { theme: None };
        assert_eq!(content.type_name(), "divider");
    }

    #[test]
    fn test_math_engine_serialization() {
        let typst_json = serde_json::to_string(&MathEngine::Typst).unwrap();
        assert_eq!(typst_json, "\"typst\"");

        let latex_json = serde_json::to_string(&MathEngine::LaTeX).unwrap();
        assert_eq!(latex_json, "\"latex\"");
    }

    #[test]
    fn test_content_type_name_matches_serde_tag() {
        let variants: Vec<Content> = vec![
            Content::Paragraph {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::Heading1 {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::Heading2 {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::Heading3 {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::Blockquote { theme: None },
            Content::ListItem {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::EnumItem {
                rich_text: RichText::default(),
                theme: None,
            },
            Content::Callout {
                icon: String::new(),
                theme: None,
            },
            Content::Code {
                language: String::new(),
                lines: vec![],
                filename: None,
                highlights: None,
                theme: None,
            },
            Content::Table {
                cells: vec![],
                col_size: None,
                row_size: None,
                col_header: None,
                row_header: None,
                fill_width: None,
                col_colors: None,
                row_colors: None,
                theme: None,
            },
            Content::Math {
                expression: String::new(),
                engine: MathEngine::default(),
                theme: None,
            },
            Content::Divider { theme: None },
        ];

        for content in variants {
            let json = serde_json::to_value(&content).unwrap();
            assert_eq!(
                json["type"].as_str().unwrap(),
                content.type_name(),
                "type_name() drifted from serde tag for {:?}",
                content.type_name(),
            );
        }
    }

    #[test]
    fn test_content_serialization() {
        let content = Content::Paragraph {
            rich_text: RichText::new(vec![
                TextSegment {
                    text: "Hello, ".to_string(),
                    marks: vec![],
                },
                TextSegment {
                    text: "world".to_string(),
                    marks: vec![InlineMark {
                        kind: TextMarkKind::Bold,
                        data: None,
                    }],
                },
                TextSegment {
                    text: "!".to_string(),
                    marks: vec![],
                },
            ]),
            theme: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let deserialized: Content = serde_json::from_str(&json).unwrap();

        assert_eq!(content, deserialized);
    }
}
