use domain::{InlineMark, RichText, TextSegment};

/// Split a `RichText` at the given byte offset into two halves.
///
/// The offset is measured across the concatenated segment texts. Returns
/// `(left, right)` where `left` contains everything before the offset and
/// `right` contains the rest.
pub fn split_at(rt: &RichText, offset: usize) -> (RichText, RichText) {
    let mut lsegs = Vec::new();
    let mut rsegs = Vec::new();
    let mut consumed = 0usize;
    let mut split_done = false;

    for seg in rt.segments() {
        if split_done {
            rsegs.push(seg.clone());
            continue;
        }

        let seg_len = seg.text.len();
        if consumed + seg_len <= offset {
            lsegs.push(seg.clone());
            consumed += seg_len;
        } else if consumed >= offset {
            rsegs.push(seg.clone());
            split_done = true;
        } else {
            let split_pos = offset - consumed;
            let (left_text, right_text) = seg.text.split_at(split_pos);
            if !left_text.is_empty() {
                lsegs.push(TextSegment {
                    text: left_text.to_string(),
                    marks: seg.marks.clone(),
                });
            }
            if !right_text.is_empty() {
                rsegs.push(TextSegment {
                    text: right_text.to_string(),
                    marks: seg.marks.clone(),
                });
            }
            consumed += seg_len;
            split_done = true;
        }
    }

    (RichText::new(lsegs), RichText::new(rsegs))
}

/// Create a new `RichText` by appending another's segments, coalescing
/// adjacent segments that share identical marks.
pub fn merge(a: &RichText, b: &RichText) -> RichText {
    let mut segments: Vec<TextSegment> = a.segments().to_vec();
    for seg in b.segments() {
        if seg.text.is_empty() {
            continue;
        }
        if let Some(last) = segments.last_mut()
            && last.marks == seg.marks
        {
            last.text.push_str(&seg.text);
            continue;
        }
        segments.push(seg.clone());
    }
    RichText::new(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::TextMarkKind;

    #[test]
    fn test_split_at_segment_boundary() {
        let rt = RichText::new(vec![
            TextSegment {
                text: "Hello, ".to_string(),
                marks: vec![],
            },
            TextSegment {
                text: "world!".to_string(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                }],
            },
        ]);
        let (left, right) = split_at(&rt, 7);
        assert_eq!(left.plain_text(), "Hello, ");
        assert_eq!(right.plain_text(), "world!");
        assert_eq!(left.segments().len(), 1);
        assert_eq!(right.segments().len(), 1);
        assert_eq!(right.segments()[0].marks[0].kind, TextMarkKind::Bold);
    }

    #[test]
    fn test_split_at_within_segment() {
        let rt = RichText::from("Hello, world!".to_string());
        let (left, right) = split_at(&rt, 5);
        assert_eq!(left.plain_text(), "Hello");
        assert_eq!(right.plain_text(), ", world!");
    }

    #[test]
    fn test_split_at_beginning() {
        let rt = RichText::from("Hello".to_string());
        let (left, right) = split_at(&rt, 0);
        assert!(left.is_empty());
        assert_eq!(right.plain_text(), "Hello");
    }

    #[test]
    fn test_split_at_end() {
        let rt = RichText::from("Hello".to_string());
        let (left, right) = split_at(&rt, 5);
        assert_eq!(left.plain_text(), "Hello");
        assert!(right.is_empty());
    }

    #[test]
    fn test_split_preserves_marks() {
        let bold = vec![InlineMark {
            kind: TextMarkKind::Bold,
            data: None,
        }];
        let rt = RichText::new(vec![TextSegment {
            text: "ABCDE".to_string(),
            marks: bold.clone(),
        }]);
        let (left, right) = split_at(&rt, 2);
        assert_eq!(left.segments()[0].marks, bold);
        assert_eq!(right.segments()[0].marks, bold);
        assert_eq!(left.plain_text(), "AB");
        assert_eq!(right.plain_text(), "CDE");
    }

    #[test]
    fn test_merge_basic() {
        let a = RichText::from("Hello".to_string());
        let b = RichText::from(", world!".to_string());
        let merged = merge(&a, &b);
        assert_eq!(merged.plain_text(), "Hello, world!");
        assert_eq!(merged.segments().len(), 1);
    }

    #[test]
    fn test_merge_different_marks() {
        let bold = vec![InlineMark {
            kind: TextMarkKind::Bold,
            data: None,
        }];
        let a = RichText::from("Hello".to_string());
        let b = RichText::new(vec![TextSegment {
            text: " world".to_string(),
            marks: bold,
        }]);
        let merged = merge(&a, &b);
        assert_eq!(merged.plain_text(), "Hello world");
        assert_eq!(merged.segments().len(), 2);
    }

    #[test]
    fn test_merge_empty() {
        let a = RichText::from("Hello".to_string());
        let b = RichText::default();
        let merged = merge(&a, &b);
        assert_eq!(merged.plain_text(), "Hello");
        assert_eq!(merged.segments().len(), 1);
    }

    #[test]
    fn test_split_then_merge_roundtrip() {
        let original = RichText::new(vec![
            TextSegment {
                text: "Hello, ".to_string(),
                marks: vec![],
            },
            TextSegment {
                text: "bold".to_string(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Bold,
                    data: None,
                }],
            },
            TextSegment {
                text: " world!".to_string(),
                marks: vec![],
            },
        ]);
        let (left, right) = split_at(&original, 9);
        let merged = merge(&left, &right);
        assert_eq!(merged.plain_text(), original.plain_text());
        assert_eq!(merged.segments().len(), 3);
    }
}
