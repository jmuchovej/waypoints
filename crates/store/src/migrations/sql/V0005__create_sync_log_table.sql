-- Append-only op log for sync. group_id is the idempotency key; the
-- (realm_id, hlc) index serves the relay protocol's "since cursor"
-- query. ops is an opaque JSON Vec<AtomicOp>; the schema never
-- interprets it.
CREATE TABLE IF NOT EXISTS sync_log (
  group_id      BLOB NOT NULL,
  device_id     BLOB NOT NULL,
  account_id    BLOB NOT NULL,
  realm_id      BLOB NOT NULL,
  hlc           INTEGER NOT NULL,
  ops           TEXT NOT NULL,
  touched_marks TEXT NOT NULL,
  applied_at    TEXT NOT NULL,
  origin        INTEGER NOT NULL, -- 0 = local-origin, 1 = remote-origin
  PRIMARY KEY (group_id)
) STRICT;

CREATE INDEX idx_sync_log_hlc        ON sync_log (hlc);
CREATE INDEX idx_sync_log_device_hlc ON sync_log (device_id, hlc);
CREATE INDEX idx_sync_log_realm_hlc  ON sync_log (realm_id, hlc);
