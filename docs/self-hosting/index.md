---
title: "Self-hosted secrets manager"
description: "Run Dopbase as a self-hosted secrets manager with one executable, SQLite storage, an embedded Admin UI, and a separate master key."
---

# Self-hosted secrets manager

Dopbase runs as one self-hosted executable with SQLite storage and an embedded Admin UI.

```bash
dopbase serve
```

The command starts the service with local defaults. Operators still need to configure networking, TLS, master-key storage, backups, monitoring, upgrades, and recovery.

## What you operate

A self-hosted installation owns:

- The Dopbase process and its network exposure
- The SQLite database and filesystem permissions
- Master encryption key material
- TLS termination
- Backups and restore testing
- Updates, monitoring, and incident response

Dopbase Cloud is planned to handle these responsibilities for its managed
endpoint. It will not manage independent self-hosted servers.

## Start small

For local evaluation, bind the server to localhost and keep the database and master key in separate protected locations. Do not expose an evaluation instance to the public internet.

Before a production deployment, read:

- [Storage and backups](./storage-backups)
- [Encryption keys](./encryption-keys)
- [Operations](./operations)
- [Security model](/reference/security)
