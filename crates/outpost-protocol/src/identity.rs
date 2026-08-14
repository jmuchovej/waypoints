//! Identity: a keypair, not an account.
//!
//! Everything a user has derives from a BIP-39 mnemonic through SLIP-10
//! (Ed25519, which admits only hardened derivation):
//!
//! ```text
//! mnemonic (12/24 words)
//!   └── SLIP-10 master
//!         ├── m/44'/9297'/0'        identity signing key (Ed25519) — the user ID
//!         │     └── …/0'/0'         identity KEM seed (X-Wing, key delivery)
//!         └── m/44'/9297'/1'/n'     per-device keys, certified by the identity
//! ```
//!
//! The **identity** signing key authors Notices and ACL records (so a
//! `MemberID` is an identity, never a device — validation keys on it).
//! Each **device** gets its own signing key, whose public half is the
//! `VesselID` that binds the transport connection; the identity certifies it
//! ([`auth::DeviceCert`](crate::auth::DeviceCert)) so a Station can learn
//! which identity is behind a device. All of a user's devices share
//! the one identity signing + KEM key, so a new device from the same mnemonic
//! can read all history the identity is entitled to.
//!
//! Subordinate decisions:
//! - Coin type 9297 is "WAYP" on a phone keypad — unregistered in SLIP-44,
//!   which is fine: these paths never interoperate with wallet software.
//! - SLIP-10 is hand-rolled over `hmac`/`sha2` — not novel crypto, just the
//!   spec's byte-plumbing over vetted HMAC, pinned to the SLIP-0010 vectors.

use crate::{CryptoRNG, KEMPublic, KEMSecret, MemberID};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha512;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The device certificate type lives in [`auth`](crate::auth) (so it can ride
/// a connection handshake); re-exported here for the identity API surface.
pub use crate::auth::DeviceCert;

#[allow(clippy::upper_case_acronyms)]
type HMAC = Hmac<Sha512>;

const HARDENED: u32 = 0x8000_0000;
const PURPOSE: u32 = 44;
const COIN: u32 = 9297;

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("invalid mnemonic: {0}")]
    Mnemonic(#[from] bip39::Error),
}

/// One SLIP-10 Ed25519 node: 32 bytes of key material and a 32-byte chain
/// code. Zeroized on drop; never leaves this module.
#[derive(Zeroize, ZeroizeOnDrop)]
struct Node {
    key: [u8; 32],
    chain: [u8; 32],
}

impl Node {
    fn master(seed: &[u8]) -> Self {
        let mut mac = HMAC::new_from_slice(b"ed25519 seed").expect("hmac takes any key len");
        mac.update(seed);
        Self::split(&mac.finalize().into_bytes())
    }

    /// SLIP-10 over Ed25519 defines hardened derivation only, so the high bit
    /// is set unconditionally and callers pass plain indices.
    fn child(&self, index: u32) -> Self {
        let mut mac = HMAC::new_from_slice(&self.chain).expect("hmac takes any key len");
        mac.update(&[0x00]);
        mac.update(&self.key);
        mac.update(&(index | HARDENED).to_be_bytes());
        Self::split(&mac.finalize().into_bytes())
    }

    fn split(digest: &[u8]) -> Self {
        let mut key = [0u8; 32];
        let mut chain = [0u8; 32];
        key.copy_from_slice(&digest[..32]);
        chain.copy_from_slice(&digest[32..]);
        Self { key, chain }
    }

    fn derive(seed: &[u8], path: &[u32]) -> Self {
        path.iter()
            .fold(Self::master(seed), |node, &i| node.child(i))
    }
}

/// The root of a user's key material: the BIP-39 seed, deriving identity and
/// device keys on demand. Keep this only where derivation happens (onboarding,
/// adding a device); day-to-day code holds an [`Identity`] or a device key.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KeyRoot {
    seed: [u8; 64],
}

impl KeyRoot {
    /// Generate a fresh 12-word mnemonic and its key root. The returned
    /// phrase is the user's only backup — surface it, then drop it.
    ///
    /// 12 words = 128 bits of entropy — matching Ed25519's ~128-bit security
    /// level, which caps the identity's strength regardless of seed size
    /// (John, 2026-07-17: halve the transcription burden of the one artifact
    /// meant to be hand-copied; recovery flows verify the derived identity
    /// against an expected `MemberID`, which neutralizes the shorter
    /// checksum). [`from_mnemonic`](Self::from_mnemonic) accepts any valid
    /// length, so earlier 24-word phrases keep working. Deliberately **no**
    /// BIP-39 passphrase parameter: it is a pure KDF input that can never be
    /// changed or removed without becoming a different identity — changeable
    /// protection belongs in the at-rest wrapping layer (app custody,
    /// escrow), not in derivation.
    pub fn generate() -> (Self, String) {
        let mnemonic = bip39::Mnemonic::generate(12).expect("12 is a valid word count");
        let root = Self {
            seed: mnemonic.to_seed(""),
        };
        (root, mnemonic.to_string())
    }

    pub fn from_mnemonic(phrase: &str) -> Result<Self, IdentityError> {
        let mnemonic = bip39::Mnemonic::parse(phrase)?;
        Ok(Self {
            seed: mnemonic.to_seed(""),
        })
    }

    /// Test/bench entry point; production callers go through the mnemonic.
    pub fn from_seed(seed: [u8; 64]) -> Self {
        Self { seed }
    }

    /// The identity keypair: authors Notices and ACL records; its public key
    /// is the user ID everywhere.
    pub fn identity(&self) -> Identity {
        let signing = Node::derive(&self.seed, &[PURPOSE, COIN, 0]);
        let kem = signing.child(0);
        Identity {
            signing: SigningKey::from_bytes(&signing.key),
            kem_seed: kem.key,
        }
    }

    /// The keypair for a fresh random hardened index — how a joining device
    /// picks its key when there is no coordinator to hand out sequential
    /// indices (§4). Collision is a birthday problem over 2³¹ indices —
    /// negligible at any realistic device count. Index 0 is skipped: it is
    /// the conventional first-device index.
    pub fn random_device(&self, rng: &mut impl CryptoRNG) -> DeviceKey {
        let index = (rng.next_u32() & 0x7FFF_FFFF).max(1);
        self.device(index)
    }

    /// The keypair for device `index`, certified by the identity. A new
    /// device picks a fresh index; its public key is the `VesselID`.
    pub fn device(&self, index: u32) -> DeviceKey {
        let node = Node::derive(&self.seed, &[PURPOSE, COIN, 1, index]);
        let signing = SigningKey::from_bytes(&node.key);
        let cert = self.identity().certify(&signing.verifying_key());
        DeviceKey {
            signing,
            cert,
            index,
        }
    }
}

/// The user-level keypair: Ed25519 for signatures, plus an X-Wing KEM
/// key (as a seed) for read-key delivery. Cheap to clone — it holds only
/// the two 32-byte secrets.
#[derive(Clone)]
pub struct Identity {
    signing: SigningKey,
    kem_seed: [u8; 32],
}

impl Identity {
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }

    /// This identity's `MemberID` — how the ACL names it.
    pub fn member_id(&self) -> MemberID {
        MemberID::from(&self.verifying_key())
    }

    /// A fresh KEM secret for unsealing read keys. Rebuilt from the
    /// stored seed so the secret material lives no longer than the caller
    /// keeps it.
    pub fn secret_key(&self) -> KEMSecret {
        KEMSecret::from_seed(self.kem_seed)
    }

    /// The KEM public key an admitting record seals read keys to.
    pub fn public_key(&self) -> KEMPublic {
        self.secret_key().public_key()
    }

    /// Certify a device key as belonging to this identity.
    pub fn certify(&self, device: &VerifyingKey) -> DeviceCert {
        DeviceCert::issue(&self.signing, device)
    }
}

impl Drop for Identity {
    fn drop(&mut self) {
        self.kem_seed.zeroize();
    }
}

/// A device keypair plus its identity-key certificate. The signing key
/// binds the transport connection; the cert proves the identity behind it.
pub struct DeviceKey {
    pub signing: SigningKey,
    pub cert: DeviceCert,
    pub index: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SLIP-0010 Ed25519 test vector 1 (seed `000102…0f`), from the spec —
    /// pins the derivation to the standard.
    #[test]
    fn slip10_vector_1() {
        let seed = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
        let cases: &[(&[u32], &str, &str, &str)] = &[
            (
                &[],
                "90046a93de5380a72b5e45010748567d5ea02bbf6522f979e05c0d8d8ca9fffb",
                "2b4be7f19ee27bbf30c667b642d5f4aa69fd169872f8fc3059c08ebae2eb19e7",
                "a4b2856bfec510abab89753fac1ac0e1112364e7d250545963f135f2a33188ed",
            ),
            (
                &[0],
                "8b59aa11380b624e81507a27fedda59fea6d0b779a778918a2fd3590e16e9c69",
                "68e0fe46dfb67e368c75379acec591dad19df3cde26e63b93a8e704f1dade7a3",
                "8c8a13df77a28f3445213a0f432fde644acaa215fc72dcdf300d5efaa85d350c",
            ),
            (
                &[0, 1],
                "a320425f77d1b5c2505a6b1b27382b37368ee640e3557c315416801243552f14",
                "b1d0bad404bf35da785a64ca1ac54b2617211d2777696fbffaf208f746ae84f2",
                "1932a5270f335bed617d5b935c80aedb1a35bd9fc1e31acafd5372c30f5c1187",
            ),
            (
                &[0, 1, 2, 2, 1_000_000_000],
                "68789923a0cac2cd5a29172a475fe9e0fb14cd6adb5ad98a3fa70333e7afa230",
                "8f94d394a8e8fd6b1bc2f3f49f5c47e385281d5c17e65324b0f62483e37e8793",
                "3c24da049451555d51a7014a37337aa4e12d41e485abccfa46b47dfb2af54b7a",
            ),
        ];
        for (path, chain, private, public) in cases {
            let node = Node::derive(&seed, path);
            assert_eq!(hex::encode(node.chain), *chain, "chain at {path:?}");
            assert_eq!(hex::encode(node.key), *private, "private at {path:?}");
            let pubkey = SigningKey::from_bytes(&node.key).verifying_key();
            assert_eq!(
                hex::encode(pubkey.as_bytes()),
                *public,
                "public at {path:?}"
            );
        }
    }

    /// SLIP-0010 Ed25519 test vector 2 (64-byte seed), deepest path only.
    #[test]
    fn slip10_vector_2() {
        let seed = hex::decode(
            "fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a2\
             9f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542",
        )
        .unwrap();
        let node = Node::derive(&seed, &[0, 2_147_483_647, 1, 2_147_483_646, 2]);
        assert_eq!(
            hex::encode(node.chain),
            "5d70af781f3a37b829f0d060924d5e960bdc02e85423494afc0b1a41bbe196d4"
        );
        assert_eq!(
            hex::encode(node.key),
            "551d333177df541ad876a60ea71f00447931c0a9da16f227c11ea080d7391b8d"
        );
    }

    #[test]
    fn mnemonic_is_deterministic_and_keys_are_distinct() {
        let phrase = "abandon abandon abandon abandon abandon abandon \
                      abandon abandon abandon abandon abandon about";
        let a = KeyRoot::from_mnemonic(phrase).unwrap();
        let b = KeyRoot::from_mnemonic(phrase).unwrap();
        assert_eq!(a.identity().member_id(), b.identity().member_id());
        assert_eq!(
            a.identity().public_key().as_bytes(),
            b.identity().public_key().as_bytes()
        );
        assert_ne!(
            a.identity().verifying_key(),
            a.device(0).signing.verifying_key()
        );
        assert_ne!(
            a.device(0).signing.to_bytes(),
            a.device(1).signing.to_bytes()
        );
    }

    #[test]
    fn generated_mnemonic_round_trips() {
        let (root, phrase) = KeyRoot::generate();
        let reparsed = KeyRoot::from_mnemonic(&phrase).unwrap();
        assert_eq!(root.identity().member_id(), reparsed.identity().member_id());
    }

    #[test]
    fn device_cert_verifies_and_rejects_forgery() {
        let root = KeyRoot::from_seed([7; 64]);
        let identity = root.identity();
        let device = root.device(0);
        device.cert.verify(&identity.verifying_key()).unwrap();

        let stranger = KeyRoot::from_seed([8; 64]).identity();
        assert!(device.cert.verify(&stranger.verifying_key()).is_err());
    }
}
