CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  admin_id TEXT NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('browser', 'cli')),
  token_hash BLOB NOT NULL UNIQUE,
  csrf_hash BLOB,
  created_at TEXT NOT NULL,
  last_used_at TEXT NOT NULL,
  recent_auth_at TEXT NOT NULL,
  idle_expires_at TEXT NOT NULL,
  absolute_expires_at TEXT NOT NULL,
  revoked_at TEXT
);

CREATE INDEX sessions_token_hash_idx ON sessions(token_hash);
