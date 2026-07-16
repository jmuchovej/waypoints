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
use std::time::{SystemTime, UNIX_EPOCH};

use crate::replica::{Replica, ReplicaError};
use ed25519_dalek::{SigningKey, VerifyingKey};
use outpost_fabric::{Endpoint, Session, TransportError, read_frame, serve, write_frame};
use outpost_hold::{StationHold, TerritoryHold};
use outpost_protocol::acl::ACLRecord;
use outpost_protocol::auth::DeviceCert;
use outpost_protocol::sync::{DeviceSighting, SyncMessage};
use outpost_protocol::{MemberID, TerritoryID};

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
    /// Device sightings (§4): per Territory, which certified devices of which
    /// identities have touched it, and when. Attribution the Station already
    /// observes in the [`Hello`] handshake, made queryable
    /// ([`DevicesRequest`]) — never consulted for an authorization decision.
    /// The in-memory map is the read path; a storage-backed Station loads it
    /// from `hold` at start and writes sightings through, so a restart does
    /// not forget a dormant device.
    ///
    /// [`Hello`]: outpost_protocol::sync::SyncMessage::Hello
    /// [`DevicesRequest`]: outpost_protocol::sync::SyncMessage::DevicesRequest
    sightings: Mutex<BTreeMap<TerritoryID, Sightings>>,
    /// The Station's own database (`station.db` in the storage dir): persisted
    /// sightings + registrations, so a restart does not make a dormant device
    /// vanish from the registry. `None` on a storage-less seeded Station —
    /// embeds and tests stay memory-only.
    hold: Option<StationHold>,
}

/// One Territory's sightings, keyed by `(member, device key bytes)` — one
/// entry per certified device per identity.
type Sightings = BTreeMap<(MemberID, [u8; 32]), DeviceSighting>;

/// The authenticated caller of a session, as proven by its [`Hello`]
/// handshake (§10.2): the identity key and the cert binding the connection's
/// device key to it.
///
/// [`Hello`]: outpost_protocol::sync::SyncMessage::Hello
#[derive(Clone)]
struct Caller {
    identity: VerifyingKey,
    cert: DeviceCert,
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
        let mut hold = None;
        let mut sightings: BTreeMap<TerritoryID, Sightings> = BTreeMap::new();
        if let Some(dir) = &storage {
            std::fs::create_dir_all(dir)?;
            for replica in Self::scan_holds(dir).await? {
                territories.insert(replica.territory_id().await?, Arc::new(replica));
            }
            // The Station's own database, beside the Territory holds. Reload
            // persisted sightings — `connected` comes back `false` (nobody is
            // live right after a start).
            let station_db = dir.join("station.db");
            let station_hold =
                StationHold::open(station_db.to_str().ok_or(StationError::BadPath)?).await?;
            for (territory, sighting) in station_hold.sightings().await? {
                let key = (
                    MemberID::from(&sighting.identity),
                    sighting.cert.device.to_bytes(),
                );
                sightings
                    .entry(territory)
                    .or_default()
                    .insert(key, sighting);
            }
            hold = Some(station_hold);
        }
        for replica in seed {
            territories.insert(replica.territory_id().await?, Arc::new(replica));
        }
        Ok(Self {
            endpoint,
            fleet_key,
            storage,
            territories: Mutex::new(territories),
            sightings: Mutex::new(sightings),
            hold,
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
    /// A first registration is remembered in the Station database (who, when)
    /// when the registrant authenticated. Returns whether the Station now
    /// serves it.
    async fn register(
        &self,
        territory: TerritoryID,
        root: ACLRecord,
        caller: Option<&Caller>,
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
        if let (Some(station_hold), Some(caller)) = (&self.hold, caller)
            && let Err(e) = station_hold
                .record_registration(
                    &territory,
                    &MemberID::from(&caller.identity),
                    unix_now() as i64,
                )
                .await
        {
            tracing::warn!("station: registration write failed: {e}");
        }
        Ok(true)
    }

    /// Who first homed `territory` on this Station, and when (seconds since
    /// the Unix epoch) — `None` on a storage-less Station or if it was never
    /// registered here (e.g. seeded, or registered by an unauthenticated
    /// caller).
    pub async fn registration(&self, territory: &TerritoryID) -> Option<(MemberID, u64)> {
        let hold = self.hold.as_ref()?;
        let row = hold.registration(territory).await.ok()??;
        Some((
            MemberID::from_bytes(row.registered_by),
            row.registered_at as u64,
        ))
    }

    /// Accept one inbound session and handle it: an optional [`Hello`]
    /// handshake (§10.2) verifying which identity is behind the device
    /// connection, then a request — register a Territory, open a scoped sync
    /// session, order an ACL submission, or list device sightings. A Station
    /// loops this; tests drive a single exchange.
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
        let (request, caller) = match first {
            Ok(SyncMessage::Hello { identity, cert }) => {
                let ok = cert.verify(&identity).is_ok() && cert.device == session.remote_key();
                let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok }).await;
                if !ok {
                    session.drain().await;
                    return Ok(());
                }
                let caller = Caller { identity, cert };
                (
                    read_frame::<SyncMessage>(&mut session.recv).await,
                    Some(caller),
                )
            }
            other => (other, None),
        };

        self.dispatch(request, caller.as_ref(), &mut session).await;
        session.drain().await;
        // The session is over: whatever this device touched, it is no longer
        // live on it (§4 connection status).
        if let Some(caller) = &caller {
            self.mark_disconnected(caller);
        }
        Ok(())
    }

    async fn dispatch(
        &self,
        request: Result<SyncMessage, outpost_fabric::FrameError>,
        caller: Option<&Caller>,
        session: &mut Session,
    ) {
        match request {
            Ok(SyncMessage::Register { territory, root }) => {
                let ok = match self.register(territory, root, caller).await {
                    Ok(ok) => ok,
                    Err(e) => {
                        tracing::warn!("station: registration failed: {e}");
                        false
                    }
                };
                if ok && let Some(caller) = caller {
                    self.record_sighting(territory, caller).await;
                }
                let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok }).await;
            }
            Ok(SyncMessage::Open { territory }) => match self.replica_for(&territory) {
                Some(replica) => {
                    if let Some(caller) = caller {
                        self.record_sighting(territory, caller).await;
                    }
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
            Ok(SyncMessage::DevicesRequest { territory }) => {
                // Viewing is member-gated (§4): only an authenticated caller
                // whose identity the Territory's ACL admits may list. Being
                // *sighted* required no membership; *seeing* does.
                let allowed = match caller {
                    Some(caller) => self.is_member(&territory, &caller.identity).await,
                    None => false,
                };
                let reply = if allowed {
                    SyncMessage::DevicesResponse {
                        devices: self.sightings_for(&territory),
                    }
                } else {
                    SyncMessage::Ack { ok: false }
                };
                let _ = write_frame(&mut session.send, &reply).await;
            }
            _ => tracing::warn!("station: unexpected request frame"),
        }
    }

    // ---- device sightings (§4: visibility + attribution) -----------------

    /// Record that `caller`'s device touched `territory` now, marking it
    /// live. First sight inserts; a re-sight refreshes `last_seen`. Written
    /// through to the Station database when one is open — a failed write is
    /// logged, not fatal (presence metadata; the in-memory registry is the
    /// read path either way).
    async fn record_sighting(&self, territory: TerritoryID, caller: &Caller) {
        let now = unix_now();
        let key = (
            MemberID::from(&caller.identity),
            caller.cert.device.to_bytes(),
        );
        let persisted = {
            let mut sightings = self.sightings.lock().unwrap();
            sightings
                .entry(territory)
                .or_default()
                .entry(key)
                .and_modify(|s| {
                    s.last_seen = now;
                    s.connected = true;
                })
                .or_insert_with(|| DeviceSighting {
                    identity: caller.identity,
                    cert: caller.cert.clone(),
                    first_seen: now,
                    last_seen: now,
                    connected: true,
                })
                .clone()
        };
        if let Some(hold) = &self.hold
            && let Err(e) = hold.upsert_sighting(&territory, &persisted).await
        {
            tracing::warn!("station: sighting write-through failed: {e}");
        }
    }

    /// The session ended — the caller's device is no longer live anywhere.
    fn mark_disconnected(&self, caller: &Caller) {
        let key = (
            MemberID::from(&caller.identity),
            caller.cert.device.to_bytes(),
        );
        let mut sightings = self.sightings.lock().unwrap();
        for by_device in sightings.values_mut() {
            if let Some(s) = by_device.get_mut(&key) {
                s.connected = false;
            }
        }
    }

    /// The sightings for one Territory. Also the local (non-wire) read used
    /// by a self-host operator or tests.
    pub fn sightings_for(&self, territory: &TerritoryID) -> Vec<DeviceSighting> {
        self.sightings
            .lock()
            .unwrap()
            .get(territory)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Whether `identity` is a member of `territory` per its folded ACL.
    async fn is_member(&self, territory: &TerritoryID, identity: &VerifyingKey) -> bool {
        let Some(replica) = self.replica_for(territory) else {
            return false;
        };
        let Ok(acl) = replica.acl_index().await else {
            return false;
        };
        acl.head_state()
            .is_some_and(|s| s.members.contains_key(&MemberID::from(identity)))
    }
}

/// Seconds since the Unix epoch, by the Station's clock — presence metadata,
/// the same trust class as everything else a Station observes.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Decode a `<territory-hex>` hold filename stem back to a [`TerritoryID`].
fn territory_from_hex(stem: &str) -> Option<TerritoryID> {
    let bytes: [u8; 32] = hex::decode(stem).ok()?.try_into().ok()?;
    Some(TerritoryID::from_bytes(bytes))
}
