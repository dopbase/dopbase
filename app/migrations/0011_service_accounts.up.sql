CREATE TABLE service_accounts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  role TEXT NOT NULL DEFAULT 'ai_agent' CHECK (role = 'ai_agent'),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
