//! The fixture importer plugin: a directory of plain markdown, one Landmark
//! per file. See the `transload-sdk` change's design D10.
//!
//! Each `.md` entry becomes a Landmark keyed `page:<stem>` with no protochart.
//! The title is the first `#` heading or the file stem; the body is one
//! block per blank-line-separated run — a heading for `#`, `##`, `###`, a
//! paragraph otherwise — keyed `block:<stem>:<n>`; `[[name]]` is a mention
//! keyed `page:<name>`. Fenced code is not carried and is reported where it
//! was found. Nothing else is parsed.

use domain::{Content, InlineMark, RichText, TextMarkKind, TextSegment};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use transload::{
    abi::{Diagnostic, Output},
    fragment::{ContentFragment, LandmarkFragment, SourceKey},
    sdk::Importer,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TitleFrom {
    /// The first `#` heading, or the file stem when there is none.
    #[default]
    Heading,
    /// The file stem.
    Filename,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Config {
    pub title_from: TitleFrom,
    /// The entry on which `entry` panics, so a trap has something to exercise.
    #[serde(default)]
    pub fail_on_entry: Option<String>,
}

pub struct PlainMarkdown {
    config: Config,
}

impl Importer for PlainMarkdown {
    type Config = Config;

    fn init(config: Config) -> Result<Self, Diagnostic> {
        Ok(Self { config })
    }

    fn entry(&mut self, name: &str, bytes: &[u8]) -> Result<Output, Diagnostic> {
        if self.config.fail_on_entry.as_deref() == Some(name) {
            panic!("asked to fail on `{name}`");
        }
        let Some(stem) = name
            .rsplit('/')
            .next()
            .and_then(|file| file.strip_suffix(".md"))
        else {
            return Ok(Output::default());
        };
        let text = String::from_utf8_lossy(bytes);
        let page = Page::parse(
            name,
            stem,
            &text,
            self.config.title_from == TitleFrom::Heading,
        );

        let title = match self.config.title_from {
            TitleFrom::Heading => page.heading.unwrap_or_else(|| stem.to_string().into()),
            TitleFrom::Filename => stem.to_string().into(),
        };
        let mut landmark = LandmarkFragment::new(key(format!("page:{stem}")), title);
        landmark.body = page.body;

        let mut output = Output::default();
        output.fragments.landmarks.push(landmark);
        output.diagnostics = page.diagnostics;
        Ok(output)
    }

    fn finalize(self) -> Result<Output, Diagnostic> {
        Ok(Output::default())
    }
}

transload::importer!(PlainMarkdown, manifest = "../importer.toml");

struct Page {
    /// Whether the first `#` heading is taken as the title rather than kept
    /// as a block.
    title_from_heading: bool,
    heading: Option<RichText>,
    body: Vec<ContentFragment>,
    diagnostics: Vec<Diagnostic>,
}

impl Page {
    fn parse(name: &str, stem: &str, text: &str, title_from_heading: bool) -> Self {
        let mut page = Self {
            title_from_heading,
            heading: None,
            body: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut lines = text.lines().enumerate().peekable();
        while let Some((index, line)) = lines.next() {
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with("```") {
                page.diagnostics.push(
                    Diagnostic::warn("fenced code is not carried").at(name, Some(index as u32 + 1)),
                );
                for (_, line) in lines.by_ref() {
                    if line.starts_with("```") {
                        break;
                    }
                }
                continue;
            }
            let mut run = vec![line];
            while let Some((_, next)) = lines.next_if(|(_, next)| !next.trim().is_empty()) {
                run.push(next);
            }
            page.block(stem, &run);
        }
        page
    }

    fn block(&mut self, stem: &str, run: &[&str]) {
        let first = run[0];
        let content = if let Some(rest) = first.strip_prefix("# ") {
            let rich_text = rich(&join(rest, &run[1..]));
            if self.title_from_heading && self.heading.is_none() {
                self.heading = Some(rich_text);
                return;
            }
            Content::Heading1 {
                rich_text,
                theme: None,
            }
        } else if let Some(rest) = first.strip_prefix("## ") {
            Content::Heading2 {
                rich_text: rich(&join(rest, &run[1..])),
                theme: None,
            }
        } else if let Some(rest) = first.strip_prefix("### ") {
            Content::Heading3 {
                rich_text: rich(&join(rest, &run[1..])),
                theme: None,
            }
        } else {
            Content::Paragraph {
                rich_text: rich(&run.join("\n")),
                theme: None,
            }
        };
        let n = self.body.len() + 1;
        self.body.push(ContentFragment::new(
            key(format!("block:{stem}:{n}")),
            content,
        ));
    }
}

fn join(first: &str, rest: &[&str]) -> String {
    std::iter::once(first)
        .chain(rest.iter().copied())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Plain text with every `[[name]]` as a mention keyed `page:<name>`.
fn rich(text: &str) -> RichText {
    let mut segments = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let Some(close) = rest[open + 2..].find("]]") else {
            break;
        };
        let name = &rest[open + 2..open + 2 + close];
        if open > 0 {
            segments.push(plain(&rest[..open]));
        }
        segments.push(TextSegment {
            text: name.to_string(),
            marks: vec![InlineMark {
                kind: TextMarkKind::Mention,
                data: Some(json!(format!("page:{name}"))),
            }],
        });
        rest = &rest[open + 2 + close + 2..];
    }
    if !rest.is_empty() {
        segments.push(plain(rest));
    }
    RichText::new(segments)
}

fn plain(text: &str) -> TextSegment {
    TextSegment {
        text: text.to_string(),
        marks: Vec::new(),
    }
}

fn key(key: String) -> SourceKey {
    SourceKey::new(key).expect("a stem is not empty")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(title_from: TitleFrom, name: &str, text: &str) -> Output {
        let mut plugin = PlainMarkdown::init(Config {
            title_from,
            fail_on_entry: None,
        })
        .unwrap();
        plugin.entry(name, text.as_bytes()).unwrap()
    }

    #[test]
    fn the_title_is_the_first_heading_or_the_stem() {
        let output = run(TitleFrom::Heading, "notes/alpha.md", "# Alpha\n\nBody.\n");
        let landmark = &output.fragments.landmarks[0];
        assert_eq!(landmark.src.as_str(), "page:alpha");
        assert_eq!(landmark.title.plain_text(), "Alpha");
        assert_eq!(landmark.protochart, None);
        assert!(landmark.readings.is_empty());
        assert_eq!(landmark.body.len(), 1);
        assert_eq!(landmark.body[0].src.as_str(), "block:alpha:1");

        let output = run(TitleFrom::Filename, "notes/alpha.md", "# Alpha\n\nBody.\n");
        let landmark = &output.fragments.landmarks[0];
        assert_eq!(landmark.title.plain_text(), "alpha");
        assert_eq!(landmark.body.len(), 2);
        assert!(matches!(landmark.body[0].content, Content::Heading1 { .. }));

        let output = run(TitleFrom::Heading, "bare.md", "Just text.\n");
        assert_eq!(output.fragments.landmarks[0].title.plain_text(), "bare");
    }

    #[test]
    fn a_wiki_link_is_a_mention_keyed_like_a_page() {
        let output = run(TitleFrom::Heading, "a.md", "See [[alpha]] and [[missing]].");
        let Content::Paragraph { rich_text, .. } = &output.fragments.landmarks[0].body[0].content
        else {
            panic!("not a paragraph");
        };
        assert_eq!(rich_text.plain_text(), "See alpha and missing.");
        let mentions: Vec<_> = rich_text
            .segments()
            .iter()
            .filter_map(|segment| segment.marks.first())
            .map(|mark| mark.data.clone().unwrap())
            .collect();
        assert_eq!(mentions, [json!("page:alpha"), json!("page:missing")]);
    }

    #[test]
    fn fenced_code_is_reported_where_it_was_found_and_not_carried() {
        let output = run(
            TitleFrom::Heading,
            "a.md",
            "One.\n\n```rust\nlet x = 1;\n```\n\nTwo.\n",
        );
        assert_eq!(output.fragments.landmarks[0].body.len(), 2);
        let warning = &output.diagnostics[0];
        assert_eq!(warning.level, transload::abi::Level::Warn);
        let at = warning.at.as_ref().unwrap();
        assert_eq!((at.entry.as_str(), at.line), ("a.md", Some(3)));
    }

    #[test]
    fn the_manifest_and_schema_describe_the_plugin() {
        let manifest = PlainMarkdown::manifest().unwrap();
        assert_eq!(manifest.id, "plain-markdown");
        assert_eq!(manifest.version, env!("CARGO_PKG_VERSION"));
        let mut fields: Vec<_> = manifest.options["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        fields.sort();
        assert_eq!(fields, ["fail_on_entry", "title_from"]);
        assert_eq!(manifest.options["required"], json!(["title_from"]));
    }

    #[test]
    #[should_panic(expected = "asked to fail on `a.md`")]
    fn the_named_entry_panics() {
        let mut plugin = PlainMarkdown::init(Config {
            title_from: TitleFrom::Heading,
            fail_on_entry: Some("a.md".to_string()),
        })
        .unwrap();
        let _ = plugin.entry("a.md", b"");
    }
}
