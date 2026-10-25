---
title: "Roadmap"
description: "The Dopbase roadmap: what ships and what is planned next."
---

# Roadmap

The roadmap describes intent, not a release commitment. Work may move as implementation and security review uncover new requirements.

## Current release: v0.1.x

Version 0.1 builds the core of a self-hosted secrets manager. One executable
stores encrypted secrets, injects them into applications, and gives operators
what they need to run, audit, and recover an instance.

- Authentication with root, admin, member, and AI agent roles
- Projects, environments, and encrypted secrets stored in SQLite
- Embedded Admin UI, REST API, OpenAPI specification, and Swagger UI
- Guided project and environment creation from `./.env` through `dopbase init`
- CLI support for dotenv, JSON, YAML, and TOML import and export, plus Docker
  env-file export
- Secret injection with `dopbase run` and an encrypted runtime cache
- Runtime cache inspection and cleanup
- Secret version history and rollback
- Single sign-on for the Admin UI through OpenID Connect, and browser
  approval for CLI login
- Environment-scoped runner tokens, AI service accounts, and audit records in
  a verifiable hash chain
- User and AI account management
- Instance status and server log management
- Encrypted full-instance backups and recovery-aware factory reset
- Self-hosted binaries for macOS and Linux on AMD64 and ARM64

## Trust and security, in every release

People trust Dopbase with production credentials, so security work does not
wait for a version number. These items ship as they are ready, and the
independent review is published before 1.0.

- Signed release archives with Sigstore, plus an SBOM and build provenance
  for every release
- A published threat model and a security policy with response times for
  reported vulnerabilities
- An independent penetration test and cryptography review, with the report
  made public
- A bug bounty program
- Written project governance and credit for outside contributors

## v0.2.0: Team features

Version 0.2 is for teams. It makes bringing people in easier and lets admins
decide what each person can see and change, down to a single environment.

- User invitations, so new members set their own password or sign in through
  single sign-on
- Teams that group users and receive access together
- Improved role-based access control, with custom roles in place of the four
  fixed ones
- Access granted per project and per environment
- SAML single sign-on next to OpenID Connect
- SCIM provisioning, so the identity provider adds and removes users
- Account identity
- Broader service-account automation
- Restoring deleted secrets, environments, and projects

## v0.3.0: Integrations and key management

Version 0.3 connects Dopbase to the places secrets are used, such as CI
pipelines, containers, and Kubernetes, and lets the master key live in a cloud
key service.

- AWS KMS, Google Cloud KMS, and Azure Key Vault
- Master-key rotation without downtime
- Environment inheritance
- GitHub Actions and GitLab CI integrations
- Docker workflows
- A Kubernetes operator that syncs secrets into clusters
- A Terraform provider
- SDKs for Node.js, Python, and Go
- Webhooks and secret references
- PostgreSQL as a storage backend next to SQLite
- Native Windows binaries

## v0.4.0: Compliance and operations

Version 0.4 is for organizations with audit and compliance requirements, and
for operators who run Dopbase where downtime is not an option.

- High availability with more than one server node
- Audit log streaming to SIEM tools
- Advanced auditing
- Compliance tooling
- Automatic secret rotation
- Scheduled backups to S3-compatible storage, with restore checks
- Automatic upgrades
- Cloud migration tools

## v1.0.0: Stable and global

Version 1.0 is the release teams can build on for years. Interfaces stop
changing without notice, and Dopbase Cloud opens to teams around the world.

- Stable REST API, CLI flags, and database migrations, with a published
  deprecation policy
- A long-term support line and a written upgrade policy
- The independent security review published
- Dopbase Cloud, a managed Dopbase server, with data regions in the EU and
  the US
- SOC 2 Type II and ISO 27001 for Dopbase Cloud, and a GDPR data processing
  agreement
- A public status page for Dopbase Cloud
- The Admin UI and documentation in more languages

## Later, based on demand

- Enterprise policies
- Dedicated deployments and more data regions

See [product status](/guide/product-status) for current capabilities and operational boundaries.
