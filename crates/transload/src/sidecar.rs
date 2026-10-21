//! The JSONL sidecars: one Rust type per line shape, written by export and
//! read back by ingest, so the two directions cannot drift apart.

use chrono::NaiveDateTime;
use domain::{Json, MarkID};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::bundle::{parse_timestamp, render_timestamp};

/// A `marks.jsonl` line — `{id, created, updated, extra?}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MarkLine {
    pub id: MarkID,
    #[serde(with = "stamp")]
    pub created: NaiveDateTime,
    #[serde(with = "stamp")]
    pub updated: NaiveDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Json>,
}

/// A `terrain.jsonl` line — `{src, dst, kind, order?, extra?}`.
///
/// `kind` is the storage spelling (`child`, `reading::<chart>::<feature>`, …);
/// `TerrainKind::to_storage` writes it and `ingest::strict_kind` reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TerrainLine {
    pub src: MarkID,
    pub dst: MarkID,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Json>,
}

/// Every non-blank line of a sidecar, numbered from 1 as a reader counts them,
/// parsed as `T`. The error is serde's message with its own "at line 1" position
/// dropped: the caller's [`crate::refusal::Location`] names the line.
pub(crate) fn lines<T: DeserializeOwned>(
    text: &str,
) -> impl Iterator<Item = (usize, Result<T, String>)> + '_ {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| (index + 1, serde_json::from_str(line).map_err(message)))
}

/// Append `row` as one JSONL line.
pub(crate) fn push_line<T: Serialize>(out: &mut String, row: &T) {
    out.push_str(&serde_json::to_string(row).expect("a sidecar line serializes"));
    out.push('\n');
}

fn message(error: serde_json::Error) -> String {
    let text = error.to_string();
    match text.rsplit_once(" at line ") {
        Some((head, _)) => head.to_string(),
        None => text,
    }
}

/// ISO-8601 UTC milliseconds, the Bundle's one timestamp spelling.
mod stamp {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};

    use crate::bundle::{parse_timestamp, render_timestamp};

    pub fn serialize<S: Serializer>(at: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&render_timestamp(*at))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<NaiveDateTime, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_timestamp(&text).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0084d8df-ded3-4f52-a01b-5958592e95c1";
    const OTHER: &str = "01015347-fe76-4ac9-930d-e79cf0f0892a";
    const AT: &str = "2026-10-22T14:31:31.272Z";

    fn parsed<T: DeserializeOwned>(line: &str) -> Result<T, String> {
        lines::<T>(line).next().expect("one line").1
    }

    fn emitted<T: Serialize>(row: &T) -> String {
        let mut out = String::new();
        push_line(&mut out, row);
        out
    }

    #[test]
    fn a_mark_line_round_trips_byte_for_byte() {
        for line in [
            format!("{{\"id\":\"{ID}\",\"created\":\"{AT}\",\"updated\":\"{AT}\"}}\n"),
            format!(
                "{{\"id\":\"{ID}\",\"created\":\"{AT}\",\"updated\":\"{AT}\",\"extra\":{{\"b\":1,\"a\":[true,null]}}}}\n"
            ),
        ] {
            let row: MarkLine = parsed(&line).expect("parses");
            assert_eq!(emitted(&row), line);
        }
    }

    #[test]
    fn a_terrain_line_round_trips_byte_for_byte() {
        for line in [
            format!("{{\"src\":\"{ID}\",\"dst\":\"{OTHER}\",\"kind\":\"custom\"}}\n"),
            format!(
                "{{\"src\":\"{ID}\",\"dst\":\"{OTHER}\",\"kind\":\"child\",\"order\":0,\"extra\":{{\"x\":1}}}}\n"
            ),
            format!(
                "{{\"src\":\"{ID}\",\"dst\":\"{OTHER}\",\"kind\":\"reading::c4d8::a1b2\",\"order\":2}}\n"
            ),
        ] {
            let row: TerrainLine = parsed(&line).expect("parses");
            assert_eq!(emitted(&row), line);
        }
    }

    #[test]
    fn a_null_extra_is_no_extra() {
        let row: MarkLine = parsed(&format!(
            "{{\"id\":\"{ID}\",\"created\":\"{AT}\",\"updated\":\"{AT}\",\"extra\":null}}"
        ))
        .expect("parses");
        assert_eq!(row.extra, None);
        assert!(!emitted(&row).contains("extra"));
    }

    #[test]
    fn an_unknown_key_is_refused_without_serdes_own_position() {
        let error = parsed::<MarkLine>(&format!(
            "{{\"id\":\"{ID}\",\"created\":\"{AT}\",\"updated\":\"{AT}\",\"colour\":1}}"
        ))
        .expect_err("refused");
        assert!(error.contains("colour"), "{error}");
        assert!(!error.contains(" at line "), "{error}");
    }

    #[test]
    fn a_missing_or_mistyped_field_is_refused_by_name() {
        let error = parsed::<MarkLine>(&format!("{{\"id\":\"{ID}\",\"created\":\"{AT}\"}}"))
            .expect_err("refused");
        assert!(error.contains("updated"), "{error}");

        let error = parsed::<MarkLine>(&format!(
            "{{\"id\":\"{ID}\",\"created\":\"yesterday\",\"updated\":\"{AT}\"}}"
        ))
        .expect_err("refused");
        assert!(error.contains("ISO-8601"), "{error}");

        let error = parsed::<TerrainLine>(&format!(
            "{{\"src\":\"{ID}\",\"dst\":\"{OTHER}\",\"kind\":\"child\",\"order\":\"1\"}}"
        ))
        .expect_err("refused");
        assert!(error.contains("order") || error.contains("u32"), "{error}");

        let error = parsed::<TerrainLine>(&format!(
            "{{\"src\":\"{ID}\",\"dst\":\"not-a-uuid\",\"kind\":\"child\"}}"
        ))
        .expect_err("refused");
        assert!(error.contains("UUID"), "{error}");
    }

    #[test]
    fn lines_are_numbered_as_a_reader_counts_them() {
        let text = format!(
            "\n{{\"id\":\"{ID}\",\"created\":\"{AT}\",\"updated\":\"{AT}\"}}\n\n   \nnot json\n"
        );
        let numbered: Vec<(usize, bool)> = lines::<MarkLine>(&text)
            .map(|(line, row)| (line, row.is_ok()))
            .collect();
        assert_eq!(numbered, vec![(2, true), (5, false)]);
    }
}
