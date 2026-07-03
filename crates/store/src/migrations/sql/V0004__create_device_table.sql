-- Local install identity for sync. This install owns exactly one row;
-- other devices' identities are tracked indirectly via sync_log.device_id.
CREATE TABLE IF NOT EXISTS devices (
  device_id     BLOB NOT NULL,
  account_id    BLOB NOT NULL,
  realm_id      BLOB NOT NULL,
  friendly_name TEXT NOT NULL,
  last_hlc      INTEGER NOT NULL DEFAULT 0,
  alias_of      BLOB,
  created_at    TEXT NOT NULL,
  retired_at    TEXT,
  PRIMARY KEY (device_id)
) STRICT;
