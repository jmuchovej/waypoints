CREATE TABLE IF NOT EXISTS terrains (
  id       BLOB NOT NULL,
  src_id   BLOB NOT NULL,
  dst_id   BLOB NOT NULL,
  kind     TEXT NOT NULL,
  sequence TEXT,
  extra    TEXT,
  PRIMARY KEY (id),
  FOREIGN KEY (src_id) REFERENCES marks (id),
  FOREIGN KEY (dst_id) REFERENCES marks (id)
) STRICT;

CREATE INDEX idx_terrains_src_id ON terrains (src_id);
CREATE INDEX idx_terrains_dst_id ON terrains (dst_id);
CREATE INDEX idx_terrains_kind   ON terrains (kind);
