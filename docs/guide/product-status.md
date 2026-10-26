---
title: "Product status: what ships in latest version"
description: "What ships in Dopbase latest version: the Rust server, CLI, Admin UI, REST API, and SQLite storage in one executable, plus current operational boundaries."
---

# Product status

Dopbase {{version}} is the current release. It includes the Rust server and
CLI, the embedded Vue Admin UI, the REST API, generated OpenAPI and Swagger
documentation, and SQLite storage in one executable.

## Available in {{version}}

- Projects, environments, and individually managed secrets
- Encryption before persistence with separate master-key material
- Guided project and environment creation from `./.env` through `dopbase init`
- dotenv, JSON, YAML, and TOML import and export through the CLI, plus Docker
  env-file export
- An embedded browser Admin UI covering setup, sign-in, project and
  environment management, secret management with a `.env` editor, runner
  tokens, audit events, and instance status
- Process injection through `dopbase run`, with `--only` and `--preserve-env`
  to choose which secrets reach the process
- Secret version history and rollback in the CLI and the Admin UI
- Human authentication and environment-scoped runner tokens
- Single sign-on for the Admin UI through one OpenID Connect provider, and
  browser approval for `dopbase login`
- Roles that are data: Admin, Member, Auditor, and AI agent built in, and
  custom roles that root creates
- Access granted per project and per environment, to people and to groups,
  with protected environments
- Personal tokens for scripts and AI agents, and AI agent accounts with an
  accountable owner
- An audit log each reader sees according to their role, and audit tokens for
  a SIEM
- User and AI agent management, account metadata, and last-sign-in details
- Instance status, resource health, and a root-only factory reset
- Root-only full-instance encrypted backups with account-aware restore
- Audit records in a hash chain, with `dopbase audit verify`, `list`, and
  `export`, and configurable retention
- Self-hosted binaries for macOS and Linux on AMD64 and ARM64

## Current boundaries

The server migrates an existing database when it starts. From 0.1.10,
migrations use timestamp versions, and a database that 0.1.10 has opened
cannot go back to 0.1.9. Back up the database and master key before you
upgrade. Databases and `.dop` backups from releases before 0.1.0 are not
supported. Keep the matching master key with every backup, and test restores
before relying on them for recovery.

Dopbase Cloud is not available yet. The current release does not include
native Windows binaries, automatic upgrades, or managed backups.

Windows users must run Dopbase in a Linux container with Docker. Self-hosted operators remain responsible for TLS, network access, database backups, master-key storage,
monitoring, upgrades, and incident response.

The source is public so anyone can inspect the implementation and report
security problems privately. Public source makes review possible, but it is not
the same as an independent security audit. Operators remain responsible for
security review, TLS, access controls, monitoring, and recovery testing.

Follow the [roadmap](/about/roadmap) for later work.
