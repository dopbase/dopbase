-- Groups collect users so access can be granted to many people at once.
-- Groups are flat and never hold root. Membership adds the group's grants
-- to each member; it never adds instance capabilities.
CREATE TABLE groups (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  description TEXT NOT NULL DEFAULT '',
  created_by TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE group_members (
  group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
  user_id TEXT NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
  added_by TEXT,
  created_at TEXT NOT NULL,
  PRIMARY KEY (group_id, user_id)
);

CREATE INDEX group_members_user_idx ON group_members(user_id);

CREATE TRIGGER grants_group_deleted AFTER DELETE ON groups
BEGIN
  DELETE FROM grants WHERE principal_type = 'group' AND principal_id = OLD.id;
END;
