use chrono::NaiveDateTime;
use domain::mark::{Mark, MarkData, MarkID};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{StoreError, StoreResult};

fn mark_before_save(mut row: MarkRow, _op: bedrock::WriteOp) -> MarkRow {
    // Truncate to milliseconds: bedrock's turso engine stores NaiveDateTime at
    // millisecond granularity and rejects sub-millisecond values.
    let now = chrono::Local::now().naive_utc();
    let millis = now.and_utc().timestamp_millis();
    row.updated_at = chrono::DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.naive_utc())
        .unwrap_or(now);
    row
}

/// Bedrock-backed storage row for a [`Mark`].
///
/// No `mark` discriminator column — the kind is encoded in `data` (MarkData is
/// internally tagged via serde). Kind-based queries will use a materialized view.
/// No sync columns (`updated_hlc`, `updated_by_device`) — sync was removed.
#[derive(Debug, Clone, PartialEq, bedrock::Collection)]
#[bedrock(before_save = mark_before_save)]
pub struct MarkRow {
    #[bedrock(id)]
    pub id: MarkID,

    /// The full mark payload, stored as jsonb. MarkData derives bedrock::Document.
    pub data: MarkData,

    /// Schemaless overflow bag — anything that doesn't belong in typed columns.
    pub extra: Option<Value>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,

    pub deleted_at: Option<NaiveDateTime>,
    pub archived_at: Option<NaiveDateTime>,
}

impl MarkRow {
    /// Construct a fresh row from domain parts. `extra` is None; `updated_at`
    /// mirrors `created_at`; audit fields are all None.
    pub fn from_parts(id: MarkID, data: MarkData, created_at: NaiveDateTime) -> Self {
        Self {
            id,
            data,
            extra: None,
            created_at,
            updated_at: created_at,
            created_by: None,
            updated_by: None,
            deleted_at: None,
            archived_at: None,
        }
    }
}

impl TryFrom<MarkRow> for Mark {
    type Error = StoreError;

    fn try_from(row: MarkRow) -> StoreResult<Self> {
        let mut builder = Mark::builder();

        builder = match row.data {
            MarkData::Content(content) => builder.with_content(content),
            MarkData::Landmark(landmark) => builder.with_landmark(landmark),
        };

        if let Some(extra) = row.extra {
            builder = builder.with_extra(extra);
        }

        let mark = builder.build();
        Ok(Mark { id: row.id, ..mark })
    }
}
