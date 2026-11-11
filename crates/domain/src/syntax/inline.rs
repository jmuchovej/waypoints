//! The inline layer: [`RichText`] segments ↔ the dialect's inline syntax.
//!
//! See `docs/dialect.md`, which is normative for everything here.
//!
//! Canonical form: a segment with exactly one mark that has an unambiguous
//! shortcut gets the shortcut; anything else gets `[text]{tokens}`. Adjacent
//! runs with identical marks coalesce and marks are emitted in one order. A
//! mention's label survives verbatim; only a bare `[[name]]` has none.

use chumsky::{
    error::{RichPattern, RichReason},
    input::{InputRef, MapExtra},
    prelude::*,
};
use derive_more::Display;

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

/// Paired character shortcuts that also need their interiors parsed, unlike
/// code and math, which `verbatim` reads raw. Longest delimiter first, so `__`
/// wins over `_`.
const SHORTCUTS_WITH_INTERIOR_PARSING: [(&str, TextMarkKind); 6] = [
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

/// Why some dialect source is not in the dialect.
#[derive(Clone, Debug, PartialEq, Eq, Display)]
#[display("{detail} (line {line}, column {column})")]
pub struct GrammarError {
    /// What went wrong, in a sentence.
    pub detail: String,
    /// Byte offset into the source handed to `parse`.
    pub at: usize,
    /// 1-indexed line within that source.
    pub line: usize,
    /// 1-indexed column within `line`, in characters.
    pub column: usize,
}

impl GrammarError {
    /// A refusal at byte `at` of `src`.
    pub(crate) fn new(src: &str, at: usize, detail: impl Into<String>) -> Self {
        let before = &src[..at.min(src.len())];
        Self {
            detail: detail.into(),
            at,
            line: before.matches('\n').count() + 1,
            column: before.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        }
    }

    /// The same refusal, moved `by` bytes into `src`, which encloses the source
    /// it was found in.
    pub(crate) fn shifted(self, src: &str, by: usize) -> Self {
        Self::new(src, self.at + by, self.detail)
    }
}

impl std::error::Error for GrammarError {}

/// A mention target as written: `[[page]]`, or `[[page#block]]` for a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkName {
    pub page: String,
    /// An id prefix naming a block within `page`.
    pub block: Option<String>,
    /// What a bare mention's label regenerates to; a label equal to it is
    /// elided.
    pub title: Option<String>,
}

/// Spells mention targets and reads them back. The dialect owns the grammar,
/// the caller the names.
pub trait Links {
    fn name(&self, target: &str) -> LinkName;
    fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String>;
}

/// The target itself as the page, for callers with no names to give.
pub struct Verbatim;

impl Links for Verbatim {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            page: target.to_string(),
            block: None,
            title: None,
        }
    }

    fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
        match block {
            None => Ok(page.to_string()),
            Some(_) => Err("a block link needs names to resolve against".into()),
        }
    }
}

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
/// A mention segment's `text` is its label, written out verbatim. An empty one,
/// or one equal to the name's `title`, emits a bare `[[name]]`, which is the
/// ingester's cue to regenerate it.
pub fn emit(rich: &RichText) -> String {
    emit_with(rich, &Verbatim)
}

/// [`emit`], spelling mention targets through `links`.
pub fn emit_with(rich: &RichText, links: &dyn Links) -> String {
    let mut out = String::new();
    for segment in coalesce(rich.segments()) {
        emit_segment(&segment, links, &mut out);
    }
    out
}

fn emit_segment(segment: &TextSegment, links: &dyn Links, out: &mut String) {
    // Carriers own the text's delimiters, so they are written outermost and
    // anything else becomes an attribute suffix on top.
    if let Some(mention) = find(&segment.marks, TextMarkKind::Mention) {
        let name = links.name(data_str(mention).unwrap_or_default());
        out.push_str("[[");
        out.push_str(&escape_page(&name.page));
        if let Some(block) = &name.block {
            out.push('#');
            out.push_str(block);
        }
        let elided = name.title.as_deref() == Some(segment.text.as_str());
        if !segment.text.is_empty() && !elided {
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

/// A mention's page also ends at `#`, where a block prefix begins.
fn escape_page(text: &str) -> String {
    escape_label(text).replace('#', "\\#")
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
    parse_with(src, &Verbatim)
}

/// [`parse`], reading mention targets back through `links`.
pub fn parse_with(src: &str, links: &dyn Links) -> Result<RichText, GrammarError> {
    let (pieces, errors) = grammar(links).parse(src).into_output_errors();
    if let Some(error) = errors.first() {
        return Err(refusal(src, error));
    }
    let mut built = Built::default();
    built.add(pieces.and_then(Result::ok).unwrap_or_default());
    built.flush();
    Ok(RichText::new(coalesce(&built.segments)))
}

/// How many inline constructs may enclose one another in the source. A debug
/// build spends tens of kilobytes of stack per level, so this keeps a parse at
/// the ceiling within half of a two-megabyte thread.
pub const MAX_DEPTH: usize = 16;

pub(crate) type Extra<'a, C = ()> = extra::Full<Rich<'a, char>, (), C>;

/// Where a run sits: how many constructs enclose it, and what closes it.
#[derive(Clone, Copy, Debug, Default)]
struct Nest {
    depth: usize,
    closer: Option<&'static str>,
}

/// A run nested past [`MAX_DEPTH`], refused rather than descended into.
#[derive(Clone, Debug)]
struct TooDeep;

#[derive(Clone, Debug)]
enum Piece {
    Text(String),
    /// A segment of its own — a mention, code, or math — and its suffix tokens.
    Leaf(TextSegment, Vec<InlineMark>),
    /// A run and the marks laid over all of it, in the order they apply.
    Group(Vec<Piece>, Vec<InlineMark>),
}

/// Segments as the source builds them: literal text accumulates, and each
/// construct closes the text before it.
#[derive(Default)]
struct Built {
    segments: Vec<TextSegment>,
    pending: String,
}

impl Built {
    fn flush(&mut self) {
        if !self.pending.is_empty() {
            self.segments.push(TextSegment {
                text: std::mem::take(&mut self.pending),
                marks: Vec::new(),
            });
        }
    }

    /// A mark over every segment from `from` that does not already carry its kind.
    fn wrap(&mut self, from: usize, mark: &InlineMark) {
        for segment in &mut self.segments[from..] {
            if !has(&segment.marks, mark.kind.clone()) {
                segment.marks.push(mark.clone());
            }
        }
    }

    fn add(&mut self, pieces: Vec<Piece>) {
        for piece in pieces {
            match piece {
                Piece::Text(text) => self.pending.push_str(&text),
                Piece::Leaf(segment, suffix) => {
                    self.flush();
                    self.segments.push(segment);
                    let from = self.segments.len() - 1;
                    for mark in &suffix {
                        self.wrap(from, mark);
                    }
                }
                Piece::Group(inner, marks) => {
                    self.flush();
                    let from = self.segments.len();
                    self.add(inner);
                    self.flush();
                    for mark in &marks {
                        self.wrap(from, mark);
                    }
                }
            }
        }
    }
}

fn too_deep() -> String {
    format!("inline marks nest deeper than {MAX_DEPTH} levels")
}

/// `\x` is `x`; a lone trailing `\` is itself.
pub(crate) fn escape<'a, C: 'a>() -> impl Parser<'a, &'a str, char, Extra<'a, C>> + Clone {
    just('\\')
        .ignore_then(any().or_not())
        .map(|escaped: Option<char>| escaped.unwrap_or('\\'))
}

/// The inline grammar. An opener commits: a construct that has read its
/// opener always succeeds as a parser and reports what it lacks as an emitted
/// refusal, so nothing falls through to literal text.
fn grammar<'a>(
    links: &'a dyn Links,
) -> impl Parser<'a, &'a str, Result<Vec<Piece>, TooDeep>, Extra<'a, Nest>> {
    recursive(move |run| {
        let closer = custom(|input: &mut InputRef<'a, '_, &'a str, Extra<'a, Nest>>| {
            let before = input.save();
            let matched = match input.ctx().closer {
                Some(closer) => closer
                    .chars()
                    .all(|expected| input.next() == Some(expected)),
                None => false,
            };
            if matched {
                return Ok(());
            }
            let span = input.span_since(before.cursor());
            input.rewind(before);
            Err(Rich::custom(span, "the run's closer"))
        });
        let deeper = move |close: &'static str| {
            empty()
                .map_with(
                    move |_, extra: &mut MapExtra<'a, '_, &'a str, Extra<'a, Nest>>| Nest {
                        depth: extra.ctx().depth + 1,
                        closer: Some(close),
                    },
                )
                .ignore_with_ctx(run.clone())
        };

        let token = choice((escape(), none_of('}'))).map_with(|c, extra| (c, extra.span()));
        let suffix = just('{')
            .map_with(|_, extra| extra.span())
            .then(token.repeated().collect::<Vec<(char, SimpleSpan)>>())
            .then(just('}').or_not())
            .validate(|((opened, chars), closed), _, emitter| {
                if closed.is_none() {
                    emitter.emit(Rich::custom(opened, "unclosed `{`"));
                    return Vec::new();
                }
                let mut marks = Vec::new();
                for (token, span) in words(&chars) {
                    match parse_token(&token) {
                        Some(mark) => marks.push(mark),
                        None => emitter.emit(Rich::custom(
                            span,
                            format!("`{token}` is not a mark token in this dialect"),
                        )),
                    }
                }
                marks
            })
            .or_not()
            .map(Option::unwrap_or_default)
            .boxed();

        let mention_text = |stops: &'static str| {
            choice((escape(), none_of(stops).and_is(just("]]").not())))
                .repeated()
                .collect::<String>()
        };
        let mention = just("[[")
            .map_with(|_, extra| extra.span())
            .then(mention_text("\\|#"))
            .then(just('#').ignore_then(mention_text("\\|")).or_not())
            .then(just('|').ignore_then(mention_text("\\")).or_not())
            .then(just("]]").or_not())
            .validate(
                move |((((opened, page), block), label), closed), _, emitter| {
                    if closed.is_none() {
                        emitter.emit(Rich::custom(opened, "unclosed `[[`"));
                        return None;
                    }
                    if page.is_empty() && block.is_none() {
                        emitter.emit(Rich::custom(opened, "a mention with no target"));
                        return None;
                    }
                    match links.resolve(&page, block.as_deref()) {
                        Ok(target) => Some(TextSegment {
                            text: label.unwrap_or_default(),
                            marks: vec![InlineMark {
                                kind: TextMarkKind::Mention,
                                data: Some(serde_json::Value::String(target)),
                            }],
                        }),
                        Err(detail) => {
                            emitter.emit(Rich::custom(opened, detail));
                            None
                        }
                    }
                },
            )
            .then(suffix.clone())
            .map(leaf);

        let href = just('(')
            .map_with(|_, extra| extra.span())
            .then(
                choice((escape(), none_of(')')))
                    .repeated()
                    .collect::<String>(),
            )
            .then(just(')').or_not())
            .validate(|((opened, href), closed), _, emitter| {
                if closed.is_none() {
                    emitter.emit(Rich::custom(opened, "unclosed `(`"));
                }
                href
            });
        let bracketed = just('[')
            .map_with(|_, extra| extra.span())
            .then(deeper("]"))
            .then(just(']').or_not())
            .validate(|((opened, inner), closed), _, emitter| {
                let inner = match inner {
                    Err(TooDeep) => {
                        emitter.emit(Rich::custom(opened, too_deep()));
                        None
                    }
                    Ok(_) if closed.is_none() => {
                        emitter.emit(Rich::custom(opened, "unclosed `]`"));
                        None
                    }
                    Ok(inner) => Some(inner),
                };
                (opened, inner)
            })
            .then(href.or_not())
            .then(just('{').rewind().or_not())
            .validate(|(((opened, inner), href), brace), _, emitter| {
                if inner.is_some() && href.is_none() && brace.is_none() {
                    emitter.emit(Rich::custom(
                        opened,
                        "a bracketed run needs a `(url)` or `{tokens}` suffix; a literal \
                         bracket is escaped",
                    ));
                }
                let link = href.map(|href| InlineMark {
                    kind: TextMarkKind::Link,
                    data: Some(serde_json::Value::String(href)),
                });
                (inner.unwrap_or_default(), link)
            })
            .then(suffix.clone())
            .map(|((inner, link), suffix)| {
                let mut marks: Vec<InlineMark> = link.into_iter().collect();
                marks.extend(suffix);
                Piece::Group(inner, marks)
            });

        let verbatim = |delim: &'static str, kind: TextMarkKind| {
            just(delim)
                .map_with(|_, extra| extra.span())
                .then(none_of(delim).repeated().to_slice())
                .then(just(delim).or_not())
                .validate(move |((opened, text), closed), _, emitter| {
                    if closed.is_none() {
                        emitter.emit(Rich::custom(opened, format!("unclosed `{delim}`")));
                        return None;
                    }
                    Some(TextSegment {
                        text: String::from(text),
                        marks: vec![InlineMark {
                            kind: kind.clone(),
                            data: None,
                        }],
                    })
                })
                .then(suffix.clone())
                .map(leaf)
        };

        let group = |open: &'static str, close: &'static str, kind: TextMarkKind| {
            just(open)
                .map_with(|_, extra| extra.span())
                .then(deeper(close))
                .then(just(close).or_not())
                .validate(move |((opened, inner), closed), _, emitter| match inner {
                    Err(TooDeep) => {
                        emitter.emit(Rich::custom(opened, too_deep()));
                        Vec::new()
                    }
                    Ok(inner) => {
                        if closed.is_none() {
                            emitter.emit(Rich::custom(opened, format!("unclosed `{close}`")));
                        }
                        inner
                    }
                })
                .then(suffix.clone())
                .map(move |(inner, suffix)| {
                    let mut marks = vec![InlineMark {
                        kind: kind.clone(),
                        data: None,
                    }];
                    marks.extend(suffix);
                    Piece::Group(inner, marks)
                })
        };

        let atom = choice((
            mention,
            bracketed,
            verbatim("`", TextMarkKind::Code),
            verbatim("$", TextMarkKind::Math),
            group("^{", "}", TextMarkKind::Superscript),
            group("_{", "}", TextMarkKind::Subscript),
            choice(SHORTCUTS_WITH_INTERIOR_PARSING.map(|(delim, kind)| group(delim, delim, kind))),
        ))
        .boxed();

        // `~`, `=` and `|` open only doubled, `^` only before `{`.
        let plain = none_of("\\[`$*_")
            .and_is(choice((just("^{"), just("~~"), just("=="), just("||"))).not());
        let text = closer
            .not()
            .ignore_then(choice((escape(), plain)))
            .repeated()
            .at_least(1)
            .collect::<String>()
            .map(Piece::Text);

        let ceiling = custom(|input: &mut InputRef<'a, '_, &'a str, Extra<'a, Nest>>| {
            let before = input.save();
            if input.ctx().depth > MAX_DEPTH {
                while input.next().is_some() {}
                return Ok(Err(TooDeep));
            }
            Err(Rich::custom(
                input.span_since(before.cursor()),
                "within the nesting ceiling",
            ))
        });
        choice((
            ceiling,
            closer
                .not()
                .ignore_then(choice((atom, text)))
                .repeated()
                .collect::<Vec<Piece>>()
                .map(Ok),
        ))
    })
}

fn leaf((segment, suffix): (Option<TextSegment>, Vec<InlineMark>)) -> Piece {
    match segment {
        Some(segment) => Piece::Leaf(segment, suffix),
        None => Piece::Text(String::new()),
    }
}

/// Whitespace-separated words of an unescaped token list, each with the span
/// of its source.
fn words(chars: &[(char, SimpleSpan)]) -> Vec<(String, SimpleSpan)> {
    let mut out: Vec<(String, SimpleSpan)> = Vec::new();
    let mut open = false;
    for (c, span) in chars {
        if c.is_whitespace() {
            open = false;
            continue;
        }
        match out.last_mut() {
            Some((word, at)) if open => {
                word.push(*c);
                *at = SimpleSpan::from(at.start..span.end);
            }
            _ => {
                out.push((c.to_string(), *span));
                open = true;
            }
        }
    }
    out
}

/// A parse error as the dialect says it: a sentence at a position.
pub(crate) fn refusal(src: &str, error: &Rich<'_, char>) -> GrammarError {
    let detail = match error.reason() {
        RichReason::Custom(sentence) => sentence.clone(),
        RichReason::ExpectedFound { expected, found } => {
            let expected: Vec<String> = expected.iter().map(pattern).collect();
            let found = match found {
                Some(found) => format!("`{}`", **found),
                None => "the end of the text".to_string(),
            };
            format!("expected {}, found {found}", expected.join(" or "))
        }
    };
    GrammarError::new(src, error.span().start, detail)
}

fn pattern(pattern: &RichPattern<'_, char>) -> String {
    match pattern {
        RichPattern::Token(token) => format!("`{}`", **token),
        RichPattern::Label(label) => label.to_string(),
        RichPattern::EndOfInput => "the end of the text".to_string(),
        _ => "something else".to_string(),
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

    /// Pages by name; a block is `page:block`.
    struct Names(Vec<(&'static str, &'static str)>);

    impl Links for Names {
        fn name(&self, target: &str) -> LinkName {
            let (page, block) = match target.split_once(':') {
                Some((page, block)) => (page, Some(block.to_string())),
                None => (target, None),
            };
            let (name, title) = self
                .0
                .iter()
                .find(|(_, id)| *id == page)
                .map(|(name, _)| (name.to_string(), name.to_string()))
                .unwrap_or_else(|| (page.to_string(), page.to_string()));
            LinkName {
                page: name,
                title: block.is_none().then_some(title),
                block,
            }
        }

        fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
            let (_, id) = self
                .0
                .iter()
                .find(|(name, _)| *name == page)
                .ok_or_else(|| format!("no page `{page}`"))?;
            Ok(match block {
                Some(block) => format!("{id}:{block}"),
                None => id.to_string(),
            })
        }
    }

    #[test]
    fn named_mentions_escape_the_page_and_elide_a_title_label() {
        let names = Names(vec![("A#b|c]d", "p1"), ("Cosmos", "p2")]);
        let mention = |target: &str, label: &str| {
            RichText::new(vec![seg(
                label,
                vec![mark_with(TextMarkKind::Mention, serde_json::json!(target))],
            )])
        };
        for (rich, spelled) in [
            (mention("p1", "x"), "[[A\\#b\\|c\\]d|x]]"),
            (mention("p2", "Cosmos"), "[[Cosmos]]"),
            (mention("p2", "the book"), "[[Cosmos|the book]]"),
            (mention("p2:4f2a", ""), "[[Cosmos#4f2a]]"),
            (mention("p2:4f2a", "that bit"), "[[Cosmos#4f2a|that bit]]"),
        ] {
            let emitted = emit_with(&rich, &names);
            assert_eq!(emitted, spelled);
            let parsed = parse_with(&emitted, &names).expect("parse");
            let target = parsed.segments()[0].marks[0].data.clone();
            assert_eq!(target, rich.segments()[0].marks[0].data, "{spelled}");
        }
        let unknown = parse_with("[[Nowhere]]", &names).expect_err("unresolvable");
        assert_eq!(unknown.detail, "no page `Nowhere`");
    }

    fn parity_names() -> Names {
        Names(vec![("A#b|c]d\\e", "p1"), ("Cosmos", "p2")])
    }

    fn with(text: &str, kind: TextMarkKind, data: &str) -> TextSegment {
        seg(text, vec![mark_with(kind, serde_json::json!(data))])
    }

    /// The emitter's branches as a checklist — every token, shortcut, prefix,
    /// mention spelling and escape — each a row of segments.
    fn spellings() -> Vec<(String, Vec<TextSegment>)> {
        use TextMarkKind::*;
        let plain = |text: &str| seg(text, vec![]);
        let one = |text: &str, kind: TextMarkKind| seg(text, vec![mark(kind)]);

        let mut rows: Vec<(String, Vec<TextSegment>)> = Vec::new();
        for (kind, _) in TOKEN_ORDER {
            rows.push((format!("{kind:?} shortcut"), vec![one("hi", kind.clone())]));
            let partner = if kind == Bold { Italic } else { Bold };
            rows.push((
                format!("{kind:?} token"),
                vec![seg("hi", vec![mark(kind), mark(partner)])],
            ));
        }
        for (delim, kind) in SHORTCUTS_WITH_INTERIOR_PARSING {
            rows.push((
                format!("`{delim}` in prose"),
                vec![plain("a "), one("b", kind), plain(" c")],
            ));
        }
        for (text, kind) in [
            ("{a}", Italic),
            ("a`b", Code),
            ("a$b", Math),
            ("~", Strikethrough),
            ("=", Highlight),
            ("|", Spoiler),
            ("a}b", Superscript),
            ("a}b", Subscript),
            ("*a* _b_", Superscript),
        ] {
            rows.push((format!("{kind:?} over {text:?}"), vec![one(text, kind)]));
        }
        rows.extend([
            ("bg-".into(), vec![with("hi", Highlight, "yellow")]),
            ("text- colour".into(), vec![with("hi", Color, "blue")]),
            ("text- size".into(), vec![with("hi", FontSize, "sm")]),
            ("font-".into(), vec![with("hi", FontFamily, "mono")]),
            (
                "every token at once".into(),
                vec![seg(
                    "hi",
                    vec![
                        mark(Bold),
                        mark_with(Highlight, serde_json::json!("yellow")),
                        mark_with(Color, serde_json::json!("red")),
                        mark_with(FontSize, serde_json::json!("lg")),
                        mark_with(FontFamily, serde_json::json!("serif")),
                    ],
                )],
            ),
            ("bare mention".into(), vec![with("", Mention, "p2")]),
            (
                "labelled mention".into(),
                vec![with("the book", Mention, "p2")],
            ),
            ("block mention".into(), vec![with("", Mention, "p2:4f2a")]),
            (
                "labelled block mention".into(),
                vec![with("that bit", Mention, "p2:4f2a")],
            ),
            ("escaped page".into(), vec![with("", Mention, "p1")]),
            (
                "escaped label".into(),
                vec![with("a|b]c\\d#e", Mention, "p2")],
            ),
            (
                "mention with a suffix".into(),
                vec![seg(
                    "x",
                    vec![mark(Bold), mark_with(Mention, serde_json::json!("p2"))],
                )],
            ),
            (
                "link".into(),
                vec![with("docs", Link, "https://waypoints.so")],
            ),
            (
                "link escaping its text and href".into(),
                vec![with("a*b]", Link, "https://x.so/(a)\\b")],
            ),
            (
                "link with a suffix".into(),
                vec![seg(
                    "docs",
                    vec![
                        mark(Italic),
                        mark_with(Link, serde_json::json!("https://waypoints.so")),
                    ],
                )],
            ),
            ("a brace-opening run".into(), vec![plain("{x}")]),
            (
                "a brace-opening run after a shortcut".into(),
                vec![one("a", Bold), plain("{b}")],
            ),
            (
                "a brace-opening run after a span".into(),
                vec![seg("a", vec![mark(Bold), mark(Italic)]), plain("{b}")],
            ),
            (
                "a tilde at a segment's edge".into(),
                vec![plain("a~"), one("b", Strikethrough)],
            ),
            (
                "a caret at a segment's edge".into(),
                vec![plain("x^"), one("{2}", Bold)],
            ),
            (
                "adjacent runs with the same marks".into(),
                vec![one("a", Bold), one("b", Bold)],
            ),
            (
                "an empty run between marked ones".into(),
                vec![one("a", Bold), plain(""), one("b", Italic)],
            ),
        ]);
        for c in ALWAYS_ESCAPED {
            rows.push((format!("literal `{c}`"), vec![plain(&format!("a{c}b"))]));
        }
        for text in [
            "a~~b", "~a", "a~", "a==b", "=a", "a=", "a||b", "|a", "a|", "a~b=c|d", "x^{2}", "x^",
            "2^10", "^",
        ] {
            rows.push((format!("prose {text:?}"), vec![plain(text)]));
        }
        rows
    }

    #[test]
    fn every_spelling_the_emitter_writes_reads_back() {
        let names = parity_names();
        for (name, segments) in spellings() {
            // Emitted as given, so the emitter's own coalescing is exercised.
            let original = rich(coalesce(&segments));
            let source = emit_with(&rich(segments), &names);
            let parsed = parse_with(&source, &names)
                .unwrap_or_else(|e| panic!("{name}: parse {source:?}: {e}"));
            assert_eq!(
                parsed.segments(),
                original.segments(),
                "{name}: via {source:?}"
            );
            assert_eq!(emit_with(&parsed, &names), source, "{name}: not canonical");
        }

        // A label equal to the title is elided; the target is what survives.
        let titled = rich(vec![with("Cosmos", TextMarkKind::Mention, "p2")]);
        let source = emit_with(&titled, &names);
        assert_eq!(source, "[[Cosmos]]");
        let parsed = parse_with(&source, &names).expect("parse");
        assert_eq!(parsed.segments(), [with("", TextMarkKind::Mention, "p2")]);
    }

    /// `^{x}` nested `depth` deep, with `opener` innermost.
    fn nested(depth: usize, opener: &str, closer: &str) -> String {
        format!(
            "{}{opener}x{closer}{}",
            "^{".repeat(depth - 1),
            "}".repeat(depth - 1)
        )
    }

    #[test]
    fn every_nestable_construct_parses_at_the_ceiling_and_refuses_past_it() {
        let constructs = [
            ("^{", "}"),
            ("_{", "}"),
            ("[", "]{bold}"),
            ("__", "__"),
            ("~~", "~~"),
            ("==", "=="),
            ("||", "||"),
            ("*", "*"),
            ("_", "_"),
        ];
        for (opener, closer) in constructs {
            let at_ceiling = nested(MAX_DEPTH, opener, closer);
            parse(&at_ceiling).unwrap_or_else(|e| panic!("{opener} at the ceiling: {e}"));

            let past = nested(MAX_DEPTH + 1, opener, closer);
            let error = parse(&past).expect_err(opener);
            assert_eq!(error.detail, too_deep(), "{opener}");
            assert_eq!(
                (error.at, error.line, error.column),
                (2 * MAX_DEPTH, 1, 2 * MAX_DEPTH + 1),
                "{opener}"
            );
        }
    }

    #[test]
    fn a_mebibyte_of_openers_refuses_on_a_small_stack() {
        let openers = "^{".repeat(512 * 1024);
        let error = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || parse(&openers).expect_err("past the ceiling"))
            .expect("thread")
            .join()
            .expect("the parse completes on a two-megabyte stack");
        assert_eq!(error.detail, "inline marks nest deeper than 16 levels");
        assert_eq!((error.line, error.column), (1, 2 * MAX_DEPTH + 1));
    }

    #[test]
    fn the_ceiling_parse_fits_a_two_megabyte_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (opener, closer) in [("*", "*"), ("[", "]{bold}"), ("^{", "}")] {
                    parse(&nested(MAX_DEPTH, opener, closer)).expect("at the ceiling");
                }
            })
            .expect("thread")
            .join()
            .expect("the parse completes on a two-megabyte stack");
    }
}
