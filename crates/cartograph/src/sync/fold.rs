//! The territory-wide fold: the one consumer that turns `waypoints.page.v1`
//! Notices back into the Mark/Terrain graph.
//!
//! Order independence comes from sorting, not from delivery: every op is
//! keyed by its [`ArbitrationKey`] (content-derived, identical on every
//! device) and the fold applies ops in `(key, batch index)` order through
//! the same [`Maneuver::apply`] path local edits use. `topo` — the local
//! arrival counter — only ever selects *which* Notices are new; it never
//! orders anything.
//!
//! Per-target last-write-wins falls out of that order: ops applying later
//! overwrite earlier ones per [`ManeuverTarget`], so each target ends at
//! the op with the greatest key — the winner [`winners`] reports and
//! persistence records.

use crate::sync::arbitration::ArbitrationKey;
use crate::{EditorError, Maneuver, ManeuverTarget};
use outpost::protocol::{EnvelopeHeader, NoticeID};
use outpost::vessel::Vessel;
use std::collections::HashMap;
use store::{Store, query::sync as sync_store};

/// The payload type tag for editor-operation Notices.
pub const PAGE_V1: &str = "waypoints.page.v1";

#[derive(Debug, thiserror::Error)]
pub enum FoldError {
    /// A payload that does not decode as a `waypoints.page.v1` batch.
    #[error("page.v1 codec: {0}")]
    Codec(#[from] serde_json::Error),
    /// An op that failed to apply to the graph.
    #[error("fold apply: {0}")]
    Apply(#[from] EditorError),
    /// Sync-owned persistence (winners, cursor) failed.
    #[error("fold store: {0}")]
    Store(#[from] store::StoreError),
    /// Reading Notices out of the Territory hold failed.
    #[error("territory hold: {0}")]
    Hold(#[from] outpost::hold::HoldError),
}

/// Encode a batch of ops as a `waypoints.page.v1` payload (the plaintext a
/// Notice seals). JSON because `Maneuver` is an internally tagged enum —
/// undo-only `#[serde(skip)]` fields are never written.
pub fn encode_page_v1(ops: &[Maneuver]) -> Result<Vec<u8>, FoldError> {
    Ok(serde_json::to_vec(ops)?)
}

/// Decode a `waypoints.page.v1` payload. Undo-only fields come back as
/// defaults and are never read on the apply path.
pub fn decode_page_v1(bytes: &[u8]) -> Result<Vec<Maneuver>, FoldError> {
    Ok(serde_json::from_slice(bytes)?)
}

/// One op positioned in the canonical fold order: its Notice's arbitration
/// key, then its index within that Notice's batch (an author's sequence
/// within one Notice is meaningful and shared by every device).
#[derive(Debug, Clone)]
pub struct KeyedOp {
    pub key: ArbitrationKey,
    pub index: usize,
    pub op: Maneuver,
}

/// Decode a decrypted Notice payload into keyed ops, deriving one key from
/// the signed header and content address — no local-state inputs.
pub fn keyed_ops(
    header: &EnvelopeHeader,
    notice: NoticeID,
    plaintext: &[u8],
) -> Result<Vec<KeyedOp>, FoldError> {
    let key = ArbitrationKey::derive(header, notice);
    Ok(decode_page_v1(plaintext)?
        .into_iter()
        .enumerate()
        .map(|(index, op)| KeyedOp { key, index, op })
        .collect())
}

/// Sort into the canonical fold order. Total: distinct Notices never share
/// a key, and within a Notice the batch index disambiguates.
pub fn sort_canonical(ops: &mut [KeyedOp]) {
    ops.sort_by(|a, b| a.key.cmp(&b.key).then(a.index.cmp(&b.index)));
}

/// The greatest key writing each target — what materializes under LWW, and
/// what persistence records per target.
pub fn winners(ops: &[KeyedOp]) -> HashMap<ManeuverTarget, ArbitrationKey> {
    let mut out: HashMap<ManeuverTarget, ArbitrationKey> = HashMap::new();
    for op in ops {
        out.entry(op.op.target())
            .and_modify(|k| *k = (*k).max(op.key))
            .or_insert(op.key);
    }
    out
}

/// Fold a batch of ops into the graph: sort canonically, apply in order.
/// Pure over its inputs — the resulting graph depends only on the op set,
/// never on the order the caller collected it in. Returns the winning key
/// per target for the caller to persist.
pub async fn fold(
    db: &Store,
    mut ops: Vec<KeyedOp>,
) -> Result<HashMap<ManeuverTarget, ArbitrationKey>, FoldError> {
    sort_canonical(&mut ops);
    for keyed in &ops {
        keyed.op.apply(db).await?;
    }
    Ok(winners(&ops))
}

/// Canonical persistence key for a target — the string the winner table is
/// keyed by. Stable across devices: every component renders from content
/// (`MarkID`/`TerrainID` lowercase-uuid `Display`, `FeatureID` base36).
pub fn target_key(target: &ManeuverTarget) -> String {
    match target {
        ManeuverTarget::Mark(id) => format!("mark:{id}"),
        ManeuverTarget::Title(id) => format!("title:{id}"),
        ManeuverTarget::Reading(id, feature) => format!("reading:{id}:{feature}"),
        ManeuverTarget::Terrain(id) => format!("terrain:{id}"),
    }
}

fn to_stored(key: ArbitrationKey) -> sync_store::StoredKey {
    sync_store::StoredKey {
        hlc: key.hlc(),
        author: *key.author(),
        notice: key.notice(),
    }
}

fn from_stored(key: sync_store::StoredKey) -> ArbitrationKey {
    ArbitrationKey::new(key.hlc, key.author, key.notice)
}

/// Incremental fold: apply ops in canonical order, rejecting any op whose
/// key is lower than the persisted winning key for its target (a
/// late-arriving older Notice must not clobber newer state), and persist
/// the advanced winners. Folding incrementally in any batching produces
/// the same graph and the same persisted winners as one fold from empty.
pub async fn fold_incremental(db: &Store, mut ops: Vec<KeyedOp>) -> Result<(), FoldError> {
    sort_canonical(&mut ops);
    let mut current: HashMap<ManeuverTarget, ArbitrationKey> = HashMap::new();
    let mut advanced: HashMap<ManeuverTarget, ArbitrationKey> = HashMap::new();

    for keyed in ops {
        let target = keyed.op.target();
        let winner = match current.get(&target) {
            Some(key) => Some(*key),
            None => {
                let persisted = sync_store::winner(db, &target_key(&target))
                    .await?
                    .map(from_stored);
                if let Some(key) = persisted {
                    current.insert(target.clone(), key);
                }
                persisted
            }
        };
        // Strictly lower loses; an equal key is the same Notice re-delivered
        // and re-applies idempotently.
        if winner.is_some_and(|w| w > keyed.key) {
            continue;
        }
        keyed.op.apply(db).await?;
        current.insert(target.clone(), keyed.key);
        advanced.insert(target, keyed.key);
    }

    for (target, key) in advanced {
        sync_store::set_winner(db, &target_key(&target), to_stored(key)).await?;
    }
    Ok(())
}

/// One incremental fold pass over the hold's new Notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldReport {
    /// `waypoints.page.v1` Notices decoded and offered to the fold.
    pub folded_notices: usize,
    /// The topo cursor after this pass.
    pub cursor: i64,
}

/// Fold every Notice the hold has accepted since the last pass.
///
/// `topo` selects *which* Notices are new locally — the arbitration key
/// alone decides order, so two replicas assigning different `topo` to the
/// same Notices converge. Non-`page.v1` payloads are skipped;
/// an undecryptable payload is skipped without writing partial state (the
/// Notice stays in the hold — a deliberate re-fold after key acquisition
/// is the recovery path).
pub async fn fold_new_notices(db: &Store, vessel: &Vessel) -> Result<FoldReport, FoldError> {
    let after = sync_store::fold_cursor(db).await?;
    let rows = vessel.replica().hold().notices_after(after).await?;

    let mut ops = Vec::new();
    let mut cursor = after;
    let mut folded_notices = 0;
    for row in rows {
        cursor = cursor.max(row.topo);
        let Ok(envelope) = row.notice.parse_and_verify() else {
            continue;
        };
        if envelope.header.payload_type != PAGE_V1 {
            continue;
        }
        let Ok(plaintext) = vessel.decrypt(&row.notice) else {
            continue;
        };
        ops.extend(keyed_ops(&envelope.header, row.id, &plaintext)?);
        folded_notices += 1;
    }

    fold_incremental(db, ops).await?;
    sync_store::set_fold_cursor(db, cursor).await?;
    Ok(FoldReport {
        folded_notices,
        cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::maneuver::TextPayload;
    use domain::{
        ChartID, Content, Json, Landmark, MarkData, MarkID, Reading, RichText, TerrainID,
        TerrainKind,
    };
    use outpost::protocol::HLC;
    use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
    use store::query::{mark, terrain};

    fn key(hlc: i64, author: u8, notice: u8) -> ArbitrationKey {
        ArbitrationKey::new(HLC(hlc), [author; 32], NoticeID::from_bytes([notice; 32]))
    }

    fn keyed(key: ArbitrationKey, index: usize, op: Maneuver) -> KeyedOp {
        KeyedOp { key, index, op }
    }

    fn rich(text: &str) -> RichText {
        RichText::from(text.to_string())
    }

    fn paragraph(text: &str) -> MarkData {
        MarkData::Content(Content::paragraph(rich(text)))
    }

    fn landmark(title: &str) -> MarkData {
        MarkData::Landmark(
            Landmark::builder()
                .with_chart(ChartID::builtin("page"))
                .with_title(title.to_string())
                .build(),
        )
    }

    fn when() -> chrono::NaiveDateTime {
        chrono::DateTime::from_timestamp(1_700_000_000, 0)
            .unwrap()
            .naive_utc()
    }

    #[test]
    fn page_v1_round_trips_every_variant_dropping_undo_fields() {
        let mark_id = MarkID::new();
        let other = MarkID::new();
        let edge_id = TerrainID::new();
        let feature: domain::FeatureID = "abcd".parse().unwrap();

        // Every variant, with every undo-only field set to a non-default
        // value — the codec must not let any of them reach the wire.
        let ops = vec![
            Maneuver::CreateMark {
                mark_id,
                data: Box::new(landmark("A page")),
            },
            Maneuver::UpdateMarkData {
                mark_id,
                old: Box::new(paragraph("undo-only")),
                new: Box::new(paragraph("new data")),
            },
            Maneuver::UpdateLandmarkTitle {
                mark_id,
                old: rich("undo-only"),
                new: rich("new title"),
            },
            Maneuver::UpdateReading {
                mark_id,
                feature_id: feature.clone(),
                old: Reading::Text(rich("undo-only")),
                new: Reading::Text(rich("new reading")),
            },
            Maneuver::UpdateRichText {
                mark_id,
                old: rich("undo-only"),
                new: TextPayload::Snapshot {
                    rich_text: rich("new text"),
                },
            },
            Maneuver::SoftDeleteMark {
                mark_id,
                deleted_at: when(),
            },
            Maneuver::RestoreMark { mark_id },
            Maneuver::CreateTerrain {
                edge_id,
                src: mark_id,
                dst: other,
                kind: TerrainKind::Child,
                order: Some("a0".into()),
                extra: Some(Json::from(serde_json::json!({"k": 1}))),
            },
            Maneuver::UpdateTerrain {
                edge_id,
                old_src: other,
                old_order: Some("undo-only".into()),
                new_src: mark_id,
                new_order: Some("a1".into()),
            },
            Maneuver::DeleteTerrain {
                edge_id,
                src: mark_id,
                dst: other,
                kind: TerrainKind::Child,
                order: Some("a1".into()),
                extra: None,
            },
        ];

        let wire = encode_page_v1(&ops).unwrap();
        let decoded = decode_page_v1(&wire).unwrap();
        assert_eq!(decoded.len(), ops.len());

        // Wire-stable: re-encoding the decoded batch is byte-identical, so
        // no undo-only field ever reached (or round-tripped through) the
        // wire, and no wire field was lost.
        assert_eq!(encode_page_v1(&decoded).unwrap(), wire);

        // Targets survive intact — the resolver keying is wire semantics.
        for (a, b) in ops.iter().zip(&decoded) {
            assert_eq!(a.target(), b.target());
        }

        // Deterministic undo-only defaults came back as their defaults.
        match &decoded[2] {
            Maneuver::UpdateLandmarkTitle { old, .. } => assert_eq!(*old, rich("")),
            other => panic!("variant order changed: {other:?}"),
        }
        match &decoded[3] {
            Maneuver::UpdateReading { old, .. } => assert_eq!(*old, Reading::Null),
            other => panic!("variant order changed: {other:?}"),
        }
        match &decoded[8] {
            Maneuver::UpdateTerrain { old_order, .. } => assert_eq!(*old_order, None),
            other => panic!("variant order changed: {other:?}"),
        }
    }

    /// A concurrent-edit scenario touching every target kind. IDs are fixed
    /// per call so independent folds are comparable.
    struct Scenario {
        page: MarkID,
        block: MarkID,
        edge: TerrainID,
        feature: domain::FeatureID,
        ops: Vec<KeyedOp>,
    }

    fn scenario() -> Scenario {
        let page = MarkID::new();
        let block = MarkID::new();
        let edge = TerrainID::new();
        let feature: domain::FeatureID = "abcd".parse().unwrap();

        // Notice 1 (author 1, hlc 1): create the page, its block, the edge.
        let create = key(1, 1, 1);
        // Concurrent title edits at the same clock: author 3 outranks 2.
        let title_low = key(2, 2, 2);
        let title_high = key(2, 3, 3);
        // A reading update on a distinct target — must never interfere.
        let reading = key(2, 2, 4);
        // Later rich-text edit and an edge reorder.
        let text = key(3, 1, 5);
        let reorder = key(4, 2, 6);

        let ops = vec![
            keyed(
                create,
                0,
                Maneuver::create_mark(page, landmark("Fresh page")),
            ),
            keyed(create, 1, Maneuver::create_mark(block, paragraph("hello"))),
            keyed(
                create,
                2,
                Maneuver::CreateTerrain {
                    edge_id: edge,
                    src: page,
                    dst: block,
                    kind: TerrainKind::Child,
                    order: Some("a0".into()),
                    extra: None,
                },
            ),
            keyed(
                title_low,
                0,
                Maneuver::update_landmark_title(page, rich(""), rich("Loser title")),
            ),
            keyed(
                title_high,
                0,
                Maneuver::update_landmark_title(page, rich(""), rich("Winner title")),
            ),
            keyed(
                reading,
                0,
                Maneuver::update_reading(
                    page,
                    feature.clone(),
                    Reading::Null,
                    Reading::Text(rich("kept")),
                ),
            ),
            keyed(
                text,
                0,
                Maneuver::update_rich_text(block, rich(""), rich("hello, world")),
            ),
            keyed(
                reorder,
                0,
                Maneuver::UpdateTerrain {
                    edge_id: edge,
                    old_src: page,
                    old_order: None,
                    new_src: page,
                    new_order: Some("a2".into()),
                },
            ),
        ];

        Scenario {
            page,
            block,
            edge,
            feature,
            ops,
        }
    }

    /// Everything the scenario can observe about the folded graph.
    async fn snapshot(db: &Store, s: &Scenario) -> impl PartialEq + std::fmt::Debug + use<> {
        let page = mark::get(db, s.page).await.unwrap().map(|m| m.data);
        let block = mark::get(db, s.block).await.unwrap().map(|m| m.data);
        let edge = terrain::get_edge_by_id(db, s.edge)
            .await
            .unwrap()
            .map(|e| (e.src_id, e.dst_id, e.kind, e.order));
        (page, block, edge)
    }

    #[tokio::test]
    async fn fold_is_order_independent_across_permutations() {
        let s = scenario();

        let db = store::test_utils::temp_db().await;
        fold(&db, s.ops.clone()).await.unwrap();
        let canonical = snapshot(&db, &s).await;

        let mut rng = StdRng::seed_from_u64(0xF01D);
        for _ in 0..12 {
            let mut shuffled = s.ops.clone();
            shuffled.shuffle(&mut rng);

            let db = store::test_utils::temp_db().await;
            fold(&db, shuffled).await.unwrap();
            assert_eq!(
                snapshot(&db, &s).await,
                canonical,
                "a delivery permutation changed the folded graph"
            );
        }
    }

    #[tokio::test]
    async fn per_target_lww_takes_the_higher_key_and_targets_never_interfere() {
        let s = scenario();

        // Two "devices" receiving the same set in opposite orders.
        for reverse in [false, true] {
            let mut ops = s.ops.clone();
            if reverse {
                ops.reverse();
            }
            let db = store::test_utils::temp_db().await;
            fold(&db, ops).await.unwrap();

            let page = mark::get(&db, s.page).await.unwrap().unwrap();
            let MarkData::Landmark(lm) = &page.data else {
                panic!("page must stay a landmark");
            };
            // Same target, concurrent clocks: the higher author key won.
            assert_eq!(lm.title.plain_text(), "Winner title");
            // Distinct target on the same mark: untouched by the title race.
            assert_eq!(
                lm.readings.get(&s.feature),
                Some(&Reading::Text(rich("kept")))
            );

            let MarkData::Content(content) = &mark::get(&db, s.block).await.unwrap().unwrap().data
            else {
                panic!("block must stay content");
            };
            assert_eq!(content.rich_text().unwrap().plain_text(), "hello, world");

            let edge = terrain::get_edge_by_id(&db, s.edge).await.unwrap().unwrap();
            assert_eq!(edge.order.as_deref(), Some("a2"));
        }
    }

    #[test]
    fn winners_report_the_greatest_key_per_target() {
        let s = scenario();
        let winners = winners(&s.ops);

        assert_eq!(
            winners[&ManeuverTarget::Title(s.page)],
            key(2, 3, 3),
            "title target must resolve to the higher concurrent key"
        );
        assert_eq!(winners[&ManeuverTarget::Mark(s.page)], key(1, 1, 1));
        assert_eq!(winners[&ManeuverTarget::Terrain(s.edge)], key(4, 2, 6));
        assert_eq!(
            winners[&ManeuverTarget::Reading(s.page, s.feature.clone())],
            key(2, 2, 4)
        );
        assert_eq!(winners.len(), 5, "one winner per distinct target");
    }

    fn title_op(page: MarkID, title: &str) -> Maneuver {
        Maneuver::update_landmark_title(page, rich(""), rich(title))
    }

    async fn title_of(db: &Store, page: MarkID) -> String {
        match mark::get(db, page).await.unwrap().unwrap().data {
            MarkData::Landmark(lm) => lm.title.plain_text().to_owned(),
            other => panic!("expected landmark, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn incremental_apply_rejects_lower_keys_and_admits_higher() {
        let s = scenario();
        let db = store::test_utils::temp_db().await;
        fold_incremental(&db, s.ops.clone()).await.unwrap();
        assert_eq!(title_of(&db, s.page).await, "Winner title");

        // A late-arriving older op for a settled target: rejected, target
        // untouched — even across a fresh pass with no in-memory state.
        let stale = keyed(key(2, 1, 7), 0, title_op(s.page, "Late loser"));
        fold_incremental(&db, vec![stale]).await.unwrap();
        assert_eq!(title_of(&db, s.page).await, "Winner title");

        // A genuinely newer op still lands.
        let newer = keyed(key(9, 1, 8), 0, title_op(s.page, "Even newer"));
        fold_incremental(&db, vec![newer]).await.unwrap();
        assert_eq!(title_of(&db, s.page).await, "Even newer");
    }

    #[tokio::test]
    async fn full_refold_from_empty_matches_incremental_folding() {
        let s = scenario();

        // Incremental: everything except the losing title edit lands first;
        // the older concurrent edit arrives late and is rejected. (Batches
        // referencing not-yet-created marks are the dangling-reference case,
        // deferred by section 8 — not exercised here.)
        let db_inc = store::test_utils::temp_db().await;
        let (stale, fresh): (Vec<_>, Vec<_>) = s
            .ops
            .clone()
            .into_iter()
            .partition(|op| op.key == key(2, 2, 2));
        fold_incremental(&db_inc, fresh).await.unwrap();
        fold_incremental(&db_inc, stale).await.unwrap();

        // Full re-fold from empty, everything in one batch.
        let db_full = store::test_utils::temp_db().await;
        fold_incremental(&db_full, s.ops.clone()).await.unwrap();

        assert_eq!(snapshot(&db_inc, &s).await, snapshot(&db_full, &s).await);
        for target in winners(&s.ops).keys() {
            let inc = sync_store::winner(&db_inc, &target_key(target))
                .await
                .unwrap();
            let full = sync_store::winner(&db_full, &target_key(target))
                .await
                .unwrap();
            assert!(inc.is_some(), "winner must be persisted for {target:?}");
            assert_eq!(inc, full, "persisted winner differs for {target:?}");
        }
    }

    #[tokio::test]
    async fn replicas_with_divergent_topo_assignments_converge() {
        use outpost::protocol::{EpochID, KeyRoot};
        use outpost::replica::Replica;
        use rand::SeedableRng;

        let root = KeyRoot::from_seed([9u8; 64]);
        let identity = root.identity();
        let mut rng = StdRng::seed_from_u64(0x7090);

        // Device A owns the Territory and authors three page.v1 Notices.
        let vessel_a = Vessel::create_territory(&identity, &root.device(0), ":memory:", &mut rng)
            .await
            .unwrap();

        let page = MarkID::new();
        let block = MarkID::new();
        let edge = TerrainID::new();
        let batches = vec![
            vec![
                Maneuver::create_mark(page, landmark("Converge")),
                Maneuver::create_mark(block, paragraph("hello")),
                Maneuver::CreateTerrain {
                    edge_id: edge,
                    src: page,
                    dst: block,
                    kind: TerrainKind::Child,
                    order: Some("a0".into()),
                    extra: None,
                },
            ],
            vec![title_op(page, "Renamed")],
            vec![Maneuver::update_rich_text(
                block,
                rich(""),
                rich("hello, world"),
            )],
        ];
        let mut notices = Vec::new();
        for batch in &batches {
            let plaintext = encode_page_v1(batch).unwrap();
            notices.push(
                vessel_a
                    .author(&mut rng, vec![], EpochID::ZERO, PAGE_V1, &plaintext)
                    .await
                    .unwrap(),
            );
        }

        // Device B (same identity, second device) receives them in reverse:
        // its topo assignment is the mirror of A's.
        let acl = vessel_a.replica().hold().acl_records().await.unwrap();
        let replica_b = Replica::bootstrap(":memory:", &acl).await.unwrap();
        let vessel_b = Vessel::from_identity(&identity, &root.device(1), replica_b)
            .await
            .unwrap();
        let reversed: Vec<_> = notices.iter().rev().cloned().collect();
        vessel_b.replica().ingest(reversed).await.unwrap();

        let db_a = store::test_utils::temp_db().await;
        let db_b = store::test_utils::temp_db().await;
        let report_a = fold_new_notices(&db_a, &vessel_a).await.unwrap();
        let report_b = fold_new_notices(&db_b, &vessel_b).await.unwrap();
        assert_eq!(report_a.folded_notices, 3);
        assert_eq!(report_b.folded_notices, 3);

        let s = Scenario {
            page,
            block,
            edge,
            feature: "abcd".parse().unwrap(),
            ops: vec![],
        };
        assert_eq!(
            snapshot(&db_a, &s).await,
            snapshot(&db_b, &s).await,
            "divergent topo assignments must not affect the folded graph"
        );
        assert_eq!(title_of(&db_a, page).await, "Renamed");

        // A second pass folds nothing new — the cursor advanced.
        let again = fold_new_notices(&db_a, &vessel_a).await.unwrap();
        assert_eq!(again.folded_notices, 0);
        assert_eq!(again.cursor, report_a.cursor);
    }
}
