# Migrations

Migrations are embedded into the `dopbase` binary with `sqlx::migrate!()` and
run transactionally before the HTTP listener starts. Runtime startup applies up
migrations only. Rollback is a maintenance and test operation, not a public
Dopbase CLI command.

## Naming

Each table and feature uses its own reversible pair, named after the UTC time
it was created:

```text
YYYYMMDDHHMMSS_<name>.up.sql
YYYYMMDDHHMMSS_<name>.down.sql
```

`<name>` is the lowercase `snake_case` description of the change, unchanged
from the original migration name. The 0.1.9 files only gained a timestamp; for
example `0001_instance_metadata.up.sql` became
`20260829031127_instance_metadata.up.sql`.

The rules are enforced by `app/tests/services/migrations.rs`:

- The version is the UTC creation time, 14 digits. Generate it with
  `sqlx migrate add -r`; do not type it by hand.
- The name stays lowercase `snake_case`.
- One logical change per pair. The down file reverses only that change and
  must stay safe when versions are reverted in reverse order.
- Every up file needs a down file with the same version.
- Describe the change and why it exists in a comment at the top of the up file.

Never edit or rename a migration after it ships in a release. sqlx checksums
applied migrations, comments included, and refuses to start when one changes.
If you need to change the schema, add a new pair.

## Current schema

The current pre-release schema is assembled in this order:

```text
20260829031127_instance_metadata.up.sql
20260829031128_admins.up.sql
20260829031129_sessions.up.sql
20260829031130_projects.up.sql
20260829031131_environments.up.sql
20260829031132_secrets.up.sql
20260829031133_runner_tokens.up.sql
20260829031134_audit_events.up.sql
20260829031135_environment_env_layout.up.sql
20260901082030_session_retention.up.sql
20260906014116_service_accounts.up.sql
20260906014117_agent_tokens.up.sql
20260926085502_runner_token_expiry.up.sql
20261023081254_audit_chain.up.sql
20261023081357_audit_chain_anchor.up.sql
20261023083943_audit_chain_integrity.up.sql
20261023095824_audit_drop_user_agent.up.sql
20261024010048_secret_versions.up.sql
20261024105858_cli_device_requests.up.sql
20261024125458_sso_providers.up.sql
20261024125459_admin_identities.up.sql
20261024125500_sso_flows.up.sql
20261024132206_sso_require.up.sql
20261024132207_sso_flows_reauth.up.sql
20261024132208_cli_device_reauth.up.sql
20261026065127_roles.up.sql
20261026065129_admin_roles.up.sql
20261026065130_grants.up.sql
20261026065131_groups.up.sql
20261026065132_environment_protection.up.sql
20261026085441_personal_tokens.up.sql
20261026093115_audit_read_all.up.sql
```

The up file applies the change. The down file removes only that version's
change and must be safe when versions are reverted in reverse order.
`runner_token_expiry` (formerly 0013) adds a nullable runner-token expiry
column; existing tokens keep no expiry. `audit_chain` (formerly 0014) adds the
hash-chain columns on `audit_events`. `audit_chain_anchor` (formerly 0015) adds
the anchor retention leaves behind when it removes expired records, so
verification continues where the pruned chain stops. `audit_chain_integrity`
(formerly 0016) records the chain's genesis time, so records written before it
are reported as legacy and never sealed, and adds a trigger that rejects
updates of sealed records. `audit_drop_user_agent` (formerly 0017) drops the
`user_agent` column from `audit_events`, which nothing ever filled.
`secret_versions` (formerly 0018) adds `secret_versions`, which keeps one
encrypted row per secret version, and copies every existing secret into it as a
`legacy` version. Rows cascade from `secrets`, so deleting a secret also
deletes its history.
`roles` adds the role and capability tables and seeds the built-in Admin,
Member, Auditor, and AI agent roles. `admin_roles` links each account to a
role and adds the `is_root` flag, an account status, an owner for roles that
need one, and the account's creator, filled in from `admins.role`. `grants`
adds grants for a user or a group on all projects, a project, or an
environment. Its triggers delete a grant together with the account, project,
or environment it names, and it gives existing admins and members the
all-projects access they already had. `groups` adds groups and their
members. `environment_protection` adds the `protected` flag to environments.
`personal_tokens` adds personal tokens, turns each service account into an
account with the AI agent role owned by root, keeps its tokens working as
personal tokens with their hashes unchanged, and drops `service_accounts`
and `agent_tokens`. Rolling it back moves AI agent accounts and their
all-projects tokens back and drops people's personal tokens.
`audit_read_all` gives the Auditor role `audit.read_all`, so it keeps reading
every audit event now that `audit.read` alone is limited to the reader's
projects, and `tokens.personal`, so auditors can hold audit tokens.

The database test suite applies all migrations, rolls back to version zero,
and reapplies them.

## Adding a migration

Create migrations with the sqlx CLI from `app/`, so the version comes from the
clock instead of the next free number:

```bash
cargo install sqlx-cli --no-default-features --features sqlite
cd app
sqlx migrate add -r <name>
```

That writes the pair shown above with the current UTC time as its version.

## Upgrading from 0.1.9

Migrations used four-digit versions through 0.1.9 and were renamed to the
timestamped form for the next release. Only the timestamp was added; every
name is unchanged. Their timestamps are the UTC times the files were first
committed; files added in the same commit step forward one second so the
order is preserved. Existing databases and backups record the old versions.
Before sqlx validates them, startup rewrites each recorded row to its
timestamped version, matching rows by checksum; the SQL is unchanged so the
checksums still match. Nothing to do on upgrade.

Downgrading to 0.1.9 after a newer release has opened the database is not
supported, because the old binary does not know the timestamped versions.
