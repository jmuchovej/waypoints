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
//! One Station homes **many** Territories (§9): all Stations in a Fleet
//! replicate all its Territories (`Everything`, §9). Territories are learned
//! over the wire — a Vessel [`Register`](outpost_protocol::sync::SyncMessage::Register)s
//! one, presenting its root ACL record, and the Station bootstraps a fresh
//! per-Territory hold from it under its storage directory. A Station opened
//! with a storage directory ([`hosting`](Self::hosting)) persists that set as
//! the holds on disk and reloads them on restart; one seeded with a fixed
//! replica ([`new`](Self::new)) serves just that Territory and registers
//! nothing new (tests, single-Territory embeds).
//!
//! What a Station does not do — originate edits, hold read keys, materialize
//! a library — is the Vessel's or nobody's (§3 rules 4–5).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::replica::{Replica, ReplicaError};
use ed25519_dalek::SigningKey;
use outpost_fabric::{Endpoint, Session, TransportError, read_frame, serve, write_frame};
use outpost_hold::TerritoryHold;
use outpost_protocol::TerritoryID;
use outpost_protocol::acl::ACLRecord;
use outpost_protocol::sync::SyncMessage;

pub use outpost_fabric::fleet;

#[derive(Debug, thiserror::Error)]
pub enum StationError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Replica(#[from] ReplicaError),
    #[error(transparent)]
    Hold(#[from] outpost_hold::HoldError),
    #[error(transparent)]
    Wire(#[from] outpost_protocol::WireError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("station storage path is not valid utf-8")]
    BadPath,
}

/// A running Station: an iroh endpoint plus the Territories it homes, each a
/// shared [`Replica`]. Storage and serving are always on; the **orderer**
/// role (§8.4) is present only when a fleet signing key is supplied
/// ([`orderer`](Self::orderer)/[`hosting_orderer`](Self::hosting_orderer)) —
/// that key is the sole secret a Station carries, and it can neither read
/// payloads nor author records with it. The blob role layers on later (§10.3).
pub struct Station {
    endpoint: Endpoint,
    /// The fleet signing key, for the orderer role. `None` on a
    /// storage-only Station.
    fleet_key: Option<SigningKey>,
    /// Where new Territories' holds are created, when this Station accepts
    /// registrations. `None` for a Station seeded with a fixed replica set —
    /// it serves what it was given and bootstraps nothing new.
    storage: Option<PathBuf>,
    /// The Territories this Station homes, keyed by id (§9). Populated by the
    /// storage scan at startup and grown by [`Register`] over the wire.
    ///
    /// [`Register`]: outpost_protocol::sync::SyncMessage::Register
    territories: Mutex<BTreeMap<TerritoryID, Arc<Replica>>>,
}

impl Station {
    /// Bind a storage-only Station on `secret` (its roster peer key, §10.2)
    /// seeded with a single `replica` — it serves that one Territory and
    /// accepts no new registrations. For tests and single-Territory embeds;
    /// a self-host Station uses [`hosting`](Self::hosting).
    pub async fn new(secret: &SigningKey, replica: Replica) -> Result<Self, StationError> {
        Self::build(secret, None, None, vec![replica]).await
    }

    /// [`new`](Self::new) that also runs the per-Fleet **orderer** (§8.4),
    /// holding `fleet_key` to countersign ACL submissions.
    pub async fn orderer(
        secret: &SigningKey,
        fleet_key: SigningKey,
        replica: Replica,
    ) -> Result<Self, StationError> {
        Self::build(secret, Some(fleet_key), None, vec![replica]).await
    }

    /// Bind a Station that homes many Territories under `storage` (a
    /// directory): it reloads the holds already there and accepts new
    /// [`Register`] requests, bootstrapping a fresh hold per Territory.
    ///
    /// [`Register`]: outpost_protocol::sync::SyncMessage::Register
    pub async fn hosting(secret: &SigningKey, storage: &Path) -> Result<Self, StationError> {
        Self::build(secret, None, Some(storage.to_owned()), Vec::new()).await
    }

    /// [`hosting`](Self::hosting) that also runs the per-Fleet **orderer**.
    pub async fn hosting_orderer(
        secret: &SigningKey,
        fleet_key: SigningKey,
        storage: &Path,
    ) -> Result<Self, StationError> {
        Self::build(
            secret,
            Some(fleet_key),
            Some(storage.to_owned()),
            Vec::new(),
        )
        .await
    }

    async fn build(
        secret: &SigningKey,
        fleet_key: Option<SigningKey>,
        storage: Option<PathBuf>,
        seed: Vec<Replica>,
    ) -> Result<Self, StationError> {
        let endpoint = Endpoint::bind_identity(secret).await?;
        let mut territories = BTreeMap::new();
        if let Some(dir) = &storage {
            std::fs::create_dir_all(dir)?;
            for replica in Self::scan_holds(dir).await? {
                territories.insert(replica.territory_id().await?, Arc::new(replica));
            }
        }
        for replica in seed {
            territories.insert(replica.territory_id().await?, Arc::new(replica));
        }
        Ok(Self {
            endpoint,
            fleet_key,
            storage,
            territories: Mutex::new(territories),
        })
    }

    /// Load a Replica for every Territory hold already in `dir` (§9 restart
    /// path). Holds are named `<territory-hex>.db`; files that do not parse as
    /// a Territory id are skipped, so an unrelated file is harmless.
    async fn scan_holds(dir: &Path) -> Result<Vec<Replica>, StationError> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue; // skip -wal/-shm and non-db files
            }
            let Some(tid) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(territory_from_hex)
            else {
                continue; // not a Territory hold
            };
            let path = path.to_str().ok_or(StationError::BadPath)?;
            let replica = Replica::reopen(TerritoryHold::open(path).await?).await?;
            debug_assert_eq!(replica.territory_id().await?, tid);
            out.push(replica);
        }
        Ok(out)
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The sole replica of a single-Territory Station — a convenience for
    /// tests and single-Territory embeds. Panics if the Station serves other
    /// than exactly one Territory; a many-Territory Station has no "the"
    /// replica ([`replica_for`](Self::replica_for)).
    pub fn replica(&self) -> Arc<Replica> {
        let map = self.territories.lock().unwrap();
        assert_eq!(
            map.len(),
            1,
            "Station::replica() on a Station serving {} Territories",
            map.len()
        );
        map.values().next().unwrap().clone()
    }

    /// The replica for `territory`, if this Station homes it.
    pub fn replica_for(&self, territory: &TerritoryID) -> Option<Arc<Replica>> {
        self.territories.lock().unwrap().get(territory).cloned()
    }

    /// Whether this Station homes `territory` (§9/§13).
    pub fn serves(&self, territory: &TerritoryID) -> bool {
        self.territories.lock().unwrap().contains_key(territory)
    }

    /// Home a Territory from its root ACL record (§13): idempotent if already
    /// served; otherwise verify the id is the root's hash and — if this
    /// Station has a storage directory — bootstrap a fresh hold from the root.
    /// Returns whether the Station now serves it.
    async fn register(
        &self,
        territory: TerritoryID,
        root: ACLRecord,
    ) -> Result<bool, StationError> {
        if self.serves(&territory) {
            return Ok(true);
        }
        if TerritoryID::of_root(&root)? != territory {
            return Ok(false); // the id must be the root's hash (§9)
        }
        let Some(dir) = &self.storage else {
            return Ok(false); // a fixed-replica Station bootstraps nothing new
        };
        let path = dir.join(format!("{}.db", hex::encode(territory.as_bytes())));
        let path = path.to_str().ok_or(StationError::BadPath)?;
        let hold = TerritoryHold::open(path).await?;
        if hold.acl_records().await?.is_empty() {
            hold.append_acl(&root, 0, None).await?;
        }
        let replica = Replica::reopen(hold).await?;
        self.territories
            .lock()
            .unwrap()
            .entry(territory)
            .or_insert_with(|| Arc::new(replica));
        Ok(true)
    }

    /// Accept one inbound session and handle it: an optional [`Hello`]
    /// handshake (§10.2) verifying which identity is behind the device
    /// connection, then a request — register a Territory, open a scoped sync
    /// session, or order an ACL submission. A Station loops this; tests drive
    /// a single exchange.
    ///
    /// [`Hello`]: outpost_protocol::sync::SyncMessage::Hello
    pub async fn serve_once(&self) -> Result<(), StationError> {
        let Some(mut session) = self.endpoint.accept().await? else {
            return Ok(());
        };
        let first = read_frame::<SyncMessage>(&mut session.recv).await;

        // Optional handshake: verify the cert binds the connection's device
        // key to the claimed identity (§10.2). A Vessel and a Station never
        // implicitly trust each other; this is how the Station learns who is
        // calling. On failure, ack false and stop.
        let request = match first {
            Ok(SyncMessage::Hello { identity, cert }) => {
                let ok = cert.verify(&identity).is_ok() && cert.device == session.remote_key();
                let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok }).await;
                if !ok {
                    session.drain().await;
                    return Ok(());
                }
                read_frame::<SyncMessage>(&mut session.recv).await
            }
            other => other,
        };

        self.dispatch(request, &mut session).await;
        session.drain().await;
        Ok(())
    }

    async fn dispatch(
        &self,
        request: Result<SyncMessage, outpost_fabric::FrameError>,
        session: &mut Session,
    ) {
        match request {
            Ok(SyncMessage::Register { territory, root }) => {
                let ok = match self.register(territory, root).await {
                    Ok(ok) => ok,
                    Err(e) => {
                        tracing::warn!("station: registration failed: {e}");
                        false
                    }
                };
                let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok }).await;
            }
            Ok(SyncMessage::Open { territory }) => match self.replica_for(&territory) {
                Some(replica) => {
                    let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: true }).await;
                    if serve(&*replica, &mut session.send, &mut session.recv)
                        .await
                        .is_err()
                    {
                        tracing::warn!("station session ended early");
                    }
                }
                None => {
                    let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: false }).await;
                }
            },
            Ok(SyncMessage::OrderRequest { territory, record }) => {
                // Order the submission (§8.4): assign a position and
                // countersign — only possible if we hold the fleet key and
                // home the Territory.
                let proof = match (self.replica_for(&territory), &self.fleet_key) {
                    (Some(replica), Some(key)) => replica.order(key, &record).await.ok().flatten(),
                    _ => None,
                };
                let _ = write_frame(&mut session.send, &SyncMessage::OrderResponse { proof }).await;
            }
            _ => tracing::warn!("station: unexpected request frame"),
        }
    }
}

/// Decode a `<territory-hex>` hold filename stem back to a [`TerritoryID`].
fn territory_from_hex(stem: &str) -> Option<TerritoryID> {
    let bytes: [u8; 32] = hex::decode(stem).ok()?.try_into().ok()?;
    Some(TerritoryID::from_bytes(bytes))
}
