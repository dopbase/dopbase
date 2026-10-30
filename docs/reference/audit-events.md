---
title: "Audit event reference"
description: "See which sensitive actions Dopbase records, who performed them, where they happened, and which secret data is excluded."
---

# Audit events

Audit records answer who performed a sensitive action, where it happened, and when. They describe the action without storing the secret value involved.

## Audit events in {{version}}

| Area                  | Events                                                                                                                                                                                                                                              |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Secrets               | `secret.created`, `secret.updated`, `secret.deleted`, `secret.revealed`, `secret.version_revealed`, `secret.rolled_back`, `secret.imported`, `secret.import_previewed`, `secret.exported`, `secret.runtime_accessed`                                |
| Projects              | `project.created`, `project.renamed`, `project.deleted`, `project.initialized`                                                                                                                                                                      |
| Environments          | `environment.created`, `environment.renamed`, `environment.deleted`, `environment.protected`, `environment.unprotected`                                                                                                                             |
| Runner tokens         | `token.created`, `token.revoked`                                                                                                                                                                                                                    |
| Authentication        | `login.succeeded`, `login.failed`, `logout.succeeded`, `cli_login.approved`, `cli_login.denied`                                                                                                                                                     |
| Single sign-on        | `sso_provider.created`, `sso_provider.updated`, `sso_provider.verified`, `sso_provider.enabled`, `sso_provider.disabled`, `sso_provider.deleted`, `sso_provider.policy_changed`, `sso_identity.linked`, `sso_identity.unlinked`, `sso_login.failed` |
| Administrator & users | `admin.bootstrapped`, `admin.created`, `admin.updated`, `admin.deleted`, `admin.reauthenticated`, `admin.password_changed`                                                                                                                          |
| Access                | `grant.set`, `grant.removed`, `group.created`, `group.updated`, `group.deleted`, `group_member.added`, `group_member.removed`                                                                                                                       |
| Roles                 | `role.created`, `role.updated`, `role.deleted`                                                                                                                                                                                                      |
| Personal tokens       | `personal_token.created`, `personal_token.revoked`                                                                                                                                                                                                  |
| Backups               | `backup.created`, `backup.downloaded`, `backup.uploaded`, `backup.restored`, `backup.deleted`, `master_key.downloaded`, `backup.restored_bootstrap`                                                                                                 |
| Audit log             | `audit.pruned`                                                                                                                                                                                                                                      |

A successful login records `login.succeeded` with the session kind and, for
single sign-on and browser CLI logins, a `method` of `sso` or `browser`.
`sso_identity.linked` records whether the link came from a verified email
(`auto`) or from a signed-in user (`manual`). `sso_login.failed` records a
reason such as `account_not_found` or `email_unverified`, never provider output.
`admin.reauthenticated` records a `method` of `sso` or `browser` when the
confirmation came from the provider or from a CLI approval in the Admin UI.
`login.failed` records `password_login_disabled` when a right password was
refused because single sign-on is required.
`sso_provider.policy_changed` records which sign-in option changed:
`requireSso` with the number of `revokedSessions`, or `hidePasswordForm`.
`sso_provider.updated` records `identitiesRemoved`, the number of links a new
issuer removed.

Successful import previews record `secret.import_previewed`. The record
identifies the actor and environment and includes only the import mode and
submitted entry count in its metadata. It contains no submitted values or
comparison results. Applying an import records `secret.imported`.

Revealing an earlier version records `secret.version_revealed` with the key
and version. A rollback records `secret.rolled_back` with the key, the
restored `sourceVersion`, the `previousVersion` that was current, and the new
`version`. Neither stores a value. Listing a secret's history is not audited,
the same as listing secrets.

## Browsing events

Administrators browse events on the Audit page in the Admin UI or with
`dopbase audit list`, newest first. Both filter by action, project,
environment, and actor, and both page with the opaque `nextCursor` that
`GET /api/v1/audit-events` returns. Each event's `seq` is its position in the
hash chain below, so an event found while browsing can be matched against
`dopbase audit verify` output. See the
[CLI reference](/cli/commands#audit-commands) for the command's options.

## Integrity and verification

Every audit record is part of a hash chain. It stores its position in the log,
the hash of the record before it, and the hash of its own stored contents, so
modifying, removing, or inserting a record leaves a trace at that point.

- `dopbase audit verify` recomputes every hash in sequence order, stops at the
  first break, and exits with code `0` when the chain is intact and `2` when it
  is broken. `--json` prints the full report, including the head sequence and
  hash.
- `GET /api/v1/audit-events/verify` serves the same check to scripts. It is
  restricted to administrators, like reading the log itself.
- Records written before the chain existed are never sealed after the fact.
  Verification reports them as `legacy`: the chain cannot vouch for them, but
  they are not a failure. A record without a chain position that is newer than
  the chain itself was inserted outside Dopbase and fails verification.
- Sealed records cannot be updated: the database rejects it. That guards
  against bugs, not people: anyone who can write the database file can remove
  the guard, which is what the chain is for.

### What the chain can and cannot detect

| Change made directly to the database                 | Chain alone  | With an outside anchor |
| ---------------------------------------------------- | ------------ | ---------------------- |
| A record edited, removed, or inserted mid-log        | Detected     | Detected               |
| Retention's pruning anchor moved to hide old records | Detected     | Detected               |
| Every hash recomputed after an edit (a full rewrite) | Not detected | Detected               |
| The newest records deleted                           | Not detected | Detected               |
| Legacy records, written before the chain existed     | Not covered  | Not covered            |

The last two rows of the chain-alone column are why the head must also live
somewhere the database's owner cannot rewrite. Keep a copy of it outside the
server and check against it:

```bash
dopbase --json audit verify             # note headSeq and headHash
dopbase audit verify --anchor 42:<HASH> # later: fails if record 42 changed or vanished
```

The anchor's status is `matched`, `mismatch` (the log was rewritten up to that
record), `missing` (that record and any after it were deleted), or `pruned`
(retention removed it; compare it with the archive). The server also logs the head as an
`audit checkpoint` at startup, at shutdown, and after every verification, so a
log collector that ships the server's output elsewhere keeps anchors without
any extra setup. `dopbase audit export` streams the records themselves, chain
fields included, for write-once storage.

### If verification reports a break

1. Do not repair the chain. Recomputing hashes or deleting the reported record
   destroys the evidence, and is exactly what someone hiding a change would do.
2. Preserve the evidence: copy the database file, `audit-archive/`, your
   backups, and the verify report somewhere read-only.
3. Find what changed. Compare the reported record with a backup, an export, an
   archive, or a logged checkpoint taken before the break.
4. Decide whether it was an accident, such as disk corruption or a bad manual
   edit, or deliberate. Review who could reach the data directory: host logins,
   container exec sessions, and volume access.
5. If it was deliberate, treat it as a breach of the host. Rotate the secrets
   Dopbase holds, passwords, runner and personal tokens,
   and consider the master key exposed.
6. Records before the break can still be trusted; records from the break on
   cannot.

## Retention

`audit_retention_days` in `server.toml`, or `DOPBASE_AUDIT_RETENTION_DAYS`,
sets how many days audit records are kept. New instances start with
`audit_retention_days = 365` in `server.toml`. Remove the line to keep the
whole log. `0` is rejected.

When records expire, Dopbase applies one clear rule:

1. The expired records are written to `audit-archive/` in the data directory
   as JSON lines, one file per contiguous sequence range, named
   `audit-000001-000042.jsonl`. Each line is the record with every stored
   column, chain fields included, so the archive can be checked and shipped
   like an export.
2. Only then are they deleted, in the same transaction, together with an
   anchor recording the newest removed record's position and hash. If any
   step fails, the whole pass is undone and runs again next time.
3. An `audit.pruned` record joins the chain, naming the range removed, the
   hash it was pruned through, and the archive file. Verification requires
   it to match the anchor, so moving the anchor by hand to hide records
   fails verification.
4. Verification continues from that anchor, so a pruned log still verifies
   as one unbroken chain, and `dopbase audit verify` reports
   `Pruned through: sequence N` instead of a gap.

Expired legacy records are archived the same way, to
`audit-legacy-<FIRST_ID>-<LAST_ID>.jsonl`, and need no anchor because they
sit outside the chain.

Retention runs once at startup and then once a day while the server is up,
so a record can stay up to a day past its window. Each run writes at most one
archive file and one `audit.pruned` record. Only the oldest contiguous run of records
is removed; a record timestamped before its predecessor, which a backwards
clock step can produce, is kept until everything before it has expired too,
because deleting it would leave a hole no anchor can describe.

## Restoring a backup

A restore replaces the audit log with the one in the backup. Before it does,
Dopbase writes every current record to
`audit-archive/pre-restore-<TIME>.jsonl`, so nothing written after the
snapshot is lost; if that file cannot be written, the restore stops. The
`backup.restored` record names the archive (`archive`), how many records it
holds (`archivedRecords`), and the position and hash the replaced chain ended
at (`previousHead`).

Archives are not part of `dopbase backup snapshot`, which stores the database
only: copy `audit-archive/` with the rest of your backups, or ship it
elsewhere with `dopbase audit export`.

## Record contents

What the Audit page and `dopbase audit` list depends on the reader's role:
every event with `audit.read_all` (Auditor and root), settings events and
events on the projects they reach with `audit.read` (Admin), and otherwise
only the reader's own actions. Accounts and AI agents appear as `admin.*`
events. Requests made with a personal token are recorded with the actor type
`personal_token` and the account as the actor. Service account events from
earlier versions keep their `service_account.*` names.

An audit record may contain:

```text
timestamp
actor
action
organization
project
environment
secret key
IP address
```

The IP address is the client that made the request. Behind a reverse proxy,
list the proxy in `trusted_proxies` so the address comes from
`X-Forwarded-For` instead of being the proxy's own; see
[operations](/self-hosting/operations). Records written outside a request,
such as `audit.pruned` during retention, have no IP address.

The actor type is `root` for the root user and `admin` for every other
administrator, so actions taken with the root account stand out. Runner tokens
record `runner`, failed sign-ins `anonymous`, a restore during setup
`bootstrap`, and retention and local maintenance commands `system`.

It must never contain the plaintext secret value, a usable authentication token, or a request body that carries credentials.

## Example

```text
2026-08-27 14:23
actor: alice@example.com
action: secret.updated
project: payment-service
environment: production
secret: DATABASE_URL
```

Retention, export, advanced filtering, and signed checkpoints remain roadmap work.
