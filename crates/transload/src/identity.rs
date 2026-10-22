//! `wp:mark` identity comments — the only place machine residue is allowed, so
//! stripping comments leaves plain text. Containers pair; leaves do not.
//!
//! ```text
//! <!-- wp:mark {"id":"018f…","created":"…","updated":"…"} -->
//! A paragraph.
//!
//! <!-- wp:mark {"id":"018e…"} -->
//! > a blockquote, its children inside the pair
//! <!-- /wp:mark -->
//! ```

use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use domain::{Json, MarkID};

use crate::{
    bundle::{parse_timestamp, render_timestamp},
    refusal::MalformedKind,
};

const MARKER: &str = "wp:mark";

/// What a `wp:mark` comment says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub id: MarkID,
    /// A Content Mark's timestamps. Absent on a Landmark, whose frontmatter
    /// carries them.
    pub created: Option<NaiveDateTime>,
    pub updated: Option<NaiveDateTime>,
    /// Fields this build does not recognize, re-emitted unchanged.
    pub unknown: BTreeMap<String, Json>,
}

impl Identity {
    pub fn new(id: MarkID) -> Self {
        Self {
            id,
            created: None,
            updated: None,
            unknown: BTreeMap::new(),
        }
    }

    pub fn stamped(id: MarkID, created: NaiveDateTime, updated: NaiveDateTime) -> Self {
        Self {
            created: Some(created),
            updated: Some(updated),
            ..Self::new(id)
        }
    }
}

/// One parsed comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Marker {
    Open(Identity),
    Close,
}

/// `id`, then timestamps, then unknown fields in key order.
pub fn emit_open(identity: &Identity) -> String {
    let mut fields = serde_json::Map::new();
    fields.insert("id".into(), Json::String(identity.id.to_string()));
    if let Some(created) = identity.created {
        fields.insert("created".into(), Json::String(render_timestamp(created)));
    }
    if let Some(updated) = identity.updated {
        fields.insert("updated".into(), Json::String(render_timestamp(updated)));
    }
    for (key, value) in &identity.unknown {
        fields.insert(key.clone(), value.clone());
    }
    format!("<!-- {MARKER} {} -->", Json::Object(fields))
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

    let mut fields: serde_json::Map<String, Json> =
        serde_json::from_str(payload.trim()).map_err(|error| {
            grammar(format!(
                "a `{MARKER}` comment whose fields are not JSON: {error}"
            ))
        })?;

    // `src` is Fragment grammar: refused, never resolved.
    if fields.contains_key("src") {
        return Err(MalformedKind::SourceKeyedIdentity);
    }

    let id = match fields.remove("id") {
        Some(Json::String(id)) => id,
        Some(_) => {
            return Err(grammar(format!(
                "a `{MARKER}` comment whose `id` is not a string"
            )));
        }
        None => return Err(grammar(format!("a `{MARKER}` comment with no `id`"))),
    };
    let id: uuid::Uuid = id
        .parse()
        .map_err(|_| grammar(format!("`{id}` is not a Mark id")))?;

    let mut stamp = |key: &str| -> Result<Option<NaiveDateTime>, MalformedKind> {
        match fields.remove(key) {
            Some(Json::String(text)) => parse_timestamp(&text).map(Some).map_err(grammar),
            Some(_) => Err(grammar(format!("`{key}` is not a timestamp string"))),
            None => Ok(None),
        }
    };
    let created = stamp("created")?;
    let updated = stamp("updated")?;

    Ok(Some(Marker::Open(Identity {
        id: MarkID(id),
        created,
        updated,
        unknown: fields.into_iter().collect(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn id(n: u128) -> MarkID {
        MarkID(uuid::Uuid::from_u128(n))
    }

    fn at(milli: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 8, 2)
            .unwrap()
            .and_hms_milli_opt(9, 41, 0, milli)
            .unwrap()
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
    fn timestamps_round_trip() {
        let identity = Identity::stamped(id(1), at(0), at(250));
        let open = emit_open(&identity);
        assert!(
            open.contains("\"updated\":\"2026-08-02T09:41:00.250Z\""),
            "{open}"
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
        let round_tripped = emit_open(&identity);
        assert_eq!(
            round_tripped,
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\
             \"flag\":7,\"from_the_future\":{\"nested\":true}} -->"
        );
        assert_eq!(
            parse(&round_tripped).expect("parse"),
            Some(Marker::Open(identity))
        );
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
            "<!-- wp:mark not json -->",
            "<!-- wp:mark {} -->",
            "<!-- wp:mark {\"id\":7} -->",
            "<!-- wp:mark {\"id\":\"not-a-uuid\"} -->",
            "<!-- wp:mark {\"id\":\"00000000-0000-0000-0000-000000000001\",\"created\":\"yesterday\"} -->",
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
