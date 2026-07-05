-- The slice of the (global, Waypoints-operated) accounts database that
-- the official relay reads for authentication. The relay only ever
-- SELECTs from these tables; the website/account system owns writes.
-- Self-hosted relays do not use this database at all (PROTOCOL.md §6).

CREATE TABLE IF NOT EXISTS device_tokens (
  token_hash TEXT PRIMARY KEY, -- sha256(token), hex
  account_id TEXT NOT NULL,
  device_id  TEXT NOT NULL,
  created_at TEXT NOT NULL,
  revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS realm_grants (
  realm_id   TEXT NOT NULL,
  account_id TEXT NOT NULL,
  granted_at TEXT NOT NULL,
  PRIMARY KEY (realm_id, account_id)
);
