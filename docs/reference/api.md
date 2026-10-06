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

| Area                                        | Base paths                                           |
| ------------------------------------------- | ---------------------------------------------------- |
| Health and compatibility                    | `/api/v1/health`                                     |
| Initial administrator                       | `/api/v1/bootstrap`                                  |
| Login and sessions                          | `/api/v1/auth`                                       |
| Projects                                    | `/api/v1/projects`                                   |
| Environments                                | `/api/v1/environments`                               |
| Secrets, import, export, and runtime values | `/api/v1/environments/{id}/secrets`                  |
| Runner tokens                               | `/api/v1/environments/{id}/tokens`, `/api/v1/tokens` |
| System backups and restoration              | `/api/v1/backups`, `/api/v1/bootstrap/restore`       |
| Audit and instance status                   | `/api/v1/audit-events`, `/api/v1/instance`           |
| Human users                                 | `/api/v1/users`                                      |
| AI service accounts                         | `/api/v1/service-accounts`                           |
| Instance summary                            | `/api/v1/status`                                     |

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
