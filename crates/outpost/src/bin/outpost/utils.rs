use anyhow::{Context as _, Result};
use outpost::protocol::{DeviceKey, Identity, KeyRoot};
use std::path::{Path, PathBuf};

pub(crate) fn mnemonic_path(data: &Path) -> PathBuf {
    data.join("mnemonic.txt")
}

pub(crate) fn territory_db(data: &Path) -> PathBuf {
    data.join("territory.db")
}

/// Load the identity + first device key from the data directory's mnemonic.
/// The app writes the mnemonic; this only reads it.
pub(crate) fn load_identity(data: &Path) -> Result<(Identity, DeviceKey)> {
    let phrase = std::fs::read_to_string(mnemonic_path(data)).with_context(|| {
        format!(
            "no identity in {} (expected a mnemonic written by the app)",
            data.display()
        )
    })?;
    let root = KeyRoot::from_mnemonic(phrase.trim())?;
    Ok((root.identity(), root.device(0)))
}
