use domain::{AccountID, DeviceID, HLC, MarkID, OpID, RealmID};
use store::{
    DatabaseConnection,
    query::{
        device, mark,
        sync_log::{self, SyncLogEntry, SyncOrigin},
        terrain,
    },
};

use super::{ConflictResolver, LastWriteResolver, Resolution, envelope::Envelope};
use crate::editor::{EditorError, EditorResult, Maneuver, ManeuverTarget};

/// The entry point for sync: wraps local command groups as [`Envelope`]s
/// and applies remote groups through a [`ConflictResolver`]. Owned by
/// `EditorService` — every mutation funnels through there, which makes it
/// the natural place to keep marks, edges, and (later) charts and assets
/// syncable.
///
/// Static dispatch on purpose: the resolver is fixed at construction, so
/// there's no `dyn` ceremony and no per-op vtable hop.
pub struct SyncCoordinator<R: ConflictResolver = LastWriteResolver> {
    device_id: DeviceID,
    account_id: AccountID,
    realm_id: RealmID,
    last_hlc: HLC,
    resolver: R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Skipped(SkipReason),
    Applied {
        applied: usize,
        dominated: usize,
        /// Always 0 in v1 — reserved for CRDT resolvers, whose merges
        /// have no loser.
        merged: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The group's idempotency key is already in `sync_log`.
    AlreadyApplied,
    /// Defense-in-depth against a misconfigured relay.
    WrongRealm,
}

impl SyncCoordinator {
    /// [`SyncCoordinator::load_with`] using the v1 default resolver.
    pub async fn load(db: &DatabaseConnection) -> EditorResult<Self> {
        Self::load_with(db, LastWriteResolver).await
    }
}

impl<R: ConflictResolver> SyncCoordinator<R> {
    /// Load the coordinator's state from the device table, minting the
    /// local install's identity on first use.
    ///
    /// ORDERING: authentication must complete before this runs —
    /// `account_id` derives from the better-auth session. v1 mints
    /// placeholder account/realm IDs instead (default realm per account);
    /// wiring the real session in is deferred until the auth integration
    /// lands. The keychain copy of `device_id` (§2.1 resilience) is also
    /// deferred — SQLite is the only store for now.
    pub async fn load_with(db: &DatabaseConnection, resolver: R) -> EditorResult<Self> {
        if let Some(row) = device::get_local(db).await? {
            return Ok(Self {
                device_id: row.device_id.into(),
                account_id: row.account_id.into(),
                realm_id: row.realm_id.into(),
                last_hlc: HLC(row.last_hlc),
                resolver,
            });
        }

        let device_id = DeviceID::new();
        let account_id = AccountID::new();
        let realm_id = RealmID::new();
        let friendly_name = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown-host".to_string());
        device::insert(db, device_id, account_id, realm_id, &friendly_name).await?;

        Ok(Self {
            device_id,
            account_id,
            realm_id,
            last_hlc: HLC::ZERO,
            resolver,
        })
    }

    pub fn device_id(&self) -> DeviceID {
        self.device_id
    }

    pub fn realm_id(&self) -> RealmID {
        self.realm_id
    }

    /// Wrap a command group as a [`Envelope`], persist it, and return
    /// it for transmission. Called after mutations have already succeeded
    /// against the database.
    pub async fn log_local(
        &mut self,
        db: &DatabaseConnection,
        ops: Vec<Maneuver>,
        touched_marks: Vec<MarkID>,
    ) -> EditorResult<Envelope> {
        let hlc = HLC::tick(self.last_hlc);
        self.last_hlc = hlc;
        device::set_last_hlc(db, self.device_id, hlc).await?;

        let group = Envelope {
            group_id: OpID::new(),
            account_id: self.account_id,
            realm_id: self.realm_id,
            device_id: self.device_id,
            hlc,
            checkpoints: ops,
            touched_marks,
        };
        sync_log::insert(db, log_entry(&group, SyncOrigin::Local)?).await?;

        for op in &group.checkpoints {
            match op.target() {
                ManeuverTarget::Mark(id) => {
                    mark::set_row_clock(db, id, hlc, self.device_id).await?
                }
                ManeuverTarget::Terrain(id) => {
                    terrain::set_row_clock(db, id, hlc, self.device_id).await?
                }
            }
        }

        Ok(group)
    }

    /// Apply a remote group through the resolver. Idempotent on `group_id`.
    pub async fn apply_remote(
        &mut self,
        db: &DatabaseConnection,
        group: Envelope,
    ) -> EditorResult<ApplyOutcome> {
        if sync_log::contains(db, group.group_id).await? {
            return Ok(ApplyOutcome::Skipped(SkipReason::AlreadyApplied));
        }
        if group.realm_id != self.realm_id {
            return Ok(ApplyOutcome::Skipped(SkipReason::WrongRealm));
        }

        self.last_hlc = HLC::receive(self.last_hlc, group.hlc);
        device::set_last_hlc(db, self.device_id, self.last_hlc).await?;

        // Persist before applying: a crash mid-apply leaves the group
        // recorded (it won't be re-fetched), which is the safe side —
        // re-applying half-applied groups would double-apply the ops
        // that did land.
        sync_log::insert(db, log_entry(&group, SyncOrigin::Remote)?).await?;

        let (mut applied, mut dominated, mut merged) = (0usize, 0usize, 0usize);
        for op in &group.checkpoints {
            match self
                .resolver
                .resolve(db, op, group.hlc, group.device_id)
                .await?
            {
                Resolution::Applied => applied += 1,
                Resolution::Dominated => dominated += 1,
                Resolution::Merged => merged += 1,
            }
        }

        Ok(ApplyOutcome::Applied {
            applied,
            dominated,
            merged,
        })
    }

    /// Cursor for "give me everything since X" queries against the relay.
    pub async fn last_remote_hlc(&self, db: &DatabaseConnection) -> EditorResult<HLC> {
        Ok(sync_log::max_remote_hlc(db, self.realm_id).await?)
    }
}

fn log_entry(group: &Envelope, origin: SyncOrigin) -> EditorResult<SyncLogEntry> {
    Ok(SyncLogEntry {
        group_id: group.group_id,
        device_id: group.device_id,
        account_id: group.account_id,
        realm_id: group.realm_id,
        hlc: group.hlc,
        ops: serde_json::to_value(&group.checkpoints)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
        touched_marks: serde_json::to_value(&group.touched_marks)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
        origin,
    })
}

/// Collect the mark IDs a group of ops touches, in first-seen order.
/// v1 populates this but never reads it (cheap insurance for future
/// dependency checks).
pub fn touched_marks(ops: &[Maneuver]) -> Vec<MarkID> {
    fn push(seen: &mut Vec<MarkID>, id: MarkID) {
        if !seen.contains(&id) {
            seen.push(id);
        }
    }

    let mut seen: Vec<MarkID> = Vec::new();
    for op in ops {
        use Maneuver::*;
        match op {
            CreateMark { mark_id, .. }
            | UpdateMarkData { mark_id, .. }
            | UpdateRichText { mark_id, .. }
            | SoftDeleteMark { mark_id, .. }
            | RestoreMark { mark_id } => push(&mut seen, *mark_id),

            CreateTerrain { src, dst, .. } | DeleteTerrain { src, dst, .. } => {
                push(&mut seen, *src);
                push(&mut seen, *dst);
            }
            // `old_src` is undo-only (absent on the wire), but at
            // origination — where touched_marks is computed — it's valid.
            UpdateTerrain {
                old_src, new_src, ..
            } => {
                push(&mut seen, *old_src);
                push(&mut seen, *new_src);
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use domain::mark::{Content, MarkData, RichText};
    use sea_orm::EntityTrait;
    use store::entities::SyncLog;
    use store::query::mark as mark_q;
    use store::test_utils::temp_db;
    use uuid::Uuid;

    use super::*;

    fn paragraph(text: &str) -> MarkData {
        MarkData::Content(Content::paragraph(RichText::from(text.to_string())))
    }

    fn remote_group(
        realm_id: RealmID,
        device_id: DeviceID,
        hlc: HLC,
        ops: Vec<Maneuver>,
    ) -> Envelope {
        let touched = touched_marks(&ops);
        Envelope {
            group_id: OpID::new(),
            account_id: AccountID::new(),
            realm_id,
            device_id,
            hlc,
            checkpoints: ops,
            touched_marks: touched,
        }
    }

    #[tokio::test]
    async fn load_mints_once_and_reloads() {
        let db = temp_db().await;
        let a = SyncCoordinator::load(&db).await.unwrap();
        let b = SyncCoordinator::load(&db).await.unwrap();
        assert_eq!(a.device_id(), b.device_id());
        assert_eq!(a.realm_id(), b.realm_id());
    }

    #[tokio::test]
    async fn log_local_persists_group_clock_and_row_clocks() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let id = MarkID::new();
        mark_q::insert(&db, id, &paragraph("hi"), chrono::Local::now().naive_utc())
            .await
            .unwrap();
        let ops = vec![Maneuver::CreateMark {
            mark_id: id,
            data: paragraph("hi"),
        }];
        let marks = touched_marks(&ops);
        let group = coord.log_local(&db, ops, marks).await.unwrap();

        let row = SyncLog::find_by_id(group.group_id.inner())
            .one(&db)
            .await
            .unwrap()
            .expect("sync_log row for local group");
        assert_eq!(row.origin, 0);
        assert_eq!(row.hlc, group.hlc.inner());

        assert_eq!(
            mark_q::get_row_clock(&db, id).await.unwrap(),
            Some((group.hlc, coord.device_id()))
        );

        let dev = device::get_local(&db).await.unwrap().unwrap();
        assert_eq!(dev.last_hlc, group.hlc.inner(), "last_hlc persisted");
    }

    #[tokio::test]
    async fn apply_remote_creates_missing_mark_with_its_clock() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let remote_device = DeviceID::new();
        let id = MarkID::new();
        let hlc = HLC::pack(1_800_000_000_000, 1);
        let group = remote_group(
            coord.realm_id(),
            remote_device,
            hlc,
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("from afar"),
            }],
        );

        let outcome = coord.apply_remote(&db, group).await.unwrap();
        assert_eq!(
            outcome,
            ApplyOutcome::Applied {
                applied: 1,
                dominated: 0,
                merged: 0
            }
        );
        assert!(mark_q::get(&db, id).await.unwrap().is_some());
        assert_eq!(
            mark_q::get_row_clock(&db, id).await.unwrap(),
            Some((hlc, remote_device))
        );
    }

    #[tokio::test]
    async fn apply_remote_is_idempotent_on_group_id() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let id = MarkID::new();
        let group = remote_group(
            coord.realm_id(),
            DeviceID::new(),
            HLC::pack(1_800_000_000_000, 1),
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("once"),
            }],
        );

        coord.apply_remote(&db, group.clone()).await.unwrap();
        let second = coord.apply_remote(&db, group.clone()).await.unwrap();
        assert_eq!(second, ApplyOutcome::Skipped(SkipReason::AlreadyApplied));

        let rows = SyncLog::find().all(&db).await.unwrap();
        assert_eq!(rows.len(), 1, "one sync_log row, no duplicate side effects");
    }

    #[tokio::test]
    async fn apply_remote_guards_realm() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let group = remote_group(
            RealmID::new(), // not ours
            DeviceID::new(),
            HLC::pack(1_800_000_000_000, 1),
            vec![],
        );
        let outcome = coord.apply_remote(&db, group).await.unwrap();
        assert_eq!(outcome, ApplyOutcome::Skipped(SkipReason::WrongRealm));
        assert!(SyncLog::find().all(&db).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn lower_hlc_is_dominated_by_stored_clock() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        // Local write stamps the row with a fresh (large) HLC.
        let id = MarkID::new();
        mark_q::insert(
            &db,
            id,
            &paragraph("local"),
            chrono::Local::now().naive_utc(),
        )
        .await
        .unwrap();
        let ops = vec![Maneuver::CreateMark {
            mark_id: id,
            data: paragraph("local"),
        }];
        let marks = touched_marks(&ops);
        coord.log_local(&db, ops, marks).await.unwrap();

        // A remote op from the distant past loses.
        let group = remote_group(
            coord.realm_id(),
            DeviceID::new(),
            HLC::pack(1, 0),
            vec![Maneuver::UpdateMarkData {
                mark_id: id,
                old: paragraph("local"),
                new: paragraph("stale remote"),
            }],
        );
        let outcome = coord.apply_remote(&db, group).await.unwrap();
        assert_eq!(
            outcome,
            ApplyOutcome::Applied {
                applied: 0,
                dominated: 1,
                merged: 0
            }
        );

        let m = mark_q::get(&db, id).await.unwrap().unwrap();
        assert_eq!(m.data, paragraph("local"), "stale remote must not win");
    }

    #[tokio::test]
    async fn identical_hlc_tiebreaks_on_device_id() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let low = DeviceID::from(Uuid::from_u128(1));
        let high = DeviceID::from(Uuid::from_u128(u128::MAX));
        let hlc = HLC::pack(1_800_000_000_000, 7);

        let id = MarkID::new();
        mark_q::insert(
            &db,
            id,
            &paragraph("seed"),
            chrono::Local::now().naive_utc(),
        )
        .await
        .unwrap();
        mark_q::set_row_clock(&db, id, hlc, low).await.unwrap();

        // Same HLC, higher device id: wins deterministically.
        let winner = remote_group(
            coord.realm_id(),
            high,
            hlc,
            vec![Maneuver::UpdateMarkData {
                mark_id: id,
                old: paragraph("seed"),
                new: paragraph("high device"),
            }],
        );
        assert_eq!(
            coord.apply_remote(&db, winner).await.unwrap(),
            ApplyOutcome::Applied {
                applied: 1,
                dominated: 0,
                merged: 0
            }
        );
        assert_eq!(
            mark_q::get_row_clock(&db, id).await.unwrap(),
            Some((hlc, high))
        );

        // Same HLC, lower device id than the stored one: loses.
        let loser = remote_group(
            coord.realm_id(),
            low,
            hlc,
            vec![Maneuver::UpdateMarkData {
                mark_id: id,
                old: paragraph("high device"),
                new: paragraph("low device"),
            }],
        );
        assert_eq!(
            coord.apply_remote(&db, loser).await.unwrap(),
            ApplyOutcome::Applied {
                applied: 0,
                dominated: 1,
                merged: 0
            }
        );
        let m = mark_q::get(&db, id).await.unwrap().unwrap();
        assert_eq!(m.data, paragraph("high device"));
    }

    #[tokio::test]
    async fn last_remote_hlc_tracks_received_groups() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();
        assert_eq!(coord.last_remote_hlc(&db).await.unwrap(), HLC::ZERO);

        let hlc = HLC::pack(1_800_000_000_000, 3);
        let group = remote_group(coord.realm_id(), DeviceID::new(), hlc, vec![]);
        coord.apply_remote(&db, group).await.unwrap();
        assert_eq!(coord.last_remote_hlc(&db).await.unwrap(), hlc);
    }
}
