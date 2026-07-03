CREATE TABLE IF NOT EXISTS marks (
  id          BLOB NOT NULL,
  mark        TEXT NOT NULL,
  data        TEXT NOT NULL,
  extra       TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  created_by  BLOB,
  updated_by  BLOB,
  deleted_at  TEXT,
  archived_at TEXT,
  PRIMARY KEY (id)
) STRICT;

CREATE INDEX idx_marks_mark       ON marks (mark);
CREATE INDEX idx_marks_created_at ON marks (created_at);
CREATE INDEX idx_marks_updated_at ON marks (updated_at);
