-- sync_log becomes the app's envelope store (relay/PROTOCOL.md §7-§8):
-- the table an embedded peer serves the protocol from. Rebuild because
-- the PK changes (STRICT + AUTOINCREMENT cannot be ALTERed in).
--
-- - local_seq: arrival order on THIS device — the seq an embedded peer
--   hands out when serving §4.2, and (later) the basis for per-peer
--   upload high-water marks. Monotonic, never reused.
-- - payload:   the wire envelope, byte-verbatim. What serving returns,
--   so this device can carry envelopes it cannot parse (a newer app
--   version's — and, under future E2E, ciphertext). NULL only on rows
--   from before this migration; readers reconstruct those from ops.
-- - applied_at becomes NULLable: NULL = stored but not applied (a
--   carrier row accepted via serving, awaiting apply_pending).
-- - ops / touched_marks become NULLable: a carrier row is face +
--   payload only; parsing checkpoints is the domain plane's job, at
--   apply time.

CREATE TABLE sync_log_v2 (
  local_seq     INTEGER PRIMARY KEY AUTOINCREMENT,
  group_id      BLOB NOT NULL UNIQUE,
  device_id     BLOB NOT NULL,
  account_id    BLOB NOT NULL,
  realm_id      BLOB NOT NULL,
  hlc           INTEGER NOT NULL,
  ops           TEXT,
  touched_marks TEXT,
  payload       TEXT,
  applied_at    TEXT,
  origin        INTEGER NOT NULL, -- 0 = local-origin, 1 = remote-origin
  uploaded_at   TEXT
) STRICT;

INSERT INTO sync_log_v2
  (group_id, device_id, account_id, realm_id, hlc, ops, touched_marks,
   payload, applied_at, origin, uploaded_at)
SELECT
  group_id, device_id, account_id, realm_id, hlc, ops, touched_marks,
  NULL, applied_at, origin, uploaded_at
FROM sync_log
ORDER BY applied_at, hlc; -- stable-ish arrival order for pre-existing rows

DROP TABLE sync_log;
ALTER TABLE sync_log_v2 RENAME TO sync_log;

CREATE INDEX idx_sync_log_hlc        ON sync_log (hlc);
CREATE INDEX idx_sync_log_device_hlc ON sync_log (device_id, hlc);
CREATE INDEX idx_sync_log_realm_hlc  ON sync_log (realm_id, hlc);
-- The serving cursor (§4.2) pages on arrival order, never HLC.
CREATE INDEX idx_sync_log_realm_seq  ON sync_log (realm_id, local_seq);
-- apply_pending scans carrier rows.
CREATE INDEX idx_sync_log_unapplied  ON sync_log (realm_id, local_seq)
  WHERE applied_at IS NULL;
