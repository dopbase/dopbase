DROP INDEX admins_role_idx;

DROP INDEX admins_is_root_idx;

ALTER TABLE admins DROP COLUMN created_by;

ALTER TABLE admins DROP COLUMN owner_id;

ALTER TABLE admins DROP COLUMN status;

ALTER TABLE admins DROP COLUMN role_id;

ALTER TABLE admins DROP COLUMN is_root;
