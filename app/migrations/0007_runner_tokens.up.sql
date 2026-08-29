CREATE TABLE runner_tokens (
  id TEXT PRIMARY KEY,
  environment_id TEXT NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  token_hash BLOB NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  last_used_at TEXT,
  revoked_at TEXT,
  UNIQUE(environment_id, name)
);

CREATE INDEX runner_tokens_token_hash_idx ON runner_tokens(token_hash);
