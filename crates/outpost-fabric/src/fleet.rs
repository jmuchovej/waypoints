//! Fleets — the deployment and trust boundary. A Fleet is a
//! cryptographic identity (the fleet key) plus a **signed, revision-numbered
//! Station roster**, distributed as an untrusted file: the signature makes
//! passing it around (QR code, well-known endpoint, invite payload) safe.
//!
//! What lives here is the connection-auth *basis*: verifying a config
//! against its FleetID, and checking whether a peer key is in the roster.
//! iroh authenticates the remote endpoint's node id for free (it *is* an
//! Ed25519 key), so "who is the peer" needs no extra challenge — the fabric
//! only has to answer "is that authenticated key in a roster I trust". The
//! security boundary for data remains the signatures + AEAD that travel
//! with it; this is admission, not the data guarantee.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

/// A Fleet's identity: the fleet public key. Verifying a [`FleetConfig`]
/// against this is what makes an otherwise-untrusted config trustworthy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FleetId(pub VerifyingKey);

/// The roles a Station plays in its Fleet. A self-hoster runs all
/// of them in one process; a hosted Fleet spreads them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StationRoles {
    pub storage: bool,
    pub orderer: bool,
    pub blob: bool,
}

/// One Station in the roster: its peer key (which it authenticates with at
/// the transport) and its dialable addresses.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StationEntry {
    pub peer_key: VerifyingKey,
    /// Opaque address hints (an iroh ticket / node addr, encoded). The
    /// fabric treats these as dial strings; their format is the endpoint's
    /// concern, not the roster's.
    pub addrs: Vec<String>,
    pub roles: StationRoles,
}

/// The signed roster body — everything the fleet key vouches for.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RosterBody {
    pub fleet: FleetId,
    /// Monotonic; a Vessel rejects a rollback to an older revision.
    pub revision: u64,
    pub stations: Vec<StationEntry>,
}

impl RosterBody {
    fn encode(&self) -> Vec<u8> {
        let mut out = postcard::to_stdvec(self).expect("roster body serializes");
        out.insert(0, ROSTER_VERSION);
        out
    }

    /// Sign this roster with the fleet key, producing a distributable
    /// [`FleetConfig`]. The signing key must match `self.fleet`.
    pub fn sign(self, fleet_key: &SigningKey) -> FleetConfig {
        let signature = fleet_key.sign(&self.encode());
        FleetConfig {
            body: self,
            signature,
        }
    }
}

/// Wire-format version of the roster encoding.
pub const ROSTER_VERSION: u8 = 1;

/// A signed Station roster — the file a Vessel holds and verifies.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FleetConfig {
    pub body: RosterBody,
    pub signature: Signature,
}

#[derive(Debug, thiserror::Error)]
pub enum FleetError {
    #[error("fleet config signature does not verify")]
    BadSignature,
    #[error("roster revision {got} is older than trusted revision {trusted}")]
    Rollback { got: u64, trusted: u64 },
}

impl FleetConfig {
    pub fn fleet(&self) -> FleetId {
        self.body.fleet
    }

    pub fn revision(&self) -> u64 {
        self.body.revision
    }

    /// Verify the roster against its own FleetID. A config that fails this
    /// is discarded, however it arrived.
    pub fn verify(&self) -> Result<(), FleetError> {
        self.body
            .fleet
            .0
            .verify_strict(&self.body.encode(), &self.signature)
            .map_err(|_| FleetError::BadSignature)
    }

    /// Verify, and reject a rollback below a revision already trusted
    /// Vessels call this when replacing a held config.
    pub fn verify_against(&self, trusted_revision: u64) -> Result<(), FleetError> {
        self.verify()?;
        if self.body.revision < trusted_revision {
            return Err(FleetError::Rollback {
                got: self.body.revision,
                trusted: trusted_revision,
            });
        }
        Ok(())
    }

    /// Is `peer` a Station in this roster? The admission check a peer runs
    /// on an authenticated connection.
    pub fn station(&self, peer: &VerifyingKey) -> Option<&StationEntry> {
        self.body.stations.iter().find(|s| &s.peer_key == peer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet_key() -> SigningKey {
        SigningKey::from_bytes(&[0xF1; 32])
    }

    fn roster(revision: u64) -> RosterBody {
        RosterBody {
            fleet: FleetId(fleet_key().verifying_key()),
            revision,
            stations: vec![StationEntry {
                peer_key: SigningKey::from_bytes(&[0x5A; 32]).verifying_key(),
                addrs: vec!["node-ticket-abc".into()],
                roles: StationRoles {
                    storage: true,
                    orderer: true,
                    blob: false,
                },
            }],
        }
    }

    #[test]
    fn signed_config_verifies_and_finds_stations() {
        let config = roster(1).sign(&fleet_key());
        assert!(config.verify().is_ok());
        let station = SigningKey::from_bytes(&[0x5A; 32]).verifying_key();
        assert!(config.station(&station).is_some());
        let stranger = SigningKey::from_bytes(&[0x0B; 32]).verifying_key();
        assert!(config.station(&stranger).is_none());
    }

    #[test]
    fn tampered_config_fails_verification() {
        let mut config = roster(1).sign(&fleet_key());
        config.body.revision = 999; // not what was signed
        assert!(matches!(config.verify(), Err(FleetError::BadSignature)));
    }

    #[test]
    fn config_signed_by_the_wrong_key_fails() {
        let imposter = SigningKey::from_bytes(&[0x00; 32]);
        // Body claims the real fleet id, but is signed by an imposter.
        let config = roster(1).sign(&imposter);
        assert!(matches!(config.verify(), Err(FleetError::BadSignature)));
    }

    #[test]
    fn rollback_is_rejected() {
        let config = roster(2).sign(&fleet_key());
        assert!(config.verify_against(2).is_ok());
        let stale = roster(1).sign(&fleet_key());
        assert!(matches!(
            stale.verify_against(2),
            Err(FleetError::Rollback { got: 1, trusted: 2 })
        ));
    }
}
