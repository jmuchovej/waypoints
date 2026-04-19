use super::{MarkdownImporter, ParsedBlock};
use crate::ImportResult;
use domain::{
    chart::ChartID,
    fractional_index,
    mark::{
        Content, InlineMark, Landmark, Mark, MarkData, MarkID, Reading, RichText, TextMarkKind,
        TextSegment,
    },
    terrain::{Terrain, TerrainKind},
};
use pulldown_cmark::{Event, HeadingLevel, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use uuid::Uuid;

/// Standard CommonMark importer for Waypoints-flavored markdown.
///
/// Supports:
/// - YAML frontmatter (`chart`, `uuid`, `title`)
/// - `@uuid` inline mentions → `TextMarkKind::Mention`
/// - Headings, paragraphs, lists, code blocks, blockquotes, dividers
/// - Inline formatting (bold, italic, strikethrough, code, links)
pub struct WaypointsMarkdown;

impl MarkdownImporter for WaypointsMarkdown {
    fn import(input: &str) -> ImportResult {
        import(input)
    }
}

/// Frontmatter extracted from YAML metadata block.
struct Frontmatter {
    chart: Option<ChartID>,
    uuid: Option<Uuid>,
    title: Option<String>,
    /// Readings keyed by feature name (e.g. "Author(s)").
    readings: Vec<(String, Reading)>,
}

impl Frontmatter {
    fn empty() -> Self {
        Self {
            chart: None,
            uuid: None,
            title: None,
            readings: Vec::new(),
        }
    }
}

/// Parse a reading value from a frontmatter string.
///
/// - `@uuid` values (space-separated) → `Reading::References`
/// - Otherwise → `Reading::Text`
fn parse_reading_value(value: &str) -> Reading {
    // Check if the value contains @uuid references
    let uuids: Vec<MarkID> = value
        .split_whitespace()
        .filter_map(|token| {
            let raw = token.strip_prefix('@')?;
            let uuid = raw.parse::<Uuid>().ok()?;
            Some(MarkID::from(uuid))
        })
        .collect();

    if !uuids.is_empty() {
        Reading::References(uuids)
    } else {
        Reading::Text(RichText::from(value.to_string()))
    }
}

/// Strip surrounding YAML quotes (single or double) from a value.
fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(value)
}

/// Parse YAML frontmatter with support for a nested `readings:` block.
///
/// Top-level keys: `chart`, `uuid`, `title`.
/// The `readings:` key opens an indented block. Scalar readings use
/// `  FeatureName: value`. List readings use YAML list syntax:
/// ```yaml
/// readings:
///   Author(s):
///     - <uuid>
///     - <uuid>
///   Summary: some text
/// ```
fn parse_frontmatter(yaml: &str) -> Frontmatter {
    let mut fm = Frontmatter::empty();
    let mut in_readings = false;
    // For collecting YAML list items under a reading key
    let mut list_key: Option<String> = None;
    let mut list_items: Vec<MarkID> = Vec::new();

    /// Flush any accumulated list items into a Reading::References.
    fn flush_list(
        fm: &mut Frontmatter,
        list_key: &mut Option<String>,
        list_items: &mut Vec<MarkID>,
    ) {
        if let Some(key) = list_key.take() {
            if !list_items.is_empty() {
                fm.readings
                    .push((key, Reading::References(std::mem::take(list_items))));
            }
        }
        list_items.clear();
    }

    for line in yaml.lines() {
        // Inside readings: block
        if in_readings {
            if line.starts_with(' ') || line.starts_with('\t') {
                let trimmed = line.trim();

                // YAML list item: "- <uuid>"
                if let Some(item) = trimmed.strip_prefix("- ") {
                    let item = item.trim();
                    if let Ok(uuid) = item.parse::<Uuid>() {
                        list_items.push(MarkID::from(uuid));
                    }
                    continue;
                }

                // Key: value line — flush any pending list first
                flush_list(&mut fm, &mut list_key, &mut list_items);

                if let Some((name, value)) = trimmed.split_once(':') {
                    let name = name.trim().to_string();
                    let value = value.trim();
                    if name.is_empty() {
                        continue;
                    }
                    if value.is_empty() {
                        // Key with no value — start collecting list items
                        list_key = Some(name);
                    } else {
                        fm.readings.push((name, parse_reading_value(value)));
                    }
                }
                continue;
            }
            // Non-indented line — flush list and exit readings block
            flush_list(&mut fm, &mut list_key, &mut list_items);
            in_readings = false;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        match key {
            "readings" => {
                // Start of readings block (value should be empty for a YAML dict)
                in_readings = true;
            }
            "chart" => fm.chart = Some(ChartID(value.to_string())),
            "uuid" => fm.uuid = value.parse::<Uuid>().ok(),
            "title" => {
                let value = unquote(value);
                if !value.is_empty() {
                    fm.title = Some(value.to_string());
                }
            }
            _ => {}
        }
    }
    // Flush any trailing list
    flush_list(&mut fm, &mut list_key, &mut list_items);
    fm
}

/// UUID regex-ish matching: 8-4-4-4-12 hex chars.
fn try_parse_mention(text: &str) -> Option<(Uuid, usize)> {
    // Minimum UUID length: 36 chars (8-4-4-4-12)
    if text.len() < 36 {
        return None;
    }
    let candidate = &text[..36];
    let uuid = candidate.parse::<Uuid>().ok()?;
    Some((uuid, 36))
}

/// Post-process text segments to split `@uuid` mentions into separate segments.
fn extract_mentions(segments: Vec<TextSegment>) -> Vec<TextSegment> {
    let mut result = Vec::new();

    for seg in segments {
        let text = &seg.text;
        let mut last = 0;

        // Scan for @uuid patterns
        for (pos, _) in text.match_indices('@') {
            let after_at = pos + 1;
            if let Some((uuid, uuid_len)) = try_parse_mention(&text[after_at..]) {
                // Flush text before the @
                if pos > last {
                    result.push(TextSegment {
                        text: text[last..pos].to_string(),
                        marks: seg.marks.clone(),
                    });
                }

                // Create mention segment
                let mut marks = seg.marks.clone();
                marks.push(InlineMark {
                    kind: TextMarkKind::Mention,
                    data: Some(serde_json::json!(uuid.to_string())),
                });
                result.push(TextSegment {
                    text: uuid.to_string(),
                    marks,
                });

                last = after_at + uuid_len;
            }
        }

        // Flush remaining text
        if last == 0 {
            // No mentions found — keep original segment
            result.push(seg);
        } else if last < text.len() {
            result.push(TextSegment {
                text: text[last..].to_string(),
                marks: seg.marks.clone(),
            });
        }
    }

    result
}

fn import(input: &str) -> ImportResult {
    let opts = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_GFM
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_MATH;
    let parser = Parser::new_ext(input, opts);

    let mut frontmatter = Frontmatter::empty();
    let mut blocks: Vec<ParsedBlock> = Vec::new();
    let mut ctx = BlockCtx::None;
    let mut first_h1: Option<String> = None;

    // Inline accumulator
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut current_text = String::new();
    let mut active_marks: Vec<InlineMark> = Vec::new();

    // Code block accumulator
    let mut code_lang = String::new();
    let mut code_lines = String::new();

    // List state
    let mut list_ordered = false;

    // Blockquote/callout accumulator
    let mut bq_items: Vec<Content> = Vec::new();
    let mut bq_kind: Option<pulldown_cmark::BlockQuoteKind> = None;

    // Metadata accumulator
    let mut metadata_text = String::new();

    fn flush_text(
        current_text: &mut String,
        active_marks: &[InlineMark],
        segments: &mut Vec<TextSegment>,
    ) {
        if current_text.is_empty() {
            return;
        }
        segments.push(TextSegment {
            text: std::mem::take(current_text),
            marks: active_marks.to_vec(),
        });
    }

    fn take_rich_text(segments: &mut Vec<TextSegment>) -> RichText {
        let segs = extract_mentions(std::mem::take(segments));
        RichText::new(segs)
    }

    for event in parser {
        match event {
            // ── Frontmatter ────────────────────────────────────────
            Event::Start(Tag::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                ctx = BlockCtx::Metadata;
                metadata_text.clear();
            }
            Event::End(TagEnd::MetadataBlock(MetadataBlockKind::YamlStyle)) => {
                frontmatter = parse_frontmatter(&metadata_text);
                ctx = BlockCtx::None;
            }

            // ── Block starts ───────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                ctx = BlockCtx::Heading(level);
                segments.clear();
                current_text.clear();
                active_marks.clear();
            }
            Event::Start(Tag::Paragraph) => {
                if ctx != BlockCtx::Blockquote && !matches!(ctx, BlockCtx::ListItem { .. }) {
                    ctx = BlockCtx::Paragraph;
                }
                segments.clear();
                current_text.clear();
                active_marks.clear();
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                ctx = BlockCtx::Blockquote;
                bq_kind = kind;
                bq_items.clear();
            }
            Event::Start(Tag::List(start)) => {
                list_ordered = start.is_some();
            }
            Event::Start(Tag::Item) => {
                ctx = BlockCtx::ListItem {
                    ordered: list_ordered,
                };
                segments.clear();
                current_text.clear();
                active_marks.clear();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                code_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                code_lines.clear();
                ctx = BlockCtx::CodeBlock;
            }

            // ── Inline formatting ──────────────────────────────────
            Event::Start(Tag::Strong) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.push(InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                });
            }
            Event::End(TagEnd::Strong) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.retain(|m| m.kind != TextMarkKind::Bold);
            }
            Event::Start(Tag::Emphasis) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.push(InlineMark {
                    kind: TextMarkKind::Italic,
                    data: None,
                });
            }
            Event::End(TagEnd::Emphasis) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.retain(|m| m.kind != TextMarkKind::Italic);
            }
            Event::Start(Tag::Strikethrough) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.push(InlineMark {
                    kind: TextMarkKind::Strikethrough,
                    data: None,
                });
            }
            Event::End(TagEnd::Strikethrough) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.retain(|m| m.kind != TextMarkKind::Strikethrough);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.push(InlineMark {
                    kind: TextMarkKind::Link,
                    data: Some(serde_json::json!(dest_url.to_string())),
                });
            }
            Event::End(TagEnd::Link) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                active_marks.retain(|m| m.kind != TextMarkKind::Link);
            }
            Event::Code(text) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                segments.push(TextSegment {
                    text: text.to_string(),
                    marks: {
                        let mut marks = active_marks.clone();
                        marks.push(InlineMark {
                            kind: TextMarkKind::Code,
                            data: None,
                        });
                        marks
                    },
                });
            }

            // ── Text ───────────────────────────────────────────────
            Event::Text(text) => match ctx {
                BlockCtx::Metadata => {
                    metadata_text.push_str(&text);
                }
                BlockCtx::CodeBlock => {
                    code_lines.push_str(&text);
                }
                _ => {
                    current_text.push_str(&text);
                }
            },

            Event::SoftBreak => {
                if ctx != BlockCtx::CodeBlock && ctx != BlockCtx::Metadata {
                    current_text.push(' ');
                }
            }
            Event::HardBreak => {
                if ctx != BlockCtx::CodeBlock && ctx != BlockCtx::Metadata {
                    current_text.push('\n');
                }
            }

            // ── Block ends ─────────────────────────────────────────
            Event::End(TagEnd::Heading(level)) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                let rt = take_rich_text(&mut segments);

                // Capture first H1 as fallback title
                if level == HeadingLevel::H1 && first_h1.is_none() {
                    first_h1 = Some(rt.plain_text().to_string());
                }

                blocks.push(ParsedBlock::leaf(match level {
                    HeadingLevel::H1 => Content::heading1(rt),
                    HeadingLevel::H2 => Content::heading2(rt),
                    _ => Content::heading3(rt),
                }));
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::Paragraph) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                let rt = take_rich_text(&mut segments);
                if rt.plain_text().is_empty() {
                    // skip
                } else {
                    match ctx {
                        BlockCtx::Blockquote => {
                            bq_items.push(Content::paragraph(rt));
                        }
                        BlockCtx::ListItem { ordered } => {
                            if ordered {
                                blocks.push(ParsedBlock::leaf(Content::enum_item(rt)));
                            } else {
                                blocks.push(ParsedBlock::leaf(Content::list_item(rt)));
                            }
                        }
                        _ => {
                            blocks.push(ParsedBlock::leaf(Content::paragraph(rt)));
                        }
                    }
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                use pulldown_cmark::BlockQuoteKind as BQK;

                let compound = match bq_kind.take() {
                    Some(BQK::Note) => Content::callout("ℹ️".to_string()),
                    Some(BQK::Tip) => Content::callout("💡".to_string()),
                    Some(BQK::Important) => Content::callout("❗".to_string()),
                    Some(BQK::Warning) => Content::callout("⚠️".to_string()),
                    Some(BQK::Caution) => Content::callout("🚨".to_string()),
                    None => Content::blockquote(),
                };
                blocks.push(ParsedBlock::compound(
                    compound,
                    std::mem::take(&mut bq_items),
                ));
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::Item) => {
                if !current_text.is_empty() || !segments.is_empty() {
                    flush_text(&mut current_text, &active_marks, &mut segments);
                    let rt = take_rich_text(&mut segments);
                    if list_ordered {
                        blocks.push(ParsedBlock::leaf(Content::enum_item(rt)));
                    } else {
                        blocks.push(ParsedBlock::leaf(Content::list_item(rt)));
                    }
                }
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::List(_)) => {
                list_ordered = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                if code_lines.ends_with('\n') {
                    code_lines.pop();
                }
                blocks.push(ParsedBlock::leaf(Content::code(
                    std::mem::take(&mut code_lang),
                    code_lines.split('\n').map(String::from).collect(),
                )));
                code_lines.clear();
                ctx = BlockCtx::None;
            }

            Event::Rule => {
                blocks.push(ParsedBlock::leaf(Content::divider()));
            }

            _ => {}
        }
    }

    // ── Build marks and terrains ───────────────────────────────────
    let chart = frontmatter
        .chart
        .unwrap_or_else(|| ChartID::builtin("page"));
    let doc_uuid = frontmatter.uuid.unwrap_or_else(Uuid::new_v4);
    let title = frontmatter.title.or(first_h1).unwrap_or_default();

    let doc_id = MarkID::from(doc_uuid);

    // The landmark mark
    let landmark_mark = Mark {
        id: doc_id,
        data: MarkData::Landmark(
            Landmark::builder()
                .with_chart(chart)
                .with_title(title)
                .build(),
        ),
        extra: None,
    };

    let mut marks = vec![landmark_mark];
    let mut terrains = Vec::new();

    // Content child marks + terrain edges
    let mut child_ids: Vec<MarkID> = Vec::new();

    for block in blocks {
        let child_mark = Mark::builder().with_content(block.content).build();
        let child_id = child_mark.id;
        marks.push(child_mark);
        child_ids.push(child_id);

        // Nested children (e.g. blockquote paragraphs)
        let mut nested_ids: Vec<MarkID> = Vec::new();
        for nested_content in block.children {
            let nested_mark = Mark::builder().with_content(nested_content).build();
            let nested_id = nested_mark.id;
            marks.push(nested_mark);
            nested_ids.push(nested_id);
        }

        if !nested_ids.is_empty() {
            let nested_orders = fractional_index::generate_n(nested_ids.len());
            for (nid, order) in nested_ids.into_iter().zip(nested_orders) {
                terrains.push(
                    Terrain::builder()
                        .with_src_id(child_id)
                        .with_dst_id(nid)
                        .with_kind(TerrainKind::Child)
                        .with_order(order)
                        .build(),
                );
            }
        }
    }

    // Top-level terrain edges: landmark → children
    let orders = fractional_index::generate_n(child_ids.len());
    for (child_id, order) in child_ids.into_iter().zip(orders) {
        terrains.push(
            Terrain::builder()
                .with_src_id(doc_id)
                .with_dst_id(child_id)
                .with_kind(TerrainKind::Child)
                .with_order(order)
                .build(),
        );
    }

    ImportResult {
        marks,
        terrains,
        readings: frontmatter.readings,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockCtx {
    None,
    Metadata,
    Paragraph,
    Heading(HeadingLevel),
    Blockquote,
    ListItem { ordered: bool },
    CodeBlock,
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::mark::TextMarkKind;

    #[test]
    fn basic_page_without_frontmatter() {
        let result = WaypointsMarkdown::import("# Hello\n\nSome text.");
        // landmark + heading + paragraph = 3 marks
        assert_eq!(result.marks.len(), 3);
        assert!(result.marks[0].is_landmark());
        assert!(result.marks[1].is_content());
        assert!(result.marks[2].is_content());
        // 2 terrain edges: landmark → heading, landmark → paragraph
        assert_eq!(result.terrains.len(), 2);
    }

    #[test]
    fn title_from_frontmatter() {
        let md = "---\ntitle: My Title\n---\n\n# Heading\n\nBody.";
        let result = WaypointsMarkdown::import(md);
        let MarkData::Landmark(ref lm) = result.marks[0].data else {
            panic!("expected landmark");
        };
        assert_eq!(lm.title.plain_text(), "My Title");
    }

    #[test]
    fn title_falls_back_to_h1() {
        let md = "# First Heading\n\nBody.";
        let result = WaypointsMarkdown::import(md);
        let MarkData::Landmark(ref lm) = result.marks[0].data else {
            panic!("expected landmark");
        };
        assert_eq!(lm.title.plain_text(), "First Heading");
    }

    #[test]
    fn chart_from_frontmatter() {
        let md = "---\nchart: builtin.book\n---\n\nContent.";
        let result = WaypointsMarkdown::import(md);
        let MarkData::Landmark(ref lm) = result.marks[0].data else {
            panic!("expected landmark");
        };
        assert_eq!(lm.chart, ChartID::builtin("book"));
    }

    #[test]
    fn default_chart_is_page() {
        let md = "# Page\n\nContent.";
        let result = WaypointsMarkdown::import(md);
        let MarkData::Landmark(ref lm) = result.marks[0].data else {
            panic!("expected landmark");
        };
        assert_eq!(lm.chart, ChartID::builtin("page"));
    }

    #[test]
    fn uuid_from_frontmatter() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let md = format!("---\nuuid: {uuid}\n---\n\nContent.");
        let result = WaypointsMarkdown::import(&md);
        assert_eq!(
            result.marks[0].id,
            MarkID::from(uuid.parse::<Uuid>().unwrap())
        );
    }

    #[test]
    fn mention_extraction() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let md = format!("Reading @{uuid} right now.");
        let result = WaypointsMarkdown::import(&md);
        // landmark + paragraph = 2 marks
        assert_eq!(result.marks.len(), 2);

        let MarkData::Content(Content::Paragraph { ref rich_text, .. }) = result.marks[1].data
        else {
            panic!("expected paragraph");
        };

        let mention_seg = rich_text
            .segments()
            .iter()
            .find(|s| s.marks.iter().any(|m| m.kind == TextMarkKind::Mention))
            .expect("expected mention segment");
        assert_eq!(mention_seg.text, uuid);
    }

    #[test]
    fn blockquote_children() {
        let md = "> Line one\n>\n> Line two";
        let result = WaypointsMarkdown::import(md);
        // landmark + blockquote + 2 blockquote children = 4 marks
        assert_eq!(result.marks.len(), 4);
        // landmark → blockquote, blockquote → child1, blockquote → child2 = 3 terrains
        assert_eq!(result.terrains.len(), 3);
    }

    #[test]
    fn bold_inline() {
        let md = "This is **bold** text";
        let result = WaypointsMarkdown::import(md);
        let MarkData::Content(Content::Paragraph { ref rich_text, .. }) = result.marks[1].data
        else {
            panic!("expected paragraph");
        };
        assert_eq!(rich_text.plain_text(), "This is bold text");
        assert_eq!(rich_text.segments().len(), 3);
        assert_eq!(rich_text.segments()[1].marks[0].kind, TextMarkKind::Bold);
    }

    #[test]
    fn reading_references_from_frontmatter() {
        let author_uuid = "70967890-0f6e-4616-bb46-af1466717449";
        let md = format!(
            "---\nchart: builtin.book\ntitle: Test Book\nreadings:\n  Author(s):\n    - {author_uuid}\n---\n\nContent."
        );
        let result = WaypointsMarkdown::import(&md);
        assert_eq!(result.readings.len(), 1);
        assert_eq!(result.readings[0].0, "Author(s)");
        let Reading::References(ref ids) = result.readings[0].1 else {
            panic!("expected references reading");
        };
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], MarkID::from(author_uuid.parse::<Uuid>().unwrap()));
    }

    #[test]
    fn reading_multiple_references() {
        let uuid1 = "70967890-0f6e-4616-bb46-af1466717449";
        let uuid2 = "353cb9c0-7e3d-4598-8a29-a5fc0372c004";
        let md =
            format!("---\nreadings:\n  Author(s):\n    - {uuid1}\n    - {uuid2}\n---\n\nContent.");
        let result = WaypointsMarkdown::import(&md);
        let Reading::References(ref ids) = result.readings[0].1 else {
            panic!("expected references reading");
        };
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn reading_text_value() {
        let md = "---\nreadings:\n  Summary: A great book about time\n---\n\nContent.";
        let result = WaypointsMarkdown::import(md);
        assert_eq!(result.readings.len(), 1);
        assert_eq!(result.readings[0].0, "Summary");
        let Reading::Text(ref rt) = result.readings[0].1 else {
            panic!("expected text reading");
        };
        assert_eq!(rt.plain_text(), "A great book about time");
    }

    #[test]
    fn reading_list_then_scalar() {
        let uuid = "70967890-0f6e-4616-bb46-af1466717449";
        let md = format!(
            "---\nreadings:\n  Author(s):\n    - {uuid}\n  Summary: Great book\n---\n\nContent."
        );
        let result = WaypointsMarkdown::import(&md);
        assert_eq!(result.readings.len(), 2);
        assert_eq!(result.readings[0].0, "Author(s)");
        assert!(matches!(result.readings[0].1, Reading::References(_)));
        assert_eq!(result.readings[1].0, "Summary");
        assert!(matches!(result.readings[1].1, Reading::Text(_)));
    }

    #[test]
    fn terrain_ordering() {
        let md = "# One\n\n## Two\n\nThree";
        let result = WaypointsMarkdown::import(md);
        // 3 child terrains from landmark
        assert_eq!(result.terrains.len(), 3);
        // Orders should be lexicographically sorted
        let orders: Vec<&str> = result
            .terrains
            .iter()
            .filter_map(|t| t.order.as_deref())
            .collect();
        let mut sorted = orders.clone();
        sorted.sort();
        assert_eq!(orders, sorted);
    }
}
