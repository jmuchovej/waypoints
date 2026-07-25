//! Encryption: payload AEAD, sealed-box read-key delivery, and the
//! backwards epoch key chain.
//!
//! Primitives (carried decisions): XChaCha20-Poly1305 for payloads
//! and key wraps; X-Wing (hybrid X25519 + ML-KEM-768) sealed boxes for
//! read-key delivery — the ACL log is world-replicated ciphertext that is
//! never re-encrypted, so key delivery must survive
//! harvest-now-decrypt-later. Signatures stay Ed25519.
//!
//! Every randomized operation takes the RNG as a parameter: this crate has
//! no ambient entropy source, which keeps it pure and every test
//! deterministic.

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use rand_core::CryptoRng;
/// The RNG bound every randomized operation here takes, re-exported so
/// callers need not depend on `rand_core` directly.
pub use rand_core::CryptoRng as CryptoRngExport;
use serde::{Deserialize, Serialize};
use x_wing::{Decapsulate, Decapsulator, Encapsulate, KeyExport};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::envelope::EnvelopeHeader;
use crate::error::CryptoError;

/// XChaCha20 nonce length.
pub const NONCE_LEN: usize = 24;
/// X-Wing encapsulation (public) key length.
pub const KEM_PUBLIC_LEN: usize = x_wing::ENCAPSULATION_KEY_SIZE;
/// X-Wing KEM ciphertext length inside a sealed box.
pub const KEM_CIPHERTEXT_LEN: usize = x_wing::CIPHERTEXT_SIZE;

/// Domain separation for the key-chain wrap, so a wrapped key can
/// never be confused with (or replayed as) a Notice payload.
const KEY_CHAIN_AAD: &[u8] = b"outpost.keychain.v1";

/// One epoch's symmetric read key. Redacted `Debug`, zeroized on
/// drop; never serialized directly — it travels only inside sealed boxes
/// ([`seal_key_to_member`]) or chain wraps ([`wrap_prev_key`]).
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct ReadKey([u8; 32]);

impl ReadKey {
    pub fn generate(rng: &mut impl CryptoRng) -> Self {
        let mut key = [0u8; 32];
        rng.fill_bytes(&mut key);
        Self(key)
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl core::fmt::Debug for ReadKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ReadKey(…)")
    }
}

/// A member's X-Wing encapsulation key as it travels in ACL records —
/// plain bytes on the wire, validated when used.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KemPublicKey(Vec<u8>);

impl KemPublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn encapsulation_key(&self) -> Result<x_wing::EncapsulationKey, CryptoError> {
        x_wing::EncapsulationKey::try_from(self.0.as_slice()).map_err(|_| CryptoError::BadKey)
    }
}

impl core::fmt::Debug for KemPublicKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "KemPublicKey({} bytes)", self.0.len())
    }
}

/// The vessel-held decapsulation seed. The 32-byte seed is a SLIP-10 node
/// key, so mnemonic recovery restores it.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KemSecretKey([u8; 32]);

impl KemSecretKey {
    pub const fn from_seed(seed: [u8; 32]) -> Self {
        Self(seed)
    }

    pub fn public_key(&self) -> KemPublicKey {
        let dk = x_wing::DecapsulationKey::from(self.0);
        KemPublicKey(dk.encapsulation_key().to_bytes().to_vec())
    }

    fn decapsulation_key(&self) -> x_wing::DecapsulationKey {
        x_wing::DecapsulationKey::from(self.0)
    }
}

impl core::fmt::Debug for KemSecretKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("KemSecretKey(…)")
    }
}

fn cipher(key: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(key.into())
}

fn random_nonce(rng: &mut impl CryptoRng) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt a Notice payload under the epoch read key, with the encoded
/// header as associated data. Output: `nonce ‖ ciphertext` — what
/// goes in [`Envelope::payload`](crate::Envelope::payload).
pub fn seal_payload(
    rng: &mut impl CryptoRng,
    key: &ReadKey,
    header: &EnvelopeHeader,
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let aad = header.aad()?;
    let nonce = random_nonce(rng);
    let ciphertext = cipher(&key.0)
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Aead)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a Notice payload. Fails if the key, the payload bytes, or any
/// header field differ from what was sealed — the AAD binding is what
/// makes a payload untransplantable.
pub fn open_payload(
    key: &ReadKey,
    header: &EnvelopeHeader,
    sealed: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let aad = header.aad()?;
    let (nonce, ciphertext) = sealed
        .split_at_checked(NONCE_LEN)
        .ok_or(CryptoError::Truncated)?;
    cipher(&key.0)
        .decrypt(
            &XNonce::try_from(nonce).map_err(|_| CryptoError::Truncated)?,
            Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::Aead)
}

/// Seal a read key to one member's KEM key: the sealed box carried
/// by `Root`/`Approve`/`Remove` ACL records. Layout:
/// `kem_ciphertext ‖ nonce ‖ AEAD(shared_secret, read_key)`.
pub fn seal_key_to_member(
    rng: &mut impl CryptoRng,
    recipient: &KemPublicKey,
    key: &ReadKey,
) -> Result<Vec<u8>, CryptoError> {
    let (kem_ciphertext, shared) = recipient.encapsulation_key()?.encapsulate_with_rng(rng);
    let shared: [u8; 32] = shared.into();
    let nonce = random_nonce(rng);
    let body = cipher(&shared)
        .encrypt(&XNonce::from(nonce), key.0.as_slice())
        .map_err(|_| CryptoError::Aead)?;
    let mut out = Vec::with_capacity(KEM_CIPHERTEXT_LEN + NONCE_LEN + body.len());
    out.extend_from_slice(kem_ciphertext.as_ref());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Open a sealed box with the vessel's decapsulation seed.
pub fn open_sealed_key(secret: &KemSecretKey, sealed: &[u8]) -> Result<ReadKey, CryptoError> {
    let (kem_ciphertext, body) = sealed
        .split_at_checked(KEM_CIPHERTEXT_LEN)
        .ok_or(CryptoError::Truncated)?;
    let (nonce, ciphertext) = body
        .split_at_checked(NONCE_LEN)
        .ok_or(CryptoError::Truncated)?;
    let shared = secret
        .decapsulation_key()
        .decapsulate_slice(kem_ciphertext)
        .map_err(|_| CryptoError::Aead)?;
    let shared: [u8; 32] = shared.into();
    let plaintext = cipher(&shared)
        .decrypt(
            &XNonce::try_from(nonce).map_err(|_| CryptoError::Truncated)?,
            ciphertext,
        )
        .map_err(|_| CryptoError::Aead)?;
    let bytes: [u8; 32] = plaintext.try_into().map_err(|_| CryptoError::BadKey)?;
    Ok(ReadKey(bytes))
}

/// Wrap epoch N−1's key under epoch N's key: the `prev_key_wrapped`
/// field of a `Remove` record. A newly admitted member holding only the
/// current key walks these wraps back through all history; a removed
/// member's knowledge ends at their last epoch.
pub fn wrap_prev_key(rng: &mut impl CryptoRng, next: &ReadKey, prev: &ReadKey) -> Vec<u8> {
    let nonce = random_nonce(rng);
    let body = cipher(&next.0)
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: prev.0.as_slice(),
                aad: KEY_CHAIN_AAD,
            },
        )
        .expect("in-memory AEAD encryption cannot fail");
    let mut out = Vec::with_capacity(NONCE_LEN + body.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    out
}

/// Unwrap one link of the backwards key chain.
pub fn unwrap_prev_key(next: &ReadKey, wrapped: &[u8]) -> Result<ReadKey, CryptoError> {
    let (nonce, ciphertext) = wrapped
        .split_at_checked(NONCE_LEN)
        .ok_or(CryptoError::Truncated)?;
    let plaintext = cipher(&next.0)
        .decrypt(
            &XNonce::try_from(nonce).map_err(|_| CryptoError::Truncated)?,
            Payload {
                msg: ciphertext,
                aad: KEY_CHAIN_AAD,
            },
        )
        .map_err(|_| CryptoError::Aead)?;
    let bytes: [u8; 32] = plaintext.try_into().map_err(|_| CryptoError::BadKey)?;
    Ok(ReadKey(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{EpochID, NoticeID, RecordID};

    fn rng() -> impl CryptoRng {
        // Deterministic test RNG: fixed-seed ChaCha via rand.
        use rand::SeedableRng;
        rand::rngs::StdRng::seed_from_u64(42)
    }

    fn header() -> EnvelopeHeader {
        EnvelopeHeader {
            parents: vec![],
            acl_head: RecordID::from_bytes([2u8; 32]),
            epoch: EpochID::ZERO,
            author: ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]).verifying_key(),
            snapshot_base: NoticeID::from_bytes([3u8; 32]),
            is_snapshot: false,
            payload_type: "waypoints.test.v1".into(),
            timestamp: 1,
        }
    }

    #[test]
    fn payload_roundtrip() {
        let mut rng = rng();
        let key = ReadKey::generate(&mut rng);
        let sealed = seal_payload(&mut rng, &key, &header(), b"the ops").unwrap();
        assert_eq!(open_payload(&key, &header(), &sealed).unwrap(), b"the ops");
    }

    #[test]
    fn payload_is_bound_to_header() {
        let mut rng = rng();
        let key = ReadKey::generate(&mut rng);
        let sealed = seal_payload(&mut rng, &key, &header(), b"the ops").unwrap();

        // Same key, transplanted under a different header: must not open.
        let mut other = header();
        other.timestamp += 1;
        assert!(matches!(
            open_payload(&key, &other, &sealed),
            Err(CryptoError::Aead)
        ));
    }

    #[test]
    fn payload_rejects_wrong_key_and_truncation() {
        let mut rng = rng();
        let key = ReadKey::generate(&mut rng);
        let sealed = seal_payload(&mut rng, &key, &header(), b"the ops").unwrap();

        let other = ReadKey::generate(&mut rng);
        assert!(open_payload(&other, &header(), &sealed).is_err());
        assert!(matches!(
            open_payload(&key, &header(), &sealed[..NONCE_LEN - 1]),
            Err(CryptoError::Truncated)
        ));
    }

    #[test]
    fn sealed_box_roundtrip() {
        let mut rng = rng();
        let secret = KemSecretKey::from_seed([9u8; 32]);
        let key = ReadKey::generate(&mut rng);
        let sealed = seal_key_to_member(&mut rng, &secret.public_key(), &key).unwrap();
        assert_eq!(open_sealed_key(&secret, &sealed).unwrap(), key);
    }

    #[test]
    fn sealed_box_rejects_wrong_recipient_and_tampering() {
        let mut rng = rng();
        let secret = KemSecretKey::from_seed([9u8; 32]);
        let key = ReadKey::generate(&mut rng);
        let mut sealed = seal_key_to_member(&mut rng, &secret.public_key(), &key).unwrap();

        let imposter = KemSecretKey::from_seed([10u8; 32]);
        assert!(open_sealed_key(&imposter, &sealed).is_err());

        // Tamper with the KEM ciphertext: decapsulation yields a different
        // shared secret (implicit rejection), so the AEAD must fail.
        sealed[0] ^= 1;
        assert!(open_sealed_key(&secret, &sealed).is_err());
    }

    #[test]
    fn key_chain_walks_backwards() {
        let mut rng = rng();
        let epoch0 = ReadKey::generate(&mut rng);
        let epoch1 = ReadKey::generate(&mut rng);
        let epoch2 = ReadKey::generate(&mut rng);

        let wrap01 = wrap_prev_key(&mut rng, &epoch1, &epoch0);
        let wrap12 = wrap_prev_key(&mut rng, &epoch2, &epoch1);

        let walked1 = unwrap_prev_key(&epoch2, &wrap12).unwrap();
        assert_eq!(walked1, epoch1);
        assert_eq!(unwrap_prev_key(&walked1, &wrap01).unwrap(), epoch0);

        // A key from the wrong epoch opens nothing.
        assert!(unwrap_prev_key(&epoch0, &wrap12).is_err());
    }
}
