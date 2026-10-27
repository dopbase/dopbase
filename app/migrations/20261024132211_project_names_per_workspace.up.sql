-- Project names are unique per workspace instead of globally.
PRAGMA foreign_keys = OFF;

CREATE TABLE projects__names_per_workspace (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

INSERT INTO projects__names_per_workspace (id, name, created_at, updated_at)
SELECT id, name, created_at, updated_at FROM projects;

DROP TABLE projects;

ALTER TABLE projects__names_per_workspace RENAME TO projects;

PRAGMA foreign_keys = ON;
