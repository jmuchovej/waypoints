//! Inputs the differential harnesses read with both the grammar and the hand
//! parser it replaces: the committed Bundle, generated stores' blocks, noise,
//! and the hand parsers' own malformed cases. `TRANSLOAD_SEEDS` raises the
//! generated count, as it does for the conformance suite.

use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    BlockTheme, Color, Content, InlineMark, MarkID, MathEngine, RichText, Size, TextMarkKind,
    TextSegment,
    syntax::{
        block,
        inline::{self, LinkName, Links},
    },
};

pub(crate) fn seeds() -> std::ops::Range<u64> {
    let count = std::env::var("TRANSLOAD_SEEDS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(64);
    0..count
}

/// Inline and block sources the hand parsers' tests refuse or treat specially.
pub(crate) const MALFORMED: &[&str] = &[
    "*unclosed",
    "[[",
    "[[|label]]",
    "[bare bracketed]",
    "[hi]{nonsense-token}",
    "`unclosed",
    "$x",
    "[a](unclosed",
    "[a]{bold",
    "^{x",
    "_{x",
    "[a",
    "**",
    "\\",
    "a\\",
    "[[a\\",
    "[[#]]",
    "[[a#b#c|d|e]]",
    "::nonsense{}",
    "::",
    "::callout",
    "::table",
    "# hi{theme-chartreuse}",
    "# hi{nonsense}",
    "# hi{theme-red}{bold}",
    "x{a}b}",
    "x{a\\\"b c}",
    "x{key=a\\\" theme-red}",
    "x{a\\}",
    "x{a\\\\}",
    "x{k=\"{\"}",
    "x{\"}",
    "| a\\||",
    "| a\\|",
    "|",
    "||",
    "| --- |",
    "|---||",
    "| - - |",
    "```",
    "````rust{filename=\"a\\\"\"}\nx\n````",
    "$$\nx",
    "$${engine=latex}\nx\n$$",
    ">text",
    ">{theme-green}",
    "---{}",
    "a\r\nb",
    "a\n\r",
];

/// Every body line of the committed `getting-started` Bundle, and each with
/// its quote and indent prefix stripped.
pub(crate) fn bundle_lines() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../store/src/collections/getting-started/library");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the committed Bundle") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                let text = std::fs::read_to_string(&path).expect("UTF-8");
                let body = text.splitn(3, "---\n").nth(2).unwrap_or_default();
                for line in body.lines() {
                    out.push(line.to_string());
                    out.push(line.trim_start_matches(['>', ' ']).to_string());
                }
            }
        }
    }
    assert!(!out.is_empty(), "the committed Bundle has bodies");
    out
}

/// Resolves every target to itself, as a reader with every name would.
pub(crate) struct AnyName;

impl Links for AnyName {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            page: target.to_string(),
            block: None,
            title: None,
        }
    }

    fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
        Ok(match block {
            Some(block) => format!("{page}#{block}"),
            None => page.to_string(),
        })
    }
}

const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'X', ' ', '*', '_', '[', ']', '$', '^', '~', '=', '|', '{', '}', '\\', '`', '#',
    '>', '-', '+', ':', '<', '!', '(', ')', '.', '"', '\'',
];

const STYLES: [TextMarkKind; 10] = [
    TextMarkKind::Bold,
    TextMarkKind::Italic,
    TextMarkKind::Underline,
    TextMarkKind::Strikethrough,
    TextMarkKind::Superscript,
    TextMarkKind::Subscript,
    TextMarkKind::Code,
    TextMarkKind::Highlight,
    TextMarkKind::Spoiler,
    TextMarkKind::Math,
];

/// The conformance suite's content generator, without the store around it.
pub(crate) struct Gen {
    rng: StdRng,
    targets: Vec<MarkID>,
}

impl Gen {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            targets: (0..3).map(|_| MarkID::new()).collect(),
        }
    }

    fn chance(&mut self, p: f64) -> bool {
        self.rng.random_bool(p)
    }

    fn pick<T: Clone>(&mut self, items: &[T]) -> T {
        items[self.rng.random_range(0..items.len())].clone()
    }

    fn text(&mut self) -> String {
        let length = self.rng.random_range(1..8);
        (0..length).map(|_| self.pick(ALPHABET)).collect()
    }

    /// Characters as a person might type them: the alphabet, newlines, and
    /// runs of openers.
    pub(crate) fn noise(&mut self) -> String {
        let length = self.rng.random_range(0..40);
        (0..length)
            .map(|_| {
                if self.chance(0.05) {
                    '\n'
                } else {
                    self.pick(ALPHABET)
                }
            })
            .collect()
    }

    pub(crate) fn rich(&mut self) -> RichText {
        let mut segments = Vec::new();
        for _ in 0..self.rng.random_range(1..4) {
            let mut marks = Vec::new();
            let text = self.text();
            if self.chance(0.15) {
                let target = self.pick(&self.targets.clone());
                marks.push(mark(TextMarkKind::Mention, Some(target.to_string())));
            } else if self.chance(0.15) {
                marks.push(mark(
                    TextMarkKind::Link,
                    Some("https://waypoints.so/a b".into()),
                ));
            }
            for style in STYLES {
                if self.chance(0.12) {
                    let data = (style == TextMarkKind::Highlight && self.chance(0.5))
                        .then(|| "yellow".to_string());
                    marks.push(mark(style, data));
                }
            }
            for (kind, value) in [
                (TextMarkKind::Color, "blue"),
                (TextMarkKind::FontSize, "sm"),
                (TextMarkKind::FontFamily, "mono"),
            ] {
                if self.chance(0.08) {
                    marks.push(mark(kind, Some(value.into())));
                }
            }
            segments.push(TextSegment { text, marks });
        }
        RichText::new(inline::coalesce(&segments))
    }

    fn theme(&mut self) -> Option<BlockTheme> {
        self.chance(0.2).then(|| BlockTheme {
            color: self.pick(&[Color::Blue, Color::Red, Color::Default]),
        })
    }

    pub(crate) fn content(&mut self) -> Content {
        let theme = self.theme();
        match self.rng.random_range(0..13) {
            0 => Content::Heading1 {
                rich_text: self.rich(),
                theme,
            },
            1 => Content::Heading2 {
                rich_text: self.rich(),
                theme,
            },
            2 => Content::Heading3 {
                rich_text: self.rich(),
                theme,
            },
            3 => Content::Blockquote { theme },
            4 => Content::ListItem {
                rich_text: self.rich(),
                theme,
            },
            5 => Content::EnumItem {
                rich_text: self.rich(),
                theme,
            },
            6 => Content::Callout {
                icon: self.pick(&["warning", "info"]).into(),
                theme,
            },
            7 => Content::Code {
                language: self.pick(&["rust", ""]).into(),
                lines: (0..self.rng.random_range(0..4))
                    .map(|_| {
                        self.pick(&["fn main() {}", "", "```", "  <!-- not a mark -->", "::"])
                            .into()
                    })
                    .collect(),
                filename: self.chance(0.5).then(|| "main.rs".into()),
                highlights: self.chance(0.3).then(|| vec![1, 3]),
                theme,
            },
            8 => {
                let (rows, cols) = (self.rng.random_range(1..4), self.rng.random_range(1..4));
                let cells = (0..rows)
                    .map(|_| (0..cols).map(|_| self.rich()).collect())
                    .collect();
                let wrapped = self.chance(0.5);
                Content::Table {
                    cells,
                    col_size: wrapped.then(|| {
                        (0..cols)
                            .map(|i| (i % 2 == 0).then_some(Size::Fixed(120)))
                            .collect()
                    }),
                    row_size: None,
                    col_header: self.pick(&[Some(true), None, Some(false)]),
                    row_header: wrapped.then_some(true),
                    fill_width: None,
                    col_colors: wrapped.then(|| vec![Some(Color::Green); cols]),
                    row_colors: None,
                    theme,
                }
            }
            9 => Content::Math {
                expression: self.pick(&["x + y", "\\frac{1}{2}\n+ 3", ""]).into(),
                engine: self.pick(&[MathEngine::Typst, MathEngine::LaTeX]),
                theme,
            },
            10 => Content::Divider { theme },
            _ => Content::Paragraph {
                rich_text: self.rich(),
                theme,
            },
        }
    }
}

fn mark(kind: TextMarkKind, data: Option<String>) -> InlineMark {
    InlineMark {
        kind,
        data: data.map(serde_json::Value::String),
    }
}

/// A block's source as the walker hands it over: its own lines, or its opener
/// where it has none.
pub(crate) fn block_source(content: &Content) -> String {
    let rendered = block::emit_with(content, &AnyName);
    match rendered.lines.is_empty() {
        true => rendered.opener(),
        false => rendered.lines.join("\n"),
    }
}
