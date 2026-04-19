CREATE TABLE IF NOT EXISTS "marks" (
  "id"          BLOB NOT NULL,
  "mark"        TEXT NOT NULL,
  "data"        TEXT NOT NULL,
  "extra"       TEXT,
  "created-at"  TEXT NOT NULL,
  "updated-at"  TEXT NOT NULL,
  "created-by"  BLOB,
  "updated-by"  BLOB,
  "deleted-at"  TEXT,
  "archived-at" TEXT,
  PRIMARY KEY ("id")
) STRICT;

CREATE INDEX "idx-marks-mark"       ON "marks" ("mark");
CREATE INDEX "idx-marks-created-at" ON "marks" ("created-at");
CREATE INDEX "idx-marks-updated-at" ON "marks" ("updated-at");
