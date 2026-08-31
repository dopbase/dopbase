---
title: "Storage and backups"
description: "Where Dopbase stores its SQLite database and configuration, and how to back up and restore a self-hosted server safely."
---

# Storage and backups

Dopbase Community uses SQLite by default. Unless `--data-dir` or
`DOPBASE_DATA_DIR` selects another location, its flat per-user layout is:

```text
~/.dopbase/
├── config.toml
├── dopbase.db
├── dopbase.db.lock
├── dopbase.db-shm
├── dopbase.db-wal
├── master.key
└── server.toml
```

The `-wal` and `-shm` files are managed by SQLite and may appear only while the
server is running. `config.toml` and `server.toml` are created only when needed.
Do not edit, remove, or copy individual SQLite files while Dopbase is running.

## Database contents

The database contains encrypted secret values, wrapped data keys, nonces,
encryption-version metadata, the administrator, sessions, projects,
environments, runner tokens, and audit records.

It does not contain the master encryption key required to decrypt those records.
The local default keeps `master.key` in the same data directory for a simple
single-binary experience. Production operators should override the key path and
back up the key through a separately protected process.

## Backup principles

- Use a SQLite-safe backup method rather than copying a database during an unknown write state.
- Protect backups with the same care as the live database.
- Back up the master key through a separate protected process.
- Do not store the database backup and master-key backup together.
- Test restoration instead of assuming a copied file is usable.
- Define retention and secure deletion rules before collecting long-term backups.

A database backup without the correct master key cannot restore plaintext secrets. A master key without the database is also insufficient. Losing either can make recovery impossible.

## Offline file backup

For v0.0.8, stop Dopbase cleanly and wait for the process to exit before copying
`dopbase.db`. Clean shutdown checkpoints the WAL into the main database. Back up
the matching master key separately; never place the database and key copies in
the same backup location.

## Restore procedure

1. Stop Dopbase and confirm that the process has exited.
2. Restore `dopbase.db` to the configured data directory.
3. Restore the matching master key to its configured path through a separate protected process.
4. Restrict both files to the Dopbase operating-system user.
5. Start Dopbase and confirm the health endpoint, authentication, resource metadata, and a controlled secret retrieval.

A database and master key from different backup points may not form a usable pair. Keep backup identifiers and restoration records without recording secret values.
