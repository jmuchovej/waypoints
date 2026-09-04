//! The Station role: an always-online peer that validates,
//! stores, serves sync, and orders ACL submissions — and never decrypts.
//!
//! A Station is content-blind, and **this module contains no key-custody
//! and no decryption code** — the discipline that made it its own crate.
//! Now that the roles share the `outpost` crate (which the Vessel's
//! decryption lives in too), that guarantee is a convention held in review
//! rather than a crate boundary; keep it. The Station composes the shared
//! [`Replica`](crate::replica::Replica) (validate + store + answer sync)
//! with two Station-only duties: the per-Fleet **orderer**, which
//! assigns ACL records a total order and countersigns the position with the
//! fleet key, and the **serve loop**, which accepts inbound sessions and
//! drives [`serve`](crate::fabric::serve).
//!
//! One Station homes **many** Territories: all Stations in a Fleet
//! replicate all its Territories (`Everything`). Territories are learned
//! over the wire — a Vessel [`Register`](crate::protocol::SyncMessage::Register)s
//! one, presenting its root ACL record, and the Station bootstraps a fresh
//! per-Territory hold from it under its storage directory. A Station opened
//! with a storage directory ([`hosting`](Self::hosting)) persists that set as
//! the holds on disk and reloads them on restart; one seeded with a fixed
//! replica ([`new`](Self::new)) serves just that Territory and registers
//! nothing new (tests, single-Territory embeds).
//!
//! What a Station does not do — originate edits, hold read keys, materialize
//! a library — is the Vessel's or nobody's.

use crate::{
    fabric::{Endpoint, Session, SyncPeer, read_frame, serve_gated, write_frame},
    hold::{ACLIndex, StationHold, TerritoryHold},
    protocol::{
        ACLRecord, DeviceCert, DeviceSighting, MemberID, NoticeID, OrderedRecord, Permissions,
        SignedNotice, SyncMessage, TallyDigest, TerritoryID, TrackID,
    },
    replica::Replica,
};
use async_trait::async_trait;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;

type StationResult<T> = ::std::result::Result<T, StationError>;

#[derive(Debug, thiserror::Error)]
pub enum StationError {
    #[error(transparent)]
    Transport(#[from] crate::fabric::TransportError),
    #[error(transparent)]
    Replica(#[from] crate::replica::ReplicaError),
    #[error(transparent)]
    Hold(#[from] crate::hold::HoldError),
    #[error(transparent)]
    Wire(#[from] crate::protocol::WireError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("station storage path is not valid utf-8")]
    BadPath,
}

/// A running Station: an iroh endpoint plus the Territories it homes, each a
/// shared [`Replica`]. Storage and serving are always on; the **orderer**
/// role is present only when a fleet signing key is supplied
/// ([`orderer`](Self::orderer)/[`hosting_orderer`](Self::hosting_orderer)) —
/// that key is the sole secret a Station carries, and it can neither read
/// payloads nor author records with it. The blob role layers on later.
pub struct Station {
    endpoint: Endpoint,
    /// The fleet signing key, for the orderer role. `None` on a
    /// storage-only Station.
    fleet_key: Option<SigningKey>,
    /// Where new Territories' holds are created, when this Station accepts
    /// registrations. `None` for a Station seeded with a fixed replica set —
    /// it serves what it was given and bootstraps nothing new.
    storage: Option<PathBuf>,
    /// The Territories this Station homes, keyed by id. Populated by the
    /// storage scan at startup and grown by [`Register`] over the wire.
    ///
    /// [`Register`]: crate::protocol::SyncMessage::Register
    territories: Mutex<BTreeMap<TerritoryID, Arc<Replica>>>,
    /// Device sightings: per Territory, which certified devices of which
    /// identities have touched it, and when. Attribution the Station already
    /// observes in the [`Hello`] handshake, made queryable
    /// ([`DevicesRequest`]) — never consulted for an authorization decision.
    /// The in-memory map is the read path; a storage-backed Station loads it
    /// from `hold` at start and writes sightings through, so a restart does
    /// not forget a dormant device.
    ///
    /// [`Hello`]: crate::protocol::SyncMessage::Hello
    /// [`DevicesRequest`]: crate::protocol::SyncMessage::DevicesRequest
    sightings: Mutex<BTreeMap<TerritoryID, Sightings>>,
    /// The Station's own database (`station.db` in the storage dir): persisted
    /// sightings + registrations, so a restart does not make a dormant device
    /// vanish from the registry. `None` on a storage-less seeded Station —
    /// embeds and tests stay memory-only.
    hold: Option<StationHold>,
    register_policy: RegisterPolicy,
    sync_policy: SyncPolicy,
    /// Per-Territory live-push bus: everything a session
    /// newly accepts is published here and fanned out to that Territory's
    /// subscribers (minus the source session, so nothing echoes).
    buses: Mutex<BTreeMap<TerritoryID, broadcast::Sender<BusEvent>>>,
    /// Monotonic session ids — the echo-suppression key on [`BusEvent`].
    next_session: AtomicU64,
    /// Operator-configured sockets to advertise to Vessels
    /// ([`with_advertised`](Self::with_advertised)). Empty ⇒
    /// [`advertised_addr`](Self::advertised_addr) falls back to the bound
    /// sockets — the same-host posture.
    advertise: Vec<std::net::SocketAddr>,
}

/// One batch of freshly-accepted records on a Territory's bus, tagged with
/// the session that brought them in.
#[derive(Clone)]
struct BusEvent {
    source: u64,
    push: BusPush,
}

#[derive(Clone)]
enum BusPush {
    Notices(Vec<SignedNotice>),
    #[allow(clippy::upper_case_acronyms)]
    ACL(Vec<OrderedRecord>),
}

/// Who may home a **new** Territory on this Station.
/// Re-registering an already-served Territory is an idempotent no-op either
/// way; members and secondary devices never register at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterPolicy {
    /// The authenticated registrant must hold **Owner** permission in the
    /// presented ACL log's fold. The root author is the degenerate case (the
    /// root makes its author Owner); a later-promoted Owner presents the log
    /// that promoted them and passes identically — authority comes from the
    /// ACL, not from who created the Territory.
    #[default]
    Owner,
    /// Any vessel presenting a valid log may register (tests, throwaway dev).
    Anyone,
}

/// Who may `Open`/sync a Territory this Station homes. Extensible
/// (`#[non_exhaustive]`) — a roster-peer arm lands with station↔station
/// replication.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyncPolicy {
    /// Requires an authenticated [`Hello`]. The **ACL phase is served to any
    /// authenticated identity** (it is what admits an invited joiner); the
    /// Notice phase is gated on membership checked *after* the ACL
    /// reconcile — a joiner approved in this very session passes, a stranger
    /// stops with the ACL and never sees envelopes or ciphertext.
    ///
    /// [`Hello`]: crate::protocol::SyncMessage::Hello
    #[default]
    Members,
    /// Anyone may pull ciphertext + envelopes (metadata is visible to any
    /// caller holding the TerritoryID).
    Everyone,
}

/// One Territory's sightings, keyed by `(member, device key bytes)` — one
/// entry per certified device per identity.
type Sightings = BTreeMap<(MemberID, [u8; 32]), DeviceSighting>;

/// The authenticated caller of a session, as proven by its [`Hello`]
/// handshake: the identity key and the cert binding the connection's
/// device key to it.
///
/// [`Hello`]: crate::protocol::SyncMessage::Hello
#[derive(Clone)]
struct Caller {
    identity: VerifyingKey,
    cert: DeviceCert,
}

impl Station {
    /// Bind a storage-only Station on `secret` (its roster peer key)
    /// seeded with a single `replica` — it serves that one Territory and
    /// accepts no new registrations. For tests and single-Territory embeds;
    /// a self-host Station uses [`hosting`](Self::hosting).
    pub async fn new(secret: &SigningKey, replica: Replica) -> StationResult<Self> {
        Self::build(secret, None, None, vec![replica], false).await
    }

    /// [`new`](Self::new) on a routable bind — reachable from other hosts.
    /// Pair with [`with_advertised`](Self::with_advertised) so
    /// [`advertised_addr`](Self::advertised_addr) hands Vessels an address
    /// that routes to this host.
    pub async fn new_routable(secret: &SigningKey, replica: Replica) -> StationResult<Self> {
        Self::build(secret, None, None, vec![replica], true).await
    }

    /// [`new`](Self::new) that also runs the per-Fleet **orderer**,
    /// holding `fleet_key` to countersign ACL submissions.
    pub async fn orderer(
        secret: &SigningKey,
        fleet_key: SigningKey,
        replica: Replica,
    ) -> StationResult<Self> {
        Self::build(secret, Some(fleet_key), None, vec![replica], false).await
    }

    /// Bind a Station that homes many Territories under `storage` (a
    /// directory): it reloads the holds already there and accepts new
    /// [`Register`] requests, bootstrapping a fresh hold per Territory.
    ///
    /// [`Register`]: crate::protocol::SyncMessage::Register
    pub async fn hosting(secret: &SigningKey, storage: &Path) -> StationResult<Self> {
        Self::build(secret, None, Some(storage.to_owned()), Vec::new(), false).await
    }

    /// [`hosting`](Self::hosting) on a routable bind — the self-host
    /// posture: reachable from other hosts at an operator-advertised
    /// address ([`with_advertised`](Self::with_advertised)).
    pub async fn hosting_routable(secret: &SigningKey, storage: &Path) -> StationResult<Self> {
        Self::build(secret, None, Some(storage.to_owned()), Vec::new(), true).await
    }

    /// [`hosting`](Self::hosting) that also runs the per-Fleet **orderer**.
    pub async fn hosting_orderer(
        secret: &SigningKey,
        fleet_key: SigningKey,
        storage: &Path,
    ) -> StationResult<Self> {
        Self::build(
            secret,
            Some(fleet_key),
            Some(storage.to_owned()),
            Vec::new(),
            false,
        )
        .await
    }

    async fn build(
        secret: &SigningKey,
        fleet_key: Option<SigningKey>,
        storage: Option<PathBuf>,
        seed: Vec<Replica>,
        routable: bool,
    ) -> StationResult<Self> {
        let endpoint = if routable {
            Endpoint::bind_routable_identity(secret).await?
        } else {
            Endpoint::bind_identity(secret).await?
        };
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
            register_policy: RegisterPolicy::default(),
            sync_policy: SyncPolicy::default(),
            buses: Mutex::new(BTreeMap::new()),
            next_session: AtomicU64::new(1),
            advertise: Vec::new(),
        })
    }

    /// Configure the sockets this Station advertises to Vessels — the
    /// operator's public `host:port`(s). Without it,
    /// [`advertised_addr`](Self::advertised_addr) falls back to the bound
    /// sockets, which only works same-host.
    pub fn with_advertised(mut self, addrs: Vec<std::net::SocketAddr>) -> Self {
        self.advertise = addrs;
        self
    }

    /// The address Vessels should dial: the configured advertised sockets
    /// when set, else the actual bound sockets (same-host fallback).
    pub fn advertised_addr(&self) -> crate::fabric::EndpointAddr {
        if self.advertise.is_empty() {
            self.endpoint.direct_addr()
        } else {
            self.endpoint
                .advertised_addr(self.advertise.iter().copied())
        }
    }

    /// Override the [`RegisterPolicy`] (default: [`RegisterPolicy::Owner`]).
    pub fn with_register_policy(mut self, policy: RegisterPolicy) -> Self {
        self.register_policy = policy;
        self
    }

    /// Override the [`SyncPolicy`] (default: [`SyncPolicy::Members`]).
    pub fn with_sync_policy(mut self, policy: SyncPolicy) -> Self {
        self.sync_policy = policy;
        self
    }

    /// Load a Replica for every Territory hold already in `dir` (restart
    /// path). Holds are named `<territory-hex>.db`; files that do not parse as
    /// a Territory id are skipped, so an unrelated file is harmless.
    async fn scan_holds(dir: &Path) -> StationResult<Vec<Replica>> {
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

    /// Whether this Station homes `territory`.
    pub fn serves(&self, territory: &TerritoryID) -> bool {
        self.territories.lock().unwrap().contains_key(territory)
    }

    /// Home a Territory from its presented ACL log: idempotent if
    /// already served; otherwise verify the id is the root's hash, fold the
    /// log (chaining + authority must hold), apply the [`RegisterPolicy`],
    /// and — if this Station has a storage directory — bootstrap a fresh
    /// hold that is *current with the whole log*, not root-only. A first
    /// registration is remembered in the Station database (who, when) when
    /// the registrant authenticated. Returns whether the Station now serves
    /// it.
    async fn register(
        &self,
        territory: TerritoryID,
        acl: Vec<OrderedRecord>,
        caller: Option<&Caller>,
    ) -> StationResult<bool> {
        if self.serves(&territory) {
            return Ok(true);
        }
        let Some(root) = acl.first().map(|o| o.record.clone()) else {
            return Ok(false); // an empty log defines nothing
        };
        if TerritoryID::of_root(&root)? != territory {
            return Ok(false); // the id must be the root's hash
        }
        // Fold the presented log — a log that does not chain or exceeds its
        // authors' authority is refused before any policy question.
        let records: Vec<ACLRecord> = acl.iter().map(|o| o.record.clone()).collect();
        let Ok(fold) = ACLIndex::from_ordered(&records) else {
            return Ok(false);
        };
        // Who may home a new Territory here: the fold is
        // the authority — a later-promoted Owner passes identically to the
        // creator.
        match self.register_policy {
            RegisterPolicy::Anyone => {}
            RegisterPolicy::Owner => {
                let Some(caller) = caller else {
                    return Ok(false); // Owner policy requires the handshake
                };
                let member = MemberID::from(&caller.identity);
                let is_owner = fold
                    .head_state()
                    .is_some_and(|s| s.members.get(&member) == Some(&Permissions::Owner));
                if !is_owner {
                    return Ok(false);
                }
            }
        }
        let Some(dir) = &self.storage else {
            return Ok(false); // a fixed-replica Station bootstraps nothing new
        };
        let path = dir.join(format!("{}.db", hex::encode(territory.as_bytes())));
        let path = path.to_str().ok_or(StationError::BadPath)?;
        let hold = TerritoryHold::open(path).await?;
        if hold.acl_records().await?.is_empty() {
            for (position, ordered) in acl.iter().enumerate() {
                hold.append_acl(&ordered.record, position as u64, ordered.proof.as_ref())
                    .await?;
            }
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

    /// Accept one inbound session and handle it to completion: an optional
    /// [`Hello`] handshake verifying which identity is behind the
    /// device connection, then a request — register a Territory, open a
    /// scoped sync session, subscribe live, order an ACL submission, or list
    /// device sightings. Sequential — a live subscription occupies it; use
    /// [`serve_loop`](Self::serve_loop) to serve sessions concurrently.
    ///
    /// [`Hello`]: crate::protocol::SyncMessage::Hello
    pub async fn serve_once(&self) -> StationResult<()> {
        let Some(session) = self.endpoint.accept().await? else {
            return Ok(());
        };
        self.handle(session).await
    }

    /// Accept sessions until the endpoint closes, handling each in its own
    /// task — the production serve shape (a live subscription must not block
    /// the next caller). Returns when [`accept`](Endpoint::accept) yields
    /// `None`.
    pub async fn serve_loop(self: &Arc<Self>) -> StationResult<()> {
        loop {
            let Some(session) = self.endpoint.accept().await? else {
                return Ok(());
            };
            let station = Arc::clone(self);
            tokio::spawn(async move {
                if let Err(e) = station.handle(session).await {
                    tracing::warn!("station session failed: {e}");
                }
            });
        }
    }

    async fn handle(&self, mut session: Session) -> StationResult<()> {
        let first = read_frame::<SyncMessage>(&mut session.recv).await;

        // Optional handshake: verify the cert binds the connection's device
        // key to the claimed identity. A Vessel and a Station never
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

        let session_id = self.next_session.fetch_add(1, Ordering::Relaxed);
        self.dispatch(request, caller.as_ref(), session_id, &mut session)
            .await;
        session.drain().await;
        // The session is over: whatever this device touched, it is no longer
        // live on it (connection status).
        if let Some(caller) = &caller {
            self.mark_disconnected(caller);
        }
        Ok(())
    }

    async fn dispatch(
        &self,
        request: Result<SyncMessage, crate::fabric::FrameError>,
        caller: Option<&Caller>,
        session_id: u64,
        session: &mut Session,
    ) {
        match request {
            Ok(SyncMessage::Register { territory, acl }) => {
                let ok = match self.register(territory, acl, caller).await {
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
            Ok(SyncMessage::Open { territory }) => {
                let replica = self.replica_for(&territory);
                // Members admission needs an authenticated caller before
                // anything is served; the *membership* check itself waits
                // until after the ACL phase (see the gate below).
                let admitted = replica.is_some()
                    && match self.sync_policy {
                        SyncPolicy::Everyone => true,
                        SyncPolicy::Members => caller.is_some(),
                    };
                if !admitted {
                    let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: false }).await;
                    return;
                }
                let replica = replica.expect("admitted implies present");
                if let Some(caller) = caller {
                    self.record_sighting(territory, caller).await;
                }
                let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: true }).await;
                // The two-phase gate: ACL phase for any
                // authenticated identity, Notice phase only for a member of
                // the *post-reconcile* fold — an invited joiner approved in
                // this very session passes; a stranger stops with the ACL.
                let policy = self.sync_policy;
                let identity = caller.map(|c| c.identity);
                let gate = || async {
                    match policy {
                        SyncPolicy::Everyone => true,
                        SyncPolicy::Members => match identity {
                            Some(identity) => self.is_member(&territory, &identity).await,
                            None => false,
                        },
                    }
                };
                // Serve through the publishing peer, so records this session
                // brings in fan out to the Territory's live subscribers.
                let peer = PublishingPeer {
                    replica,
                    bus: self.bus_for(&territory),
                    source: session_id,
                };
                if serve_gated(&peer, &mut session.send, &mut session.recv, gate)
                    .await
                    .is_err()
                {
                    tracing::warn!("station session ended early");
                }
            }
            Ok(SyncMessage::Subscribe { territory }) => {
                self.subscribe_session(territory, caller, session_id, session)
                    .await;
            }
            Ok(SyncMessage::OrderRequest { territory, record }) => {
                // Order the submission: assign a position and
                // countersign — only possible if we hold the fleet key and
                // home the Territory.
                let proof = match (self.replica_for(&territory), &self.fleet_key) {
                    (Some(replica), Some(key)) => replica.order(key, &record).await.ok().flatten(),
                    _ => None,
                };
                if let Some(proof) = proof {
                    // An ordered ACL record is exactly what other devices
                    // wait on — fan it out to the Territory's subscribers.
                    let ordered = OrderedRecord {
                        record: record.clone(),
                        proof: Some(proof),
                    };
                    let _ = self.bus_for(&territory).send(BusEvent {
                        source: session_id,
                        push: BusPush::ACL(vec![ordered]),
                    });
                }
                let _ = write_frame(&mut session.send, &SyncMessage::OrderResponse { proof }).await;
            }
            Ok(SyncMessage::DevicesRequest { territory }) => {
                // Viewing is member-gated: only an authenticated caller
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

    // ---- device sightings (visibility + attribution) -----------------

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

impl Station {
    /// The live half of a [`Subscribe`](SyncMessage::Subscribe) session
    /// admission + initial reconcile exactly like `Open`,
    /// then park — forwarding this Territory's bus to the vessel and the
    /// vessel's pushes onto the bus — until either side closes.
    async fn subscribe_session(
        &self,
        territory: TerritoryID,
        caller: Option<&Caller>,
        session_id: u64,
        session: &mut Session,
    ) {
        let replica = self.replica_for(&territory);
        let admitted = replica.is_some()
            && match self.sync_policy {
                SyncPolicy::Everyone => true,
                SyncPolicy::Members => caller.is_some(),
            };
        if !admitted {
            let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: false }).await;
            return;
        }
        let replica = replica.expect("admitted implies present");
        if let Some(caller) = caller {
            self.record_sighting(territory, caller).await;
        }
        let _ = write_frame(&mut session.send, &SyncMessage::Ack { ok: true }).await;

        // Initial reconcile through the publishing peer, with the same
        // two-phase admission gate as a one-shot Open.
        let policy = self.sync_policy;
        let identity = caller.map(|c| c.identity);
        let bus = self.bus_for(&territory);
        let peer = PublishingPeer {
            replica: Arc::clone(&replica),
            bus: bus.clone(),
            source: session_id,
        };
        let gate = || async {
            match policy {
                SyncPolicy::Everyone => true,
                SyncPolicy::Members => match identity {
                    Some(identity) => self.is_member(&territory, &identity).await,
                    None => false,
                },
            }
        };
        if serve_gated(&peer, &mut session.send, &mut session.recv, gate)
            .await
            .is_err()
        {
            return;
        }
        // The gate may have refused inside the reconcile (the vessel saw it
        // and is gone); do not park a refused session in the live phase.
        let stays = match policy {
            SyncPolicy::Everyone => true,
            SyncPolicy::Members => match identity {
                Some(identity) => self.is_member(&territory, &identity).await,
                None => false,
            },
        };
        if !stays {
            return;
        }

        // Live phase: vessel pushes in, bus events out (minus our own).
        let mut rx = bus.subscribe();
        loop {
            tokio::select! {
                frame = read_frame::<SyncMessage>(&mut session.recv) => match frame {
                    Ok(SyncMessage::NoticePush { notices }) => {
                        if notices.is_empty() {
                            continue;
                        }
                        if let Err(e) = replica.ingest(notices.clone()).await {
                            tracing::warn!("live notice ingest failed: {e}");
                            continue;
                        }
                        let _ = bus.send(BusEvent {
                            source: session_id,
                            push: BusPush::Notices(notices),
                        });
                    }
                    Ok(SyncMessage::ACLPush { records }) => {
                        if records.is_empty() {
                            continue;
                        }
                        if let Err(e) = replica.ingest_ordered(records.clone()).await {
                            tracing::warn!("live acl ingest failed: {e}");
                            continue;
                        }
                        let _ = bus.send(BusEvent {
                            source: session_id,
                            push: BusPush::ACL(records),
                        });
                    }
                    Ok(_) => {} // unknown live frame: ignore
                    Err(_) => break, // vessel closed / transport gone
                },
                event = rx.recv() => match event {
                    Ok(event) => {
                        if event.source == session_id {
                            continue; // no echo
                        }
                        let frame = match event.push {
                            BusPush::Notices(notices) => SyncMessage::NoticePush { notices },
                            BusPush::ACL(records) => SyncMessage::ACLPush { records },
                        };
                        if write_frame(&mut session.send, &frame).await.is_err() {
                            break;
                        }
                    }
                    // Lagged: this subscriber missed events. Disconnect it —
                    // honest failure; its resubscribe reconciles the gap.
                    Err(broadcast::error::RecvError::Lagged(_)) => break,
                    Err(broadcast::error::RecvError::Closed) => break,
                },
            }
        }
    }

    /// This Territory's live-push bus, created on first use.
    fn bus_for(&self, territory: &TerritoryID) -> broadcast::Sender<BusEvent> {
        self.buses
            .lock()
            .unwrap()
            .entry(*territory)
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }
}

/// A [`SyncPeer`] adapter over the Territory replica that **publishes what
/// it newly accepts** to the Territory's bus — how records
/// arriving through an ordinary reconcile reach live subscribers. Receivers
/// ingest idempotently, so over-publishing a duplicate is harmless.
struct PublishingPeer {
    replica: Arc<Replica>,
    bus: broadcast::Sender<BusEvent>,
    source: u64,
}

#[async_trait]
impl SyncPeer for PublishingPeer {
    async fn digest(&self) -> TallyDigest {
        self.replica.digest().await
    }

    async fn track_notices(&self, track: TrackID) -> Vec<SignedNotice> {
        SyncPeer::track_notices(&*self.replica, track).await
    }

    async fn held_ids(&self) -> std::collections::BTreeSet<NoticeID> {
        SyncPeer::held_ids(&*self.replica).await
    }

    async fn ingest(&self, notices: Vec<SignedNotice>) {
        if notices.is_empty() {
            return;
        }
        SyncPeer::ingest(&*self.replica, notices.clone()).await;
        let _ = self.bus.send(BusEvent {
            source: self.source,
            push: BusPush::Notices(notices),
        });
    }

    async fn acl_len(&self) -> u64 {
        SyncPeer::acl_len(&*self.replica).await
    }

    async fn acl_suffix(&self, from: u64) -> Vec<OrderedRecord> {
        SyncPeer::acl_suffix(&*self.replica, from).await
    }

    async fn ingest_ordered_acl(&self, records: Vec<OrderedRecord>) {
        if records.is_empty() {
            return;
        }
        SyncPeer::ingest_ordered_acl(&*self.replica, records.clone()).await;
        let _ = self.bus.send(BusEvent {
            source: self.source,
            push: BusPush::ACL(records),
        });
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
