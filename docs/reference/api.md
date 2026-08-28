# REST API

The Dopbase server will expose a REST API used by the CLI and admin interface. Self-hosted deployments and Dopbase Cloud are intended to follow the same API model.

::: warning API not published
There is no stable endpoint reference or OpenAPI document yet. Do not infer production paths, request bodies, or compatibility guarantees from conceptual examples in these docs.
:::

## Planned resource areas

The API is expected to cover:

- Authentication and sessions
- Projects and environments
- Secret metadata and encrypted value operations
- Human membership and service tokens
- Import and export
- Audit records

## Security requirements

- Authenticate every protected request.
- Authorize access at the relevant organization, project, environment, and operation.
- Use TLS for networked clients.
- Never include plaintext values in error messages or logs.
- Treat reveal and export as distinct, auditable operations.
- Use stable error shapes before declaring API compatibility.

## Compatibility

Versioning, pagination, rate limits, idempotency, error schemas, and deprecation policy remain open design work. This page will link to the OpenAPI definition when that contract exists.
