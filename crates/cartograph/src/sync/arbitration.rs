//! The arbitration key: the content-derived total order the fold sorts by.
//!
//! Per-target last-write-wins needs every device to agree on which of two
//! concurrent operations is "last" without coordination. The key is
//! `(hlc, author verifying-key bytes, notice id)` compared lexicographically —
//! every component is read from the signed, AAD-bound `EnvelopeHeader` (or is
//! the Notice's content address), so two devices holding the same Notice
//! derive an identical key and the induced order is total: the trailing
//! `NoticeID` never lets two distinct Notices compare equal.
//!
//! Derivation deliberately takes no local state: no arrival time, no `topo`,
//! nothing a replica could observe differently from its peers.

use outpost::protocol::{EnvelopeHeader, HLC, NoticeID};

/// Total order over operations for the per-target LWW fold. Higher wins.
///
/// Field order carries the semantics: the derived `Ord` compares `hlc`,
/// then `author`, then `notice` — lexicographic, exactly the arbitration
/// rule. Do not reorder fields.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ArbitrationKey {
    /// Causality-respecting wall-clock intent, from `EnvelopeHeader.timestamp`.
    hlc: HLC,
    /// Author's Ed25519 verifying-key bytes: deterministic inter-device tie-break.
    author: [u8; 32],
    /// Content address of the carrying Notice: makes the order total.
    notice: NoticeID,
}

impl ArbitrationKey {
    /// Derive the key for an operation carried by the Notice with header
    /// `header` and content address `notice`. Pure: reads only signed
    /// content, never local state.
    pub fn derive(header: &EnvelopeHeader, notice: NoticeID) -> Self {
        Self {
            hlc: HLC(header.timestamp),
            author: header.author.to_bytes(),
            notice,
        }
    }

    pub fn hlc(&self) -> HLC {
        self.hlc
    }

    pub fn author(&self) -> &[u8; 32] {
        &self.author
    }

    pub fn notice(&self) -> NoticeID {
        self.notice
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use outpost::protocol::{Envelope, EpochID, RecordID};
    use rand::seq::SliceRandom;
    use rand::{RngExt, SeedableRng, rngs::StdRng};

    fn key(hlc: i64, author: u8, notice: u8) -> ArbitrationKey {
        ArbitrationKey {
            hlc: HLC(hlc),
            author: [author; 32],
            notice: NoticeID::from_bytes([notice; 32]),
        }
    }

    fn header(timestamp: i64, author_seed: u8) -> EnvelopeHeader {
        EnvelopeHeader {
            parents: vec![],
            acl_head: RecordID::from_bytes([2u8; 32]),
            epoch: EpochID::ZERO,
            author: SigningKey::from_bytes(&[author_seed; 32]).verifying_key(),
            snapshot_base: NoticeID::from_bytes([3u8; 32]),
            is_snapshot: false,
            payload_type: "waypoints.page.v1".into(),
            timestamp,
        }
    }

    #[test]
    fn equal_clocks_tie_break_by_author_then_notice() {
        let a = key(7, 1, 9);
        let b = key(7, 2, 0);
        assert!(a < b, "same hlc orders by author bytes");

        let c = key(7, 1, 1);
        let d = key(7, 1, 2);
        assert!(c < d, "same hlc and author orders by notice id");

        // hlc dominates both tie-breaks.
        assert!(key(8, 0, 0) > key(7, u8::MAX, u8::MAX));
    }

    #[test]
    fn distinct_notices_never_compare_equal() {
        let a = key(7, 1, 1);
        let b = key(7, 1, 2);
        assert_ne!(a.cmp(&b), std::cmp::Ordering::Equal);
        assert_eq!(a.cmp(&a), std::cmp::Ordering::Equal);
    }

    #[test]
    fn derivation_is_deterministic_over_header_bytes() {
        // Round-trip the header through its wire encoding: two independent
        // parses of the same bytes must derive the same key.
        let envelope = Envelope {
            header: header(1_234_567, 5),
            payload: b"opaque".to_vec(),
        };
        let bytes = envelope.encode().unwrap();
        let notice = NoticeID::from_bytes([9u8; 32]);

        let first = ArbitrationKey::derive(&Envelope::parse(&bytes).unwrap().header, notice);
        let second = ArbitrationKey::derive(&Envelope::parse(&bytes).unwrap().header, notice);
        assert_eq!(first, second);
        assert_eq!(first.hlc(), HLC(1_234_567));
        assert_eq!(
            *first.author(),
            SigningKey::from_bytes(&[5u8; 32])
                .verifying_key()
                .to_bytes()
        );
        assert_eq!(first.notice(), notice);
    }

    #[test]
    fn sorting_shuffles_yields_one_canonical_sequence() {
        // Property: any permutation of the same key set sorts to the same
        // sequence. Seeded shuffles keep the test deterministic while
        // exercising many permutations, including duplicate-heavy inputs
        // (colliding hlc and author values force the deeper tie-breaks).
        let mut rng = StdRng::seed_from_u64(0xA11CE);
        let keys: Vec<ArbitrationKey> = (0..200)
            .map(|_| {
                key(
                    rng.random_range(0..4) as i64,
                    rng.random_range(0..3),
                    rng.random(),
                )
            })
            .collect();

        let mut canonical = keys.clone();
        canonical.sort();

        for _ in 0..100 {
            let mut shuffled = keys.clone();
            shuffled.shuffle(&mut rng);
            shuffled.sort();
            assert_eq!(shuffled, canonical);
        }
    }
}
