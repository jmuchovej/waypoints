//! How a database is opened: read-pool size, per-connection pragmas, and the
//! enforced schema version.

use std::collections::BTreeMap;

/// Options for [`Database::open_with`](crate::Database::open_with).
///
/// [`Config::default`] is what plain [`Database::open`](crate::Database::open)
/// uses: a handful of read connections, WAL journaling on file-backed
/// databases, and version `0`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Upper bound on pooled read connections, created lazily as reads
    /// contend. turso allows one in-flight operation per connection, so this
    /// is the read concurrency ceiling.
    pub read_connections: usize,

    /// `PRAGMA k = v` applied to every connection the pool opens, in key
    /// order. Rejections are logged, not fatal — pragmas are best-effort.
    pub pragma: BTreeMap<String, String>,

    /// Enforced `PRAGMA user_version`. Set on a fresh database; on an existing
    /// one a mismatch is refused with
    /// [`BedrockError::IncompatibleVersion`](crate::BedrockError::IncompatibleVersion).
    pub version: i64,

    /// Set `PRAGMA journal_mode = WAL` on file-backed databases when true.
    /// WAL lets pooled readers run concurrently with the writer. Ignored for
    /// `:memory:`, which has no on-disk journal.
    pub wal: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            read_connections: 4,
            pragma: BTreeMap::new(),
            version: 0,
            wal: true,
        }
    }
}

impl Config {
    /// A default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the read-connection ceiling.
    pub fn read_connections(mut self, n: usize) -> Self {
        self.read_connections = n;
        self
    }

    /// Add a `PRAGMA k = v` applied to every connection.
    pub fn pragma(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.pragma.insert(key.into(), value.into());
        self
    }

    /// Set the enforced `PRAGMA user_version`.
    pub fn version(mut self, version: i64) -> Self {
        self.version = version;
        self
    }
}
