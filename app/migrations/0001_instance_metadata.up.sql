CREATE TABLE instance_metadata (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  verification_ciphertext BLOB NOT NULL,
  verification_nonce BLOB NOT NULL,
  created_at TEXT NOT NULL
);
