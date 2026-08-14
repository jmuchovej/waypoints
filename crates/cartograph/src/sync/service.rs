//! The app-side driver of outpost's Vessel role (outpost design §12): this
//! device's identity, its sync Territory, and its relationship to a Station.
//! Outpost offers the capability; this service orchestrates it for the UI —
//! key custody, on-disk layout, and pairing are app concerns, not protocol
//! ones.
//!
//! On-disk layout under the sync root (CLI-compatible: `outpost sync --data
//! <sync_root>` reads the same files):
//! - `mnemonic.txt` — the BIP-39 recovery phrase (the identity)
//! - `territory.db` — the Territory hold (replica storage)
//! - `sync.json`   — device index, Station address, cached Territory id

use crate::{IdentityCreated, StationStatus, SyncDevice, SyncError, SyncResult, SyncStatus};
use outpost::{
    fabric::EndpointAddr,
    hold::TerritoryHold,
    protocol::{DeviceKey, Identity, KeyRoot, MemberID, TerritoryID},
    replica::Replica,
    vessel::{PairingPayload, Vessel, fetch_acl_log, open_replica, reach},
};
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Instant};
use tokio::sync::{Mutex, MutexGuard};

/// Per-device sync configuration persisted beside the identity.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncConfig {
    device_index: u32,
    station: Option<EndpointAddr>,
    /// Hex `TerritoryID`, cached at create/join time so `status()` needs no
    /// database open.
    territory: Option<String>,
}

pub struct SyncService {
    root: PathBuf,
    /// The live Vessel (an open replica + bound endpoint), built lazily and
    /// dropped whenever identity or Territory change underneath it.
    vessel: Mutex<Option<Vessel>>,
}

impl SyncService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            vessel: Mutex::new(None),
        }
    }

    // ---- paths & small persistence ------------------------------------

    fn mnemonic_path(&self) -> PathBuf {
        self.root.join("mnemonic.txt")
    }

    fn territory_db_path(&self) -> PathBuf {
        self.root.join("territory.db")
    }

    fn config_path(&self) -> PathBuf {
        self.root.join("sync.json")
    }

    fn load_config(&self) -> SyncResult<SyncConfig> {
        match std::fs::read(self.config_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| SyncError::StationAddr(format!("corrupt sync.json: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SyncConfig::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn save_config(&self, config: &SyncConfig) -> SyncResult<()> {
        std::fs::create_dir_all(&self.root)?;
        let bytes = serde_json::to_vec_pretty(config)
            .map_err(|e| SyncError::StationAddr(format!("encode sync.json: {e}")))?;
        std::fs::write(self.config_path(), bytes)?;
        Ok(())
    }

    /// Read the recovery phrase, if this device has an identity.
    fn load_mnemonic(&self) -> SyncResult<Option<String>> {
        match std::fs::read_to_string(self.mnemonic_path()) {
            Ok(phrase) => Ok(Some(phrase.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Persist the recovery phrase. Plaintext, owner-only permissions — the
    /// same custody posture as the operator CLI; OS-keychain custody is a
    /// planned refinement.
    fn save_mnemonic(&self, phrase: &str) -> SyncResult<()> {
        std::fs::create_dir_all(&self.root)?;
        let path = self.mnemonic_path();
        std::fs::write(&path, phrase)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    fn load_root(&self) -> SyncResult<Option<KeyRoot>> {
        let Some(phrase) = self.load_mnemonic()? else {
            return Ok(None);
        };
        KeyRoot::from_mnemonic(&phrase)
            .map(Some)
            .map_err(|e| SyncError::Mnemonic(e.to_string()))
    }

    /// The identity + this device's key, or `NoIdentity`.
    fn identity_pair(&self) -> SyncResult<(Identity, DeviceKey)> {
        let root = self.load_root()?.ok_or(SyncError::NoIdentity)?;
        let index = self.load_config()?.device_index;
        Ok((root.identity(), root.device(index)))
    }

    fn station(&self) -> SyncResult<EndpointAddr> {
        self.load_config()?.station.ok_or(SyncError::NoStation)
    }

    // ---- vessel lifecycle ----------------------------------------------

    /// Lock the vessel slot, opening the Vessel from disk on first use.
    /// Requires both an identity and a Territory.
    async fn vessel(&self) -> SyncResult<MutexGuard<'_, Option<Vessel>>> {
        let mut guard = self.vessel.lock().await;
        if guard.is_none() {
            let (identity, device) = self.identity_pair()?;
            let db = self.territory_db_path();
            if !db.exists() {
                return Err(SyncError::NoTerritory);
            }
            let hold =
                TerritoryHold::open(db.to_str().ok_or_else(|| {
                    SyncError::Io(std::io::Error::other("non-utf8 sync root path"))
                })?)
                .await
                .map_err(outpost::vessel::VesselError::from)?;
            let replica = Replica::reopen(hold)
                .await
                .map_err(outpost::vessel::VesselError::from)?;
            *guard = Some(Vessel::from_identity(&identity, &device, replica).await?);
        }
        Ok(guard)
    }

    /// Drop the cached Vessel — call after anything that changes identity or
    /// Territory on disk.
    async fn reset_vessel(&self) {
        *self.vessel.lock().await = None;
    }

    // ---- status ---------------------------------------------------------

    pub async fn status(&self) -> SyncResult<SyncStatus> {
        let config = self.load_config()?;
        let root = self.load_root()?;
        let station_addr = config
            .station
            .as_ref()
            .and_then(|a| serde_json::to_string(a).ok());
        Ok(SyncStatus {
            has_identity: root.is_some(),
            member_id: root.as_ref().map(|r| r.identity().member_id().to_string()),
            device_index: root.as_ref().map(|_| config.device_index),
            has_territory: self.territory_db_path().exists(),
            territory_id: config.territory,
            station_configured: config.station.is_some(),
            station_addr,
        })
    }

    // ---- identity (onboarding) ------------------------------------------

    /// Mint a fresh identity for this device. Refuses to overwrite an
    /// existing one — the mnemonic is the only backup there is.
    pub async fn identity_create(&self) -> SyncResult<IdentityCreated> {
        if self.load_mnemonic()?.is_some() {
            return Err(SyncError::IdentityExists);
        }
        let (root, mnemonic) = KeyRoot::generate();
        self.save_mnemonic(&mnemonic)?;
        self.save_config(&SyncConfig::default())?;
        self.reset_vessel().await;
        Ok(IdentityCreated {
            mnemonic,
            member_id: root.identity().member_id().to_string(),
        })
    }

    /// Adopt an existing identity from its recovery phrase (same-mnemonic
    /// device onboarding, or plain recovery). The phrase is validated before
    /// anything touches disk.
    pub async fn identity_import(&self, phrase: String) -> SyncResult<SyncStatus> {
        if self.load_mnemonic()?.is_some() {
            return Err(SyncError::IdentityExists);
        }
        let phrase = phrase.trim().to_string();
        let root =
            KeyRoot::from_mnemonic(&phrase).map_err(|e| SyncError::Mnemonic(e.to_string()))?;
        self.save_mnemonic(&phrase)?;
        // A joining device gets a fresh random hardened index (no
        // coordinator hands out sequential ones) — the policy lives in
        // outpost's identity module.
        let index = root.random_device(&mut rand::rng()).index;
        self.save_config(&SyncConfig {
            device_index: index,
            ..SyncConfig::default()
        })?;
        self.reset_vessel().await;
        self.status().await
    }

    /// Reveal the recovery phrase (for backup display and the add-device
    /// flow).
    pub async fn identity_mnemonic(&self) -> SyncResult<String> {
        self.load_mnemonic()?.ok_or(SyncError::NoIdentity)
    }

    // ---- territory -------------------------------------------------------

    /// Create this identity's sync Territory on this device. Registers it
    /// with the configured Station when there is one (best-effort — the
    /// Territory exists locally regardless).
    pub async fn territory_create(&self) -> SyncResult<SyncStatus> {
        let (identity, device) = self.identity_pair()?;
        if self.territory_db_path().exists() {
            return Err(SyncError::TerritoryExists);
        }
        std::fs::create_dir_all(&self.root)?;
        let db = self.territory_db_path();
        let path = db
            .to_str()
            .ok_or_else(|| SyncError::Io(std::io::Error::other("non-utf8 sync root path")))?;

        // A seeded `StdRng` rather than the thread rng: the handle must be
        // `Send` for the command future that holds it across awaits.
        let mut rng = StdRng::from_rng(&mut rand::rng());
        let vessel = Vessel::create_territory(&identity, &device, path, &mut rng).await?;
        let territory = vessel.territory_id().await?;

        let mut config = self.load_config()?;
        config.territory = Some(territory.to_string());
        self.save_config(&config)?;

        if let Some(station) = config.station
            && let Err(e) = vessel.register(station, territory).await
        {
            tracing::warn!("territory created but station registration failed: {e}");
        }

        *self.vessel.lock().await = Some(vessel);
        self.status().await
    }

    // ---- station ---------------------------------------------------------

    /// Configure the Station to sync with. Accepts the JSON form of
    /// `EndpointAddr` — the exact content `station-run` publishes to its
    /// `station.addr` file. Probes it before returning.
    pub async fn station_set(&self, addr_json: String) -> SyncResult<StationStatus> {
        let addr: EndpointAddr = serde_json::from_str(addr_json.trim())
            .map_err(|e| SyncError::StationAddr(e.to_string()))?;
        let mut config = self.load_config()?;
        config.station = Some(addr);
        self.save_config(&config)?;
        self.station_status().await
    }

    /// Probe the configured Station — the connection-status "ping". With a
    /// Territory open this asks whether the Station serves it; without one it
    /// only measures reachability.
    pub async fn station_status(&self) -> SyncResult<StationStatus> {
        let Some(station) = self.load_config()?.station else {
            return Ok(StationStatus {
                configured: false,
                reachable: false,
                serves_territory: false,
                latency_ms: None,
                error: None,
            });
        };

        let started = Instant::now();
        let outcome: Result<bool, String> = if self.territory_db_path().exists() {
            match self.vessel().await {
                Ok(mut guard) => {
                    let vessel = guard.as_mut().expect("vessel opened above");
                    vessel.probe(station).await.map_err(|e| e.to_string())
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            // No Territory yet — a bare dial still proves the Station is
            // there.
            match self.identity_pair() {
                Ok((_, device)) => reach(&device, station)
                    .await
                    .map(|()| false)
                    .map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        };
        let latency = u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX);

        Ok(match outcome {
            Ok(serves) => StationStatus {
                configured: true,
                reachable: true,
                serves_territory: serves,
                latency_ms: Some(latency),
                error: None,
            },
            Err(error) => StationStatus {
                configured: true,
                reachable: false,
                serves_territory: false,
                latency_ms: None,
                error: Some(error),
            },
        })
    }

    /// Register the Territory with the configured Station (idempotent) and
    /// run one full reconcile.
    pub async fn sync_now(&self) -> SyncResult<SyncStatus> {
        let station = self.station()?;
        let mut guard = self.vessel().await?;
        let vessel = guard.as_mut().expect("vessel opened above");
        let territory = vessel.territory_id().await?;
        vessel.register(station.clone(), territory).await?;
        vessel.sync_with(station).await?;
        drop(guard);
        self.status().await
    }

    // ---- device registry -------------------------------------------------

    /// The Station's device registry for this Territory: every sighted
    /// Vessel, attribution verified client-side. Own devices sort first.
    pub async fn devices(&self) -> SyncResult<Vec<SyncDevice>> {
        let station = self.station()?;
        let (identity, device) = self.identity_pair()?;
        let my_member = identity.member_id().to_string();
        let my_device = hex::encode(device.signing.verifying_key().to_bytes());

        let guard = self.vessel().await?;
        let vessel = guard.as_ref().expect("vessel opened above");
        let sightings = vessel.device_sightings(station).await?;
        drop(guard);

        let mut devices: Vec<SyncDevice> = sightings
            .into_iter()
            .map(|s| {
                let member_id = MemberID::from(&s.identity).to_string();
                let device_key = hex::encode(s.device().to_bytes());
                SyncDevice {
                    mine: member_id == my_member,
                    this_device: device_key == my_device,
                    member_id,
                    device_key,
                    first_seen: s.first_seen as f64,
                    last_seen: s.last_seen as f64,
                    connected: s.connected,
                }
            })
            .collect();
        devices.sort_by(|a, b| {
            (b.mine, b.connected, b.last_seen)
                .partial_cmp(&(a.mine, a.connected, a.last_seen))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(devices)
    }

    // ---- pairing (add a device) -----------------------------------------

    /// The pairing payload to render as a QR code / copyable code on this
    /// (trusted, already-onboarded) device. Carries the recovery phrase —
    /// the caller must treat it with the same care as the phrase itself.
    pub async fn pairing_payload(&self) -> SyncResult<String> {
        let mnemonic = self.load_mnemonic()?.ok_or(SyncError::NoIdentity)?;
        let config = self.load_config()?;
        let territory = parse_territory(&config.territory.ok_or(SyncError::NoTerritory)?)?;
        let station = config.station.ok_or(SyncError::NoStation)?;
        PairingPayload::new(mnemonic, territory, station)
            .encode()
            .map_err(|e| SyncError::Pairing(e.to_string()))
    }

    /// The pairing code ready for display: URL envelope + QR SVG (liquid
    /// style — an on-screen, short-lived code can afford the styling; the
    /// Emergency Kit keeps plain squares).
    pub async fn pairing_code(&self) -> SyncResult<super::PairingCode> {
        let payload = self.pairing_payload().await?;
        let url = super::encode_pair_url(&payload);
        let qr_svg = super::setup_code::qr_svg_liquid(&url)?;
        Ok(super::PairingCode { url, qr_svg })
    }

    /// QR of the recovery phrase itself, for the Emergency Kit — scan it to
    /// avoid typing 24 words. Deliberately NOT a URL: the kit is a
    /// decade-scale paper artifact and the phrase must never route through
    /// a browser.
    pub async fn recovery_qr(&self) -> SyncResult<String> {
        let phrase = self.identity_mnemonic().await?;
        super::setup_code::qr_svg(&phrase)
    }

    /// Classify-and-dispatch for anything a paste, scan, or deep link hands
    /// the setup flow — the single entry point every platform shares.
    pub async fn submit_setup_code(&self, input: String) -> SyncResult<SyncStatus> {
        match super::classify(&input) {
            super::SetupCode::Pairing(code) => self.pairing_join(code).await,
            super::SetupCode::Phrase(phrase) => self.identity_import(phrase).await,
            super::SetupCode::Unknown => Err(SyncError::UnrecognizedSetupCode),
        }
    }

    /// Consume a pairing payload on a fresh device: adopt the identity, point
    /// at the Station, fetch the Territory's ACL log to bootstrap a local
    /// replica, and run the first full sync.
    pub async fn pairing_join(&self, code: String) -> SyncResult<SyncStatus> {
        if self.load_mnemonic()?.is_some() {
            return Err(SyncError::IdentityExists);
        }
        if self.territory_db_path().exists() {
            return Err(SyncError::TerritoryExists);
        }

        let payload =
            PairingPayload::decode(&code).map_err(|e| SyncError::Pairing(e.to_string()))?;
        let root = KeyRoot::from_mnemonic(payload.mnemonic.trim())
            .map_err(|e| SyncError::Mnemonic(e.to_string()))?;
        let territory = payload
            .territory_id()
            .map_err(|e| SyncError::Pairing(e.to_string()))?;

        // Fresh random hardened device index — see `identity_import`.
        let identity = root.identity();
        let device = root.random_device(&mut rand::rng());
        let index = device.index;

        // Bootstrap the replica from the Station's copy of the ACL log
        // before anything is persisted locally.
        let log = fetch_acl_log(&identity, &device, payload.station.clone(), territory).await?;
        std::fs::create_dir_all(&self.root)?;
        let db = self.territory_db_path();
        let path = db
            .to_str()
            .ok_or_else(|| SyncError::Io(std::io::Error::other("non-utf8 sync root path")))?;
        let replica = open_replica(path, &log).await?;
        let mut vessel = Vessel::from_identity(&identity, &device, replica).await?;
        vessel.sync_with(payload.station.clone()).await?;

        self.save_mnemonic(payload.mnemonic.trim())?;
        self.save_config(&SyncConfig {
            device_index: index,
            station: Some(payload.station),
            territory: Some(territory.to_string()),
        })?;
        *self.vessel.lock().await = Some(vessel);
        self.status().await
    }
}

fn parse_territory(hex_id: &str) -> SyncResult<TerritoryID> {
    let bytes = hex::decode(hex_id).map_err(|e| SyncError::Pairing(e.to_string()))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| SyncError::Pairing("territory id must be 32 bytes".into()))?;
    Ok(TerritoryID::from_bytes(bytes))
}
