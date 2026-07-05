-- The relay's own envelope store (Turso / any SQLite).
-- See PROTOCOL.md §7. The relay never parses `payload`.
CREATE TABLE IF NOT EXISTS envelopes (
  seq         INTEGER PRIMARY KEY AUTOINCREMENT, -- arrival order: THE cursor
  group_id    TEXT NOT NULL UNIQUE,              -- idempotency key (uuid)
  account_id  TEXT NOT NULL,
  realm_id    TEXT NOT NULL,
  device_id   TEXT NOT NULL,
  hlc         INTEGER NOT NULL,
  payload     TEXT NOT NULL,                     -- Envelope JSON, verbatim
  received_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_envelopes_realm_seq    ON envelopes (realm_id, seq);
CREATE INDEX IF NOT EXISTS idx_envelopes_realm_device ON envelopes (realm_id, device_id, seq);
