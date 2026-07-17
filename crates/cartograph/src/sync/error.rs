use serde::Serialize;

pub type SyncResult<T> = ::std::result::Result<T, SyncError>;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("no identity on this device yet — create or import one first")]
    NoIdentity,
    #[error("an identity already exists on this device")]
    IdentityExists,
    #[error("invalid recovery phrase: {0}")]
    Mnemonic(String),
    #[error("no sync territory on this device yet — create or join one first")]
    NoTerritory,
    #[error("a sync territory already exists on this device")]
    TerritoryExists,
    #[error("no station configured")]
    NoStation,
    #[error("invalid station address: {0}")]
    StationAddr(String),
    #[error("invalid pairing code: {0}")]
    Pairing(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Vessel(#[from] outpost::vessel::VesselError),
}

impl Serialize for SyncError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl specta::Type for SyncError {
    fn definition(_types: &mut specta::Types) -> specta::datatype::DataType {
        specta::datatype::DataType::Primitive(specta::datatype::Primitive::str)
    }
}
