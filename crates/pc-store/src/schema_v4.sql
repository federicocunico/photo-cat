ALTER TABLE organize_entry ADD COLUMN identity TEXT;
ALTER TABLE organize_entry ADD COLUMN fingerprint TEXT;
CREATE UNIQUE INDEX organize_created ON organize_entry(op_id,created);
