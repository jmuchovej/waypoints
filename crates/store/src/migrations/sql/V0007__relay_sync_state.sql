-- Per-source fetch cursors. seq values are assigned by each relay in its
-- own arrival order, so a cursor is meaningful only for the relay that
-- issued it.
CREATE TABLE IF NOT EXISTS relay_cursors (
  relay_id TEXT NOT NULL,
  realm_id BLOB NOT NULL,
  last_seq INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (relay_id, realm_id)
) STRICT;

-- Outbox tracking: when the relay acknowledged this local-origin
-- envelope. NULL = pending upload. Remote-origin rows stay NULL forever.
ALTER TABLE sync_log ADD COLUMN uploaded_at TEXT;
