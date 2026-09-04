//! The Vessel role: an end-device that holds a user's keys. The Vessel is the
//! *only* party that can **originate** edits, **decrypt** payloads, and merge
//! — the whole reason it and a Station, which can do none of these, never
//! implicitly trust each other even under the same user identity.
//!
//! It composes the shared [`Replica`] (validate + store + answer sync, the
//! half every peer runs) with Vessel-only duties: authoring Notices under the
//! identity key, the read-key cache that unseals payloads, and dialing Stations
//! to sync. Snapshot cadence layers on here later.

use crate::{
    fabric::{
        Endpoint, EndpointAddr, Session, SessionError, fleet::FleetConfig, initiate, read_frame,
        write_frame,
    },
    hold::ACLIndex,
    protocol::{
        ACLPayload, CryptoRNG, DeviceCert, DeviceKey, DeviceSighting, Envelope, EnvelopeHeader,
        EpochID, Identity, InviteKey, KEMPublic, KEMSecret, MemberID, NoticeID, OrderProof,
        OrderedRecord, Permissions, ReadKey, RecordBody, SignedNotice, SyncMessage, TerritoryID,
        TrackID, join_binding, open_payload, open_sealed_key, seal_key_to_member, seal_payload,
        unwrap_prev_key, wrap_prev_key,
    },
    replica::{Replica, ReplicaError},
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex as StdMutex},
};
use tokio::sync::mpsc;

pub use crate::protocol::ACLRecord;

type VesselResult<T> = ::std::result::Result<T, VesselError>;

/// Write a framed message to a session, mapping framing errors.
async fn frame_write(session: &mut Session, msg: &SyncMessage) -> VesselResult<()> {
    write_frame(&mut session.send, msg)
        .await
        .map_err(|e| VesselError::Session(e.to_string()))
}

/// Read a framed message from a session, mapping framing errors.
async fn frame_read(session: &mut Session) -> VesselResult<SyncMessage> {
    read_frame::<SyncMessage>(&mut session.recv)
        .await
        .map_err(|e| VesselError::Session(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum VesselError {
    #[error(transparent)]
    Transport(#[from] crate::fabric::TransportError),
    #[error(transparent)]
    Replica(#[from] crate::replica::ReplicaError),
    #[error(transparent)]
    Hold(#[from] crate::hold::HoldError),
    #[error(transparent)]
    Wire(#[from] crate::protocol::WireError),
    #[error(transparent)]
    Crypto(#[from] crate::protocol::CryptoError),
    #[error("no read key cached for epoch {0}")]
    NoKey(EpochID),
    #[error("no pending join for that member — nothing to approve")]
    UnknownJoin,
    #[error("orderer exchange failed: {0}")]
    Order(String),
    #[error("the orderer refused to order the record")]
    OrderRejected,
    #[error("session framing failed: {0}")]
    Session(String),
    #[error("the Station is not in the trusted fleet roster")]
    UntrustedStation,
    #[error("the Station refused the territory registration")]
    RegisterRejected,
    #[error("the Station does not serve that Territory — register it first")]
    NotServed,
    #[error("the Station refused the device listing (not a member, or no identity presented)")]
    DevicesRefused,
    #[error("the Station refused the Notice phase — this identity is not a member")]
    SyncRefused,
    #[error("a device sighting failed verification — the Station may be lying")]
    BadSighting,
}

/// A Vessel: the user's identity (authors + decrypts), a per-device key
/// binding the connection, the shared replica, and the per-epoch read-key
/// cache. The cache is what makes this a Vessel; a Station has no
/// equivalent and cannot obtain one.
///
/// The `identity` key authors Notices and ACL records — a `MemberID` is an
/// identity, never a device. The endpoint is bound with the *device*
/// key (the `VesselID`); its cert proves which identity is behind the
/// connection during the handshake ([`from_identity`](Self::from_identity)
/// carries the derived device cert, [`new`](Self::new) a self-cert).
pub struct Vessel {
    identity: SigningKey,
    kem: Arc<KEMSecret>,
    endpoint: Endpoint,
    /// Certificate binding the endpoint's device key to `identity`,
    /// presented in the connection handshake. For the low-level
    /// [`new`](Self::new) constructor this is a **self-cert** — the identity
    /// certifying its own key as the device key — so every Vessel can
    /// authenticate to an admission-gated Station.
    cert: DeviceCert,
    /// Shared with subscription tasks (the replica is the convergence
    /// point; the cache is what auto-refresh fills) — `Arc` so a live
    /// session can ingest and refresh in the background while the app keeps
    /// using the Vessel.
    replica: Arc<Replica>,
    read_keys: Arc<StdMutex<BTreeMap<EpochID, ReadKey>>>,
}

impl Vessel {
    /// Open a Vessel from a derived [`Identity`] and one of its
    /// [`DeviceKey`]s. The identity authors and decrypts; the endpoint
    /// is bound with the device key (the `VesselID`). Read keys the ACL log
    /// already seals to this identity are derived immediately
    /// ([`refresh_keys`](Self::refresh_keys)).
    pub async fn from_identity(
        identity: &Identity,
        device: &DeviceKey,
        replica: Replica,
    ) -> VesselResult<Self> {
        let endpoint = Endpoint::bind_identity(&device.signing).await?;
        Self::with_endpoint(identity, device, replica, endpoint).await
    }

    /// [`from_identity`](Self::from_identity) with a routable bind, so
    /// outbound packets can leave the host and reach a Station elsewhere.
    /// Inbound reachability is still not required — a Vessel only ever
    /// dials.
    pub async fn from_identity_routable(
        identity: &Identity,
        device: &DeviceKey,
        replica: Replica,
    ) -> VesselResult<Self> {
        let endpoint = Endpoint::bind_routable_identity(&device.signing).await?;
        Self::with_endpoint(identity, device, replica, endpoint).await
    }

    async fn with_endpoint(
        identity: &Identity,
        device: &DeviceKey,
        replica: Replica,
        endpoint: Endpoint,
    ) -> VesselResult<Self> {
        let mut vessel = Self {
            identity: identity.signing_key().clone(),
            kem: Arc::new(identity.secret_key()),
            endpoint,
            cert: device.cert.clone(),
            replica: Arc::new(replica),
            read_keys: Arc::new(StdMutex::new(BTreeMap::new())),
        };
        vessel.refresh_keys().await?;
        Ok(vessel)
    }

    /// Open a Vessel from raw keys, binding the endpoint with the identity
    /// key itself (no separate device key). Lower-level than
    /// [`from_identity`](Self::from_identity); useful where a full derived
    /// identity is overkill. The handshake cert is a **self-cert** (the
    /// identity certifies its own key), so this Vessel authenticates to
    /// admission-gated Stations like any other.
    pub async fn new(identity: SigningKey, kem: KEMSecret, replica: Replica) -> VesselResult<Self> {
        let endpoint = Endpoint::bind_identity(&identity).await?;
        let cert = DeviceCert::issue(&identity, &identity.verifying_key());
        let mut vessel = Self {
            identity,
            kem: Arc::new(kem),
            endpoint,
            cert,
            replica: Arc::new(replica),
            read_keys: Arc::new(StdMutex::new(BTreeMap::new())),
        };
        vessel.refresh_keys().await?;
        Ok(vessel)
    }

    /// The device certificate proving the endpoint key belongs to this
    /// identity — presented in the connection handshake.
    pub fn device_cert(&self) -> &DeviceCert {
        &self.cert
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// A shared handle to the replica — what a background task converges
    /// into.
    pub fn replica_handle(&self) -> Arc<Replica> {
        Arc::clone(&self.replica)
    }

    pub fn kem_public(&self) -> KEMPublic {
        self.kem.public_key()
    }

    /// Cache a read key directly — an escape hatch for keys obtained out of
    /// band. The normal path is [`refresh_keys`](Self::refresh_keys), which
    /// derives keys from the ACL log.
    pub fn cache_read_key(&mut self, epoch: EpochID, key: ReadKey) {
        self.read_keys.lock().unwrap().insert(epoch, key);
    }

    /// Populate the read-key cache from the ACL log: for every
    /// record that sealed a key to *this* Vessel's member identity, unseal
    /// it with the KEM secret; then walk the backwards key chain so a member
    /// admitted at a later epoch recovers every earlier epoch it is entitled
    /// to read. Idempotent — call after opening a Territory and after any
    /// sync that delivered new ACL records.
    ///
    /// This is a Vessel-only operation: it is exactly the key acquisition a
    /// Station cannot perform (it is not a member, so no box is sealed to
    /// it). A box sealed to someone else simply fails to open — that is
    /// expected, not an error, so those failures are skipped silently.
    pub async fn refresh_keys(&mut self) -> VesselResult<()> {
        let me = MemberID::from(&self.identity.verifying_key());
        derive_keys(&self.replica, &self.kem, me, &self.read_keys).await
    }

    /// Which epochs this Vessel currently holds a read key for — the history
    /// it can decrypt. Useful for tests and diagnostics.
    pub fn readable_epochs(&self) -> Vec<EpochID> {
        self.read_keys.lock().unwrap().keys().copied().collect()
    }

    /// Author a Notice: seal `plaintext` under the current epoch's read
    /// key, sign the envelope, and ingest it locally. Only a Vessel can do
    /// this. Returns the stored Notice.
    pub async fn author(
        &self,
        rng: &mut impl CryptoRNG,
        parents: Vec<NoticeID>,
        epoch: EpochID,
        payload_type: &str,
        plaintext: &[u8],
    ) -> VesselResult<SignedNotice> {
        let key = self
            .read_keys
            .lock()
            .unwrap()
            .get(&epoch)
            .cloned()
            .ok_or(VesselError::NoKey(epoch))?;
        let snapshot_base = if parents.is_empty() {
            NoticeID::SELF
        } else {
            // A non-root's snapshot base is resolved by the snapshot layer
            // (deferred); until then it points at the first parent.
            parents[0]
        };
        let header = EnvelopeHeader {
            parents,
            acl_head: self.replica.acl_head().await?,
            epoch,
            author: self.identity.verifying_key(),
            snapshot_base,
            is_snapshot: false,
            payload_type: payload_type.to_owned(),
            // Advisory at the protocol layer (never inspected by
            // validation); app-layer consumers read it as the HLC
            // component of the arbitration key.
            timestamp: self.replica.clock_tick().await?.inner(),
        };
        let payload = seal_payload(rng, &key, &header, plaintext)?;
        let notice = SignedNotice::sign(&Envelope { header, payload }, &self.identity)?;
        self.replica.ingest(vec![notice.clone()]).await?;
        Ok(notice)
    }

    /// Decrypt a stored Notice's payload with the cached read key —
    /// the Vessel-only step a Station cannot perform. The AAD binding means
    /// this fails unless every header field matches what was sealed.
    pub fn decrypt(&self, notice: &SignedNotice) -> VesselResult<Vec<u8>> {
        let envelope = notice.parse_and_verify()?;
        let key = self
            .read_keys
            .lock()
            .unwrap()
            .get(&envelope.header.epoch)
            .cloned()
            .ok_or(VesselError::NoKey(envelope.header.epoch))?;
        Ok(open_payload(&key, &envelope.header, &envelope.payload)?)
    }

    /// Dial a Station and run one full sync: reconciles the ACL log
    /// and both directions of the Notice DAG, then **auto-refreshes read
    /// keys** — any membership change the sync delivered takes effect without
    /// a separate call. Sends the identity handshake when this Vessel
    /// was opened from a derived identity. Does not verify the Station's
    /// roster membership — use [`sync_with_fleet`](Self::sync_with_fleet) for
    /// that.
    pub async fn sync_with(&mut self, station: impl Into<EndpointAddr>) -> VesselResult<()> {
        let mut session = self.endpoint.connect(station).await?;
        self.run_sync(&mut session).await
    }

    /// Like [`sync_with`](Self::sync_with), but first verifies the Station is
    /// in the trusted `config`'s roster: the config's signature must
    /// check out and the Station's authenticated key must be listed. Refuses
    /// to sync with a Station the fleet did not vouch for.
    pub async fn sync_with_fleet(
        &mut self,
        config: &FleetConfig,
        station: impl Into<EndpointAddr>,
    ) -> VesselResult<()> {
        let mut session = self.endpoint.connect(station).await?;
        self.check_roster(config, &session)?;
        self.run_sync(&mut session).await
    }

    /// Register `territory` with a Station so it will hold and serve it —
    /// the "connect to a fleet's servers" step. Presents the Territory's root
    /// ACL record (so a Station that does not yet hold it can bootstrap), the
    /// identity handshake, and waits for the acknowledgement. Does **not**
    /// verify the Station's roster membership — use
    /// [`register_with_fleet`](Self::register_with_fleet) for that.
    pub async fn register(
        &self,
        station: impl Into<EndpointAddr>,
        territory: TerritoryID,
    ) -> VesselResult<()> {
        let mut session = self.endpoint.connect(station).await?;
        self.do_register(&mut session, territory).await
    }

    /// Like [`register`](Self::register), but first verifies the Station is in
    /// the trusted `config`'s roster.
    pub async fn register_with_fleet(
        &self,
        config: &FleetConfig,
        station: impl Into<EndpointAddr>,
        territory: TerritoryID,
    ) -> VesselResult<()> {
        let mut session = self.endpoint.connect(station).await?;
        self.check_roster(config, &session)?;
        self.do_register(&mut session, territory).await
    }

    /// The shared registration exchange: handshake, present the ACL log,
    /// await the ack. The full log (not just the root) travels so the
    /// Station bootstraps *current* and can check the registrant's authority
    /// against the fold.
    async fn do_register(&self, session: &mut Session, territory: TerritoryID) -> VesselResult<()> {
        self.send_hello(session).await?;
        let acl = self.replica.acl_suffix(0).await?;
        frame_write(session, &SyncMessage::Register { territory, acl }).await?;
        let ack = frame_read(session).await?;
        session.close();
        match ack {
            SyncMessage::Ack { ok: true } => Ok(()),
            _ => Err(VesselError::RegisterRejected),
        }
    }

    /// The shared post-connect flow: handshake, scope to the Territory,
    /// reconcile, refresh keys. An admission refusal after the ACL phase
    /// surfaces as [`SyncRefused`](VesselError::SyncRefused) — the ACL
    /// records received up to that point are kept and keys still refresh
    /// (they are exactly what a pending joiner came for); other session
    /// hiccups stay best-effort, repaired by the next reconcile.
    async fn run_sync(&mut self, session: &mut Session) -> VesselResult<()> {
        self.send_hello(session).await?;
        self.open_territory(session).await?;
        let result = initiate(&*self.replica, &mut session.send, &mut session.recv).await;
        session.close();
        self.refresh_keys().await?;
        match result {
            Err(SessionError::Refused) => Err(VesselError::SyncRefused),
            _ => Ok(()),
        }
    }

    /// Ask a Station which certified devices have touched this Vessel's
    /// Territory (visibility + attribution). Member-gated on the Station
    /// side, so this Vessel must present its identity handshake and be in the
    /// Territory's ACL. Every returned sighting is **verified here** — the
    /// cert must check out against its claimed identity — so a lying Station
    /// cannot attribute a device to an identity that never certified it; a
    /// sighting that fails verification fails the whole call
    /// ([`BadSighting`](VesselError::BadSighting)).
    ///
    /// What a Station *can* still do is under-report or fabricate timestamps
    /// — presence metadata is attested by nobody but the Station observing
    /// it. Attribution is the verified part.
    pub async fn device_sightings(
        &self,
        station: impl Into<EndpointAddr>,
    ) -> VesselResult<Vec<DeviceSighting>> {
        let territory = self.replica.territory_id().await?;
        let mut session = self.endpoint.connect(station).await?;
        self.send_hello(&mut session).await?;
        frame_write(&mut session, &SyncMessage::DevicesRequest { territory }).await?;
        let reply = frame_read(&mut session).await?;
        session.close();
        let devices = match reply {
            SyncMessage::DevicesResponse { devices } => devices,
            _ => return Err(VesselError::DevicesRefused),
        };
        for sighting in &devices {
            if sighting.cert.verify(&sighting.identity).is_err() {
                return Err(VesselError::BadSighting);
            }
        }
        Ok(devices)
    }

    /// Probe a Station: dial it, present the identity handshake, and ask
    /// whether it serves this Vessel's Territory — `Ok(true)` if it does,
    /// `Ok(false)` if it answered but does not (register it first), `Err` if
    /// it could not be reached at all. A liveness check, not a sync: the
    /// session is closed right after the ack, so the Station may log the
    /// early end of the sync flow it was ready to serve.
    pub async fn probe(&self, station: impl Into<EndpointAddr>) -> VesselResult<bool> {
        let territory = self.replica.territory_id().await?;
        let mut session = self.endpoint.connect(station).await?;
        self.send_hello(&mut session).await?;
        frame_write(&mut session, &SyncMessage::Open { territory }).await?;
        let ack = frame_read(&mut session).await?;
        session.close();
        match ack {
            SyncMessage::Ack { ok } => Ok(ok),
            _ => Err(VesselError::Session("unexpected probe reply".into())),
        }
    }

    /// Scope the session to this Vessel's Territory and confirm the ends
    /// Station serves it — a Station that does not returns
    /// [`NotServed`](VesselError::NotServed) (register it first).
    async fn open_territory(&self, session: &mut Session) -> VesselResult<()> {
        let territory = self.replica.territory_id().await?;
        frame_write(session, &SyncMessage::Open { territory }).await?;
        match frame_read(session).await? {
            SyncMessage::Ack { ok: true } => Ok(()),
            _ => Err(VesselError::NotServed),
        }
    }

    /// Verify a Station is one the fleet vouches for.
    fn check_roster(&self, config: &FleetConfig, session: &Session) -> VesselResult<()> {
        config.verify().map_err(|_| VesselError::UntrustedStation)?;
        if config.station(&session.remote_key()).is_none() {
            return Err(VesselError::UntrustedStation);
        }
        Ok(())
    }

    /// Present the identity handshake and read the Station's
    /// acknowledgement. Every Vessel carries a cert (a self-cert at minimum),
    /// so the handshake is unconditional — admission-gated Stations require
    /// it.
    async fn send_hello(&self, session: &mut Session) -> VesselResult<()> {
        frame_write(
            session,
            &SyncMessage::Hello {
                identity: self.identity.verifying_key(),
                cert: self.cert.clone(),
            },
        )
        .await?;
        // The Station acks the handshake; a rejection ends the session.
        match frame_read(session).await? {
            SyncMessage::Ack { ok: true } => {}
            _ => return Err(VesselError::UntrustedStation),
        }
        Ok(())
    }

    // ---- Territory & membership -----------------------------------
    //
    // Each operation builds the right ACL record signed by the identity and
    // applies it locally (single-writer). The orderer submission flow (a
    // later phase) routes these through the fleet orderer before they
    // propagate; the record shapes are unchanged.

    /// Create a new Territory owned by `identity`, returning a ready Vessel.
    /// A fresh epoch-0 read key is sealed to the owner's KEM, so the returned
    /// Vessel can immediately author and read.
    pub async fn create_territory(
        identity: &Identity,
        device: &DeviceKey,
        path: &str,
        rng: &mut impl CryptoRNG,
    ) -> VesselResult<Self> {
        let replica = Self::bootstrap_territory(identity, path, rng).await?;
        Self::from_identity(identity, device, replica).await
    }

    /// [`create_territory`](Self::create_territory) with a routable bind —
    /// see [`from_identity_routable`](Self::from_identity_routable).
    pub async fn create_territory_routable(
        identity: &Identity,
        device: &DeviceKey,
        path: &str,
        rng: &mut impl CryptoRNG,
    ) -> VesselResult<Self> {
        let replica = Self::bootstrap_territory(identity, path, rng).await?;
        Self::from_identity_routable(identity, device, replica).await
    }

    async fn bootstrap_territory(
        identity: &Identity,
        path: &str,
        rng: &mut impl CryptoRNG,
    ) -> VesselResult<Replica> {
        let mut territory_seed = [0u8; 32];
        rng.fill_bytes(&mut territory_seed);
        let epoch0 = ReadKey::generate(rng);
        let sealed = seal_key_to_member(rng, &identity.public_key(), &epoch0)?;
        let root = RecordBody {
            prev: None,
            author: identity.verifying_key(),
            payload: ACLPayload::Root {
                territory_seed,
                owner_kem: identity.public_key(),
                epoch0_key_wrapped: sealed,
            },
        }
        .sign(identity.signing_key())?;
        Ok(Replica::bootstrap(path, &[root]).await?)
    }

    /// This Vessel's Territory id (hash of the root ACL record).
    pub async fn territory_id(&self) -> VesselResult<TerritoryID> {
        let acl = self.replica.acl_index().await?;
        Ok(acl.head_state().ok_or(ReplicaError::NoACL)?.territory)
    }

    /// Build an ACL record authored by this identity, chained on the current
    /// head, and apply it locally. Shared by the membership operations.
    async fn author_acl(&self, payload: ACLPayload) -> VesselResult<ACLRecord> {
        let prev = self.replica.acl_head().await?;
        let record = RecordBody {
            prev: Some(prev),
            author: self.identity.verifying_key(),
            payload,
        }
        .sign(&self.identity)?;
        self.replica.apply_local(&record, None).await?;
        Ok(record)
    }

    /// Submit an ACL record to a fleet orderer Station: the orderer
    /// assigns it a position and returns a countersigned proof. The ordered
    /// record then propagates to peers with the proof, which they verify.
    /// Errors if the orderer refuses (e.g. the record does not chain).
    pub async fn submit_acl(
        &self,
        orderer: impl Into<EndpointAddr>,
        record: &ACLRecord,
    ) -> VesselResult<OrderProof> {
        let territory = self.replica.territory_id().await?;
        let mut session = self.endpoint.connect(orderer).await?;
        write_frame(
            &mut session.send,
            &SyncMessage::OrderRequest {
                territory,
                record: record.clone(),
            },
        )
        .await
        .map_err(|e| VesselError::Order(e.to_string()))?;
        let proof = match read_frame::<SyncMessage>(&mut session.recv)
            .await
            .map_err(|e| VesselError::Order(e.to_string()))?
        {
            SyncMessage::OrderResponse { proof } => proof,
            _ => None,
        };
        session.close();
        proof.ok_or(VesselError::OrderRejected)
    }

    /// Issue an invite at `permissions` (Admin+ only). Returns the invite
    /// public key, the invite **secret** to hand the invitee out of band, and
    /// the record. Single-use: a `Join` consumes it.
    pub async fn invite(
        &self,
        rng: &mut impl CryptoRNG,
        permissions: Permissions,
    ) -> Result<(InviteKey, [u8; 32], ACLRecord), VesselError> {
        let mut secret = [0u8; 32];
        rng.fill_bytes(&mut secret);
        let invite_signing = SigningKey::from_bytes(&secret);
        let invite_key = InviteKey::from_bytes(invite_signing.verifying_key().to_bytes());
        let record = self
            .author_acl(ACLPayload::Invite {
                invite_key,
                permissions,
            })
            .await?;
        Ok((invite_key, secret, record))
    }

    /// Join a Territory with an invite secret: proves the invite, presents
    /// this identity's KEM key. An admin must then [`approve`] the
    /// resulting record. The Vessel must already hold the ACL up to the
    /// invite (bootstrap or sync).
    ///
    /// [`approve`]: Self::approve
    pub async fn join(
        &self,
        invite_key: InviteKey,
        invite_secret: [u8; 32],
    ) -> VesselResult<ACLRecord> {
        let invite_signing = SigningKey::from_bytes(&invite_secret);
        let member_kem = self.kem.public_key();
        let invite_proof = invite_signing.sign(&join_binding(
            &invite_key,
            &self.identity.verifying_key(),
            &member_kem,
        ));
        self.author_acl(ACLPayload::Join {
            invite_key,
            member_kem,
            invite_proof,
        })
        .await
    }

    /// Approve a pending joiner (Admin+ only), sealing the current epoch's
    /// read key to their KEM. No rotation — adding a member does not
    /// change the epoch.
    pub async fn approve(
        &self,
        rng: &mut impl CryptoRNG,
        member: MemberID,
        permissions: Permissions,
    ) -> VesselResult<ACLRecord> {
        let acl = self.replica.acl_index().await?;
        let state = acl.head_state().ok_or(ReplicaError::NoACL)?;
        let member_kem = state
            .pending_joins
            .get(&member)
            .map(|(kem, _)| kem.clone())
            .ok_or(VesselError::UnknownJoin)?;
        let epoch = state.epoch;
        let key = self
            .read_keys
            .lock()
            .unwrap()
            .get(&epoch)
            .cloned()
            .ok_or(VesselError::NoKey(epoch))?;
        let current_key_wrapped = seal_key_to_member(rng, &member_kem, &key)?;
        self.author_acl(ACLPayload::Approve {
            member,
            permissions,
            current_key_wrapped,
        })
        .await
    }

    /// Change a member's role (authority per the lattice; touching Admin+ is
    /// Owner-only).
    pub async fn change_role(
        &self,
        member: MemberID,
        permissions: Permissions,
    ) -> VesselResult<ACLRecord> {
        self.author_acl(ACLPayload::ChangeRole {
            member,
            permissions,
        })
        .await
    }

    /// Revoke an outstanding invite (Admin+ only).
    pub async fn revoke_invite(&self, invite_key: InviteKey) -> VesselResult<ACLRecord> {
        self.author_acl(ACLPayload::RevokeInvite { invite_key })
            .await
    }

    /// Remove a member, rotating to a new epoch: the new read key is
    /// sealed to every remaining member and the old key is wrapped under it
    /// (backward chain). This Vessel's own key cache rotates forward
    /// afterward.
    pub async fn remove(
        &mut self,
        rng: &mut impl CryptoRNG,
        member: MemberID,
    ) -> VesselResult<ACLRecord> {
        let (current_epoch, wraps, prev_wrapped) = {
            let acl = self.replica.acl_index().await?;
            let state = acl.head_state().ok_or(ReplicaError::NoACL)?;
            let current_epoch = state.epoch;
            let current_key = self
                .read_keys
                .lock()
                .unwrap()
                .get(&current_epoch)
                .cloned()
                .ok_or(VesselError::NoKey(current_epoch))?;
            let next_key = ReadKey::generate(rng);
            let mut wraps = Vec::new();
            for (m, kem) in &state.member_kems {
                if *m == member {
                    continue;
                }
                wraps.push((*m, seal_key_to_member(rng, kem, &next_key)?));
            }
            let prev_wrapped = wrap_prev_key(rng, &next_key, &current_key);
            (current_epoch, wraps, prev_wrapped)
        };
        let record = self
            .author_acl(ACLPayload::Remove {
                member,
                new_epoch: current_epoch.next(),
                new_key_wrapped: wraps,
                prev_key_wrapped: prev_wrapped,
            })
            .await?;
        // We sealed the new epoch key to ourselves; pick it up.
        self.refresh_keys().await?;
        Ok(record)
    }
}

/// Bootstrap a Vessel replica from an ACL log (thin helper mirroring the
/// Station's).
pub async fn open_replica(path: &str, acl_log: &[ACLRecord]) -> VesselResult<Replica> {
    Ok(Replica::bootstrap(path, acl_log).await?)
}

// ---- read-key derivation (shared with subscription tasks) ----------------

/// The key-acquisition walk, factored free of `Vessel` so a live
/// subscription task can refresh the *shared* cache after an ACL push, with
/// the same logic [`Vessel::refresh_keys`] uses. Reads the log first (async),
/// then works the sealed boxes and the backwards chain under one short lock.
async fn derive_keys(
    replica: &Replica,
    kem: &KEMSecret,
    me: MemberID,
    cache: &StdMutex<BTreeMap<EpochID, ReadKey>>,
) -> VesselResult<()> {
    let records = replica.hold().acl_records().await?;
    let mut keys = cache.lock().unwrap();

    // Forward pass: unseal the one key sealed directly to me, and record
    // each rotation so the backward pass can chain through it. `fold`
    // tracks the epoch in force *before* each record is applied.
    let mut fold = ACLIndex::new();
    let mut rotations: Vec<(EpochID, EpochID, Vec<u8>)> = Vec::new();
    for record in &records {
        let body = record.parse_and_verify()?;
        let epoch_before = fold.head_state().map(|s| s.epoch).unwrap_or(EpochID::ZERO);
        match &body.payload {
            ACLPayload::Root {
                epoch0_key_wrapped, ..
            } if MemberID::from(&body.author) == me => {
                if let Ok(key) = open_sealed_key(kem, epoch0_key_wrapped) {
                    keys.insert(EpochID::ZERO, key);
                }
            }
            ACLPayload::Approve {
                member,
                current_key_wrapped,
                ..
            } if *member == me => {
                // Approve does not rotate: the key it seals is the epoch
                // already in force.
                if let Ok(key) = open_sealed_key(kem, current_key_wrapped) {
                    keys.insert(epoch_before, key);
                }
            }
            ACLPayload::Remove {
                new_epoch,
                new_key_wrapped,
                prev_key_wrapped,
                ..
            } => {
                if let Some((_, sealed)) = new_key_wrapped.iter().find(|(m, _)| *m == me)
                    && let Ok(key) = open_sealed_key(kem, sealed)
                {
                    keys.insert(*new_epoch, key);
                }
                rotations.push((*new_epoch, epoch_before, prev_key_wrapped.clone()));
            }
            _ => {}
        }
        fold.push(record)?;
    }

    // Backward pass: newest rotation first, so a key acquired at a high
    // epoch cascades down. Each Remove wraps epoch N−1's key under epoch
    // N's; if we hold N, we can recover N−1, and so on to genesis.
    for (new_epoch, epoch_before, prev_wrapped) in rotations.into_iter().rev() {
        if let Some(newer) = keys.get(&new_epoch).cloned()
            && let Ok(prev) = unwrap_prev_key(&newer, &prev_wrapped)
        {
            keys.entry(epoch_before).or_insert(prev);
        }
    }
    Ok(())
}

// ---- live-push subscription ------------------------------

/// The doorbell: a coarse "look again" ping *after* the
/// pushed records are already validated and stored. The store stays the
/// single source of truth — the app re-queries it (its topo cursor yields
/// everything new); events carry addresses of change, never payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionEvent {
    /// New validated Notices landed in these Tracks.
    Notices { tracks: Vec<TrackID> },
    /// The ACL advanced (keys already auto-refreshed).
    Acl,
    /// The session ended — any reason. Reconnect policy is the app's
    /// (resubscribing reconciles whatever was missed).
    Closed,
}

/// A live session to a Station: the background task ingests
/// pushes into the shared replica, auto-refreshes read keys after ACL
/// pushes, and rings the doorbell. Dropping the handle (or calling
/// [`close`](Self::close)) ends the session; a final
/// [`Closed`](SubscriptionEvent::Closed) event fires.
pub struct Subscription {
    push_tx: mpsc::Sender<Vec<SignedNotice>>,
}

impl Subscription {
    /// Hand a locally-stored Notice to the live session (typically right
    /// after [`Vessel::author`]). Errors if the session has ended.
    pub async fn push(&self, notice: SignedNotice) -> VesselResult<()> {
        self.push_tx
            .send(vec![notice])
            .await
            .map_err(|_| VesselError::Session("subscription ended".into()))
    }

    /// End the session. Equivalent to dropping the handle; explicit for
    /// clarity at call sites.
    pub fn close(self) {}
}

impl Vessel {
    /// Open a live session to a Station: handshake, scope to
    /// this Territory, run the same initial reconcile as
    /// [`sync_with`](Self::sync_with) — then stay. Returns the handle plus
    /// the doorbell channel. Admission follows the Station's policy; a
    /// non-member is refused after the ACL phase exactly as in a one-shot
    /// sync ([`SyncRefused`](VesselError::SyncRefused)).
    ///
    /// No auto-reconnect: when the session ends for any reason the channel
    /// yields [`Closed`](SubscriptionEvent::Closed) and the app decides.
    pub async fn subscribe(
        &self,
        station: impl Into<EndpointAddr>,
    ) -> Result<(Subscription, mpsc::Receiver<SubscriptionEvent>), VesselError> {
        self.subscribe_with(station, crate::fabric::Liveness::default())
            .await
    }

    /// [`subscribe`](Self::subscribe) with explicit liveness timings — tests
    /// inject short durations to exercise reaping and reconnect.
    pub async fn subscribe_with(
        &self,
        station: impl Into<EndpointAddr>,
        liveness: crate::fabric::Liveness,
    ) -> Result<(Subscription, mpsc::Receiver<SubscriptionEvent>), VesselError> {
        let mut session = self.endpoint.connect(station).await?;
        self.send_hello(&mut session).await?;
        let territory = self.replica.territory_id().await?;
        frame_write(&mut session, &SyncMessage::Subscribe { territory }).await?;
        match frame_read(&mut session).await? {
            SyncMessage::Ack { ok: true } => {}
            _ => return Err(VesselError::NotServed),
        }
        // Initial reconcile — identical to a one-shot sync, including the
        // admission gate and the key refresh.
        let result = initiate(&*self.replica, &mut session.send, &mut session.recv).await;
        let me = MemberID::from(&self.identity.verifying_key());
        derive_keys(&self.replica, &self.kem, me, &self.read_keys).await?;
        if let Err(SessionError::Refused) = result {
            session.close();
            return Err(VesselError::SyncRefused);
        }

        let (push_tx, push_rx) = mpsc::channel(16);
        let (event_tx, event_rx) = mpsc::channel(64);
        let worker = SubscriptionWorker {
            replica: Arc::clone(&self.replica),
            kem: Arc::clone(&self.kem),
            me,
            read_keys: Arc::clone(&self.read_keys),
            event_tx,
            liveness,
        };
        tokio::spawn(worker.run(session, push_rx));
        Ok((Subscription { push_tx }, event_rx))
    }
}

/// The state a live session's background task owns — shared views of the
/// Vessel's replica and key cache, plus the doorbell.
struct SubscriptionWorker {
    replica: Arc<Replica>,
    kem: Arc<KEMSecret>,
    me: MemberID,
    read_keys: Arc<StdMutex<BTreeMap<EpochID, ReadKey>>>,
    event_tx: mpsc::Sender<SubscriptionEvent>,
    liveness: crate::fabric::Liveness,
}

impl SubscriptionWorker {
    async fn run(self, mut session: Session, mut push_rx: mpsc::Receiver<Vec<SignedNotice>>) {
        let mut heartbeat = tokio::time::interval(self.liveness.heartbeat);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Idle deadline reset only by inbound frames — kept out of the read
        // future so the heartbeat tick does not keep restarting it.
        let idle = tokio::time::sleep(self.liveness.idle_timeout);
        tokio::pin!(idle);
        loop {
            tokio::select! {
                cmd = push_rx.recv() => match cmd {
                    Some(notices) => {
                        let frame = SyncMessage::NoticePush { notices };
                        if write_frame(&mut session.send, &frame)
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    // The handle was dropped/closed: end the session.
                    None => break,
                },
                frame = read_frame::<SyncMessage>(&mut session.recv) => {
                    idle
                        .as_mut()
                        .reset(tokio::time::Instant::now() + self.liveness.idle_timeout);
                    match frame {
                        // Empty NoticePush is a keepalive: proves the Station
                        // is alive, no payload to ingest.
                        Ok(SyncMessage::NoticePush { notices }) if notices.is_empty() => {}
                        Ok(SyncMessage::NoticePush { notices }) => self.on_notices(notices).await,
                        Ok(SyncMessage::ACLPush { records }) => self.on_acl(records).await,
                        Ok(_) => {} // unknown live frame: ignore
                        Err(_) => break, // peer closed / transport gone
                    }
                }
                _ = heartbeat.tick() => {
                    if write_frame(&mut session.send, &crate::fabric::keepalive())
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                // Silently-severed session (sleep, network change): no frame,
                // not even a keepalive, within the idle threshold. Dead.
                _ = &mut idle => break,
            }
        }
        session.close();
        let _ = self.event_tx.send(SubscriptionEvent::Closed).await;
    }

    /// Ingest pushed Notices (validating, idempotent), then ring the
    /// doorbell with the Tracks that actually gained stored Notices.
    async fn on_notices(&self, notices: Vec<SignedNotice>) {
        let ids: Vec<NoticeID> = notices
            .iter()
            .filter_map(|n| NoticeID::of(n).ok())
            .collect();
        if let Err(e) = self.replica.ingest(notices).await {
            tracing::warn!("subscription notice ingest failed: {e}");
            return;
        }
        let mut tracks = Vec::new();
        for id in ids {
            if let Ok(Some(stored)) = self.replica.hold().get_notice(id).await
                && !tracks.contains(&stored.track)
            {
                tracks.push(stored.track);
            }
        }
        if !tracks.is_empty() {
            // A full channel is fine: a pending event already says "look
            // again", and the topo-cursor pull picks these up too.
            let _ = self
                .event_tx
                .try_send(SubscriptionEvent::Notices { tracks });
        }
    }

    /// Ingest pushed ACL records, auto-refresh read keys (the sync_with
    /// invariant), then ring the doorbell.
    async fn on_acl(&self, records: Vec<OrderedRecord>) {
        if records.is_empty() {
            return;
        }
        if let Err(e) = self.replica.ingest_ordered(records).await {
            tracing::warn!("subscription acl ingest failed: {e}");
            return;
        }
        if let Err(e) = derive_keys(&self.replica, &self.kem, self.me, &self.read_keys).await {
            tracing::warn!("subscription key refresh failed: {e}");
        }
        let _ = self.event_tx.try_send(SubscriptionEvent::Acl);
    }
}

// ---- device pairing surface ------------------------------

/// Dial a Station and immediately hang up — the cheapest "is it there at
/// all" reachability check, for a device that has no Territory yet (so
/// [`Vessel::probe`] cannot run). Proves transport reachability only; says
/// nothing about what the Station serves.
pub async fn reach(device: &DeviceKey, station: impl Into<EndpointAddr>) -> VesselResult<()> {
    let endpoint = Endpoint::bind_identity(&device.signing).await?;
    let outcome = match endpoint.connect(station).await {
        Ok(mut session) => {
            session.close();
            Ok(())
        }
        Err(e) => Err(VesselError::from(e)),
    };
    endpoint.close().await;
    outcome
}

/// The out-of-band payload that moves a user onto a new device: the
/// recovery phrase (the identity), which Territory, and which Station to
/// fetch it from — everything [`fetch_acl_log`] + [`Vessel::from_identity`]
/// need on a fresh device. The app renders [`encode`](Self::encode)'s JSON
/// as a QR code / copyable string on a trusted device; treat it exactly
/// like the recovery phrase itself.
///
/// Lives here rather than outpost-protocol because it names a Station
/// address — a fabric concern the protocol layer sits below.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingPayload {
    pub v: u8,
    pub kind: String,
    pub mnemonic: String,
    /// Hex `TerritoryID` — hex rather than serde's byte array, for a
    /// compact QR-friendly encoding.
    pub territory: String,
    /// The Station's dialable address.
    pub station: EndpointAddr,
}

/// The `kind` discriminator of [`PairingPayload`].
pub const PAIRING_KIND: &str = "waypoints-pairing";
/// The current [`PairingPayload`] version.
pub const PAIRING_VERSION: u8 = 1;

#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("not a Waypoints pairing code")]
    Unrecognized,
    #[error("unsupported pairing code version {0}")]
    Version(u8),
    #[error("malformed pairing code: {0}")]
    Encoding(String),
}

impl PairingPayload {
    pub fn new(mnemonic: String, territory: TerritoryID, station: EndpointAddr) -> Self {
        Self {
            v: PAIRING_VERSION,
            kind: PAIRING_KIND.to_string(),
            mnemonic,
            territory: territory.to_string(),
            station,
        }
    }

    /// The JSON form the app puts in a QR code / on the clipboard.
    pub fn encode(&self) -> Result<String, PairingError> {
        serde_json::to_string(self).map_err(|e| PairingError::Encoding(e.to_string()))
    }

    /// Parse and validate a pairing code (kind and version are checked; the
    /// mnemonic and Territory are validated by their own consumers).
    pub fn decode(code: &str) -> Result<Self, PairingError> {
        let payload: Self =
            serde_json::from_str(code.trim()).map_err(|e| PairingError::Encoding(e.to_string()))?;
        if payload.kind != PAIRING_KIND {
            return Err(PairingError::Unrecognized);
        }
        if payload.v != PAIRING_VERSION {
            return Err(PairingError::Version(payload.v));
        }
        Ok(payload)
    }

    pub fn territory_id(&self) -> Result<TerritoryID, PairingError> {
        let bytes =
            hex::decode(&self.territory).map_err(|e| PairingError::Encoding(e.to_string()))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| PairingError::Encoding("territory id must be 32 bytes".into()))?;
        Ok(TerritoryID::from_bytes(bytes))
    }
}

/// Fetch a Territory's full ordered ACL log from a Station that serves it,
/// **without** holding a replica yet — the bootstrap step for a new device
/// that knows only the `TerritoryID` (from a pairing payload) and a Station
/// address. Same-mnemonic devices need no admission, but they do need
/// the log to build a [`Replica`] before a [`Vessel`] can open; feed the
/// returned records to [`open_replica`].
///
/// Runs Hello → Open → the initiator's ACL phase from position 0, then ends
/// the session before the Notice phase (the first real sync fills that in).
pub async fn fetch_acl_log(
    identity: &Identity,
    device: &DeviceKey,
    station: impl Into<EndpointAddr>,
    territory: TerritoryID,
) -> VesselResult<Vec<ACLRecord>> {
    let endpoint = Endpoint::bind_identity(&device.signing).await?;
    let mut session = endpoint.connect(station).await?;

    frame_write(
        &mut session,
        &SyncMessage::Hello {
            identity: identity.verifying_key(),
            cert: device.cert.clone(),
        },
    )
    .await?;
    match frame_read(&mut session).await? {
        SyncMessage::Ack { ok: true } => {}
        _ => return Err(VesselError::UntrustedStation),
    }

    frame_write(&mut session, &SyncMessage::Open { territory }).await?;
    match frame_read(&mut session).await? {
        SyncMessage::Ack { ok: true } => {}
        _ => return Err(VesselError::NotServed),
    }

    frame_write(&mut session, &SyncMessage::ACLRequest { len: 0 }).await?;
    let records = match frame_read(&mut session).await? {
        SyncMessage::ACLResponse { records, .. } => records,
        _ => return Err(VesselError::Session("unexpected ACL reply".into())),
    };
    // Finish the ACL phase politely (we have nothing to push) before ending
    // the session ahead of the Notice phase.
    frame_write(&mut session, &SyncMessage::ACLPush { records: vec![] }).await?;
    session.close();
    endpoint.close().await;

    Ok(records.into_iter().map(|o| o.record).collect())
}
