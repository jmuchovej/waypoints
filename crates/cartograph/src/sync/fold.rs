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
use std::collections::HashMap;
use store::Store;

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
}
