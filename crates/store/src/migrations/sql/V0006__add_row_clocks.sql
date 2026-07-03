-- Per-row LWW clocks. DEFAULT 0 means "never touched by sync": any
-- incoming op dominates a 0-HLC row, which is the correct behavior for
-- first-sync after enabling the feature.
ALTER TABLE marks    ADD COLUMN updated_hlc       INTEGER NOT NULL DEFAULT 0;
ALTER TABLE marks    ADD COLUMN updated_by_device BLOB;
ALTER TABLE terrains ADD COLUMN updated_hlc       INTEGER NOT NULL DEFAULT 0;
ALTER TABLE terrains ADD COLUMN updated_by_device BLOB;
