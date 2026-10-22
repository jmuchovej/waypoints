//! The inline layer: [`RichText`] segments ↔ the dialect's inline syntax.
//!
//! See `docs/dialect.md`, which is normative for everything here.
//!
//! Canonical form: a segment with exactly one mark that has an unambiguous
//! shortcut gets the shortcut; anything else gets `[text]{tokens}`. Adjacent
//! runs with identical marks coalesce and marks are emitted in one order. A
//! mention's label survives verbatim; only a bare `[[id]]` has none.

use crate::{Color, InlineMark, RichText, TextMarkKind, TextSegment};

/// A span's attribute tokens, in the one order the emitter writes them.
/// Links and mentions have no token and are absent.
const TOKEN_ORDER: [(TextMarkKind, &str); 10] = [
    (TextMarkKind::Bold, "bold"),
    (TextMarkKind::Italic, "italic"),
    (TextMarkKind::Underline, "underline"),
    (TextMarkKind::Strikethrough, "strike"),
    (TextMarkKind::Superscript, "sup"),
    (TextMarkKind::Subscript, "sub"),
    (TextMarkKind::Code, "code"),
    (TextMarkKind::Highlight, "highlight"),
    (TextMarkKind::Spoiler, "spoiler"),
    (TextMarkKind::Math, "math"),
];

/// Paired character shortcuts, longest delimiter first so `__` wins over `_`.
const PAIRED: [(&str, TextMarkKind); 6] = [
    ("__", TextMarkKind::Underline),
    ("~~", TextMarkKind::Strikethrough),
    ("==", TextMarkKind::Highlight),
    ("||", TextMarkKind::Spoiler),
    ("*", TextMarkKind::Bold),
    ("_", TextMarkKind::Italic),
];

/// Characters that open a construct wherever they appear, and so are escaped in
/// literal text.
///
/// `~`, `=` and `|` are significant only doubled, and `^` only before `{`, so
/// those are handled contextually in [`escape_text`] and prose keeps them bare.
const ALWAYS_ESCAPED: [char; 7] = ['\\', '[', ']', '*', '_', '`', '$'];

/// Why some inline source is not in the dialect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarError {
    /// What went wrong, in a sentence.
    pub detail: String,
    /// Byte offset into the inline source.
    pub at: usize,
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at byte {})", self.detail, self.at)
    }
}

impl std::error::Error for GrammarError {}

fn has(marks: &[InlineMark], kind: TextMarkKind) -> bool {
    marks.iter().any(|mark| mark.kind == kind)
}

fn find(marks: &[InlineMark], kind: TextMarkKind) -> Option<&InlineMark> {
    marks.iter().find(|mark| mark.kind == kind)
}

/// The string a mark's `data` holds, when it holds a bare string.
fn data_str(mark: &InlineMark) -> Option<&str> {
    mark.data.as_ref()?.as_str()
}

/// Sort position for a mark, so one set of marks has one order.
fn rank(kind: &TextMarkKind) -> usize {
    TOKEN_ORDER
        .iter()
        .position(|(candidate, _)| candidate == kind)
        .unwrap_or_else(|| {
            TOKEN_ORDER.len()
                + match kind {
                    TextMarkKind::Color => 0,
                    TextMarkKind::FontSize => 1,
                    TextMarkKind::FontFamily => 2,
                    TextMarkKind::Link => 3,
                    TextMarkKind::Mention => 4,
                    _ => 5,
                }
        })
}

/// Collapse adjacent segments carrying the same marks, and put every segment's
/// marks in canonical order. Without it, two adjacent bold segments would emit
/// `*a**b*` and read back as one.
pub fn coalesce(segments: &[TextSegment]) -> Vec<TextSegment> {
    let mut out: Vec<TextSegment> = Vec::with_capacity(segments.len());
    for segment in segments {
        // An empty run is dropped unless it carries data (an unlabelled mention
        // still has a target).
        let carries_data = segment.marks.iter().any(|mark| mark.data.is_some());
        if segment.text.is_empty() && !carries_data {
            continue;
        }
        let mut segment = segment.clone();
        segment.marks.sort_by_key(|mark| rank(&mark.kind));

        let mergeable =
            !has(&segment.marks, TextMarkKind::Mention) && !has(&segment.marks, TextMarkKind::Link);
        match out.last_mut() {
            // Carriers never merge: each holds its own target.
            Some(last) if mergeable && last.marks == segment.marks => {
                last.text.push_str(&segment.text);
            }
            _ => out.push(segment),
        }
    }
    out
}

/// Render [`RichText`] as dialect inline source.
///
/// A mention segment's `text` is its label, written out verbatim. An empty one
/// emits a bare `[[id]]`, which is the ingester's cue to regenerate it.
pub fn emit(rich: &RichText) -> String {
    let mut out = String::new();
    for segment in coalesce(rich.segments()) {
        emit_segment(&segment, &mut out);
    }
    out
}

fn emit_segment(segment: &TextSegment, out: &mut String) {
    // Carriers own the text's delimiters, so they are written outermost and
    // anything else becomes an attribute suffix on top.
    if let Some(mention) = find(&segment.marks, TextMarkKind::Mention) {
        out.push_str("[[");
        out.push_str(data_str(mention).unwrap_or_default());
        if !segment.text.is_empty() {
            out.push('|');
            out.push_str(&escape_label(&segment.text));
        }
        out.push_str("]]");
        emit_attrs(&segment.marks, &[TextMarkKind::Mention], out);
        return;
    }

    if let Some(link) = find(&segment.marks, TextMarkKind::Link) {
        out.push('[');
        out.push_str(&escape_text(&segment.text));
        out.push_str("](");
        out.push_str(&escape_href(data_str(link).unwrap_or_default()));
        out.push(')');
        emit_attrs(&segment.marks, &[TextMarkKind::Link], out);
        return;
    }

    match segment.marks.as_slice() {
        // A plain run opening with `{` may follow a bracketed atom or a
        // shortcut, and would read as its attribute list.
        [] if segment.text.starts_with('{') => {
            out.push('\\');
            out.push_str(&escape_text(&segment.text));
        }
        [] => out.push_str(&escape_text(&segment.text)),
        [only] if let Some(spelled) = shortcut(only, &segment.text) => out.push_str(&spelled),
        _ => {
            out.push('[');
            out.push_str(&escape_text(&segment.text));
            out.push(']');
            emit_attrs(&segment.marks, &[], out);
        }
    }
}

/// The character spelling for one mark, when it has an unambiguous one; `None`
/// sends the caller to the span form.
fn shortcut(mark: &InlineMark, text: &str) -> Option<String> {
    let wrap = |delim: &str| Some(format!("{delim}{}{delim}", escape_text(text)));
    match mark.kind {
        TextMarkKind::Bold => wrap("*"),
        // An italic run opening with `{` would make `_{` read as the subscript
        // opener, so that segment takes the span form instead.
        TextMarkKind::Italic if text.starts_with('{') => None,
        TextMarkKind::Italic => wrap("_"),
        TextMarkKind::Underline => wrap("__"),
        TextMarkKind::Strikethrough => wrap("~~"),
        TextMarkKind::Highlight if mark.data.is_none() => wrap("=="),
        TextMarkKind::Spoiler => wrap("||"),
        TextMarkKind::Superscript => Some(format!("^{{{}}}", escape_script(text))),
        TextMarkKind::Subscript => Some(format!("_{{{}}}", escape_script(text))),
        // Code and math interiors are literal: nothing inside them can be
        // escaped, so a text carrying their own delimiter has no shortcut.
        TextMarkKind::Code if !text.contains('`') => Some(format!("`{text}`")),
        TextMarkKind::Math if !text.contains('$') => Some(format!("${text}$")),
        _ => None,
    }
}

/// `{token token …}`, or nothing when no mark needs one.
fn emit_attrs(marks: &[InlineMark], skip: &[TextMarkKind], out: &mut String) {
    let mut tokens = Vec::new();
    for (kind, token) in TOKEN_ORDER {
        if skip.contains(&kind) {
            continue;
        }
        let Some(mark) = find(marks, kind.clone()) else {
            continue;
        };
        match (&kind, data_str(mark)) {
            // A parameterized highlight is a surface color, not the bare mark.
            (TextMarkKind::Highlight, Some(color)) => tokens.push(format!("bg-{color}")),
            _ => tokens.push(token.to_string()),
        }
    }
    for (kind, prefix) in [
        (TextMarkKind::Color, "text-"),
        (TextMarkKind::FontSize, "text-"),
        (TextMarkKind::FontFamily, "font-"),
    ] {
        if skip.contains(&kind) {
            continue;
        }
        if let Some(mark) = find(marks, kind)
            && let Some(value) = data_str(mark)
        {
            tokens.push(format!("{prefix}{value}"));
        }
    }

    if tokens.is_empty() {
        return;
    }
    out.push('{');
    out.push_str(&tokens.join(" "));
    out.push('}');
}

/// Escape what would otherwise open a construct, and nothing more.
///
/// A segment's edges count as neighbours of unknown characters: the next
/// segment may supply the second `~` of a `~~` or the `{` after a `^`.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let last = chars.len().saturating_sub(1);
    for (index, &current) in chars.iter().enumerate() {
        let edge = index == 0 || index == last;
        let doubled =
            chars.get(index + 1) == Some(&current) || (index > 0 && chars[index - 1] == current);
        let opens_script = current == '^' && (index == last || chars.get(index + 1) == Some(&'{'));
        if ALWAYS_ESCAPED.contains(&current)
            || (matches!(current, '~' | '=' | '|') && (doubled || edge))
            || opens_script
        {
            out.push('\\');
        }
        out.push(current);
    }
    out
}

/// A braced script's text, where a `}` would close the script early.
fn escape_script(text: &str) -> String {
    escape_text(text).replace('}', "\\}")
}

/// A link target, where a `)` would end the link early.
fn escape_href(href: &str) -> String {
    href.replace('\\', "\\\\").replace(')', "\\)")
}

/// A mention's label is delimited by `|` and `]]`, so only those need hiding.
fn escape_label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(']', "\\]")
}

/// Read dialect inline source back into [`RichText`].
///
/// Accepts more than the emitter writes (nested shortcuts, tokens in any order)
/// and re-emits canonically.
pub fn parse(src: &str) -> Result<RichText, GrammarError> {
    let mut parser = Parser {
        src,
        at: 0,
        segments: Vec::new(),
        pending: String::new(),
    };
    parser.run(None)?;
    parser.flush();
    Ok(RichText::new(coalesce(&parser.segments)))
}

struct Parser<'a> {
    src: &'a str,
    at: usize,
    segments: Vec<TextSegment>,
    pending: String,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.src[self.at..]
    }

    fn starts(&self, token: &str) -> bool {
        self.rest().starts_with(token)
    }

    fn error(&self, detail: impl Into<String>) -> GrammarError {
        GrammarError {
            detail: detail.into(),
            at: self.at,
        }
    }

    /// Close the run of literal text built up so far.
    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        self.segments.push(TextSegment {
            text: std::mem::take(&mut self.pending),
            marks: Vec::new(),
        });
    }

    /// Apply a mark to every segment produced since `from`.
    fn wrap(&mut self, from: usize, mark: InlineMark) {
        for segment in &mut self.segments[from..] {
            if !has(&segment.marks, mark.kind.clone()) {
                segment.marks.push(mark.clone());
            }
        }
    }

    /// Consume until the end of input, or until `closer` is next.
    fn run(&mut self, closer: Option<&str>) -> Result<(), GrammarError> {
        while self.at < self.src.len() {
            if let Some(closer) = closer
                && self.starts(closer)
            {
                return Ok(());
            }
            if self.atom()? {
                continue;
            }
            self.literal_char();
        }
        match closer {
            Some(closer) => Err(self.error(format!("unclosed `{closer}`"))),
            None => Ok(()),
        }
    }

    /// One literal character, honouring backslash escapes.
    fn literal_char(&mut self) {
        let mut chars = self.rest().chars();
        let Some(first) = chars.next() else {
            self.at = self.src.len();
            return;
        };
        if first == '\\'
            && let Some(escaped) = chars.next()
        {
            self.pending.push(escaped);
            self.at += first.len_utf8() + escaped.len_utf8();
            return;
        }
        self.pending.push(first);
        self.at += first.len_utf8();
    }

    /// Try to read one construct. `false` means the next character is literal.
    fn atom(&mut self) -> Result<bool, GrammarError> {
        if self.starts("[[") {
            self.mention()?;
            return Ok(true);
        }
        if self.starts("[") {
            self.bracketed()?;
            return Ok(true);
        }
        if self.starts("`") {
            self.verbatim('`', TextMarkKind::Code)?;
            return Ok(true);
        }
        if self.starts("$") {
            self.verbatim('$', TextMarkKind::Math)?;
            return Ok(true);
        }
        for (open, kind) in [
            ("^{", TextMarkKind::Superscript),
            ("_{", TextMarkKind::Subscript),
        ] {
            if self.starts(open) {
                self.braced(open, kind)?;
                return Ok(true);
            }
        }
        for (delim, kind) in PAIRED {
            if self.starts(delim) {
                self.paired(delim, kind)?;
                return Ok(true);
            }
        }
        // No bare `^c`: superscript is braced, so a lone caret is ordinary text.
        Ok(false)
    }

    /// `[[<MarkID>|label]]` — the interior is normative, the label advisory.
    fn mention(&mut self) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += 2;

        let mut target = String::new();
        let mut label = String::new();
        let mut in_label = false;
        loop {
            if self.at >= self.src.len() {
                self.at = opened;
                return Err(self.error("unclosed `[[`"));
            }
            if self.starts("]]") {
                self.at += 2;
                break;
            }
            let mut chars = self.rest().chars();
            let current = chars.next().expect("not at end");
            if current == '\\'
                && let Some(escaped) = chars.next()
            {
                let sink = if in_label { &mut label } else { &mut target };
                sink.push(escaped);
                self.at += current.len_utf8() + escaped.len_utf8();
                continue;
            }
            if current == '|' && !in_label {
                in_label = true;
                self.at += 1;
                continue;
            }
            let sink = if in_label { &mut label } else { &mut target };
            sink.push(current);
            self.at += current.len_utf8();
        }

        if target.is_empty() {
            self.at = opened;
            return Err(self.error("a mention with no target"));
        }
        self.segments.push(TextSegment {
            text: label,
            marks: vec![InlineMark {
                kind: TextMarkKind::Mention,
                data: Some(serde_json::Value::String(target)),
            }],
        });
        self.suffix_attrs(self.segments.len() - 1)
    }

    /// `[inline](url)` or `[inline]{tokens}` — a bare `[inline]` is not in the
    /// dialect, since literal brackets are escaped.
    fn bracketed(&mut self) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += 1;
        let from = self.segments.len();
        self.run(Some("]"))?;
        self.flush();
        if !self.starts("]") {
            self.at = opened;
            return Err(self.error("unclosed `[`"));
        }
        self.at += 1;

        if self.starts("(") {
            let href = self.delimited('(', ')')?;
            self.wrap(
                from,
                InlineMark {
                    kind: TextMarkKind::Link,
                    data: Some(serde_json::Value::String(href)),
                },
            );
            return self.suffix_attrs(from);
        }
        if self.starts("{") {
            return self.suffix_attrs(from);
        }
        self.at = opened;
        Err(self.error(
            "a bracketed run needs a `(url)` or `{tokens}` suffix; a literal bracket is escaped",
        ))
    }

    /// An optional `{tokens}` suffix applied to everything from `from`.
    fn suffix_attrs(&mut self, from: usize) -> Result<(), GrammarError> {
        if !self.starts("{") {
            return Ok(());
        }
        let tokens = self.delimited('{', '}')?;
        for token in tokens.split_whitespace() {
            self.wrap(
                from,
                parse_token(token).ok_or_else(|| {
                    self.error(format!("`{token}` is not a mark token in this dialect"))
                })?,
            );
        }
        Ok(())
    }

    /// The text between one `open` and its matching `close`.
    fn delimited(&mut self, open: char, close: char) -> Result<String, GrammarError> {
        let opened = self.at;
        self.at += open.len_utf8();
        let mut out = String::new();
        loop {
            let mut chars = self.rest().chars();
            let Some(current) = chars.next() else {
                self.at = opened;
                return Err(self.error(format!("unclosed `{open}`")));
            };
            if current == close {
                self.at += close.len_utf8();
                return Ok(out);
            }
            if current == '\\'
                && let Some(escaped) = chars.next()
            {
                out.push(escaped);
                self.at += current.len_utf8() + escaped.len_utf8();
                continue;
            }
            out.push(current);
            self.at += current.len_utf8();
        }
    }

    /// A literal-interior atom: code and math take their text verbatim.
    fn verbatim(&mut self, delim: char, kind: TextMarkKind) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += delim.len_utf8();
        let Some(end) = self.rest().find(delim) else {
            self.at = opened;
            return Err(self.error(format!("unclosed `{delim}`")));
        };
        let text = self.rest()[..end].to_string();
        self.at += end + delim.len_utf8();
        self.segments.push(TextSegment {
            text,
            marks: vec![InlineMark { kind, data: None }],
        });
        self.suffix_attrs(self.segments.len() - 1)
    }

    /// `^{…}` / `_{…}` — super- and subscript over a braced run.
    fn braced(&mut self, open: &str, kind: TextMarkKind) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += open.len() - 1; // leave the `{` for `run`'s closer
        self.at += 1;
        let from = self.segments.len();
        self.run(Some("}"))?;
        self.flush();
        if !self.starts("}") {
            self.at = opened;
            return Err(self.error(format!("unclosed `{open}`")));
        }
        self.at += 1;
        self.wrap(from, InlineMark { kind, data: None });
        self.suffix_attrs(from)
    }

    /// A paired character shortcut: `*…*`, `_…_`, `__…__`, and the rest.
    fn paired(&mut self, delim: &str, kind: TextMarkKind) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += delim.len();
        let from = self.segments.len();
        self.run(Some(delim))?;
        self.flush();
        if !self.starts(delim) {
            self.at = opened;
            return Err(self.error(format!("unclosed `{delim}`")));
        }
        self.at += delim.len();
        self.wrap(from, InlineMark { kind, data: None });
        self.suffix_attrs(from)
    }
}

/// One attribute token back into the mark it spells.
fn parse_token(token: &str) -> Option<InlineMark> {
    if let Some((kind, _)) = TOKEN_ORDER.iter().find(|(_, spelled)| *spelled == token) {
        return Some(InlineMark {
            kind: kind.clone(),
            data: None,
        });
    }
    let parameterized = |kind: TextMarkKind, value: &str| {
        Some(InlineMark {
            kind,
            data: Some(serde_json::Value::String(value.to_string())),
        })
    };
    if let Some(color) = token.strip_prefix("bg-") {
        return parameterized(TextMarkKind::Highlight, color);
    }
    if let Some(family) = token.strip_prefix("font-") {
        return parameterized(TextMarkKind::FontFamily, family);
    }
    if let Some(value) = token.strip_prefix("text-") {
        // `text-` prefixes both color and font size; `Color` decides which.
        let is_color =
            serde_json::from_value::<Color>(serde_json::Value::String(value.to_string())).is_ok();
        return if is_color {
            parameterized(TextMarkKind::Color, value)
        } else {
            parameterized(TextMarkKind::FontSize, value)
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Json;

    fn seg(text: &str, marks: Vec<InlineMark>) -> TextSegment {
        TextSegment {
            text: text.into(),
            marks,
        }
    }

    fn mark(kind: TextMarkKind) -> InlineMark {
        InlineMark { kind, data: None }
    }

    fn mark_with(kind: TextMarkKind, data: Json) -> InlineMark {
        InlineMark {
            kind,
            data: Some(data),
        }
    }

    fn rich(segments: Vec<TextSegment>) -> RichText {
        RichText::new(segments)
    }

    fn round_trips(source: &str) {
        let parsed = parse(source).unwrap_or_else(|e| panic!("parse {source:?}: {e}"));
        assert_eq!(emit(&parsed), source, "emit(parse({source:?})) != source");
    }

    fn survives(segments: Vec<TextSegment>) {
        let original = rich(coalesce(&segments));
        let source = emit(&original);
        let parsed = parse(&source).unwrap_or_else(|e| panic!("parse {source:?}: {e}"));
        assert_eq!(parsed.segments(), original.segments(), "via {source:?}");
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(emit(&rich(vec![seg("hello", vec![])])), "hello");
    }

    #[test]
    fn one_mark_takes_its_shortcut() {
        let cases = [
            (TextMarkKind::Bold, "*hi*"),
            (TextMarkKind::Italic, "_hi_"),
            (TextMarkKind::Underline, "__hi__"),
            (TextMarkKind::Strikethrough, "~~hi~~"),
            (TextMarkKind::Highlight, "==hi=="),
            (TextMarkKind::Spoiler, "||hi||"),
            (TextMarkKind::Superscript, "^{hi}"),
            (TextMarkKind::Subscript, "_{hi}"),
            (TextMarkKind::Code, "`hi`"),
            (TextMarkKind::Math, "$hi$"),
        ];
        for (kind, expected) in cases {
            let got = emit(&rich(vec![seg("hi", vec![mark(kind.clone())])]));
            assert_eq!(got, expected, "{kind:?}");
        }
    }

    #[test]
    fn bold_is_one_star_never_two() {
        let got = emit(&rich(vec![seg("hi", vec![mark(TextMarkKind::Bold)])]));
        assert_eq!(got, "*hi*");
        assert!(!got.contains("**"), "`**` never appears in an emitted file");
    }

    #[test]
    fn more_than_one_mark_takes_the_span_form() {
        let got = emit(&rich(vec![seg(
            "hi",
            vec![mark(TextMarkKind::Italic), mark(TextMarkKind::Bold)],
        )]));
        assert_eq!(got, "[hi]{bold italic}");
    }

    #[test]
    fn an_italic_run_opening_with_a_brace_avoids_the_subscript_opener() {
        let got = emit(&rich(vec![seg("{a}", vec![mark(TextMarkKind::Italic)])]));
        assert_eq!(got, "[{a}]{italic}");
        assert!(!got.starts_with("_{"), "`_{{` is the subscript opener");
    }

    #[test]
    fn parameterized_marks_use_tailwind_parallel_tokens() {
        let got = emit(&rich(vec![seg(
            "hi",
            vec![
                mark_with(TextMarkKind::Color, serde_json::json!("blue")),
                mark_with(TextMarkKind::FontFamily, serde_json::json!("mono")),
            ],
        )]));
        assert_eq!(got, "[hi]{text-blue font-mono}");
    }

    #[test]
    fn a_parameterized_highlight_is_a_surface_color() {
        assert_eq!(
            emit(&rich(vec![seg("hi", vec![mark(TextMarkKind::Highlight)])])),
            "==hi=="
        );
        assert_eq!(
            emit(&rich(vec![seg(
                "hi",
                vec![mark_with(
                    TextMarkKind::Highlight,
                    serde_json::json!("yellow")
                )]
            )])),
            "[hi]{bg-yellow}"
        );
    }

    #[test]
    fn a_mention_is_a_wiki_link_carrying_a_bare_id() {
        let id = "018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55";
        let got = emit(&rich(vec![seg(
            "Foundation",
            vec![mark_with(TextMarkKind::Mention, serde_json::json!(id))],
        )]));
        assert_eq!(got, format!("[[{id}|Foundation]]"));
    }

    #[test]
    fn literal_delimiters_are_escaped() {
        assert_eq!(
            emit(&rich(vec![seg("a*b_c[d]", vec![])])),
            "a\\*b\\_c\\[d\\]"
        );
    }

    #[test]
    fn singly_occurring_tildes_and_pipes_stay_legible() {
        assert_eq!(emit(&rich(vec![seg("a~b|c=d", vec![])])), "a~b|c=d");
        assert_eq!(
            emit(&rich(vec![seg("a~~b", vec![])])),
            "a\\~\\~b",
            "a doubled tilde would open strikethrough"
        );
    }

    #[test]
    fn adjacent_runs_with_the_same_marks_coalesce() {
        let got = emit(&rich(vec![
            seg("a", vec![mark(TextMarkKind::Bold)]),
            seg("b", vec![mark(TextMarkKind::Bold)]),
        ]));
        assert_eq!(
            got, "*ab*",
            "not `*a**b*`, which would read back as one run"
        );
    }

    #[test]
    fn adjacent_mentions_never_coalesce() {
        let coalesced = coalesce(&[
            seg(
                "x",
                vec![mark_with(TextMarkKind::Mention, serde_json::json!("a"))],
            ),
            seg(
                "y",
                vec![mark_with(TextMarkKind::Mention, serde_json::json!("b"))],
            ),
        ]);
        assert_eq!(coalesced.len(), 2, "each mention carries its own target");
    }

    #[test]
    fn empty_segments_are_dropped() {
        assert!(coalesce(&[seg("", vec![])]).is_empty());
    }

    #[test]
    fn code_and_math_interiors_are_literal() {
        assert_eq!(
            emit(&rich(vec![seg("a`b", vec![mark(TextMarkKind::Code)])])),
            "[a\\`b]{code}"
        );
    }

    #[test]
    fn escapes_come_back_as_literals() {
        let parsed = parse("a\\*b").expect("parse");
        assert_eq!(parsed.plain_text(), "a*b");
        assert_eq!(parsed.segments().len(), 1);
        assert!(parsed.segments()[0].marks.is_empty());
    }

    #[test]
    fn each_shortcut_reads_back_as_its_mark() {
        let cases = [
            ("*hi*", TextMarkKind::Bold),
            ("_hi_", TextMarkKind::Italic),
            ("__hi__", TextMarkKind::Underline),
            ("~~hi~~", TextMarkKind::Strikethrough),
            ("==hi==", TextMarkKind::Highlight),
            ("||hi||", TextMarkKind::Spoiler),
            ("^{hi}", TextMarkKind::Superscript),
            ("_{hi}", TextMarkKind::Subscript),
            ("`hi`", TextMarkKind::Code),
            ("$hi$", TextMarkKind::Math),
        ];
        for (source, kind) in cases {
            let parsed = parse(source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(parsed.plain_text(), "hi", "{source}");
            assert_eq!(parsed.segments()[0].marks, vec![mark(kind)], "{source}");
        }
    }

    #[test]
    fn a_lone_caret_is_ordinary_text() {
        let parsed = parse("2^10").expect("parse");
        assert_eq!(parsed.plain_text(), "2^10");
        assert!(parsed.segments()[0].marks.is_empty());
        assert_eq!(emit(&parsed), "2^10");
    }

    #[test]
    fn a_literal_script_opener_is_escaped() {
        assert_eq!(emit(&rich(vec![seg("x^{2}", vec![])])), "x\\^{2}");
        round_trips("x\\^{2}");
    }

    #[test]
    fn a_hand_edited_label_is_kept_as_the_anchor_text() {
        let id = "018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55";
        let labelled = parse(&format!("[[{id}|the Foundation series]]")).expect("parse");
        assert_eq!(labelled.segments()[0].text, "the Foundation series");

        let bare = parse(&format!("[[{id}]]")).expect("parse");
        assert_eq!(
            bare.segments()[0].text,
            "",
            "nothing to keep, so nothing is"
        );
        assert_eq!(
            labelled.segments()[0].marks,
            bare.segments()[0].marks,
            "the target is identical either way"
        );
    }

    #[test]
    fn tokens_are_accepted_in_any_order_and_emitted_in_one() {
        let parsed = parse("[hi]{italic bold}").expect("parse");
        assert_eq!(emit(&parsed), "[hi]{bold italic}");
    }

    #[test]
    fn nested_shortcuts_canonicalize_to_a_span() {
        let parsed = parse("*_hi_*").expect("parse");
        assert_eq!(parsed.plain_text(), "hi");
        assert_eq!(emit(&parsed), "[hi]{bold italic}");
    }

    #[test]
    fn a_text_size_is_not_a_color() {
        let color = parse("[hi]{text-blue}").expect("parse");
        assert_eq!(color.segments()[0].marks[0].kind, TextMarkKind::Color);
        let size = parse("[hi]{text-sm}").expect("parse");
        assert_eq!(size.segments()[0].marks[0].kind, TextMarkKind::FontSize);
    }

    #[test]
    fn a_mention_label_is_read_but_the_target_is_what_matters() {
        let id = "018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55";
        let parsed = parse(&format!("[[{id}|Foundation]]")).expect("parse");
        let segment = &parsed.segments()[0];
        assert_eq!(segment.text, "Foundation");
        assert_eq!(segment.marks[0].kind, TextMarkKind::Mention);
        assert_eq!(segment.marks[0].data, Some(serde_json::json!(id)));
    }

    #[test]
    fn malformed_inlines_are_refused_with_a_location() {
        for source in [
            "*unclosed",
            "[[",
            "[[|label]]",
            "[bare bracketed]",
            "[hi]{nonsense-token}",
            "`unclosed",
        ] {
            let error = parse(source).expect_err(source);
            assert!(!error.detail.is_empty(), "{source}");
            assert!(error.at <= source.len(), "{source}");
        }
    }

    #[test]
    fn canonical_sources_round_trip() {
        for source in [
            "hello",
            "*bold*",
            "_italic_",
            "__underline__",
            "~~struck~~",
            "==marked==",
            "||hidden||",
            "^{up}",
            "_{down}",
            "`code`",
            "$x + y$",
            "[hi]{bold italic}",
            "[hi]{text-blue}",
            "[hi]{text-sm font-mono}",
            "[hi]{bg-yellow}",
            "[{a}]{italic}",
            "[[018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55|Foundation]]",
            "[docs](https://waypoints.so)",
            "a \\*literal\\* star",
            "mixed *bold* and _italic_ prose",
        ] {
            round_trips(source);
        }
    }

    #[test]
    fn stores_survive_a_trip_through_syntax() {
        survives(vec![seg("plain", vec![])]);
        survives(vec![seg("hi", vec![mark(TextMarkKind::Bold)])]);
        survives(vec![
            seg("a ", vec![]),
            seg("bold", vec![mark(TextMarkKind::Bold)]),
            seg(" and ", vec![]),
            seg("italic", vec![mark(TextMarkKind::Italic)]),
        ]);
        survives(vec![seg(
            "hi",
            vec![mark(TextMarkKind::Bold), mark(TextMarkKind::Underline)],
        )]);
        survives(vec![seg(
            "blue",
            vec![mark_with(TextMarkKind::Color, serde_json::json!("blue"))],
        )]);
        survives(vec![seg(
            "Foundation",
            vec![mark_with(
                TextMarkKind::Mention,
                serde_json::json!("018f3c2a-5f1e-7c3b-9a44-2d9f0b1c7e55"),
            )],
        )]);
        survives(vec![seg(
            "docs",
            vec![mark_with(
                TextMarkKind::Link,
                serde_json::json!("https://waypoints.so"),
            )],
        )]);
        survives(vec![seg("* _ ` $ ^ [ ] \\ ~~ == ||", vec![])]);
    }
}
