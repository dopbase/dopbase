---
title: "REST API"
description: "Dopbase exposes a versioned REST API with a generated OpenAPI 3 specification and Swagger UI from a running server."
---

# REST API

Dopbase exposes the versioned REST API used by the CLI and embedded Admin UI.
Fresh startup enters [automatic setup](/cli/setup); complete it before using
authenticated endpoints.
Enable API documentation with `dopbase server start --docs`. The generated OpenAPI 3
specification is then available from the running server:

```text
OpenAPI JSON: http://localhost:8840/api/v1/openapi.json
Swagger UI:   http://localhost:8840/api/docs
```

Use the documentation on your running server to explore the API available
in your installed version.

## Resource areas

| Area                                                  | Base paths                                           |
| ----------------------------------------------------- | ---------------------------------------------------- |
| Health and compatibility                              | `/api/v1/health`                                     |
| Initial administrator                                 | `/api/v1/bootstrap`                                  |
| Login and sessions                                    | `/api/v1/auth`                                       |
| Projects                                              | `/api/v1/projects`                                   |
| Environments                                          | `/api/v1/environments`                               |
| Secrets, rotation, import, export, and runtime values | `/api/v1/environments/{id}/secrets`                  |
| Runner tokens                                         | `/api/v1/environments/{id}/tokens`, `/api/v1/tokens` |
| System backups and restoration                        | `/api/v1/backups`, `/api/v1/bootstrap/restore`       |
| Audit log, chain verification, and instance status    | `/api/v1/audit-events`, `/api/v1/instance`           |
| Human users                                           | `/api/v1/users`                                      |
| AI service accounts                                   | `/api/v1/service-accounts`                           |
| Instance summary                                      | `/api/v1/status`                                     |

Consult Swagger for request bodies, parameters, authentication schemes, and
the responses supported by each operation.

## Response format

A successful request uses a typed envelope:

```json
{
  "success": true,
  "message": "PROJECT_CREATED",
  "data": {
    "id": "prj_01...",
    "name": "payment-service"
  }
}
```

Errors map stable codes to safe messages:

```json
{
  "success": false,
  "error": {
    "EMAIL_INVAILD": "Please use proper email"
  }
}
```

Validation may report more than one code in the `error` object. Clients must
branch on the code rather than the English message or object ordering. Request
correlation is returned in the `X-Request-Id` header. Errors never include
request bodies, plaintext secrets, tokens, key material, SQL, or filesystem
details.

## Import authentication

`POST /api/v1/environments/{id}/secrets/import` requires a human project
account. Browser sessions must include `X-Dopbase-CSRF` and have confirmed
their password within the last ten minutes. This applies to merge and replace,
for both previews (`dryRun: true`) and actual imports.

An expired password confirmation returns `403` with
`RECENT_AUTHENTICATION_REQUIRED` before any stored values are compared. Confirm
the password through `/api/v1/auth/reauthenticate`, then retry the same request.
Existing CLI authentication behavior is unchanged.

Successful previews record `secret.import_previewed` with the actor,
project/environment, mode, and submitted entry count. They do not change
secrets, versions, or layout. If the audit event cannot be saved, the server
returns an error without the comparison results.

## Secret editor snapshots

`POST /api/v1/environments/{id}/secrets/export` returns `entries`, `revision`,
and nullable `envLayout` from one snapshot. Export retains its existing human
access requirements and audit event. Layout contains comments and empty key
slots; comments are unencrypted metadata.

Pass `expectedRevision` to the import endpoint for both the dry run and apply.
A stale revision returns `409` with `IMPORT_PREVIEW_STALE`, including during a
dry run. Replace apply requires a revision. Invalid layout slots, including
slots containing values, return `422` with `ENV_LAYOUT_INVALID`.

Existing export clients can ignore the added fields. `env edit` requires a
server that supplies the snapshot revision and fails safely when it is missing.

## Audit chain verification

`GET /api/v1/audit-events/verify` walks the audit log in sequence order,
recomputes every record's hash from its stored contents, and reports the first
inconsistency. Administrators only, like reading the audit log; runner tokens
and AI service accounts are refused with `403`.

The response carries `ok`, the number of `records` checked before the first
break, `headSeq` and `headHash` for external anchoring, `legacy` for records
written before the chain existed (not a failure), `unsequenced` for records
inserted outside the chain after it existed (a failure), `prunedSeq` when
[retention](/reference/audit-events#retention) removed records (the chain
continues after it), and `broken` with the sequence number, record ID, and
reason when something does not match.

Pass `anchorSeq` and `anchorHash`, saved from an earlier `headSeq` and
`headHash`, to compare the log with that outside copy. The response then
carries `anchor` with `seq` and a `status` of `matched`, `mismatch`, `missing`,
or `pruned`; `mismatch` and `missing` set `ok` to `false`. Passing only one of
the two returns `400`.

The chain columns are additive: the audit list keeps its shape and filters,
and every item now carries `ipAddress`, `seq`, `prevHash`, and `recordHash`, so an exported
record can serve as an anchor. Recomputing hashes from an export is not
supported yet: the list returns metadata re-serialized, so it does not match
the stored bytes the hash covers. Verification reads no secret values. `dopbase audit
verify` runs the same check from the terminal, and `dopbase audit export`
streams the filtered list as JSON lines.

## Secret rotation

`POST /api/v1/environments/{id}/secrets/{key}/rotate` stores a new value while
keeping the replaced one available for a grace period: `graceSeconds` selects
the window, an omitted field means 24 hours, and `0` retires the previous value
immediately. An out-of-range grace returns `422` with `SECRET_GRACE_INVALID`.
`POST /api/v1/environments/{id}/secrets/{key}/promote` swaps the two values
back without re-encrypting them; a secret with no live previous value returns
`404` with `SECRET_PREVIOUS_NOT_FOUND`.

While a grace period runs, secret metadata carries `previousVersion` and
`previousExpiresAt`, and both fields disappear once the value is retired.
`GET .../secrets/{key}/reveal?includePrevious=true` returns the previous value
alongside the current one for verification tooling; without the parameter the
response keeps its original shape. Rotation never changes the current value, so
runtime consumers and older clients are unaffected.

## Authentication

- Browser sessions use an HttpOnly, SameSite Strict cookie and require the
  server-issued `X-Dopbase-CSRF` header for mutations.
- CLI sessions, runner identities, and AI service accounts use `Authorization: Bearer <token>`.
- A runner token can retrieve runtime secrets and export the snapshot of its
  assigned environment. It cannot list secret metadata, mutate secrets, reveal
  individual values, or access another environment. Export retains the existing
  `entries`, `revision`, and `envLayout` response fields and records the runner
  token ID in the audit log. Browser exports still require recent password
  authentication and a valid CSRF header.

Runner exports require server support for runner access to the export endpoint.
Update the server before or alongside the CLI; older servers reject the request.

The OpenAPI document declares the `cookieAuth`, `bearerAuth`, and `csrfHeader`
security schemes.

Account and agent-token administration requires a root/admin browser session;
CLI bearer sessions cannot administer accounts. AI bearer tokens may only read
metadata and [instance status](/ui/audit-instance#instance-status). See
[identity and tokens](/reference/identity) for the complete role matrix.

## Compatibility

`GET /api/v1/health` identifies the product, binary version, and API version.
`dopbase client connect` validates this response before changing local client
configuration. Version {{version}} does not promise compatibility with future major
API versions or implement idempotency keys.
