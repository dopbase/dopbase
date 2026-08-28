# Self-hosting Dopbase

Self-hosting is a first-class Dopbase workflow. The planned Community edition runs as one executable with SQLite storage and an embedded admin interface.

```bash
dopbase serve
```

That command is intentionally simple. Operating a secrets server safely still requires decisions about networking, TLS, master-key storage, backups, monitoring, upgrades, and recovery.

::: warning Pre-release operations guide
Dopbase has not published a production release or support policy. This section describes the intended operational model, not a production-ready deployment procedure.
:::

## What you operate

A self-hosted installation owns:

- The Dopbase process and its network exposure
- The SQLite database and filesystem permissions
- Master encryption key material
- TLS termination
- Backups and restore testing
- Updates, monitoring, and incident response

Dopbase Cloud handles these responsibilities for its managed endpoint. It does not manage independent self-hosted servers.

## Start small

For local evaluation, bind the server to localhost and keep the database and master key in separate protected locations. Do not expose an evaluation instance to the public internet.

Before a production deployment, read:

- [Storage and backups](./storage-backups)
- [Encryption keys](./encryption-keys)
- [Operations](./operations)
- [Security model](/reference/security)
