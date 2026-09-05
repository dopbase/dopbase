---
title: "REST API"
description: "Dopbase exposes a versioned REST API with a generated OpenAPI 3 specification and Swagger UI from a running server."
---

# REST API

Dopbase exposes the versioned REST API used by the CLI and embedded Admin UI.
The generated OpenAPI 3 specification is available from a running server:

```text
OpenAPI JSON: http://localhost:8840/api/v1/openapi.json
Swagger UI:   http://localhost:8840/api/docs
```

The Rust request and response types are the source of truth. Every Axum route
is registered in the generated specification and checked by an API contract
test.

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
correlation is returned in the `X-Request-Id` header; errors never include
request bodies, plaintext secrets, tokens, key material, SQL, or filesystem
details.

## Authentication

- Browser sessions use an HttpOnly, SameSite Strict cookie and require the
  server-issued `X-Dopbase-CSRF` header for mutations.
- CLI sessions and runner identities use `Authorization: Bearer <token>`.
- A runner token can retrieve runtime secrets only from its assigned
  environment. It cannot list metadata, mutate secrets, reveal, or export.

The OpenAPI document declares the `cookieAuth`, `bearerAuth`, and `csrfHeader`
security schemes.

## Compatibility

`GET /api/v1/health` identifies the product, binary version, and API version.
`dopbase client connect` validates this response before changing local client
configuration. v0.0.14 does not yet promise compatibility with future major API
versions or implement idempotency keys.
