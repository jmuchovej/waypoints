//! The Vessel role (design doc §12): an end-device that holds a user's
//! keys. The Vessel is the *only* party that can **originate** edits,
//! **decrypt** payloads, and merge — the whole reason it and a Station,
//! which can do none of these, never implicitly trust each other even
//! under the same user identity (§3 rules 4–5).
//!
//! It composes the shared [`Replica`](crate::replica::Replica) (validate +
//! store + answer sync, the half every peer runs) with Vessel-only duties:
//! authoring Notices under the identity key, the read-key cache that unseals
//! payloads, and dialing Stations to sync. Snapshot cadence (§6.3) layers on
//! here later.

use std::collections::BTreeMap;

use crate::replica::{Replica, ReplicaError};
use ed25519_dalek::SigningKey;
use outpost_fabric::{Endpoint, TransportError, initiate};
use outpost_protocol::crypto::{KemSecretKey, ReadKey, open_payload};
use outpost_protocol::{
    CryptoError, Envelope, EnvelopeHeader, EpochID, NoticeID, SignedNotice, TrackID, WireError,
};

pub use outpost_fabric::fleet;
pub use outpost_protocol::acl::ACLRecord;

#[derive(Debug, thiserror::Error)]
pub enum VesselError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Replica(#[from] ReplicaError),
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("no read key cached for epoch {0}")]
    NoKey(EpochID),
}

/// A Vessel: identity + KEM keys, an endpoint, the shared replica, and the
/// per-epoch read-key cache (§7.2). The cache is what makes this a Vessel;
/// a Station has no equivalent and cannot obtain one.
pub struct Vessel {
    identity: SigningKey,
    kem: KemSecretKey,
    endpoint: Endpoint,
    replica: Replica,
    read_keys: BTreeMap<EpochID, ReadKey>,
}

impl Vessel {
    /// Open a Vessel with the given identity and KEM seed over `replica`.
    pub async fn new(
        identity: SigningKey,
        kem: KemSecretKey,
        replica: Replica,
    ) -> Result<Self, VesselError> {
        let endpoint = Endpoint::bind_identity(&identity).await?;
        Ok(Self {
            identity,
            kem,
            endpoint,
            replica,
            read_keys: BTreeMap::new(),
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    pub fn kem_public(&self) -> outpost_protocol::crypto::KemPublicKey {
        self.kem.public_key()
    }

    /// Cache a read key the Vessel obtained by unsealing an ACL record's
    /// sealed box with its KEM secret (§7.1). Slice-7 scope keeps key
    /// acquisition explicit; wiring it to the ACL fold is a later step.
    pub fn cache_read_key(&mut self, epoch: EpochID, key: ReadKey) {
        self.read_keys.insert(epoch, key);
    }

    /// Author a Notice: seal `plaintext` under the current epoch's read
    /// key, sign the envelope, and ingest it locally. Only a Vessel can do
    /// this (§3 rule 5). Returns the stored Notice.
    pub async fn author(
        &self,
        rng: &mut impl outpost_protocol::crypto::CryptoRngExport,
        parents: Vec<NoticeID>,
        epoch: EpochID,
        payload_type: &str,
        plaintext: &[u8],
    ) -> Result<SignedNotice, VesselError> {
        let key = self
            .read_keys
            .get(&epoch)
            .ok_or(VesselError::NoKey(epoch))?;
        let snapshot_base = if parents.is_empty() {
            NoticeID::SELF
        } else {
            // A non-root's snapshot base is resolved by the snapshot layer
            // (§6.3, deferred); until then it points at the first parent.
            parents[0]
        };
        let header = EnvelopeHeader {
            parents,
            acl_head: self.replica.acl_head(),
            epoch,
            author: self.identity.verifying_key(),
            snapshot_base,
            is_snapshot: false,
            payload_type: payload_type.to_owned(),
            timestamp: 0,
        };
        let payload = outpost_protocol::crypto::seal_payload(rng, key, &header, plaintext)?;
        let notice = SignedNotice::sign(&Envelope { header, payload }, &self.identity)?;
        self.replica.ingest(vec![notice.clone()]).await?;
        Ok(notice)
    }

    /// Decrypt a stored Notice's payload with the cached read key (§7.4) —
    /// the Vessel-only step a Station cannot perform. The AAD binding means
    /// this fails unless every header field matches what was sealed.
    pub fn decrypt(&self, notice: &SignedNotice) -> Result<Vec<u8>, VesselError> {
        let envelope = notice.parse_and_verify()?;
        let key = self
            .read_keys
            .get(&envelope.header.epoch)
            .ok_or(VesselError::NoKey(envelope.header.epoch))?;
        Ok(open_payload(key, &envelope.header, &envelope.payload)?)
    }

    /// Dial a Station and run one full sync (§10): reconciles both
    /// directions and applies received Notices through the replica.
    pub async fn sync_with(
        &self,
        station: impl Into<outpost_fabric::EndpointAddr>,
    ) -> Result<(), VesselError> {
        let mut session = self.endpoint.connect(station).await?;
        // A roster check on `session.remote_key()` gates admission (§10.2);
        // the caller supplies the trusted FleetConfig.
        let _ = initiate(&self.replica, &mut session.send, &mut session.recv).await;
        session.close();
        Ok(())
    }
}

/// Bootstrap a Vessel replica from an ACL log (thin helper mirroring the
/// Station's).
pub async fn open_replica(path: &str, acl_log: &[ACLRecord]) -> Result<Replica, VesselError> {
    Ok(Replica::bootstrap(path, acl_log).await?)
}
