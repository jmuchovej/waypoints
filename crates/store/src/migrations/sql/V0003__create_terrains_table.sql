CREATE TABLE IF NOT EXISTS "terrains" (
  "id"     BLOB NOT NULL,
  "src-id" BLOB NOT NULL,
  "dst-id" BLOB NOT NULL,
  "kind"   TEXT NOT NULL,
  "order"  TEXT,
  "extra"  TEXT,
  PRIMARY KEY ("id"),
  FOREIGN KEY ("src-id") REFERENCES "marks" ("id"),
  FOREIGN KEY ("dst-id") REFERENCES "marks" ("id")
) STRICT;

CREATE INDEX "idx-terrains-src-id" ON "terrains" ("src-id");
CREATE INDEX "idx-terrains-dst-id" ON "terrains" ("dst-id");
CREATE INDEX "idx-terrains-kind"   ON "terrains" ("kind");
