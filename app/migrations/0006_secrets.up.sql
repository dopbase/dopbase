CREATE TABLE secrets (
  environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1,
  ciphertext BLOB NOT NULL,
  value_nonce BLOB NOT NULL,
  wrapped_key BLOB NOT NULL,
  key_nonce BLOB NOT NULL,
  encryption_version INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(environment_id, key)
);
