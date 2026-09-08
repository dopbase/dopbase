---
title: "Product status"
description: "What ships in Dopbase 0.1.1: the Rust server, CLI, Admin UI, REST API, and SQLite storage in one executable, plus current operational boundaries."
---

# Product status

Dopbase 0.1.1 is the current release. It includes the Rust server and
CLI, the embedded Vue Admin UI, the REST API, generated OpenAPI and Swagger
documentation, and SQLite storage in one executable.

## Available in 0.1.1

- Projects, environments, and individually managed secrets
- Encryption before persistence with separate master-key material
- `.env` import and export
- An embedded browser Admin UI covering setup, sign-in, project and
  environment management, secret management with a `.env` editor, runner
  tokens, audit events, and instance status
- Process injection through `dopbase run`
- Human authentication and environment-scoped runner tokens
- Root, admin, member, and AI agent identities with role-aware permissions
- User and AI account management, account metadata, and last-sign-in details
- Instance status, resource health, and a root-only factory reset
- Root-only full-instance encrypted backups with account-aware restore
- Audit records
- Self-hosted binaries for macOS and Linux on AMD64 and ARM64

## Current boundaries

Version 0.1.0 requires a fresh data directory. Databases and `.dop` backups
created by earlier releases are not supported. Keep the matching master key
with every backup, and test restores before relying on them for recovery.

Dopbase Cloud is not available yet. The current release does not include
native Windows binaries, automatic upgrades, or managed backups. Windows users
must run Dopbase in a Linux container with Docker. Self-hosted operators remain
responsible for TLS, network access, database backups, master-key storage,
monitoring, upgrades, and incident response.

The source is public so anyone can inspect the implementation and report
security problems privately. Public source makes review possible, but it is not
the same as an independent security audit. Operators remain responsible for
security review, TLS, access controls, monitoring, and recovery testing.

Follow the [roadmap](/about/roadmap) for later work.
