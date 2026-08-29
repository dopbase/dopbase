Migrations are embedded into the `dopbase` binary with `sqlx::migrate!()` and
run transactionally before the HTTP listener starts.

Every schema change uses a reversible pair with the same numeric version:

```text
0009_feature_name.up.sql
0009_feature_name.down.sql
```

The up file applies the change. The down file removes only that version's
change and must be safe when versions are reverted in reverse order. Runtime
startup applies up migrations only; rollback is a maintenance and test
operation, not a public Dopbase CLI command.

The database test suite applies all migrations, rolls back to version zero,
and reapplies them. Never edit a migration after release; add a new numbered
pair instead.
