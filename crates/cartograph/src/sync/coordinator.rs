use domain::sync::{Clock, SystemClock};
use domain::{AccountID, DeviceID, HLC, MarkID, OpID, RealmID};
use store::{
    DatabaseConnection,
    query::{
        device, mark,
        sync_log::{self, SyncLogEntry, SyncOrigin},
        terrain,
    },
};

use outpost::sync::peers::wire::EnvelopeFace;

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

/// Maximum tolerated future-drift of an incoming envelope's HLC relative
/// to the local wall clock. Healthy NTP-disciplined devices skew by
/// milliseconds; this only catches genuinely broken clocks, whose writes
/// would otherwise dominate LWW realm-wide until real time caught up.
pub const MAX_HLC_DRIFT_MS: i64 = 15 * 60 * 1000;

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
    /// The envelope's HLC is implausibly far ahead of our wall clock —
    /// the sender's clock is broken. Applying it would poison LWW (and
    /// receive() would drag our own clock forward, spreading it).
    ExcessiveDrift,
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
            face: EnvelopeFace {
                group_id: OpID::new(),
                account_id: self.account_id,
                realm_id: self.realm_id,
                device_id: self.device_id,
                hlc,
            },
            checkpoints: ops,
            touched_marks,
        };
        // Serialize once at mint: these are the canonical wire bytes,
        // stored verbatim so upload (and serving) never re-serializes.
        let payload =
            serde_json::to_string(&group).map_err(|e| EditorError::Serialization(e.to_string()))?;
        sync_log::insert(db, log_entry(&group, payload, SyncOrigin::Local)?).await?;

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

    /// Apply a remote group through the resolver. Idempotent on
    /// `group_id`; `payload` is the wire envelope, byte-verbatim, stored
    /// so this device can serve (and carry) it later.
    pub async fn apply_remote(
        &mut self,
        db: &DatabaseConnection,
        group: Envelope,
        payload: &str,
    ) -> EditorResult<ApplyOutcome> {
        // Tri-state: applied rows are done; carrier rows (stored via
        // serving, never applied) still need their ops run.
        let existing = sync_log::get(db, group.face.group_id).await?;
        if existing.as_ref().is_some_and(|r| r.applied_at.is_some()) {
            return Ok(ApplyOutcome::Skipped(SkipReason::AlreadyApplied));
        }
        if group.face.realm_id != self.realm_id {
            return Ok(ApplyOutcome::Skipped(SkipReason::WrongRealm));
        }
        // Before receive(): adopting a poisoned HLC would drag our own
        // clock into the future and spread the drift to our next writes.
        if group.face.hlc.physical() > SystemClock.now_ms() + MAX_HLC_DRIFT_MS {
            return Ok(ApplyOutcome::Skipped(SkipReason::ExcessiveDrift));
        }

        self.last_hlc = HLC::receive(self.last_hlc, group.face.hlc);
        device::set_last_hlc(db, self.device_id, self.last_hlc).await?;

        // Persist before applying: a crash mid-apply leaves the group
        // recorded (it won't be re-fetched), which is the safe side —
        // re-applying half-applied groups would double-apply the ops
        // that did land.
        match existing {
            Some(_) => {
                // Carrier row: backfill the parsed ops and stamp applied.
                sync_log::mark_applied(
                    db,
                    group.face.group_id,
                    serde_json::to_value(&group.checkpoints)
                        .map_err(|e| EditorError::Serialization(e.to_string()))?,
                    serde_json::to_value(&group.touched_marks)
                        .map_err(|e| EditorError::Serialization(e.to_string()))?,
                )
                .await?;
            }
            None => {
                sync_log::insert(
                    db,
                    log_entry(&group, payload.to_string(), SyncOrigin::Remote)?,
                )
                .await?;
            }
        }

        let (mut applied, mut dominated, mut merged) = (0usize, 0usize, 0usize);
        for op in &group.checkpoints {
            match self
                .resolver
                .resolve(db, op, group.face.hlc, group.face.device_id)
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

    /// Apply carrier rows — envelopes accepted through serving (§4.1)
    /// and stored face-and-payload only. One batch of at most `limit`,
    /// arrival order; returns how many groups were applied. Rows whose
    /// payload no longer parses stay pending: they are envelopes this
    /// version can only carry, not apply.
    pub async fn apply_pending(
        &mut self,
        db: &DatabaseConnection,
        limit: u64,
    ) -> EditorResult<usize> {
        let rows = sync_log::pending_apply(db, self.realm_id, limit).await?;
        let mut applied = 0usize;
        for row in rows {
            let Some(payload) = row.payload else {
                // Unreachable in practice (carrier rows always carry a
                // payload); leave it pending rather than guess.
                continue;
            };
            let Ok(group) = serde_json::from_str::<Envelope>(&payload) else {
                tracing::warn!(
                    group_id = %row.group_id,
                    "carrying envelope this version cannot parse; leaving pending"
                );
                continue;
            };
            if matches!(
                self.apply_remote(db, group, &payload).await?,
                ApplyOutcome::Applied { .. }
            ) {
                applied += 1;
            }
        }
        Ok(applied)
    }

    /// Cursor for "give me everything since X" queries against the relay.
    pub async fn last_remote_hlc(&self, db: &DatabaseConnection) -> EditorResult<HLC> {
        Ok(sync_log::max_remote_hlc(db, self.realm_id).await?)
    }
}

fn log_entry(group: &Envelope, payload: String, origin: SyncOrigin) -> EditorResult<SyncLogEntry> {
    Ok(SyncLogEntry {
        group_id: group.face.group_id,
        device_id: group.face.device_id,
        account_id: group.face.account_id,
        realm_id: group.face.realm_id,
        hlc: group.face.hlc,
        ops: serde_json::to_value(&group.checkpoints)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
        touched_marks: serde_json::to_value(&group.touched_marks)
            .map_err(|e| EditorError::Serialization(e.to_string()))?,
        payload,
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

    /// Test shorthand: apply a group whose wire payload is its own
    /// serialization (what a relay would have carried).
    async fn apply(
        coord: &mut SyncCoordinator,
        db: &DatabaseConnection,
        group: Envelope,
    ) -> ApplyOutcome {
        let payload = serde_json::to_string(&group).unwrap();
        coord.apply_remote(db, group, &payload).await.unwrap()
    }

    fn remote_group(
        realm_id: RealmID,
        device_id: DeviceID,
        hlc: HLC,
        ops: Vec<Maneuver>,
    ) -> Envelope {
        let touched = touched_marks(&ops);
        Envelope {
            face: EnvelopeFace {
                group_id: OpID::new(),
                account_id: AccountID::new(),
                realm_id,
                device_id,
                hlc,
            },
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

        let row = store::query::sync_log::get(&db, group.face.group_id)
            .await
            .unwrap()
            .expect("sync_log row for local group");
        assert_eq!(row.origin, 0);
        assert_eq!(row.hlc, group.face.hlc.inner());

        assert_eq!(
            mark_q::get_row_clock(&db, id).await.unwrap(),
            Some((group.face.hlc, coord.device_id()))
        );

        let dev = device::get_local(&db).await.unwrap().unwrap();
        assert_eq!(dev.last_hlc, group.face.hlc.inner(), "last_hlc persisted");
    }

    #[tokio::test]
    async fn apply_remote_creates_missing_mark_with_its_clock() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let remote_device = DeviceID::new();
        let id = MarkID::new();
        let hlc = HLC::pack(1_750_000_000_000, 1);
        let group = remote_group(
            coord.realm_id(),
            remote_device,
            hlc,
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("from afar"),
            }],
        );

        let outcome = apply(&mut coord, &db, group).await;
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
            HLC::pack(1_750_000_000_000, 1),
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("once"),
            }],
        );

        apply(&mut coord, &db, group.clone()).await;
        let second = apply(&mut coord, &db, group.clone()).await;
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
            HLC::pack(1_750_000_000_000, 1),
            vec![],
        );
        let outcome = apply(&mut coord, &db, group).await;
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
        let outcome = apply(&mut coord, &db, group).await;
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
        let hlc = HLC::pack(1_750_000_000_000, 7);

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
            apply(&mut coord, &db, winner).await,
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
            apply(&mut coord, &db, loser).await,
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
    async fn far_future_hlc_is_quarantined() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let id = MarkID::new();
        let poisoned = HLC::pack(SystemClock.now_ms() + 2 * MAX_HLC_DRIFT_MS, 0);
        let group = remote_group(
            coord.realm_id(),
            DeviceID::new(),
            poisoned,
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("from the future"),
            }],
        );

        let outcome = apply(&mut coord, &db, group).await;
        assert_eq!(outcome, ApplyOutcome::Skipped(SkipReason::ExcessiveDrift));

        // Nothing persisted, nothing applied, clock not dragged: a later
        // legitimate envelope still applies with its own key.
        assert!(SyncLog::find().all(&db).await.unwrap().is_empty());
        assert!(mark_q::get(&db, id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn plausible_clock_skew_still_applies() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();

        let id = MarkID::new();
        let skewed = HLC::pack(SystemClock.now_ms() + MAX_HLC_DRIFT_MS / 2, 0);
        let group = remote_group(
            coord.realm_id(),
            DeviceID::new(),
            skewed,
            vec![Maneuver::CreateMark {
                mark_id: id,
                data: paragraph("slightly ahead"),
            }],
        );

        let outcome = apply(&mut coord, &db, group).await;
        assert_eq!(
            outcome,
            ApplyOutcome::Applied {
                applied: 1,
                dominated: 0,
                merged: 0
            }
        );
        assert!(mark_q::get(&db, id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn last_remote_hlc_tracks_received_groups() {
        let db = temp_db().await;
        let mut coord = SyncCoordinator::load(&db).await.unwrap();
        assert_eq!(coord.last_remote_hlc(&db).await.unwrap(), HLC::ZERO);

        let hlc = HLC::pack(1_750_000_000_000, 3);
        let group = remote_group(coord.realm_id(), DeviceID::new(), hlc, vec![]);
        apply(&mut coord, &db, group).await;
        assert_eq!(coord.last_remote_hlc(&db).await.unwrap(), hlc);
    }
}
