CREATE INDEX sessions_revoked_retention_idx ON sessions(revoked_at);
CREATE INDEX sessions_idle_retention_idx ON sessions(idle_expires_at);
CREATE INDEX sessions_absolute_retention_idx ON sessions(absolute_expires_at);
