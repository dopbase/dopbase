---
title: "REST API reference"
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

| Area                                               | Base paths                                           |
| -------------------------------------------------- | ---------------------------------------------------- |
| Health and compatibility                           | `/api/v1/health`                                     |
| Initial administrator                              | `/api/v1/bootstrap`                                  |
| Login and sessions                                 | `/api/v1/auth`                                       |
| Single sign-on settings                            | `/api/v1/sso`                                        |
| Projects                                           | `/api/v1/projects`                                   |
| Environments                                       | `/api/v1/environments`                               |
| Secrets, import, export, and runtime values        | `/api/v1/environments/{id}/secrets`                  |
| Comparing environments, copying a value across     | `/api/v1/environments/{id}/compare`                  |
| Runner tokens                                      | `/api/v1/environments/{id}/tokens`, `/api/v1/tokens` |
| System backups and restoration                     | `/api/v1/backups`, `/api/v1/bootstrap/restore`       |
| Audit log, chain verification, and instance status | `/api/v1/audit-events`, `/api/v1/instance`           |
| People and AI agents                               | `/api/v1/users`                                      |
| Personal tokens                                    | `/api/v1/users/{id}/tokens`                          |
| Where someone's access comes from                  | `/api/v1/users/{id}/access`                          |
| Roles                                              | `/api/v1/roles`                                      |
| Grants                                             | `/api/v1/grants`, `/api/v1/projects/{ref}/access`    |
| Groups                                             | `/api/v1/groups`                                     |
| Instance summary                                   | `/api/v1/status`                                     |

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
    "EMAIL_INVALID": "Please use proper email"
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

## Secret versions

`GET /api/v1/environments/{id}/secrets/{key}/versions` lists a secret's
versions newest first, with the same access as listing secrets. Each item has
`version`, `changeType` (`created`, `updated`, `imported`, `rolled_back`, or
`legacy`), `sourceVersion` for rollbacks, `actorType`, `actorLabel`,
`createdAt`, `current`, and `restorable`, which is `false` once retention has
removed the value. Pass `limit` (1 to 200, default 50) and `before` to page;
the response carries `nextBefore` for the next page and `retainedVersions`,
the server's retention count. Items never contain values.

`POST /api/v1/environments/{id}/secrets/{key}/versions/{version}/reveal`
returns one version's value, and `POST /api/v1/environments/{id}/secrets/{key}/rollback`
with `{ "version": 3, "expectedVersion": 5 }` saves the value of v3 as a new
version. Both need a project manager, the CSRF header and recent password
confirmation for browser sessions, and are audited. Runner tokens and AI agent
tokens are refused with `403`.

A rollback answers with the new `version`, `sourceVersion`, and
`previousVersion`. It is refused, without writing anything, with:

| Status | Code                        | Meaning                                            |
| ------ | --------------------------- | -------------------------------------------------- |
| `404`  | `SECRET_NOT_FOUND`          | The secret does not exist                          |
| `404`  | `SECRET_VERSION_NOT_FOUND`  | The version does not exist                         |
| `409`  | `SECRET_VERSION_IS_CURRENT` | The version is already the current value           |
| `409`  | `SECRET_VERSION_CONFLICT`   | The current version is no longer `expectedVersion` |
| `410`  | `SECRET_VERSION_EXPIRED`    | Retention removed the value; reveal returns it too |

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
inconsistency. It needs `audit.verify`, which Admin and Auditor hold, from a
session or an audit token; runner tokens and other personal tokens are refused
with `403`.

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

## Authentication

- Browser sessions use an HttpOnly, SameSite Strict cookie and require the
  server-issued `X-Dopbase-CSRF` header for mutations. Sign-in returns the
  token, and `GET /api/v1/auth/session` returns `csrfToken` too, so a tab
  opened later can get one. That token is derived from the session's own, and
  both stay valid.
- CLI sessions, runner tokens, and personal tokens use `Authorization: Bearer <token>`.
- A personal token acts for its account within its own scope and level. It
  never holds the account's capabilities, so it can't manage people, grants,
  projects, or tokens.
- Projects, environments, and secrets the caller can't reach answer `404`;
  those it can see but not change answer `403`. Project and environment
  responses include the caller's `level`.
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

## Browser CLI login

The CLI signs in through the Admin UI with a device flow:

- `POST /api/v1/auth/cli/device` starts a request and returns a secret
  `deviceCode`, a `userCode` such as `WDJB-MJHT`, the approval link, the
  expiry, and the polling `interval`. It needs no authentication.
- `POST /api/v1/auth/cli/token` polls with the `deviceCode`. The status is
  `pending`, `slowDown` when polled faster than the interval, or `approved`
  with the new CLI session. A denied request fails with
  `DEVICE_ACCESS_DENIED`, an expired one with `DEVICE_CODE_EXPIRED`, and a
  used or cancelled one with `DEVICE_CODE_INVALID`.
- `POST /api/v1/auth/cli/cancel` withdraws a waiting request.
- `POST /api/v1/auth/cli/inspect`, `/approve`, and `/deny` take the `userCode`
  from a browser session with the CSRF header. Approving also needs a recent
  confirmation.

`GET /api/v1/auth/methods` is public and reports whether the server accepts
passwords, shows the password form, and offers single sign-on.

## Single sign-on

Root manages the provider under `/api/v1/sso/provider` (`GET`, `PUT`,
`DELETE`) and turns it on, requires it, or hides the password form with `PUT`
on `/enabled`, `/required`, and `/password-form`. All of these need a root
browser session; changes also need the CSRF header and a recent
confirmation. `GET` also returns `presets`, the providers the settings page
lists, and `PUT` refuses a `preset` that is not one of them with
`SSO_PRESET_INVALID`.

The browser signs in with `GET /api/v1/auth/sso/start`, which redirects to the
provider. The provider returns to `GET /api/v1/auth/sso/callback`, which sends
the browser to the Admin UI with a one-time code in the URL fragment, and
`POST /api/v1/auth/sso/exchange` trades that code for a session. Signed-in
users link with `POST /api/v1/auth/sso/link`, read or remove their identity
with `GET` or `DELETE /api/v1/auth/sso/identity`, and confirm sensitive
actions with `POST /api/v1/auth/sso/reauth`. Administrators remove another
user's link with `DELETE /api/v1/users/{id}/sso`.

## Compatibility

`GET /api/v1/health` identifies the product, binary version, and API version.
`dopbase client connect` validates this response before changing local client
configuration. Version {{version}} does not promise compatibility with future major
API versions or implement idempotency keys.
