//! The Station role (design doc §12): an always-online peer that validates,
//! stores, serves sync, and orders ACL submissions — and never decrypts.
//!
//! A Station is content-blind, and **this module contains no key-custody
//! and no decryption code** — the discipline that made it its own crate.
//! Now that the roles share the `outpost` crate (which the Vessel's
//! decryption lives in too), that guarantee is a convention held in review
//! rather than a crate boundary; keep it. The Station composes the shared
//! [`Replica`](crate::replica::Replica) (validate + store + answer sync)
//! with two Station-only duties: the per-Fleet **orderer** (§8.4), which
//! assigns ACL records a total order and countersigns the position with the
//! fleet key, and the **serve loop**, which accepts inbound sessions and
//! drives [`serve`](outpost_fabric::serve).
//!
//! What a Station does not do — originate edits, hold read keys, materialize
//! a library — is the Vessel's or nobody's (§3 rules 4–5).

use crate::replica::{Replica, ReplicaError};
use ed25519_dalek::SigningKey;
use outpost_fabric::{Endpoint, TransportError, serve};
use outpost_protocol::RecordID;
use outpost_protocol::acl::{ACLRecord, OrderProof};

pub use outpost_fabric::fleet;

#[derive(Debug, thiserror::Error)]
pub enum StationError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Replica(#[from] ReplicaError),
}

/// The per-Fleet orderer (§8.4): assigns each ACL record its position and
/// countersigns `(record, position)` with the fleet key. Holds the fleet
/// *signing* key — the only secret a Station carries, and it can neither
/// read payloads nor author records with it.
pub struct Orderer {
    fleet_key: SigningKey,
    next_position: u64,
}

impl Orderer {
    pub fn new(fleet_key: SigningKey) -> Self {
        Self {
            fleet_key,
            next_position: 0,
        }
    }

    /// Resume an orderer whose log already has `assigned` records.
    pub fn resume(fleet_key: SigningKey, assigned: u64) -> Self {
        Self {
            fleet_key,
            next_position: assigned,
        }
    }

    /// Assign the next position to `record` and countersign it. The caller
    /// has already checked chaining against the current head; the orderer
    /// only sequences (it cannot author or read, §8.4).
    pub fn order(&mut self, record: RecordID) -> OrderProof {
        let position = self.next_position;
        self.next_position += 1;
        OrderProof::sign(&self.fleet_key, record, position)
    }

    pub fn next_position(&self) -> u64 {
        self.next_position
    }
}

/// A running Station: an iroh endpoint plus the shared replica. Storage and
/// orderer roles compose here; the blob role layers on later (§10.3).
pub struct Station {
    endpoint: Endpoint,
    replica: Replica,
}

impl Station {
    /// Bind a Station on `secret` (its roster peer key, §10.2) serving
    /// `replica`.
    pub async fn new(secret: &SigningKey, replica: Replica) -> Result<Self, StationError> {
        let endpoint = Endpoint::bind_identity(secret).await?;
        Ok(Self { endpoint, replica })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// Accept and serve exactly one inbound session, then return. A Station
    /// loops this; tests drive a single exchange. In-flight frames are
    /// drained before the connection closes.
    pub async fn serve_once(&self) -> Result<(), StationError> {
        let Some(mut session) = self.endpoint.accept().await? else {
            return Ok(());
        };
        // A Vessel and a Station never implicitly trust each other; the
        // authenticated `session.remote_key()` is what a roster admission
        // check keys on (§10.2). Admission policy layers on here.
        if serve(&self.replica, &mut session.send, &mut session.recv)
            .await
            .is_err()
        {
            tracing::warn!("station session ended early");
        }
        session.drain().await;
        Ok(())
    }
}

/// Bootstrap a Station replica from an ACL log (thin helper mirroring the
/// Vessel's).
pub async fn open_replica(path: &str, acl_log: &[ACLRecord]) -> Result<Replica, StationError> {
    Ok(Replica::bootstrap(path, acl_log).await?)
}
