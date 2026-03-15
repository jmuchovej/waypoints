use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use waypoints_domain::mark::{Content, InlineMark, RichText, TextMarkKind, TextSegment};

/// A parsed block with optional nested children (for compound blocks like blockquote).
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub content: Content,
    /// Children that should be terrain-linked under this block (e.g. blockquote paragraphs).
    pub children: Vec<Content>,
}

impl ParsedBlock {
    fn leaf(content: Content) -> Self {
        Self {
            content,
            children: Vec::new(),
        }
    }

    fn compound(content: Content, children: Vec<Content>) -> Self {
        Self { content, children }
    }
}

/// Parse a markdown string into a list of `ParsedBlock`s.
///
/// Compound blocks (e.g., blockquote) carry their children in `ParsedBlock::children`
/// so the seeder can create proper terrain edges.
///
/// Uses `pulldown-cmark` for robust CommonMark parsing, then maps the AST
/// events into our domain types. Inline formatting (bold, italic,
/// strikethrough, code, links) is captured as segment-based marks.
pub fn parse_markdown(input: &str) -> Vec<ParsedBlock> {
    let opts = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(input, opts);

    let mut blocks: Vec<ParsedBlock> = Vec::new();
    let mut ctx = BlockCtx::None;

    // Inline accumulator — segment-based
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut current_text = String::new();
    let mut active_marks: Vec<InlineMark> = Vec::new();

    // Code block accumulator
    let mut code_lang = String::new();
    let mut code_lines = String::new();

    // List state
    let mut list_ordered = false;

    // Blockquote accumulator — collects inner Content items
    let mut bq_items: Vec<Content> = Vec::new();

    /// Flush accumulated text into a segment with the current active marks.
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

    /// Take the accumulated segments as a RichText, computing plain_text.
    fn take_rich_text(segments: &mut Vec<TextSegment>) -> RichText {
        let segs = std::mem::take(segments);
        let plain_text: String = segs.iter().map(|s| s.text.as_str()).collect();
        RichText {
            plain_text,
            segments: segs,
        }
    }

    for event in parser {
        match event {
            // ── Block starts ────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                ctx = BlockCtx::Heading(level);
                segments.clear();
                current_text.clear();
                active_marks.clear();
            }
            Event::Start(Tag::Paragraph) => {
                // Don't override blockquote or list-item context —
                // pulldown-cmark nests paragraphs inside these.
                if ctx != BlockCtx::Blockquote && !matches!(ctx, BlockCtx::ListItem { .. }) {
                    ctx = BlockCtx::Paragraph;
                }
                segments.clear();
                current_text.clear();
                active_marks.clear();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                ctx = BlockCtx::Blockquote;
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

            // ── Inline formatting ───────────────────────────────────
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

            // Inline code — pulldown-cmark emits this as a single event
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

            // ── Text ────────────────────────────────────────────────
            Event::Text(text) => match ctx {
                BlockCtx::CodeBlock => {
                    code_lines.push_str(&text);
                }
                _ => {
                    current_text.push_str(&text);
                }
            },

            Event::SoftBreak => {
                if ctx != BlockCtx::CodeBlock {
                    current_text.push(' ');
                }
            }
            Event::HardBreak => {
                if ctx != BlockCtx::CodeBlock {
                    current_text.push('\n');
                }
            }

            // ── Block ends ──────────────────────────────────────────
            Event::End(TagEnd::Heading(level)) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                let rt = take_rich_text(&mut segments);
                blocks.push(ParsedBlock::leaf(match level {
                    HeadingLevel::H1 => Content::Heading1 { rich_text: rt },
                    HeadingLevel::H2 => Content::Heading2 { rich_text: rt },
                    _ => Content::Heading3 { rich_text: rt },
                }));
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::Paragraph) => {
                flush_text(&mut current_text, &active_marks, &mut segments);
                let rt = take_rich_text(&mut segments);
                if rt.plain_text.is_empty() {
                    // Skip empty paragraphs regardless of context
                } else {
                    match ctx {
                        BlockCtx::Blockquote => {
                            bq_items.push(Content::Paragraph { rich_text: rt });
                            // Don't reset ctx yet — BlockQuote end will do it
                        }
                        BlockCtx::ListItem { ordered } => {
                            if ordered {
                                blocks.push(ParsedBlock::leaf(Content::EnumItem { rich_text: rt }));
                            } else {
                                blocks.push(ParsedBlock::leaf(Content::ListItem { rich_text: rt }));
                            }
                        }
                        _ => {
                            blocks.push(ParsedBlock::leaf(Content::Paragraph { rich_text: rt }));
                        }
                    }
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                blocks.push(ParsedBlock::compound(
                    Content::Blockquote,
                    std::mem::take(&mut bq_items),
                ));
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::Item) => {
                // If the list item had no inner paragraph (rare), flush here
                if !current_text.is_empty() || !segments.is_empty() {
                    flush_text(&mut current_text, &active_marks, &mut segments);
                    let rt = take_rich_text(&mut segments);
                    if list_ordered {
                        blocks.push(ParsedBlock::leaf(Content::EnumItem { rich_text: rt }));
                    } else {
                        blocks.push(ParsedBlock::leaf(Content::ListItem { rich_text: rt }));
                    }
                }
                ctx = BlockCtx::None;
            }
            Event::End(TagEnd::List(_)) => {
                list_ordered = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                // Strip trailing newline if present
                if code_lines.ends_with('\n') {
                    code_lines.pop();
                }
                blocks.push(ParsedBlock::leaf(Content::Code {
                    language: std::mem::take(&mut code_lang),
                    lines: code_lines.split('\n').map(String::from).collect(),
                }));
                code_lines.clear();
                ctx = BlockCtx::None;
            }

            Event::Rule => {
                blocks.push(ParsedBlock::leaf(Content::Divider));
            }

            // Ignore everything else (HTML, footnotes, etc.)
            _ => {}
        }
    }

    blocks
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockCtx {
    None,
    Paragraph,
    Heading(HeadingLevel),
    Blockquote,
    ListItem { ordered: bool },
    CodeBlock,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headings() {
        let blocks = parse_markdown("# Title\n## Subtitle\n### Section");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content.type_name(), "heading1");
        assert_eq!(blocks[1].content.type_name(), "heading2");
        assert_eq!(blocks[2].content.type_name(), "heading3");
    }

    #[test]
    fn test_bold_inline() {
        let blocks = parse_markdown("This is **bold** text");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "This is bold text");
        // Should be 3 segments: "This is ", "bold", " text"
        assert_eq!(rt.segments.len(), 3);
        assert!(rt.segments[0].marks.is_empty());
        assert_eq!(rt.segments[1].text, "bold");
        assert_eq!(rt.segments[1].marks.len(), 1);
        assert_eq!(rt.segments[1].marks[0].kind, TextMarkKind::Bold);
        assert!(rt.segments[2].marks.is_empty());
    }

    #[test]
    fn test_italic_inline() {
        let blocks = parse_markdown("Some *italic* here");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "Some italic here");
        let italic_seg = rt.segments.iter().find(|s| !s.marks.is_empty()).unwrap();
        assert_eq!(italic_seg.text, "italic");
        assert_eq!(italic_seg.marks[0].kind, TextMarkKind::Italic);
    }

    #[test]
    fn test_inline_code() {
        let blocks = parse_markdown("Use `cargo build` to compile");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "Use cargo build to compile");
        let code_seg = rt.segments.iter().find(|s| !s.marks.is_empty()).unwrap();
        assert_eq!(code_seg.text, "cargo build");
        assert_eq!(code_seg.marks[0].kind, TextMarkKind::Code);
    }

    #[test]
    fn test_lists() {
        let blocks = parse_markdown("- Item one\n- Item two\n\n1. First\n2. Second");
        assert_eq!(blocks[0].content.type_name(), "list-item");
        assert_eq!(blocks[1].content.type_name(), "list-item");
        assert_eq!(blocks[2].content.type_name(), "enum-item");
        assert_eq!(blocks[3].content.type_name(), "enum-item");
    }

    #[test]
    fn test_code_block() {
        let blocks = parse_markdown("```rust\nfn main() {\n    println!(\"hi\");\n}\n```");
        assert_eq!(blocks.len(), 1);
        if let Content::Code { language, lines } = &blocks[0].content {
            assert_eq!(language, "rust");
            assert_eq!(lines.len(), 3);
            assert_eq!(lines[0], "fn main() {");
        } else {
            panic!("expected code block");
        }
    }

    #[test]
    fn test_blockquote() {
        let blocks = parse_markdown("> This is a quote");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content.type_name(), "blockquote");
        assert_eq!(blocks[0].children.len(), 1);
        assert_eq!(blocks[0].children[0].type_name(), "paragraph");
    }

    #[test]
    fn test_divider() {
        let blocks = parse_markdown("Before\n\n---\n\nAfter");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1].content.type_name(), "divider");
    }

    #[test]
    fn test_strikethrough() {
        let blocks = parse_markdown("This is ~~deleted~~ text");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "This is deleted text");
        let strike_seg = rt.segments.iter().find(|s| !s.marks.is_empty()).unwrap();
        assert_eq!(strike_seg.text, "deleted");
        assert_eq!(strike_seg.marks[0].kind, TextMarkKind::Strikethrough);
    }

    #[test]
    fn test_multibyte_char_offsets() {
        // Segment model doesn't need offset math — just verify plain_text is correct
        let blocks = parse_markdown("mornings can be ~~cold~~ chilly");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "mornings can be cold chilly");
        let strike_seg = rt.segments.iter().find(|s| !s.marks.is_empty()).unwrap();
        assert_eq!(strike_seg.text, "cold");
        assert_eq!(strike_seg.marks[0].kind, TextMarkKind::Strikethrough);
    }

    #[test]
    fn test_multibyte_before_mark() {
        let blocks = parse_markdown("café **latte**");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "café latte");
        let bold_seg = rt
            .segments
            .iter()
            .find(|s| s.marks.iter().any(|m| m.kind == TextMarkKind::Bold))
            .unwrap();
        assert_eq!(bold_seg.text, "latte");
    }

    #[test]
    fn test_link() {
        let blocks = parse_markdown("Check [this link](https://example.com) out");
        let Content::Paragraph { rich_text: rt } = &blocks[0].content else {
            panic!("expected paragraph");
        };
        assert_eq!(rt.plain_text, "Check this link out");
        let link_seg = rt.segments.iter().find(|s| !s.marks.is_empty()).unwrap();
        assert_eq!(link_seg.text, "this link");
        assert_eq!(link_seg.marks.len(), 1);
        assert_eq!(link_seg.marks[0].kind, TextMarkKind::Link);
        assert_eq!(
            link_seg.marks[0].data,
            Some(serde_json::json!("https://example.com"))
        );
    }

    #[test]
    fn test_demo_doc_parses() {
        let input = include_str!("docs/demo.md");
        let blocks = parse_markdown(input);
        // Should produce a non-trivial number of blocks
        assert!(blocks.len() > 10, "got {} blocks", blocks.len());
    }
}
