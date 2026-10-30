-- Links each account to a role and records whether it is the root account.
-- is_root replaces the fixed 'root' value of admins.role, which stays in
-- place, unread by authorization, until every caller has moved off it.
-- role_id is NULL for root. owner_id names the accountable person for roles
-- that require an owner, such as AI agents. A disabled account cannot sign
-- in or use its tokens.
ALTER TABLE admins ADD COLUMN is_root INTEGER NOT NULL DEFAULT 0 CHECK (is_root IN (0, 1));

ALTER TABLE admins ADD COLUMN role_id TEXT;

ALTER TABLE admins ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
  CHECK (status IN ('active', 'disabled'));

ALTER TABLE admins ADD COLUMN owner_id TEXT;

ALTER TABLE admins ADD COLUMN created_by TEXT;

UPDATE admins SET is_root = 1 WHERE role = 'root';

UPDATE admins SET role_id = CASE role
  WHEN 'admin' THEN 'rol_admin'
  WHEN 'member' THEN 'rol_member'
  WHEN 'ai_agent' THEN 'rol_ai_agent'
END
WHERE role <> 'root';

CREATE UNIQUE INDEX admins_is_root_idx ON admins(is_root) WHERE is_root = 1;

CREATE INDEX admins_role_idx ON admins(role_id);
