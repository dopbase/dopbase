-- Overlapping (dual-key) secret rotation: the previous value stays encrypted
-- next to the current one until its grace period ends.
ALTER TABLE secrets ADD COLUMN previous_version INTEGER;
ALTER TABLE secrets ADD COLUMN previous_ciphertext BLOB;
ALTER TABLE secrets ADD COLUMN previous_value_nonce BLOB;
ALTER TABLE secrets ADD COLUMN previous_wrapped_key BLOB;
ALTER TABLE secrets ADD COLUMN previous_key_nonce BLOB;
ALTER TABLE secrets ADD COLUMN previous_expires_at TEXT;
