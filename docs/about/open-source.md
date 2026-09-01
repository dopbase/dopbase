---
title: "Open-source secrets manager"
description: "Inspect how the open-source Dopbase secrets manager stores, encrypts, reveals, and audits application credentials."
---

# Open-source secrets manager

Secrets management asks users to trust software with credentials that can open databases, cloud accounts, and payment systems. Dopbase is open source so anyone can inspect how those credentials are stored, encrypted, revealed, and audited.

The public [source code](https://github.com/dopbase/dopbase) lets you inspect
the implementation, build the executable, review changes, and report a
vulnerability without relying on product copy alone. Open source does not prove
that software is secure, but it gives developers and reviewers evidence they
can examine.

## Public commitments

The open-source edition includes:

- The single server and client executable
- SQLite storage
- The admin interface and REST API
- Projects, environments, and encrypted secrets
- `.env` import and export
- Process injection
- Service tokens and basic permissions
- Audit records

Self-hosting is a supported product experience rather than a trial for Cloud.

## Project license

Dopbase is licensed under the [Apache License 2.0](https://github.com/dopbase/dopbase/blob/main/LICENSE). Project attribution is available in the repository's [NOTICE](https://github.com/dopbase/dopbase/blob/main/NOTICE) file.

Dependencies and bundled assets retain their own licenses. A Dopbase release must include any third-party notices required by the contents of that distribution.

## Contributing

The repository's [contribution guide](https://github.com/dopbase/dopbase/blob/main/CONTRIBUTING.md) explains local setup, checks, pull-request expectations, and how contributions are licensed. Dopbase does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.

Security vulnerabilities must be submitted through GitHub private vulnerability reporting as described in the [security policy](https://github.com/dopbase/dopbase/security/policy). Never include real credentials, tokens, private endpoints, or undisclosed vulnerability details in a public issue or pull request.
