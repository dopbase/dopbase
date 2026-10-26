-- A personal token is a credential of one account. It reaches at most what
-- the account reaches right now, and never more than its own scope: all
-- projects, one project, or one environment, up to `level`. Audit tokens
-- read the audit log and no project data.
CREATE TABLE personal_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  token_hash BLOB NOT NULL UNIQUE,
  scope_type TEXT NOT NULL DEFAULT 'all'
    CHECK (scope_type IN ('all', 'project', 'environment', 'audit')),
  scope_id TEXT NOT NULL DEFAULT '',
  level TEXT NOT NULL CHECK (level IN ('none', 'metadata', 'read', 'write')),
  created_by TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT,
  last_used_at TEXT,
  revoked_at TEXT,
  UNIQUE (user_id, name),
  CHECK ((scope_type IN ('all', 'audit')) = (scope_id = '')),
  CHECK ((scope_type = 'audit') = (level = 'none'))
);

CREATE INDEX personal_tokens_user_idx ON personal_tokens(user_id);

-- A token scoped to a project or environment that is gone stops working.
CREATE TRIGGER personal_tokens_project_deleted AFTER DELETE ON projects
BEGIN
  UPDATE personal_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
  WHERE scope_type = 'project' AND scope_id = OLD.id AND revoked_at IS NULL;
END;

CREATE TRIGGER personal_tokens_environment_deleted AFTER DELETE ON environments
BEGIN
  UPDATE personal_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
  WHERE scope_type = 'environment' AND scope_id = OLD.id AND revoked_at IS NULL;
END;

-- Service accounts become accounts with the AI agent role, owned by root.
-- Their name is their handle; names never contain the @ an email needs,
-- except where an old name clashes with an email, which gets a suffix.
-- They keep reading secret names on every project, as before.
INSERT INTO admins (id, email, password_hash, role, role_id, status, owner_id, is_root, created_at, updated_at)
SELECT s.id,
  CASE WHEN EXISTS (SELECT 1 FROM admins a WHERE a.email = s.name) THEN s.name || ' (agent)' ELSE s.name END,
  '', 'ai_agent', 'rol_ai_agent', 'active',
  (SELECT id FROM admins WHERE is_root = 1), 0, s.created_at, s.updated_at
FROM service_accounts s;

INSERT INTO grants (id, principal_type, principal_id, scope_type, scope_id, level, granted_by, created_at)
SELECT 'grt_' || lower(hex(randomblob(13))), 'user', id, 'all', '', 'metadata', NULL, created_at
FROM service_accounts;

-- Agent tokens keep their hashes, so the dpa_ values already handed out
-- keep working as personal tokens of the agent account.
INSERT INTO personal_tokens (id, user_id, name, token_hash, scope_type, scope_id, level, created_at, expires_at, last_used_at, revoked_at)
SELECT id, service_account_id, name, token_hash, 'all', '', 'metadata', created_at, expires_at, last_used_at, revoked_at
FROM agent_tokens;

DROP TABLE agent_tokens;
DROP TABLE service_accounts;
