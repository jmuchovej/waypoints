CREATE TABLE IF NOT EXISTS charts (
  id          TEXT NOT NULL,
  name        TEXT NOT NULL,
  description TEXT,
  icon        TEXT,
  features    TEXT NOT NULL,
  PRIMARY KEY (id)
) STRICT;
