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
use outpost_fabric::{Endpoint, TransportError, read_frame, serve_sync, write_frame};
use outpost_protocol::acl::ACLRecord;
use outpost_protocol::sync::SyncMessage;

pub use outpost_fabric::fleet;

#[derive(Debug, thiserror::Error)]
pub enum StationError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Replica(#[from] ReplicaError),
}

/// A running Station: an iroh endpoint plus the shared replica. Storage and
/// serving are always on; the **orderer** role (§8.4) is present only when a
/// fleet signing key is supplied ([`orderer`](Self::orderer)) — that key is
/// the sole secret a Station carries, and it can neither read payloads nor
/// author records with it. The blob role layers on later (§10.3).
pub struct Station {
    endpoint: Endpoint,
    replica: Replica,
    /// The fleet signing key, for the orderer role. `None` on a
    /// storage-only Station.
    fleet_key: Option<SigningKey>,
}

impl Station {
    /// Bind a storage-only Station on `secret` (its roster peer key, §10.2)
    /// serving `replica`.
    pub async fn new(secret: &SigningKey, replica: Replica) -> Result<Self, StationError> {
        let endpoint = Endpoint::bind_identity(secret).await?;
        Ok(Self {
            endpoint,
            replica,
            fleet_key: None,
        })
    }

    /// Bind a Station that also runs the per-Fleet **orderer** (§8.4),
    /// holding `fleet_key` to countersign ACL submissions.
    pub async fn orderer(
        secret: &SigningKey,
        fleet_key: SigningKey,
        replica: Replica,
    ) -> Result<Self, StationError> {
        let endpoint = Endpoint::bind_identity(secret).await?;
        Ok(Self {
            endpoint,
            replica,
            fleet_key: Some(fleet_key),
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// Accept one inbound session, dispatch on its first frame — an ACL
    /// submission to the orderer, otherwise the sync flow — then return. A
    /// Station loops this; tests drive a single exchange.
    pub async fn serve_once(&self) -> Result<(), StationError> {
        let Some(mut session) = self.endpoint.accept().await? else {
            return Ok(());
        };
        // A Vessel and a Station never implicitly trust each other; the
        // authenticated `session.remote_key()` is what a roster admission
        // check keys on (§10.2). Admission policy layers on here.
        match read_frame::<SyncMessage>(&mut session.recv).await {
            Ok(SyncMessage::OrderRequest { record }) => {
                // Order the submission (§8.4): assign a position and
                // countersign — only possible if we hold the fleet key.
                let proof = match &self.fleet_key {
                    Some(key) => self.replica.order(key, &record).await.ok().flatten(),
                    None => None,
                };
                let _ = write_frame(&mut session.send, &SyncMessage::OrderResponse { proof }).await;
            }
            Ok(SyncMessage::AclRequest { len }) => {
                if serve_sync(&self.replica, len, &mut session.send, &mut session.recv)
                    .await
                    .is_err()
                {
                    tracing::warn!("station session ended early");
                }
            }
            _ => tracing::warn!("station: unexpected leading frame"),
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
