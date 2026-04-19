use crate::editor::Maneuver;
use domain::{MarkID, mark::RichText};
use outpost::sync::peers::wire::EnvelopeFace;
use serde::{Deserialize, Serialize};

/// The unit of sync: one command group's ops, wrapped with the identity and
/// clock context needed to apply it on any device. This is the wire format —
/// every field except the skip-marked internals of `ops` is transmitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// The §4 routing face — outpost's wire type, flattened so the JSON
    /// shape is unchanged. What a relay reads is literally this field;
    /// everything below it is opaque to the log plane.
    #[serde(flatten)]
    pub face: EnvelopeFace,
    pub checkpoints: Vec<Maneuver>,
    /// Mark IDs touched by this group. Used for cheap dependency checks
    /// ("does the receiver have all these marks already?") before applying.
    /// Not load-bearing in v1 (LWW handles missing marks gracefully via
    /// CreateMark forward-fill), but cheap to populate and useful later.
    pub touched_marks: Vec<MarkID>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TextPayload {
    /// v1: full snapshot; LWW replaces the receiver's RichText.
    Snapshot { rich_text: RichText },
    // Future: CrdtOps { ops: Vec<u8> } — opaque CRDT delta.
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{AccountID, DeviceID, HLC, MarkData, OpID, RealmID, TerrainID, TerrainKind};

    fn representative_group() -> Envelope {
        let mark_id = MarkID::new();
        let parent_id = MarkID::new();
        Envelope {
            face: EnvelopeFace {
                group_id: OpID::new(),
                account_id: AccountID::new(),
                realm_id: RealmID::new(),
                device_id: DeviceID::new(),
                hlc: HLC::pack(1_735_689_600_000, 42),
            },
            checkpoints: vec![
                Maneuver::CreateMark {
                    mark_id,
                    data: MarkData::default(),
                },
                Maneuver::UpdateMarkData {
                    mark_id,
                    old: MarkData::default(),
                    new: MarkData::default(),
                },
                Maneuver::UpdateRichText {
                    mark_id,
                    old: RichText::new(Vec::new()),
                    new: TextPayload::Snapshot {
                        rich_text: RichText::new(Vec::new()),
                    },
                },
                Maneuver::CreateTerrain {
                    edge_id: TerrainID::new(),
                    src: parent_id,
                    dst: mark_id,
                    kind: TerrainKind::Child,
                    order: Some("i000".into()),
                    extra: None,
                },
                Maneuver::UpdateTerrain {
                    edge_id: TerrainID::new(),
                    old_src: parent_id,
                    old_order: Some("i000".into()),
                    new_src: parent_id,
                    new_order: Some("i001".into()),
                },
            ],
            touched_marks: vec![mark_id, parent_id],
        }
    }

    #[test]
    fn logged_group_round_trips_through_json() {
        let group = representative_group();
        let json = serde_json::to_value(&group).unwrap();
        let back: Envelope = serde_json::from_value(json.clone()).unwrap();

        // Skip-marked fields (`old`, `old_src`, `old_order`) come back as
        // defaults, so compare the wire representation, not the structs.
        let rewire = serde_json::to_value(&back).unwrap();
        assert_eq!(json, rewire);

        assert!(
            json["hlc"].is_string(),
            "hlc must be a string on the wire: JS number precision"
        );

        // Non-skipped fields survive intact.
        assert_eq!(back.face, group.face);
        assert_eq!(back.touched_marks, group.touched_marks);
        assert_eq!(back.checkpoints.len(), group.checkpoints.len());
    }

    #[test]
    fn skip_fields_are_absent_from_wire_output() {
        let group = representative_group();
        let json = serde_json::to_value(&group).unwrap();

        for op in json["checkpoints"].as_array().unwrap() {
            let keys: Vec<&str> = op.as_object().unwrap().keys().map(|k| k.as_str()).collect();
            assert!(!keys.contains(&"old"), "`old` leaked into wire: {op}");
            assert!(!keys.contains(&"old_src"), "`old_src` leaked: {op}");
            assert!(!keys.contains(&"old_order"), "`old_order` leaked: {op}");
        }
    }

    #[test]
    fn text_payload_uses_kind_tag() {
        let payload = TextPayload::Snapshot {
            rich_text: RichText::new(Vec::new()),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["kind"], "Snapshot");
    }
}
