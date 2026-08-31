---
title: "Product status"
description: "What ships in Dopbase 0.0.8: the Rust server, CLI, Admin UI, REST API, and SQLite storage in one executable, plus what is planned for 0.1.0."
---

# Product status

Dopbase 0.0.8 is the current testing release. It includes the Rust server and
CLI, the embedded Vue Admin UI, the REST API, generated OpenAPI and Swagger
documentation, and SQLite storage in one executable. It is intended for testing
and evaluation before the first public release, 0.1.0.

## Available in 0.0.8

- Projects, environments, and individually managed secrets
- Encryption before persistence with separate master-key material
- `.env` import and export
- An embedded browser Admin UI covering setup, sign-in, project and
  environment management, secret management with a `.env` editor, runner
  tokens, audit events, and instance status
- Process injection through `dopbase run`
- Human authentication and environment-scoped runner tokens
- Audit records
- Self-hosted binaries for macOS and Linux on AMD64 and ARM64

## Current boundaries

Dopbase Cloud is not available yet. The current release does not include a
Windows binary, automatic upgrades, or managed backups. Self-hosted operators
remain responsible for TLS, network access, database backups, master-key
storage, monitoring, upgrades, and incident response.

The source is public so anyone can inspect the implementation and report
security problems privately. Public source makes review possible, but it is not
the same as an independent security audit. Do not treat this testing release as
production-ready secret storage.

Follow the [roadmap](/about/roadmap) for the planned 0.1.0 public release and
later work.
