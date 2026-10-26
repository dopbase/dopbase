-- Roles replace the fixed admin, member, and ai_agent values of admins.role.
-- A role is data: the instance capabilities it holds, the deepest level of
-- project and environment access any grant can give its holders, and how its
-- holders sign in and use personal tokens. Root is not a role. It is the
-- admins.is_root flag and implicitly holds every capability.
CREATE TABLE roles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  description TEXT NOT NULL DEFAULT '',
  builtin INTEGER NOT NULL DEFAULT 0 CHECK (builtin IN (0, 1)),
  max_level TEXT NOT NULL
    CHECK (max_level IN ('metadata', 'read', 'write', 'manage', 'project_manager')),
  sign_in TEXT NOT NULL DEFAULT 'interactive'
    CHECK (sign_in IN ('interactive', 'token_only')),
  requires_owner INTEGER NOT NULL DEFAULT 0 CHECK (requires_owner IN (0, 1)),
  token_scope TEXT NOT NULL DEFAULT 'data'
    CHECK (token_scope IN ('none', 'data', 'audit')),
  token_max_level TEXT NOT NULL DEFAULT 'write'
    CHECK (token_max_level IN ('metadata', 'read', 'write')),
  token_default_ttl_days INTEGER NOT NULL DEFAULT 30
    CHECK (token_default_ttl_days BETWEEN 1 AND 1095),
  token_max_ttl_days INTEGER NOT NULL DEFAULT 365
    CHECK (token_max_ttl_days BETWEEN 1 AND 1095),
  rank INTEGER NOT NULL DEFAULT 100,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  CHECK (token_default_ttl_days <= token_max_ttl_days)
);

-- One row per capability a role holds. Capability names are defined in
-- app/src/services/authx.rs; unknown names are ignored when a role loads.
CREATE TABLE role_capabilities (
  role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
  capability TEXT NOT NULL,
  PRIMARY KEY (role_id, capability)
);

INSERT INTO roles (
  id, name, description, builtin, max_level, sign_in, requires_owner,
  token_scope, token_max_level, token_default_ttl_days, token_max_ttl_days,
  rank, created_at, updated_at
) VALUES
  ('rol_admin', 'Admin', 'Manages people, groups, and the projects they are given.',
    1, 'project_manager', 'interactive', 0, 'data', 'write', 30, 365, 10,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
  ('rol_member', 'Member', 'Works on the projects and environments they are given.',
    1, 'manage', 'interactive', 0, 'data', 'write', 30, 365, 20,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
  ('rol_auditor', 'Auditor', 'Reads the audit log and access reports. Never sees secret values.',
    1, 'metadata', 'interactive', 0, 'audit', 'metadata', 30, 365, 30,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
  ('rol_ai_agent', 'AI agent', 'Signs in with tokens only and never exceeds its owner.',
    1, 'read', 'token_only', 1, 'data', 'read', 30, 90, 40,
    strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'));

INSERT INTO role_capabilities (role_id, capability) VALUES
  ('rol_admin', 'users.view'),
  ('rol_admin', 'users.manage'),
  ('rol_admin', 'groups.manage'),
  ('rol_admin', 'grants.manage'),
  ('rol_admin', 'projects.create'),
  ('rol_admin', 'audit.read'),
  ('rol_admin', 'audit.verify'),
  ('rol_admin', 'instance.view'),
  ('rol_admin', 'access.report'),
  ('rol_admin', 'tokens.personal'),
  ('rol_admin', 'tokens.revoke_others'),
  ('rol_member', 'tokens.personal'),
  ('rol_auditor', 'users.view'),
  ('rol_auditor', 'audit.read'),
  ('rol_auditor', 'audit.verify'),
  ('rol_auditor', 'access.report'),
  ('rol_auditor', 'instance.view'),
  ('rol_ai_agent', 'tokens.personal');
