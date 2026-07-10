use chrono::NaiveDateTime;
use domain::mark::{MarkData, MarkID, RichText};
use domain::{Json, TerrainID, TerrainKind};
use serde::{Deserialize, Serialize};
use store::Store;
use store::query::{mark, terrain};

use super::error::{EditorError, EditorResult};

/// Wire-stable wrapper for rich-text content in ops: the tagged enum lets a
/// new payload kind (e.g. char-level deltas) join without breaking the wire
/// format. `Snapshot` — a full replacement — is the only variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TextPayload {
    Snapshot { rich_text: RichText },
}

fn empty_rich_text() -> RichText {
    RichText::new(Vec::new())
}

/// An atomic, invertible operation on a single mark or terrain edge.
///
/// This is a wire type: ops travel between devices inside `Envelope`s.
/// Fields marked `#[serde(skip)]` exist only for local `CommandHistory`
/// undo and are never transmitted — a receiver deserializes them to
/// defaults and never reads them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Maneuver {
    // ── Mark layer (LWW target = MarkID) ─────────────────────────────
    CreateMark {
        mark_id: MarkID,
        data: MarkData,
    },
    UpdateMarkData {
        mark_id: MarkID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip)]
        old: MarkData,
        new: MarkData,
    },
    UpdateRichText {
        mark_id: MarkID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip, default = "empty_rich_text")]
        old: RichText,
        new: TextPayload,
    },
    SoftDeleteMark {
        mark_id: MarkID,
        deleted_at: NaiveDateTime,
    },
    RestoreMark {
        mark_id: MarkID,
    },

    // ── Edge layer (LWW target = TerrainID) ──────────────────────────
    CreateTerrain {
        edge_id: TerrainID,
        src: MarkID,
        dst: MarkID,
        kind: TerrainKind,
        order: Option<String>,
        extra: Option<Json>,
    },
    UpdateTerrain {
        edge_id: TerrainID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip, default = "MarkID::new")]
        old_src: MarkID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip)]
        old_order: Option<String>,
        new_src: MarkID,
        new_order: Option<String>,
    },
    DeleteTerrain {
        edge_id: TerrainID,
        // Full snapshot for invertibility.
        src: MarkID,
        dst: MarkID,
        kind: TerrainKind,
        order: Option<String>,
        extra: Option<Json>,
    },
}

/// The entity an op addresses — the row whose state (and clock) conflict
/// resolution consults. This mapping is part of the wire protocol's
/// semantics, not resolver policy: every device must derive the same
/// target from the same op, or their resolutions diverge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManeuverTarget {
    Mark(MarkID),
    Terrain(TerrainID),
}

impl Maneuver {
    pub fn target(&self) -> ManeuverTarget {
        use Maneuver::*;
        match self {
            CreateMark { mark_id, .. }
            | UpdateMarkData { mark_id, .. }
            | UpdateRichText { mark_id, .. }
            | SoftDeleteMark { mark_id, .. }
            | RestoreMark { mark_id, .. } => ManeuverTarget::Mark(*mark_id),

            CreateTerrain { edge_id, .. }
            | UpdateTerrain { edge_id, .. }
            | DeleteTerrain { edge_id, .. } => ManeuverTarget::Terrain(*edge_id),
        }
    }

    /// Return the inverse operation that undoes this one.
    pub fn invert(&self) -> Maneuver {
        use Maneuver::*;
        match self {
            CreateMark { mark_id, .. } => SoftDeleteMark {
                mark_id: *mark_id,
                deleted_at: chrono::Local::now().naive_utc(),
            },
            UpdateMarkData { mark_id, old, new } => UpdateMarkData {
                mark_id: *mark_id,
                old: new.clone(),
                new: old.clone(),
            },
            UpdateRichText { mark_id, old, new } => {
                let TextPayload::Snapshot { rich_text } = new;
                UpdateRichText {
                    mark_id: *mark_id,
                    old: rich_text.clone(),
                    new: TextPayload::Snapshot {
                        rich_text: old.clone(),
                    },
                }
            }
            SoftDeleteMark { mark_id, .. } => RestoreMark { mark_id: *mark_id },
            RestoreMark { mark_id } => SoftDeleteMark {
                mark_id: *mark_id,
                deleted_at: chrono::Local::now().naive_utc(),
            },
            CreateTerrain {
                edge_id,
                src,
                dst,
                kind,
                order,
                extra,
            } => DeleteTerrain {
                edge_id: *edge_id,
                src: *src,
                dst: *dst,
                kind: kind.clone(),
                order: order.clone(),
                extra: extra.clone(),
            },
            UpdateTerrain {
                edge_id,
                old_src,
                old_order,
                new_src,
                new_order,
            } => UpdateTerrain {
                edge_id: *edge_id,
                old_src: *new_src,
                old_order: new_order.clone(),
                new_src: *old_src,
                new_order: old_order.clone(),
            },
            DeleteTerrain {
                edge_id,
                src,
                dst,
                kind,
                order,
                extra,
            } => CreateTerrain {
                edge_id: *edge_id,
                src: *src,
                dst: *dst,
                kind: kind.clone(),
                order: order.clone(),
                extra: extra.clone(),
            },
        }
    }

    /// Execute this operation against the database.
    pub async fn apply(&self, db: &Store) -> EditorResult<()> {
        match self {
            Maneuver::CreateMark { mark_id, data } => apply_create_mark(db, *mark_id, data).await,
            Maneuver::UpdateMarkData { mark_id, new, .. } => {
                apply_mark_data_update(db, *mark_id, new).await
            }
            Maneuver::UpdateRichText { mark_id, new, .. } => {
                let TextPayload::Snapshot { rich_text } = new;
                apply_rich_text_update(db, *mark_id, rich_text).await
            }
            Maneuver::SoftDeleteMark { mark_id, .. } => {
                mark::soft_delete(db, *mark_id).await?;
                Ok(())
            }
            Maneuver::RestoreMark { mark_id } => {
                mark::restore(db, *mark_id).await?;
                Ok(())
            }
            Maneuver::CreateTerrain {
                edge_id,
                src,
                dst,
                kind,
                order,
                extra,
            } => {
                // Upsert: redoing a create (or a remote create winning LWW
                // over a tombstone) revives the existing row.
                terrain::upsert_edge(
                    db,
                    *edge_id,
                    *src,
                    *dst,
                    kind,
                    order.as_deref(),
                    extra.clone(),
                )
                .await?;
                Ok(())
            }
            Maneuver::UpdateTerrain {
                edge_id,
                new_src,
                new_order,
                ..
            } => {
                terrain::update_edge_by_id(db, *edge_id, *new_src, new_order.as_deref()).await?;
                Ok(())
            }
            Maneuver::DeleteTerrain { edge_id, .. } => {
                terrain::delete_edge_by_id(db, *edge_id).await?;
                Ok(())
            }
        }
    }
}

/// Create a mark, or restore it if it already exists soft-deleted
/// (e.g. redoing a create after an undo).
async fn apply_create_mark(db: &Store, mark_id: MarkID, data: &MarkData) -> EditorResult<()> {
    match mark::restore(db, mark_id).await {
        Ok(()) => {
            // Restored a soft-deleted mark — update its data to match
            mark::update_data(db, mark_id, data).await?;
        }
        Err(_) => {
            // Mark doesn't exist at all — fresh insert
            let now = chrono::Local::now().naive_utc();
            mark::insert(db, mark_id, data, now).await?;
        }
    }
    Ok(())
}

/// Update the full MarkData of a mark.
async fn apply_mark_data_update(
    db: &Store,
    mark_id: MarkID,
    new_data: &MarkData,
) -> EditorResult<()> {
    // Verify the mark exists
    mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    mark::update_data(db, mark_id, new_data).await?;
    Ok(())
}

/// Update the RichText content of a mark (for Content variants that have RichText).
async fn apply_rich_text_update(
    db: &Store,
    mark_id: MarkID,
    new_rt: &RichText,
) -> EditorResult<()> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Content(content) => {
            if !content.set_rich_text(new_rt.clone()) {
                return Err(EditorError::InvalidContent(
                    "content variant has no rich_text field".into(),
                ));
            }
        }
        _ => {
            return Err(EditorError::InvalidContent(
                "cannot update RichText on a Landmark".into(),
            ));
        }
    }

    mark::update_data(db, mark_id, &mark_data).await?;
    Ok(())
}
