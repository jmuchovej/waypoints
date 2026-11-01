//! Optional timestamps in the one spelling a Bundle uses: ISO-8601 UTC with an
//! explicit `Z` and millisecond precision. `#[serde(with = "timestamp")]`.

use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::bundle::{parse_timestamp, render_timestamp};

pub(crate) fn serialize<S: Serializer>(
    at: &Option<NaiveDateTime>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match at {
        Some(at) => render_timestamp(*at).serialize(serializer),
        None => serializer.serialize_none(),
    }
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<NaiveDateTime>, D::Error> {
    let text: Option<String> = Option::deserialize(deserializer)?;
    text.map(|text| parse_timestamp(&text).map_err(serde::de::Error::custom))
        .transpose()
}
