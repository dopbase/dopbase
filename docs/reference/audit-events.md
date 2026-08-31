# Audit events

Audit records answer who performed a sensitive action, where it happened, and when. They describe the action without storing the secret value involved.

## Audit events in 0.0.8

| Area           | Events                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------ |
| Secrets        | `secret.created`, `secret.updated`, `secret.deleted`, `secret.revealed`, `secret.exported` |
| Projects       | `project.created`, `project.deleted`                                                       |
| Environments   | `environment.created`, `environment.deleted`                                               |
| Members        | `member.invited`, `member.removed`                                                         |
| Tokens         | `token.created`, `token.revoked`                                                           |
| Authentication | `login.succeeded`, `login.failed`                                                          |

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
