//! The hand-written inline reader, kept as the differential oracle while the
//! grammar replaces it.

use super::{GrammarError, Links, PAIRED, coalesce, has, parse_token};
use crate::{InlineMark, RichText, TextMarkKind, TextSegment};

pub(crate) fn parse_with(src: &str, links: &dyn Links) -> Result<RichText, GrammarError> {
    let mut parser = Parser {
        src,
        links,
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
    links: &'a dyn Links,
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
        GrammarError::new(self.src, self.at, detail)
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        self.segments.push(TextSegment {
            text: std::mem::take(&mut self.pending),
            marks: Vec::new(),
        });
    }

    fn wrap(&mut self, from: usize, mark: InlineMark) {
        for segment in &mut self.segments[from..] {
            if !has(&segment.marks, mark.kind.clone()) {
                segment.marks.push(mark.clone());
            }
        }
    }

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
        Ok(false)
    }

    fn mention(&mut self) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += 2;

        let mut page = String::new();
        let mut block: Option<String> = None;
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
            let (character, width) = match (current, chars.next()) {
                ('\\', Some(escaped)) => (escaped, current.len_utf8() + escaped.len_utf8()),
                ('|', _) if !in_label => {
                    in_label = true;
                    self.at += 1;
                    continue;
                }
                ('#', _) if !in_label && block.is_none() => {
                    block = Some(String::new());
                    self.at += 1;
                    continue;
                }
                _ => (current, current.len_utf8()),
            };
            let sink = match (in_label, &mut block) {
                (true, _) => &mut label,
                (false, Some(block)) => block,
                (false, None) => &mut page,
            };
            sink.push(character);
            self.at += width;
        }

        if page.is_empty() && block.is_none() {
            self.at = opened;
            return Err(self.error("a mention with no target"));
        }
        let target = self
            .links
            .resolve(&page, block.as_deref())
            .map_err(|detail| GrammarError::new(self.src, opened, detail))?;
        self.segments.push(TextSegment {
            text: label,
            marks: vec![InlineMark {
                kind: TextMarkKind::Mention,
                data: Some(serde_json::Value::String(target)),
            }],
        });
        self.suffix_attrs(self.segments.len() - 1)
    }

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

    fn braced(&mut self, open: &str, kind: TextMarkKind) -> Result<(), GrammarError> {
        self.flush();
        let opened = self.at;
        self.at += open.len() - 1;
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
