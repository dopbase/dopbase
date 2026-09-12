---
title: "Roadmap"
description: "The Dopbase roadmap: what ships and what is planned next."
---

# Roadmap

The roadmap describes intent, not a release commitment. Work may move as implementation and security review uncover new requirements.

## Current release: v0.1.x

- Authentication with root, admin, member, and AI agent roles
- Projects, environments, and encrypted secrets stored in SQLite
- Embedded Admin UI, REST API, OpenAPI specification, and Swagger UI
- CLI support for dotenv, JSON, and YAML import and export
- Secret injection with `dopbase run` and an encrypted runtime cache
- Runtime cache inspection and cleanup
- Environment-scoped runner tokens, AI service accounts, and audit records
- User and AI account management
- Instance status and server log management
- Encrypted full-instance backups and recovery-aware factory reset
- Self-hosted binaries for macOS and Linux on AMD64 and ARM64

## v0.2.0

- Secret version history and rollback
- User invitations
- Broader service-account automation
- Improved role-based access control

## v0.3.0

- Environment inheritance
- GitHub Actions and GitLab CI integrations
- Docker workflows
- Webhooks and secret references

## v0.4.0

- AWS KMS, Google Cloud KMS, and Azure Key Vault
- Single sign-on
- Advanced auditing
- Cloud migration tools

## Later, based on demand

- High availability
- Enterprise policies and SCIM
- Dedicated or regional deployments
- Compliance tooling
- Automatic secret rotation

See [product status](/guide/product-status) for current capabilities and operational boundaries.
