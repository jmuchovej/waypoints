use crate::{Database, MarkError, MarkResult};
use domain::{ChartID, Content, Mark, MarkData, MarkID, TerrainKind, fractional_index};
use uuid::Uuid;

pub struct MarkHandle<'a> {
    pub(crate) db: &'a Database,
}

impl MarkHandle<'_> {
    async fn db(&self) -> MarkResult<store::Store> {
        self.db
            .lock()
            .await
            .clone()
            .ok_or(MarkError::NoActiveTerritory)
    }

    pub async fn list(
        &self,
        prefix: Option<&str>,
        chart_ids: Option<&[ChartID]>,
    ) -> MarkResult<Vec<Mark>> {
        let db = self.db().await?;
        let marks = store::query::mark::list(&db, prefix, chart_ids).await?;
        tracing::info!(count = marks.len(), ?prefix, "mark_list");
        Ok(marks)
    }

    pub async fn create(
        &self,
        id: Uuid,
        parent_id: Uuid,
        content: Content,
        created_at: String,
        after_id: Option<Uuid>,
    ) -> MarkResult<()> {
        tracing::info!(%id, %parent_id, "mark_create");

        let db = self.db().await?;
        let parsed_ts = parse_iso_timestamp(&created_at)?;
        let mark_id = MarkID::from(id);
        let parent_mark_id = MarkID::from(parent_id);

        let mark_data = MarkData::Content(content);
        store::query::mark::insert(&db, mark_id, &mark_data, parsed_ts).await?;

        // Compute order for the terrain edge
        let existing_edges = store::query::terrain::get_child_edges(&db, parent_mark_id).await?;

        let new_order = if let Some(after) = after_id {
            let after_idx = existing_edges
                .iter()
                .position(|e| e.dst_id.inner() == after);
            match after_idx {
                Some(idx) => {
                    let after_order = existing_edges[idx].order.as_deref();
                    let next_order = existing_edges.get(idx + 1).and_then(|e| e.order.as_deref());
                    fractional_index::key_between(after_order, next_order)
                }
                None => fractional_index::key_between(
                    existing_edges.last().and_then(|e| e.order.as_deref()),
                    None,
                ),
            }
        } else if let Some(first) = existing_edges.first() {
            fractional_index::key_between(None, first.order.as_deref())
        } else {
            fractional_index::key_between(None, None)
        };

        store::query::terrain::insert_child_edge(&db, parent_mark_id, mark_id, &new_order).await?;

        tracing::info!(%id, %parent_id, "mark_create: done");
        Ok(())
    }

    pub async fn update(&self, id: Uuid, content: Content) -> MarkResult<()> {
        tracing::info!(%id, "mark_update");
        let db = self.db().await?;
        let mark_data = MarkData::Content(content);
        store::query::mark::update_data(&db, MarkID::from(id), &mark_data).await?;
        tracing::info!(%id, "mark_update: done");
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> MarkResult<()> {
        tracing::info!(%id, "mark_delete");
        let db = self.db().await?;
        let mark_id = MarkID::from(id);
        store::query::mark::soft_delete(&db, mark_id).await?;
        store::query::terrain::delete_edges_by_dst(&db, mark_id, None).await?;
        tracing::info!(%id, "mark_delete: done");
        Ok(())
    }

    pub async fn move_mark(
        &self,
        parent_id: Uuid,
        child_id: Uuid,
        after_id: Option<Uuid>,
    ) -> MarkResult<()> {
        tracing::info!(%parent_id, %child_id, ?after_id, "mark_move");

        let db = self.db().await?;
        let parent_mark_id = MarkID::from(parent_id);
        let child_mark_id = MarkID::from(child_id);

        // Fetch current child edges in order
        let edges = store::query::terrain::get_child_edges(&db, parent_mark_id).await?;

        // Compute the new order key based on the target position
        let new_order = if let Some(after) = after_id {
            let after_idx = edges.iter().position(|e| e.dst_id.inner() == after);
            match after_idx {
                Some(idx) => {
                    let after_order = edges[idx].order.as_deref();
                    let next_order = edges[idx + 1..]
                        .iter()
                        .find(|e| e.dst_id.inner() != child_id)
                        .and_then(|e| e.order.as_deref());
                    fractional_index::key_between(after_order, next_order)
                }
                None => fractional_index::key_between(
                    edges.last().and_then(|e| e.order.as_deref()),
                    None,
                ),
            }
        } else {
            // Move to the beginning — before the first sibling that isn't us
            let first_order = edges
                .iter()
                .find(|e| e.dst_id.inner() != child_id)
                .and_then(|e| e.order.as_deref());
            fractional_index::key_between(None, first_order)
        };

        // Delete existing child edges pointing to this mark (handles reparenting)
        store::query::terrain::delete_edges_by_dst(&db, child_mark_id, Some(&TerrainKind::Child))
            .await?;

        store::query::terrain::insert_child_edge(&db, parent_mark_id, child_mark_id, &new_order)
            .await?;

        tracing::info!(%parent_id, %child_id, "mark_move: done");
        Ok(())
    }
}

fn parse_iso_timestamp(s: &str) -> MarkResult<chrono::NaiveDateTime> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.naive_utc())
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f"))
        .map_err(|e| MarkError::InvalidTimestamp(format!("{s}: {e}")))
}
