//! Test support, compiled only with the `test-utils` feature.

use sea_orm::DatabaseConnection;

/// Open a fresh, fully-migrated database in a new temporary directory.
///
/// Refinery migrations run through turso against a file path, so tests
/// can't use `sqlite::memory:` (the migration connection and the sea-orm
/// connection would see different databases). The tempdir is intentionally
/// kept (not cleaned up on drop) so lazily-opened pool connections stay
/// valid for the duration of the test process; the OS reclaims it.
pub async fn temp_db() -> DatabaseConnection {
    let dir = tempfile::tempdir().expect("create tempdir").keep();
    crate::tools::open(&dir.join("test.db"))
        .await
        .expect("open migrated test db")
}
