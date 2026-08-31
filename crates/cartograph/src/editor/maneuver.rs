use crate::{EditorError, EditorResult};
use chrono::NaiveDateTime;
use domain::{FeatureID, Json, MarkData, MarkID, Reading, RichText, TerrainID, TerrainKind};
use serde::{Deserialize, Serialize};
use store::{
    Store,
    query::{mark, terrain},
};

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

/// `Reading` has no `Default`; `Null` is the encoding for "no reading".
fn reading_null() -> Reading {
    Reading::Null
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
        data: Box<MarkData>,
    },
    UpdateMarkData {
        mark_id: MarkID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip)]
        old: Box<MarkData>,
        new: Box<MarkData>,
    },
    UpdateLandmarkTitle {
        mark_id: MarkID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip, default = "empty_rich_text")]
        old: RichText,
        new: RichText,
    },
    UpdateReading {
        mark_id: MarkID,
        feature_id: FeatureID,
        /// Local-only, for CommandHistory undo.
        #[serde(skip, default = "reading_null")]
        old: Reading,
        new: Reading,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManeuverTarget {
    Mark(MarkID),
    Title(MarkID),
    Reading(MarkID, FeatureID),
    Terrain(TerrainID),
}

impl Maneuver {
    pub fn create_mark(mark_id: MarkID, data: MarkData) -> Self {
        Maneuver::CreateMark {
            mark_id,
            data: Box::new(data),
        }
    }

    pub fn update_landmark_title(mark_id: MarkID, old: RichText, new: RichText) -> Self {
        Maneuver::UpdateLandmarkTitle { mark_id, old, new }
    }

    pub fn update_reading(
        mark_id: MarkID,
        feature_id: FeatureID,
        old: Reading,
        new: Reading,
    ) -> Self {
        Maneuver::UpdateReading {
            mark_id,
            feature_id,
            old,
            new,
        }
    }

    /// A full-snapshot text replacement (the only [`TextPayload`] kind today).
    pub fn update_rich_text(mark_id: MarkID, old: RichText, new: RichText) -> Self {
        Maneuver::UpdateRichText {
            mark_id,
            old,
            new: TextPayload::Snapshot { rich_text: new },
        }
    }

    pub fn target(&self) -> ManeuverTarget {
        use Maneuver::*;
        match self {
            CreateMark { mark_id, .. }
            | UpdateMarkData { mark_id, .. }
            | UpdateRichText { mark_id, .. }
            | SoftDeleteMark { mark_id, .. }
            | RestoreMark { mark_id, .. } => ManeuverTarget::Mark(*mark_id),

            UpdateLandmarkTitle { mark_id, .. } => ManeuverTarget::Title(*mark_id),
            UpdateReading {
                mark_id,
                feature_id,
                ..
            } => ManeuverTarget::Reading(*mark_id, feature_id.clone()),

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
            UpdateLandmarkTitle { mark_id, old, new } => UpdateLandmarkTitle {
                mark_id: *mark_id,
                old: new.clone(),
                new: old.clone(),
            },
            UpdateReading {
                mark_id,
                feature_id,
                old,
                new,
            } => UpdateReading {
                mark_id: *mark_id,
                feature_id: feature_id.clone(),
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
            Maneuver::UpdateLandmarkTitle { mark_id, new, .. } => {
                apply_landmark_title_update(db, *mark_id, new).await
            }
            Maneuver::UpdateReading {
                mark_id,
                feature_id,
                new,
                ..
            } => apply_reading_update(db, *mark_id, feature_id, new).await,
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

/// Set a landmark's title. Mechanical writer — no validation beyond "the mark
/// exists and is a landmark". `apply` also runs on the undo/redo path, so an
/// op that was legal when recorded must stay applicable; anything stricter
/// belongs in the mutation layer, before the op is constructed.
async fn apply_landmark_title_update(
    db: &Store,
    mark_id: MarkID,
    new_title: &RichText,
) -> EditorResult<()> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            landmark.title = new_title.clone();
        }
        _ => return Err(EditorError::NotLandmark(mark_id.inner())),
    }

    mark::update_data(db, mark_id, &mark_data).await?;
    Ok(())
}

/// Set or remove a reading on a landmark. `Reading::Null` removes the entry.
/// Mechanical writer — no schema validation: if it revalidated, undoing to a
/// previously-valid reading could fail after an intervening Chart edit
/// narrowed the feature's constraints, leaving the group half-reverted.
/// Validation runs once in the mutation layer, before the op exists.
async fn apply_reading_update(
    db: &Store,
    mark_id: MarkID,
    feature_id: &FeatureID,
    new: &Reading,
) -> EditorResult<()> {
    let m = mark::get(db, mark_id)
        .await?
        .ok_or(EditorError::NotFound(mark_id.inner()))?;

    let mut mark_data = m.data;
    match &mut mark_data {
        MarkData::Landmark(landmark) => {
            if matches!(new, Reading::Null) {
                landmark.readings.shift_remove(feature_id);
            } else {
                landmark.readings.insert(feature_id.clone(), new.clone());
            }
        }
        _ => return Err(EditorError::NotLandmark(mark_id.inner())),
    }

    mark::update_data(db, mark_id, &mark_data).await?;
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
