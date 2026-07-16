//! The Notice wire structure (§5.1) and the raw-bytes discipline (§5.2).
//!
//! Three nested layers, and the nesting order is what makes IDs and
//! signatures meaningful:
//!
//! ```text
//! NoticeID = blake3( SignedNotice bytes )          ── content address
//! SignedNotice { bytes, signature }                ── signature covers `bytes`
//! Envelope { header…, payload: AEAD ciphertext }   ── header plaintext
//! ```
//!
//! **The encoded bytes are the object.** Store them, forward them, hash
//! them, verify signatures against them; parse lazily into [`Envelope`]
//! views and never verify against a re-encoding.
//!
//! Subordinate decisions (refining the design doc, not contradicting it):
//! - The envelope encoding is prefixed with a format-version byte
//!   ([`ENVELOPE_VERSION`]); parsers reject unknown versions. Postcard
//!   structs carry no tags, so this byte is the only room the wire format
//!   has to evolve.
//! - Signature verification uses `verify_strict`, rejecting the malleable /
//!   weak-key edge cases plain `verify` accepts. Content-addressed history
//!   should have exactly one accepted byte form.

use bytes::Bytes;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::error::WireError;
use crate::id::{EpochID, NoticeID, RecordID};

/// Wire-format version of the [`Envelope`] encoding. Bumped only for
/// wire-incompatible changes.
pub const ENVELOPE_VERSION: u8 = 1;

/// Parsed view of [`SignedNotice::bytes`]. Never re-encoded for
/// verification (§5.2).
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Envelope {
    /// Parent Notice IDs. Empty ⇒ this is a track root.
    pub parents: Vec<NoticeID>,
    /// The ACL record in force when this Notice was authored (§8.3).
    pub acl_head: RecordID,
    /// Which read-key epoch encrypts `payload` (§7.2).
    pub epoch: EpochID,
    /// Author's identity public key. Must have write permission at
    /// `acl_head` (validated in §5.4, slice 3).
    pub author: VerifyingKey,
    /// Nearest ancestor snapshot (§6.3). Roots reference themselves.
    pub snapshot_base: NoticeID,
    /// True ⇒ payload is a full materialized state, not a delta.
    pub is_snapshot: bool,
    /// Vessel-side type tag (e.g. `waypoints.page.v1`). Opaque to Stations.
    pub payload_type: String,
    /// Vessel clock; advisory only, never ordering.
    pub timestamp: i64,
    /// XChaCha20-Poly1305 ciphertext. AAD = the version byte plus all
    /// header fields (§7.4, slice 2).
    pub payload: Vec<u8>,
}

impl Envelope {
    /// Canonical encoding: the version byte, then the postcard body. The
    /// result is the byte string signatures cover and stores keep.
    pub fn encode(&self) -> Result<Bytes, WireError> {
        let mut out = postcard::to_allocvec(self)?;
        out.insert(0, ENVELOPE_VERSION);
        Ok(Bytes::from(out))
    }

    /// Parse an encoded envelope. This is a *view* of the bytes — callers
    /// keep the original bytes as the object of record.
    pub fn parse(bytes: &[u8]) -> Result<Self, WireError> {
        let (&version, body) = bytes.split_first().ok_or(WireError::Truncated)?;
        if version != ENVELOPE_VERSION {
            return Err(WireError::UnsupportedVersion(version));
        }
        Ok(postcard::from_bytes(body)?)
    }
}

/// Signed, immutable unit of replication (§5.1). `bytes` is canonical:
/// hashing and signature verification always operate on these exact bytes.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SignedNotice {
    /// The encoded [`Envelope`] — the object of record.
    pub bytes: Bytes,
    /// The author's Ed25519 signature over `bytes`.
    pub signature: Signature,
}

impl SignedNotice {
    /// Encode `envelope` and sign the resulting bytes with the author's
    /// identity key. The caller is responsible for `key` matching
    /// `envelope.author` — [`parse_and_verify`](Self::parse_and_verify)
    /// enforces it on every read path.
    pub fn sign(envelope: &Envelope, key: &SigningKey) -> Result<Self, WireError> {
        let bytes = envelope.encode()?;
        let signature = key.sign(&bytes);
        Ok(Self { bytes, signature })
    }

    /// Parse the envelope view and verify the signature over the raw bytes
    /// against the `author` key found inside them — steps 1 and 2 of the
    /// validation function (§5.4).
    pub fn parse_and_verify(&self) -> Result<Envelope, WireError> {
        let envelope = Envelope::parse(&self.bytes)?;
        envelope
            .author
            .verify_strict(&self.bytes, &self.signature)?;
        Ok(envelope)
    }

    /// Canonical encoding of the whole notice — what [`NoticeID`] hashes
    /// and what stores and the wire carry.
    pub fn encode(&self) -> Result<Bytes, WireError> {
        Ok(Bytes::from(postcard::to_allocvec(self)?))
    }

    /// Decode a notice received from a peer. No validation happens here;
    /// every received notice goes through §5.4 before acceptance.
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        Ok(postcard::from_bytes(bytes)?)
    }
}

impl NoticeID {
    /// The content address: BLAKE3 of the encoded [`SignedNotice`]. Derived,
    /// not checked — it *is* the hash (§5.4).
    pub fn of(notice: &SignedNotice) -> Result<Self, WireError> {
        let encoded = notice.encode()?;
        Ok(Self::from_bytes(*blake3::hash(&encoded).as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn test_envelope() -> Envelope {
        Envelope {
            parents: vec![NoticeID::from_bytes([1u8; 32])],
            acl_head: RecordID::from_bytes([2u8; 32]),
            epoch: EpochID::ZERO,
            author: test_key().verifying_key(),
            snapshot_base: NoticeID::from_bytes([3u8; 32]),
            is_snapshot: false,
            payload_type: "waypoints.test.v1".into(),
            timestamp: 1_700_000_000,
            payload: b"opaque ciphertext".to_vec(),
        }
    }

    #[test]
    fn encode_parse_roundtrip() {
        let envelope = test_envelope();
        let bytes = envelope.encode().unwrap();
        assert_eq!(bytes[0], ENVELOPE_VERSION);
        assert_eq!(Envelope::parse(&bytes).unwrap(), envelope);
    }

    #[test]
    fn unknown_version_is_rejected() {
        let envelope = test_envelope();
        let mut bytes = envelope.encode().unwrap().to_vec();
        bytes[0] = 99;
        assert!(matches!(
            Envelope::parse(&bytes),
            Err(WireError::UnsupportedVersion(99))
        ));
        assert!(matches!(Envelope::parse(&[]), Err(WireError::Truncated)));
    }

    #[test]
    fn sign_then_verify() {
        let notice = SignedNotice::sign(&test_envelope(), &test_key()).unwrap();
        let envelope = notice.parse_and_verify().unwrap();
        assert_eq!(envelope, test_envelope());
    }

    #[test]
    fn tampered_bytes_fail_verification() {
        let notice = SignedNotice::sign(&test_envelope(), &test_key()).unwrap();
        let mut tampered = notice.bytes.to_vec();
        *tampered.last_mut().unwrap() ^= 0x01;
        let tampered = SignedNotice {
            bytes: Bytes::from(tampered),
            signature: notice.signature,
        };
        assert!(matches!(
            tampered.parse_and_verify(),
            Err(WireError::Signature(_))
        ));
    }

    #[test]
    fn wrong_key_fails_verification() {
        let envelope = test_envelope();
        let imposter = SigningKey::from_bytes(&[8u8; 32]);
        // Signed by a key that is not `envelope.author`.
        let forged = SignedNotice {
            bytes: envelope.encode().unwrap(),
            signature: imposter.sign(&envelope.encode().unwrap()),
        };
        assert!(matches!(
            forged.parse_and_verify(),
            Err(WireError::Signature(_))
        ));
    }

    #[test]
    fn notice_id_is_stable_and_content_sensitive() {
        let notice = SignedNotice::sign(&test_envelope(), &test_key()).unwrap();
        let id = NoticeID::of(&notice).unwrap();
        assert_eq!(NoticeID::of(&notice).unwrap(), id);

        let mut other = test_envelope();
        other.timestamp += 1;
        let other = SignedNotice::sign(&other, &test_key()).unwrap();
        assert_ne!(NoticeID::of(&other).unwrap(), id);
    }

    #[test]
    fn notice_encode_decode_roundtrip() {
        let notice = SignedNotice::sign(&test_envelope(), &test_key()).unwrap();
        let wire = notice.encode().unwrap();
        assert_eq!(SignedNotice::decode(&wire).unwrap(), notice);
    }
}
