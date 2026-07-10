//! Test support, compiled only with the `test-utils` feature.

use crate::store::Store;

/// Open a fresh, fully-migrated `Store` in a new temporary directory.
///
/// The tempdir is intentionally kept (not cleaned up on drop) so lazily-opened
/// connections stay valid for the duration of the test process; the OS reclaims it.
pub async fn temp_db() -> Store {
    let dir = tempfile::tempdir().expect("create tempdir").keep();
    crate::tools::open(&dir.join("test.db"))
        .await
        .expect("open store")
}
