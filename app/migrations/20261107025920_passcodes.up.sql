-- Short one-time codes tied to an account, for anything that needs one:
-- `dopbase admin passcode` signing an account in from the server today,
-- one-time passwords or verification later. `purpose` names the use; the
-- server keeps the list of purposes, so adding one needs no migration. Only
-- a hash of the purpose and code is stored. An account holds at most one
-- live code per purpose. A row is deleted when it is used, replaced, tried
-- wrong too many times, or swept after it expires.
CREATE TABLE passcodes (
  id TEXT PRIMARY KEY,
  purpose TEXT NOT NULL,
  admin_id TEXT NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
  code_hash BLOB NOT NULL,
  failed_attempts INTEGER NOT NULL DEFAULT 0,
  issued_by TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  UNIQUE (purpose, admin_id)
);

CREATE INDEX passcodes_expires_idx ON passcodes(expires_at);
