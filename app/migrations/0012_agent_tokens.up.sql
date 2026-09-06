CREATE TABLE agent_tokens (
  id TEXT PRIMARY KEY,
  service_account_id TEXT NOT NULL
    REFERENCES service_accounts(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  token_hash BLOB NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  expires_at TEXT,
  last_used_at TEXT,
  revoked_at TEXT,
  UNIQUE(service_account_id, name)
);

CREATE INDEX agent_tokens_hash_idx ON agent_tokens(token_hash);
CREATE INDEX agent_tokens_account_idx ON agent_tokens(service_account_id);
