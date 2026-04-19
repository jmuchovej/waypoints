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

-- Gossip pump state (PROTOCOL.md §1 "Gossip"): last seq pulled from each
-- (peer, realm) source. Peer identity is the peer's base URL; a changed
-- URL form resets the cursor, which idempotent refetch makes safe.
CREATE TABLE IF NOT EXISTS peer_cursors (
  peer_id  TEXT NOT NULL,
  realm_id TEXT NOT NULL,
  last_seq INTEGER NOT NULL,
  PRIMARY KEY (peer_id, realm_id)
);
