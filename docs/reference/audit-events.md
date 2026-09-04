---
title: "Audit events"
description: "See which sensitive actions Dopbase records, who performed them, where they happened, and which secret data is excluded."
---

# Audit events

Audit records answer who performed a sensitive action, where it happened, and when. They describe the action without storing the secret value involved.

## Audit events in 0.0.13

| Area           | Events                                                                                                                                        |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Secrets        | `secret.created`, `secret.updated`, `secret.deleted`, `secret.revealed`, `secret.imported`, `secret.exported`, `secret.runtime_accessed`    |
| Projects       | `project.created`, `project.renamed`, `project.deleted`, `project.initialized`                                                                 |
| Environments   | `environment.created`, `environment.renamed`, `environment.deleted`                                                                            |
| Tokens         | `token.created`, `token.revoked`                                                                                                                |
| Authentication | `login.succeeded`, `login.failed`, `logout.succeeded`                                                                                           |
| Administrator  | `admin.bootstrapped`, `admin.reauthenticated`, `admin.password_changed`                                                                          |

The API schema defines the event names used by this release.

## Record contents

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
user agent
```

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

Retention, export, advanced filtering, and additional integrity guarantees remain roadmap work.
