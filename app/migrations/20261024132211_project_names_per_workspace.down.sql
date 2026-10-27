PRAGMA foreign_keys = OFF;

CREATE TABLE projects__global_name (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

INSERT INTO projects__global_name (id, name, created_at, updated_at)
SELECT id, name, created_at, updated_at FROM projects;

DROP TABLE projects;

ALTER TABLE projects__global_name RENAME TO projects;

PRAGMA foreign_keys = ON;
