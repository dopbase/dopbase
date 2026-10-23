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
0013_runner_token_expiry.up.sql
0014_audit_chain.up.sql
0015_audit_chain_anchor.up.sql
0016_audit_chain_integrity.up.sql
0017_audit_drop_user_agent.up.sql
0019_secret_rotation.up.sql
```

The up file applies the change. The down file removes only that version's
change and must be safe when versions are reverted in reverse order. Version
0013 adds a nullable runner-token expiry column; existing tokens keep no
expiry. Version 0014 adds the hash-chain columns on `audit_events`. Version
0015 adds the anchor retention leaves behind when it removes expired records,
so verification continues where the pruned chain stops. Version 0016 records
the chain's genesis time, so records written before it are reported as legacy
and never sealed, and adds a trigger that rejects updates of sealed records. Version 0017 drops
the `user_agent` column from `audit_events`, which nothing ever filled. Version 0019 adds the nullable previous-value
columns used by secret rotation; secrets without a rotation keep them empty.
Runtime startup
applies up migrations only. Rollback is a maintenance and test
operation, not a public Dopbase CLI command.

The database test suite applies all migrations, rolls back to version zero,
and reapplies them. Never edit a migration after release. Add a new numbered
pair instead.
