CREATE TABLE service_accounts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  role TEXT NOT NULL DEFAULT 'ai_agent' CHECK (role = 'ai_agent'),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

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

-- AI agent accounts go back to service accounts with their all-projects
-- tokens. Personal tokens of people have nowhere to go and are dropped.
INSERT INTO service_accounts (id, name, role, created_at, updated_at)
SELECT id, email, 'ai_agent', created_at, updated_at FROM admins WHERE role_id = 'rol_ai_agent';

INSERT INTO agent_tokens (id, service_account_id, name, token_hash, created_at, expires_at, last_used_at, revoked_at)
SELECT t.id, t.user_id, t.name, t.token_hash, t.created_at, t.expires_at, t.last_used_at, t.revoked_at
FROM personal_tokens t JOIN admins a ON a.id = t.user_id
WHERE a.role_id = 'rol_ai_agent' AND t.scope_type = 'all';

DELETE FROM admins WHERE role_id = 'rol_ai_agent';

DROP TRIGGER personal_tokens_environment_deleted;
DROP TRIGGER personal_tokens_project_deleted;
DROP TABLE personal_tokens;
