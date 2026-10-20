//! `wp:mark` identity comments — the one place machine residue is allowed.
//!
//! The carrier rule splits a Bundle in two: anything a reader should perceive
//! rides in dialect syntax or attributes, and identity rides in comments.
//! Nothing else exists. That is what makes the 1980 test possible — strip every
//! comment and what remains is text, proper.
//!
//! Containers are paired, leaves are not:
//!
//! ```text
//! <!-- wp:mark {"id":"018f…"} -->
//! A paragraph. One comment, no closer.
//!
//! <!-- wp:mark {"id":"018e…"} -->
//! > a blockquote, whose children sit inside the pair
//! <!-- /wp:mark -->
//! ```
//!
//! An identity comment is `id`-keyed, always. The `src` form — importer
//! provenance — belongs to Fragment grammar, and an export-Bundle carrying one
//! is refused rather than resolved.

use std::collections::BTreeMap;

use domain::{Json, MarkID};

use crate::refusal::MalformedKind;

/// The marker word every identity comment opens with.
const MARKER: &str = "wp:mark";

/// What a `wp:mark` comment says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    /// The Mark this comment names.
    pub id: MarkID,
    /// Fields this build does not recognize, kept so a comment written by a
    /// newer exporter survives a trip through an older one unchanged.
    ///
    /// A `BTreeMap`, so re-serialization is byte-stable without trusting the
    /// order they arrived in.
    pub unknown: BTreeMap<String, Json>,
}

impl Identity {
    /// An identity carrying nothing but its id.
    pub fn new(id: MarkID) -> Self {
        Self {
            id,
            unknown: BTreeMap::new(),
        }
    }
}

/// One parsed comment: a Mark opening, or a container closing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Marker {
    /// `<!-- wp:mark {…} -->` — this Mark begins here.
    Open(Identity),
    /// `<!-- /wp:mark -->` — the innermost open container ends here.
    Close,
}

// ── Emit ─────────────────────────────────────────────────────────────────────

/// `<!-- wp:mark {"id":"…"} -->`
///
/// `id` comes first and unknown fields follow in key order, so one identity has
/// one spelling.
pub fn emit_open(identity: &Identity) -> String {
    let mut fields = serde_json::Map::new();
    fields.insert(
        "id".to_string(),
        Json::String(identity.id.inner().to_string().to_lowercase()),
    );
    for (key, value) in &identity.unknown {
        fields.insert(key.clone(), value.clone());
    }
    let json = Json::Object(fields);
    format!("<!-- {MARKER} {json} -->")
}

/// `<!-- /wp:mark -->` — the closer every container pairs with.
pub fn emit_close() -> String {
    format!("<!-- /{MARKER} -->")
}

// ── Parse ────────────────────────────────────────────────────────────────────

/// Read one HTML comment.
///
/// `Ok(None)` means the comment is not an identity marker at all. The caller
/// decides what to do with that — in a Bundle, nothing else is allowed, but
/// this module does not own that rule.
pub fn parse(raw: &str) -> Result<Option<Marker>, MalformedKind> {
    let trimmed = raw.trim();
    let Some(body) = trimmed
        .strip_prefix("<!--")
        .and_then(|rest| rest.strip_suffix("-->"))
    else {
        return Ok(None);
    };
    let body = body.trim();

    if let Some(rest) = body.strip_prefix('/') {
        return if rest.trim() == MARKER {
            Ok(Some(Marker::Close))
        } else {
            Ok(None)
        };
    }

    let Some(payload) = body.strip_prefix(MARKER) else {
        return Ok(None);
    };
    let payload = payload.trim();
    if payload.is_empty() {
        return Err(MalformedKind::Grammar {
            detail: format!("a `{MARKER}` comment with no fields"),
        });
    }

    let fields: serde_json::Map<String, Json> =
        serde_json::from_str(payload).map_err(|error| MalformedKind::Grammar {
            detail: format!("a `{MARKER}` comment whose fields are not JSON: {error}"),
        })?;

    // `src` is Fragment grammar. Refuse it rather than resolving it, so a
    // Fragment never silently ingests as though it were an export.
    if fields.contains_key("src") {
        return Err(MalformedKind::SourceKeyedIdentity);
    }

    let Some(id) = fields.get("id") else {
        return Err(MalformedKind::Grammar {
            detail: format!("a `{MARKER}` comment with no `id`"),
        });
    };
    let Some(id) = id.as_str() else {
        return Err(MalformedKind::Grammar {
            detail: format!("a `{MARKER}` comment whose `id` is not a string"),
        });
    };
    let id: uuid::Uuid = id.parse().map_err(|_| MalformedKind::Grammar {
        detail: format!("`{id}` is not a Mark id"),
    })?;

    let unknown = fields
        .into_iter()
        .filter(|(key, _)| key != "id")
        .collect::<BTreeMap<_, _>>();

    Ok(Some(Marker::Open(Identity {
        id: MarkID(id),
        unknown,
    })))
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
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\"} -->"
        );
        assert_eq!(parse(&open).expect("parse"), Some(Marker::Open(identity)));
    }

    #[test]
    fn a_container_pairs() {
        assert_eq!(emit_close(), "<!-- /wp:mark -->");
        assert_eq!(parse(&emit_close()).expect("parse"), Some(Marker::Close));
    }

    #[test]
    fn unknown_fields_survive_re_serialization() {
        let raw = "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\
                   \"from_the_future\":{\"nested\":true},\"flag\":7} -->";
        let Some(Marker::Open(identity)) = parse(raw).expect("parse") else {
            panic!("expected an opening marker");
        };
        assert_eq!(identity.unknown.len(), 2);

        // Re-emitted: `id` first, unknowns in key order.
        let round_tripped = emit_open(&identity);
        assert_eq!(
            round_tripped,
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\
             \"flag\":7,\"from_the_future\":{\"nested\":true}} -->"
        );
        // And parsing that again is a fixed point, which is what byte-stability
        // over an unknown field actually requires.
        assert_eq!(
            parse(&round_tripped).expect("parse"),
            Some(Marker::Open(identity))
        );
    }

    #[test]
    fn a_src_keyed_identity_is_refused_not_resolved() {
        let raw = "<!-- wp:mark {\"src\":\"logseq/page-7\"} -->";
        assert_eq!(
            parse(raw).expect_err("must refuse"),
            MalformedKind::SourceKeyedIdentity
        );

        // Even alongside an id: a Fragment is not half an export.
        let both = "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\
                    \"src\":\"logseq/page-7\"} -->";
        assert_eq!(
            parse(both).expect_err("must refuse"),
            MalformedKind::SourceKeyedIdentity
        );
    }

    #[test]
    fn malformed_comments_are_refused() {
        for raw in [
            "<!-- wp:mark -->",
            "<!-- wp:mark not json -->",
            "<!-- wp:mark {} -->",
            "<!-- wp:mark {\"id\":7} -->",
            "<!-- wp:mark {\"id\":\"not-a-uuid\"} -->",
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

    #[test]
    fn ids_are_lowercase_so_one_mark_has_one_spelling() {
        let emitted = emit_open(&Identity::new(id(0xABCDEF)));
        assert!(!emitted.contains("ABCDEF"), "got {emitted}");
        assert!(emitted.contains("abcdef"), "got {emitted}");
    }
}
