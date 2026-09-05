---
title: "Encryption keys"
description: "Keep the Dopbase master encryption key outside the database that stores encrypted secrets, and plan for its backup and recovery."
---

# Encryption keys

Dopbase must keep the master encryption key outside the database that stores encrypted secrets.

```text
dopbase.db
    +
master encryption key stored separately
```

This separation means that stealing the database alone should not reveal plaintext secret values.

## Current key source

Dopbase 0.0.14 uses a 256-bit master key in a local owner-only file. Set its
location in `server.toml`, with `DOPBASE_MASTER_KEY_PATH`, or with the
`--master-key-file` option. Dopbase creates the file when it initializes a new
instance and verifies it before opening the HTTP listener.

External key managers remain roadmap work. Future providers may include:

- AWS KMS
- Google Cloud KMS
- Azure Key Vault
- A hardware security module

Only implemented and reviewed providers appear in the configuration reference.

## Operator responsibilities

- Restrict access to key material to the Dopbase process and authorized operators.
- Keep key backups separate from database backups.
- Prevent keys from appearing in shell history, process arguments, logs, or support bundles.
- Plan rotation before the original key is compromised or retired.
- Test recovery with the same provider and access policy used in production.

The current release does not automate master-key rotation. Protect and back up
the file separately from the SQLite database.
