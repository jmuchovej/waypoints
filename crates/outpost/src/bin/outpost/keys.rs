//! Key files: 32 raw bytes, an Ed25519 seed, for a Station's identity and a
//! Fleet's signing key alike. The role is decided by which flag a file is
//! handed to, never by its contents.

use anyhow::{Context as _, Result, anyhow};
use ed25519_dalek::SigningKey;
use std::{io::Write as _, path::Path};

/// Load a key an operator provisioned. The file must exist and hold exactly
/// 32 bytes; nothing is ever written here — a read-only Secret mount is the
/// expected home, and inventing a key at a path the operator named would
/// silently replace an identity.
pub(crate) fn load(path: &Path) -> Result<SigningKey> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading key file {}", path.display()))?;
    let seed: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        anyhow!(
            "key file {} holds {} bytes, expected 32",
            path.display(),
            bytes.len()
        )
    })?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Load the key at `path`, generating and persisting one if there is none —
/// the development convenience for the default location inside the data
/// directory. A file that exists but is malformed is an error, not replaced.
pub(crate) fn load_or_generate(path: &Path) -> Result<SigningKey> {
    if path.exists() {
        return load(path);
    }
    let key = SigningKey::generate(&mut rand::rng());
    write_new(path, &key.to_bytes())?;
    Ok(key)
}

/// Create `path` exclusively — failing if it already exists — with mode
/// `0600`, and write the seed into it.
pub(crate) fn write_new(path: &Path, seed: &[u8; 32]) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("creating key file {}", path.display()))?;
    file.write_all(seed)
        .with_context(|| format!("writing key file {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_provisioned_key_is_an_error_and_nothing_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station.key");
        let err = load(&path).unwrap_err();
        assert!(err.to_string().contains("station.key"), "{err}");
        assert!(!path.exists(), "load never creates a file");
    }

    #[test]
    fn a_short_key_file_is_an_error_naming_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.key");
        std::fs::write(&path, [7u8; 31]).unwrap();
        let err = load(&path).unwrap_err().to_string();
        assert!(err.contains("fleet.key") && err.contains("31"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_key_file_loads() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station.key");
        std::fs::write(&path, [0x5A; 32]).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        let key = load(&path).unwrap();
        assert_eq!(key.to_bytes(), [0x5A; 32]);
    }

    #[test]
    fn the_default_location_generates_once_and_reloads_the_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("station.key");
        let first = load_or_generate(&path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);
        let second = load_or_generate(&path).unwrap();
        assert_eq!(first.to_bytes(), second.to_bytes());
    }

    #[test]
    fn write_new_is_exclusive_and_private() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.key");
        write_new(&path, &[0xF1; 32]).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), [0xF1; 32]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "mode {mode:o}");
        }
        let err = write_new(&path, &[0xEE; 32]).unwrap_err();
        assert!(err.to_string().contains("fleet.key"), "{err}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            [0xF1; 32],
            "a refused overwrite leaves the file untouched"
        );
    }
}
