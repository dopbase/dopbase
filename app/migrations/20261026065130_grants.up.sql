-- A grant gives a user or a group one access level on all projects, one
-- project, or one environment. The most specific grant decides, so an
-- environment grant overrides a project grant, which overrides an
-- all-projects grant. project_manager exists only above the environment,
-- and none exists only on an environment, to exclude it from a wider grant.
-- Root needs no grants.
CREATE TABLE grants (
  id TEXT PRIMARY KEY,
  principal_type TEXT NOT NULL CHECK (principal_type IN ('user', 'group')),
  principal_id TEXT NOT NULL,
  scope_type TEXT NOT NULL CHECK (scope_type IN ('all', 'project', 'environment')),
  scope_id TEXT NOT NULL DEFAULT '',
  level TEXT NOT NULL
    CHECK (level IN ('none', 'metadata', 'read', 'write', 'manage', 'project_manager')),
  granted_by TEXT,
  created_at TEXT NOT NULL,
  UNIQUE (principal_type, principal_id, scope_type, scope_id),
  CHECK ((scope_type = 'all') = (scope_id = '')),
  CHECK (scope_type <> 'environment' OR level <> 'project_manager'),
  CHECK (scope_type = 'environment' OR level <> 'none')
);

CREATE INDEX grants_scope_idx ON grants(scope_type, scope_id);

-- Grants name their principal and scope by type and id, so foreign keys
-- cannot cascade them. These triggers remove a grant with what it names.
CREATE TRIGGER grants_user_deleted AFTER DELETE ON admins
BEGIN
  DELETE FROM grants WHERE principal_type = 'user' AND principal_id = OLD.id;
END;

CREATE TRIGGER grants_project_deleted AFTER DELETE ON projects
BEGIN
  DELETE FROM grants WHERE scope_type = 'project' AND scope_id = OLD.id;
END;

CREATE TRIGGER grants_environment_deleted AFTER DELETE ON environments
BEGIN
  DELETE FROM grants WHERE scope_type = 'environment' AND scope_id = OLD.id;
END;

-- Every project stays shared after the upgrade: admins manage all
-- projects and members can change all of them, as they could before.
INSERT INTO grants (id, principal_type, principal_id, scope_type, scope_id, level, granted_by, created_at)
SELECT 'grt_' || lower(hex(randomblob(13))), 'user', id, 'all', '',
  CASE role_id WHEN 'rol_admin' THEN 'project_manager' ELSE 'write' END,
  NULL, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
FROM admins
WHERE is_root = 0 AND role_id IN ('rol_admin', 'rol_member');
