CREATE TABLE audit_events (
  id TEXT PRIMARY KEY,
  actor_type TEXT NOT NULL,
  actor_id TEXT,
  actor_label TEXT,
  action TEXT NOT NULL,
  project_id TEXT,
  environment_id TEXT,
  resource_type TEXT,
  resource_id TEXT,
  ip_address TEXT,
  user_agent TEXT,
  metadata TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL
);

CREATE INDEX audit_events_created_idx ON audit_events(created_at DESC, id DESC);
CREATE INDEX audit_events_action_idx ON audit_events(action);
