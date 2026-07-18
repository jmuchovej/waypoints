use serde::{Deserialize, Serialize};

/// Where this device stands in the sync lifecycle — the UI's single source
/// for gating onboarding vs. settings surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub has_identity: bool,
    /// The identity's `MemberID` as hex (the user ID — the key *is* the
    /// account).
    pub member_id: Option<String>,
    /// This device's key-derivation index under the identity.
    pub device_index: Option<u32>,
    pub has_territory: bool,
    /// The sync Territory's ID as hex.
    pub territory_id: Option<String>,
    pub station_configured: bool,
    /// The configured Station address, as its JSON form (what `station-run`
    /// publishes to `station.addr`).
    pub station_addr: Option<String>,
}

/// The Add-device pairing code, ready to display: the shareable URL form
/// and its QR rendering. Treat both like the recovery phrase itself.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    /// `https://waypoints.so/pair#…` — the copyable link.
    pub url: String,
    /// SVG of the same URL, for direct rendering.
    pub qr_svg: String,
}

/// A freshly minted identity. The mnemonic is returned exactly once, for the
/// reveal-and-back-up step — it is the only backup that exists.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreated {
    pub mnemonic: String,
    pub member_id: String,
}

/// Result of probing the configured Station — the "ping".
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StationStatus {
    pub configured: bool,
    pub reachable: bool,
    /// Whether the Station serves this device's Territory (false until
    /// registered, or when probing without a local Territory).
    pub serves_territory: bool,
    pub latency_ms: Option<u32>,
    pub error: Option<String>,
}

/// One sighted device (Vessel) from the Station's device registry, with its
/// attribution verified client-side against the claimed identity.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncDevice {
    /// The identity behind the device, as hex `MemberID`.
    pub member_id: String,
    /// The device's public key (its VesselID), as hex.
    pub device_key: String,
    /// Belongs to this device's identity.
    pub mine: bool,
    /// Is this very device.
    pub this_device: bool,
    /// Station-attested unix seconds — presence metadata, not verified.
    /// `f64` because specta forbids `u64` at the IPC boundary.
    pub first_seen: f64,
    pub last_seen: f64,
    /// Live-session presence: true only while the device holds an open
    /// session with the Station.
    pub connected: bool,
}
