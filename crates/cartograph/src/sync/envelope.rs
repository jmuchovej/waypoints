use domain::mark::RichText;
use serde::{Deserialize, Serialize};

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

    #[test]
    fn text_payload_uses_kind_tag() {
        let payload = TextPayload::Snapshot {
            rich_text: RichText::new(Vec::new()),
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["kind"], "Snapshot");
    }
}
