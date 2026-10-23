ALTER TABLE secrets DROP COLUMN previous_version;
ALTER TABLE secrets DROP COLUMN previous_ciphertext;
ALTER TABLE secrets DROP COLUMN previous_value_nonce;
ALTER TABLE secrets DROP COLUMN previous_wrapped_key;
ALTER TABLE secrets DROP COLUMN previous_key_nonce;
ALTER TABLE secrets DROP COLUMN previous_expires_at;
