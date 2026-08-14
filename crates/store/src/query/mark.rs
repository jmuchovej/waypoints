use chrono::NaiveDateTime;
use domain::{ChartID, Mark, MarkData, MarkID};
use uuid::Uuid;

use bedrock::{Filter, Query, ScalarValue, Update};

use crate::error::{StoreError, StoreResult};
use crate::store::Store;

/// Truncate a `NaiveDateTime` to millisecond precision.
///
/// bedrock's turso engine stores timestamps at millisecond granularity and
/// rejects sub-millisecond values with `BedrockError::SubMillisecond`.
fn trunc_millis(dt: NaiveDateTime) -> NaiveDateTime {
    let millis = dt.and_utc().timestamp_millis();
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|d| d.naive_utc())
        .unwrap_or(dt)
}

/// Fetch a single non-deleted mark by ID.
pub async fn get(store: &Store, id: MarkID) -> StoreResult<Option<Mark>> {
    let mark = store.db().get::<Mark>(&id).await?;
    Ok(mark.filter(|m| m.deleted_at.is_none()))
}

/// List non-deleted marks, optionally filtered by type prefix and/or chart IDs.
///
/// When `prefix` is `None`, returns all non-deleted marks.
/// When `prefix` is `Some("landmark")`, returns marks whose `data.mark` tag is `"landmark"`.
/// When `chart_ids` is provided, further filters to landmarks matching those chart IDs.
pub async fn list(
    store: &Store,
    prefix: Option<&str>,
    chart_ids: Option<&[ChartID]>,
) -> StoreResult<Vec<Mark>> {
    let mut clauses = vec![Filter::missing("deleted_at")];

    if let Some(ids) = chart_ids {
        // Filter for landmark marks whose data.chart matches one of the given ChartIDs.
        // ChartID is `#[serde(transparent)]` over String, so the raw stored value is
        // the full string including the "builtin." prefix (e.g. "builtin.page").
        clauses.push(Filter::json_eq("data", "mark", "landmark"));
        clauses.push(Filter::any(
            ids.iter()
                .map(|id| Filter::json_eq("data", "chart", id.0.clone())),
        ));
    } else if let Some(p) = prefix {
        // MarkData is #[serde(tag = "mark")] — the outer discriminant is at $.mark.
        // "landmark" → filter mark == "landmark"; "content" → mark == "content".
        // Sub-type filtering (e.g. "landmark.page") is done in two steps.
        let mut parts = p.splitn(2, '.');
        let outer = parts.next().unwrap();
        clauses.push(Filter::json_eq("data", "mark", outer));
        if let Some(sub) = parts.next() {
            // Content's inner tag is `type`; Landmark's inner discriminant is `chart`.
            // Per mark_data.rs: Content uses #[serde(tag = "type")], Landmark has `chart`.
            // For a prefix like "content.paragraph" we filter $.type == "paragraph".
            // For "landmark.page" we filter $.chart == "builtin.page" — but callers
            // who want a specific landmark by chart should pass `chart_ids` instead.
            // Treat the sub as a `type` filter (covers content sub-types).
            clauses.push(Filter::json_eq("data", "type", sub));
        }
    }

    store
        .db()
        .query::<Mark>(&Query::new().filter(Filter::all(clauses)))
        .await
        .map_err(Into::into)
}

/// Batch-fetch non-deleted marks by IDs (arbitrary order).
pub async fn get_many(store: &Store, ids: &[Uuid]) -> StoreResult<Vec<Mark>> {
    // turso's `uuid` custom column coerces a bound TEXT uuid for `=` but NOT for
    // `IN (...)`, so an `is_in` on the id column silently matches nothing. Use an
    // OR of equalities (each `=` coerces) instead.
    let any_id = Filter::any(
        ids.iter()
            .map(|u| Filter::eq("id", ScalarValue::Text(u.to_string()))),
    );

    store
        .db()
        .query::<Mark>(&Query::new().filter(Filter::all([any_id, Filter::missing("deleted_at")])))
        .await
        .map_err(Into::into)
}

/// Insert a new mark, returning the created [`Mark`].
pub async fn insert(
    store: &Store,
    id: MarkID,
    data: &MarkData,
    created_at: NaiveDateTime,
) -> StoreResult<Mark> {
    // Truncate to milliseconds: bedrock rejects sub-millisecond datetimes.
    let ts = trunc_millis(created_at);
    let mark = Mark {
        id,
        data: data.clone(),
        extra: None,
        created_at: ts,
        updated_at: ts,
        deleted_at: None,
    };
    store.db().insert(mark).await.map_err(Into::into)
}

/// Update the data column. Errors with NotFound if missing/deleted.
pub async fn update_data(store: &Store, id: MarkID, data: &MarkData) -> StoreResult<()> {
    let mark = store
        .db()
        .get::<Mark>(&id)
        .await?
        .filter(|m| m.deleted_at.is_none())
        .ok_or(StoreError::MarkNotFound(id))?;

    let updated = Mark {
        data: data.clone(),
        ..mark
    };
    store.db().update(updated).await?;
    Ok(())
}

/// Soft-delete (set deleted_at). Errors with NotFound if missing or already deleted.
pub async fn soft_delete(store: &Store, id: MarkID) -> StoreResult<()> {
    let mark = store
        .db()
        .get::<Mark>(&id)
        .await?
        .filter(|m| m.deleted_at.is_none())
        .ok_or(StoreError::MarkNotFound(id))?;

    let updated = Mark {
        deleted_at: Some(trunc_millis(chrono::Local::now().naive_utc())),
        ..mark
    };
    store.db().update(updated).await?;
    Ok(())
}

/// Restore a soft-deleted mark (clear deleted_at). Errors if not found.
pub async fn restore(store: &Store, id: MarkID) -> StoreResult<()> {
    let mark = store
        .db()
        .get::<Mark>(&id)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    let updated = Mark {
        deleted_at: None,
        ..mark
    };
    store.db().update(updated).await?;
    Ok(())
}

/// Hard-delete (actually deletes). Errors if the record is absent.
pub async fn hard_delete(store: &Store, id: MarkID) -> StoreResult<()> {
    store
        .db()
        .get::<Mark>(&id)
        .await?
        .ok_or(StoreError::MarkNotFound(id))?;

    store.db().delete::<Mark>(&id).await?;
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Content, RichText};
    use tempfile::tempdir;

    async fn open_store() -> (Store, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db"))
            .await
            .expect("Store::open");
        (store, dir)
    }

    fn paragraph(text: &str) -> MarkData {
        MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
    }

    fn landmark_page(title: &str) -> MarkData {
        use domain::Landmark;
        MarkData::Landmark(
            Landmark::builder()
                .with_chart(ChartID::builtin("page"))
                .with_title(title.to_string())
                .build(),
        )
    }

    fn new_id() -> MarkID {
        MarkID::new()
    }

    fn now() -> NaiveDateTime {
        chrono::Local::now().naive_utc()
    }

    /// Verify uuid-filter: `get_many` returns exactly the requested IDs and
    /// skips soft-deleted rows. This exercises the `Filter::is_in` with
    /// `ScalarValue::Text(uuid.to_string())` binding that is the one risky
    /// detail of the bedrock migration.
    #[tokio::test]
    async fn get_many_uuid_filter_correctness() {
        let (store, _dir) = open_store().await;

        let a = new_id();
        let b = new_id();
        let c = new_id();
        insert(&store, a, &paragraph("a"), now()).await.unwrap();
        insert(&store, b, &paragraph("b"), now()).await.unwrap();
        insert(&store, c, &paragraph("c"), now()).await.unwrap();

        // Soft-delete c — it must not appear in get_many results.
        soft_delete(&store, c).await.unwrap();

        let results = get_many(&store, &[a.inner(), b.inner(), c.inner()])
            .await
            .unwrap();
        assert_eq!(results.len(), 2, "soft-deleted mark must be excluded");
        let returned_ids: Vec<MarkID> = results.iter().map(|m| m.id).collect();
        assert!(returned_ids.contains(&a));
        assert!(returned_ids.contains(&b));
        assert!(!returned_ids.contains(&c));
    }

    /// Verify landmark list by chart: seeding a landmark for a builtin chart and
    /// querying `list(None, Some(&[chart_id]))` returns it. This exercises the
    /// raw ChartID string path (e.g. "builtin.page") used in `Filter::json_eq`.
    #[tokio::test]
    async fn list_by_chart_id_returns_landmark() {
        let (store, _dir) = open_store().await;

        let page_chart = ChartID::builtin("page");
        let note_chart = ChartID::builtin("note");

        let page_id = new_id();
        let note_id = new_id();
        insert(&store, page_id, &landmark_page("My Page"), now())
            .await
            .unwrap();
        insert(
            &store,
            note_id,
            &{
                use domain::Landmark;
                MarkData::Landmark(
                    Landmark::builder()
                        .with_chart(note_chart.clone())
                        .with_title("My Note".to_string())
                        .build(),
                )
            },
            now(),
        )
        .await
        .unwrap();

        // Query only for page landmarks.
        let results = list(&store, None, Some(std::slice::from_ref(&page_chart)))
            .await
            .unwrap();

        let ids: Vec<MarkID> = results.iter().map(|m| m.id).collect();
        assert!(
            ids.contains(&page_id),
            "page landmark must appear when filtering by page chart id"
        );
        assert!(
            !ids.contains(&note_id),
            "note landmark must not appear when filtering by page chart id"
        );
    }
}
