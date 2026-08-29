CREATE TABLE environment_env_layout (
  environment_id TEXT PRIMARY KEY REFERENCES environments(id) ON DELETE CASCADE,
  layout TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
