//! `wp:mark` identity comments — the only place machine residue is allowed, so
//! stripping comments leaves plain text. Containers pair; leaves do not.
//!
//! ```text
//! <!-- wp:mark 018f… -->
//! A paragraph.
//!
//! <!-- wp:mark 018e… -->
//! > a blockquote, its children inside the pair
//! <!-- /wp:mark -->
//! ```
//!
//! A comment carries the id and nothing else; timestamps and `extra` ride in
//! `marks.jsonl`.

use domain::{Json, MarkID};

use crate::refusal::MalformedKind;

const MARKER: &str = "wp:mark";

/// What a `wp:mark` comment says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub id: MarkID,
}

impl Identity {
    pub fn new(id: MarkID) -> Self {
        Self { id }
    }
}

/// One parsed comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Marker {
    Open(Identity),
    Close,
}

pub fn emit_open(identity: &Identity) -> String {
    format!("<!-- {MARKER} {} -->", identity.id)
}

pub fn emit_close() -> String {
    format!("<!-- /{MARKER} -->")
}

/// `Ok(None)` when the comment is not an identity marker.
pub fn parse(raw: &str) -> Result<Option<Marker>, MalformedKind> {
    let grammar = |detail: String| MalformedKind::Grammar { detail };
    let Some(body) = raw
        .trim()
        .strip_prefix("<!--")
        .and_then(|rest| rest.strip_suffix("-->"))
    else {
        return Ok(None);
    };
    let body = body.trim();

    if let Some(rest) = body.strip_prefix('/') {
        return Ok((rest.trim() == MARKER).then_some(Marker::Close));
    }
    let Some(payload) = body.strip_prefix(MARKER) else {
        return Ok(None);
    };
    let payload = payload.trim();

    // `src` is Fragment grammar: refused, never resolved.
    if let Ok(Json::Object(fields)) = serde_json::from_str::<Json>(payload)
        && fields.contains_key("src")
    {
        return Err(MalformedKind::SourceKeyedIdentity);
    }
    let id: uuid::Uuid = payload.parse().map_err(|_| {
        grammar(format!(
            "a `{MARKER}` comment takes a Mark id, not `{payload}`"
        ))
    })?;
    Ok(Some(Marker::Open(Identity::new(MarkID(id)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> MarkID {
        MarkID(uuid::Uuid::from_u128(n))
    }

    #[test]
    fn a_leaf_carries_one_comment_and_no_closer() {
        let identity = Identity::new(id(1));
        let open = emit_open(&identity);
        assert_eq!(
            open,
            "<!-- wp:mark 00000000-0000-0000-0000-000000000001 -->"
        );
        assert_eq!(parse(&open).expect("parse"), Some(Marker::Open(identity)));
    }

    #[test]
    fn a_container_pairs() {
        assert_eq!(emit_close(), "<!-- /wp:mark -->");
        assert_eq!(parse(&emit_close()).expect("parse"), Some(Marker::Close));
    }

    #[test]
    fn a_src_keyed_identity_is_refused_even_beside_an_id() {
        for raw in [
            "<!-- wp:mark {\"src\":\"logseq/page-7\"} -->",
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\"src\":\"x\"} -->",
        ] {
            assert_eq!(
                parse(raw).expect_err("must refuse"),
                MalformedKind::SourceKeyedIdentity
            );
        }
    }

    #[test]
    fn malformed_comments_are_refused() {
        for raw in [
            "<!-- wp:mark -->",
            "<!-- wp:mark not-a-uuid -->",
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\"} -->",
        ] {
            assert!(parse(raw).is_err(), "{raw} should be refused");
        }
    }

    #[test]
    fn an_ordinary_comment_is_not_an_identity() {
        assert_eq!(parse("<!-- just a note -->").expect("parse"), None);
        assert_eq!(parse("not a comment at all").expect("parse"), None);
        assert_eq!(parse("<!-- /something-else -->").expect("parse"), None);
    }
}
