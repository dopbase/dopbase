# Open source

Secrets management asks users to trust software with credentials that can open databases, cloud accounts, and payment systems. Dopbase is open source so anyone can inspect how those credentials are stored, encrypted, revealed, and audited.

Public code makes security claims verifiable. You can read the implementation,
build the executable yourself, review changes, and report a vulnerability
without relying only on product copy. Open source does not prove that software
is secure, but it gives users and reviewers the evidence needed to test its
claims.

## Public commitments

The open-source edition includes the useful core:

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

Dopbase is licensed under the Apache License 2.0. The canonical license terms and project attribution are available in the repository's `LICENSE` and `NOTICE` files.

Dependencies and bundled assets retain their own licenses. A Dopbase release must include any third-party notices required by the contents of that distribution.

## Contributing

The repository's `CONTRIBUTING.md` explains local setup, checks, pull-request expectations, and how contributions are licensed. Dopbase does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.

Security vulnerabilities must be submitted through GitHub private vulnerability reporting as described in `SECURITY.md`. Never include real credentials, tokens, private endpoints, or undisclosed vulnerability details in a public issue or pull request.
