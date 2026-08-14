mod waypoints;

pub use waypoints::WaypointsMarkdown;

use crate::ImportResult;
use domain::Content;

/// A parsed block with optional nested children (for compound blocks like blockquote).
#[derive(Debug, Clone)]
pub struct ParsedBlock {
    pub content: Content,
    /// Children that should be terrain-linked under this block (e.g. blockquote paragraphs).
    pub children: Vec<Content>,
}

impl ParsedBlock {
    pub fn leaf(content: Content) -> Self {
        Self {
            content,
            children: Vec::new(),
        }
    }

    pub fn compound(content: Content, children: Vec<Content>) -> Self {
        Self { content, children }
    }
}

/// Converts a markdown document into Waypoints marks and terrains.
///
/// Each flavor (Waypoints, Obsidian, Logseq, etc.) maps its dialect's syntax
/// into domain types, producing the full mark + terrain graph.
pub trait MarkdownImporter {
    fn import(input: &str) -> ImportResult;
}
