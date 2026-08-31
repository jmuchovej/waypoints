use std::path::Path;

use crate::error::StoreResult;
use crate::store::Store;

/// Open (or create) the store at `path`. Runs bedrock DDL + seeds builtins.
pub async fn open(path: &Path) -> StoreResult<Store> {
    Store::open(path).await
}

/// Wipe and recreate the store at `path`, returning a fresh `Store`.
///
/// Drops the existing store, deletes the database file, and calls
/// `Store::open` which re-runs `ensure_collection` and re-seeds builtins.
pub async fn reset(_old_store: Store, path: &Path) -> StoreResult<Store> {
    tracing::info!(path = %path.display(), "resetting database");

    // Drop the store first so no connections hold the file.
    drop(_old_store);

    // Remove the SQLite database file (and any WAL/SHM siblings).
    for suffix in ["", "-wal", "-shm"] {
        let p = if suffix.is_empty() {
            path.to_path_buf()
        } else {
            let mut s = path.as_os_str().to_owned();
            s.push(suffix);
            std::path::PathBuf::from(s)
        };
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }

    tracing::info!("database files removed; re-opening");
    Store::open(path).await
}

/// Checkpoint the write-ahead log.
///
/// Closing the bedrock engine does *not* checkpoint — it only releases
/// connections — which is why this explicit flush at territory-close time
/// exists. Committed writes are already durable either way; checkpointing
/// here trades a little close-time work for a compact log and a faster
/// reopen.
pub async fn checkpoint(store: &Store) {
    if let Err(e) = store.db().flush().await {
        tracing::warn!("flush failed: {e}");
    } else {
        tracing::info!("store flushed");
    }
}
