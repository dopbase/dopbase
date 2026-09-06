Migrations are embedded into the `dopbase` binary with `sqlx::migrate!()` and
run transactionally before the HTTP listener starts.

Each table and feature uses its own reversible pair with the same numeric
version. The current pre-release schema is assembled in this order:

```text
0001_instance_metadata.up.sql
0002_admins.up.sql
0003_sessions.up.sql
0004_projects.up.sql
0005_environments.up.sql
0006_secrets.up.sql
0007_runner_tokens.up.sql
0008_audit_events.up.sql
0009_environment_env_layout.up.sql
0010_session_retention.up.sql
0011_service_accounts.up.sql
0012_agent_tokens.up.sql
```

The up file applies the change. The down file removes only that version's
change and must be safe when versions are reverted in reverse order. No
existing table is rebuilt or altered by these migrations. Runtime
startup applies up migrations only; rollback is a maintenance and test
operation, not a public Dopbase CLI command.

The database test suite applies all migrations, rolls back to version zero,
and reapplies them. Never edit a migration after release; add a new numbered
pair instead.
