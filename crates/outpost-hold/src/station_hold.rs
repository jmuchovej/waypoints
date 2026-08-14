//! Station-scope storage: the records a *Station itself* owns, as opposed to the per-Territory
//! records in a [`TerritoryHold`](crate::TerritoryHold). One bedrock database
//! (`station.db`) beside the Territory holds.
//!
//! First residents: **device sightings** (visibility — a restart must not
//! make a dormant device vanish from the "which devices have access" list)
//! and **registrations** (who first homed a Territory here, when). Presence
//! (`connected`) is deliberately *not* persisted — it is runtime truth,
//! rebuilt `false` on load.

use crate::{
    HoldError,
    protocol::{DeviceSighting, MemberID, TerritoryID},
};
use bedrock::{Collection, Database, Scalar};
use ed25519_dalek::VerifyingKey;

/// The sighting's synthetic 32-byte primary key:
/// `BLAKE3(territory ‖ member ‖ device)` — one row per certified device per
/// identity per Territory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Scalar)]
pub struct SightingKey(pub [u8; 32]);

fn sighting_key(territory: &TerritoryID, member: &MemberID, device: &[u8; 32]) -> SightingKey {
    let mut hasher = blake3::Hasher::new();
    hasher.update(territory.as_bytes());
    hasher.update(member.as_bytes());
    hasher.update(device);
    SightingKey(*hasher.finalize().as_bytes())
}

/// A Territory's 32-byte id as a binary primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Scalar)]
pub struct TerritoryKey(pub [u8; 32]);

/// One persisted device sighting. The wire/type of record is
/// [`DeviceSighting`]; this row denormalizes it for storage plus the
/// Territory it was sighted in. `connected` is absent by design.
#[derive(Debug, Clone, PartialEq, Collection)]
pub struct SightingRow {
    #[bedrock(id)]
    pub id: SightingKey,
    #[bedrock(index)]
    pub territory: [u8; 32],
    pub identity: [u8; 32],
    /// Postcard-encoded [`DeviceCert`](outpost_protocol::DeviceCert).
    pub cert: Vec<u8>,
    pub first_seen: i64,
    pub last_seen: i64,
}

/// Who first homed a Territory on this Station, and when (attribution).
/// The holds on disk remain the authoritative *serving* registry; this row
/// only remembers the act of registration.
#[derive(Debug, Clone, PartialEq, Collection)]
pub struct RegistrationRow {
    #[bedrock(id)]
    pub id: TerritoryKey,
    pub registered_by: [u8; 32],
    pub registered_at: i64,
}

/// The Station's own database. Absent on a seeded, storage-less Station
/// ([`Station::new`]-style embeds stay memory-only).
///
/// [`Station::new`]: crate — the role crate's constructor; see its docs.
pub struct StationHold {
    db: Database,
}

impl StationHold {
    /// Open (creating if absent) the Station database at `path`.
    pub async fn open(path: &str) -> Result<Self, HoldError> {
        let db = Database::open(path).await?;
        db.ensure_collection::<SightingRow>().await?;
        db.ensure_collection::<RegistrationRow>().await?;
        Ok(Self { db })
    }

    /// Write-through for one sighting: insert on first sight, refresh on a
    /// re-sight.
    pub async fn upsert_sighting(
        &self,
        territory: &TerritoryID,
        sighting: &DeviceSighting,
    ) -> Result<(), HoldError> {
        let member = MemberID::from(&sighting.identity);
        let device = sighting.device().to_bytes();
        self.db
            .upsert(SightingRow {
                id: sighting_key(territory, &member, &device),
                territory: *territory.as_bytes(),
                identity: sighting.identity.to_bytes(),
                cert: postcard::to_allocvec(&sighting.cert)?,
                first_seen: sighting.first_seen as i64,
                last_seen: sighting.last_seen as i64,
            })
            .await?;
        Ok(())
    }

    /// Every persisted sighting, with the Territory it belongs to —
    /// `connected` rebuilt as `false` (nobody is live right after a load).
    /// A row whose cert bytes fail to decode is skipped, not fatal.
    pub async fn sightings(&self) -> Result<Vec<(TerritoryID, DeviceSighting)>, HoldError> {
        let rows = self.db.all::<SightingRow>().await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let identity = VerifyingKey::from_bytes(&row.identity).ok()?;
                let cert = postcard::from_bytes(&row.cert).ok()?;
                Some((
                    TerritoryID::from_bytes(row.territory),
                    DeviceSighting {
                        identity,
                        cert,
                        first_seen: row.first_seen as u64,
                        last_seen: row.last_seen as u64,
                        connected: false,
                    },
                ))
            })
            .collect())
    }

    /// Remember who first homed `territory` here. First write wins — an
    /// idempotent re-registration does not rewrite history.
    pub async fn record_registration(
        &self,
        territory: &TerritoryID,
        registered_by: &MemberID,
        registered_at: i64,
    ) -> Result<(), HoldError> {
        let key = TerritoryKey(*territory.as_bytes());
        if self.db.get::<RegistrationRow>(&key).await?.is_some() {
            return Ok(());
        }
        self.db
            .insert(RegistrationRow {
                id: key,
                registered_by: *registered_by.as_bytes(),
                registered_at,
            })
            .await?;
        Ok(())
    }

    /// The registration record for `territory`, if one was ever made.
    pub async fn registration(
        &self,
        territory: &TerritoryID,
    ) -> Result<Option<RegistrationRow>, HoldError> {
        self.db
            .get::<RegistrationRow>(&TerritoryKey(*territory.as_bytes()))
            .await
            .map_err(HoldError::from)
    }
}
