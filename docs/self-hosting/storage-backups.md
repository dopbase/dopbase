---
title: "Storage and backups"
description: "Where Dopbase stores its SQLite database and configuration, and how to back up and restore a self-hosted server safely."
---

# Storage and backups

Dopbase Community uses SQLite by default. Unless `--data-dir` or
`DOPBASE_DATA_DIR` selects another location, its flat per-user layout is:

```text
~/.dopbase/
├── backups/
│   ├── dopbase_backup_20260905_120000.dop
│   └── ...
├── config.toml
├── dopbase.db
├── dopbase.db.lock
├── dopbase.db-shm
├── dopbase.db-wal
├── master.key
└── server.toml
```

The `-wal` and `-shm` files are managed by SQLite and may appear only while the
server is running. The `backups/` directory holds encrypted system snapshot
archives (`.dop` files). Do not edit or copy individual SQLite files while Dopbase is running.

## Database contents

The database contains encrypted secret values, wrapped data keys, nonces,
encryption-version metadata, the administrator, sessions, projects,
environments, runner tokens, and audit records.

It does not contain the master encryption key required to decrypt those records.
The local default keeps `master.key` in the same data directory for a simple
single-binary experience. Production operators should override the key path and
back up the key through a separately protected process.

## Built-in backup and restore system

Dopbase includes a comprehensive system snapshot and restoration engine. Unlike
per-environment export and import (which only touch secret values in dotenv format),
the backup system snapshots the entire database: projects, environments, secret
histories, runner tokens, and administrator accounts.

### Encrypted `.dop` archives

Backups are packaged as `.dop` files. Every `.dop` archive is:

1. **Snapshotted safely**: Uses SQLite `VACUUM INTO` or dynamic table attachment to extract a consistent point-in-time snapshot without stopping the server.
2. **Encrypted with the master key**: Encrypted with XChaCha20-Poly1305 using the server's 256-bit master key. The authentication tag ensures backups cannot be modified, inspected, or restored onto a server without the matching master key.
3. **Stored on the server**: Retained under `~/.dopbase/backups/` and accessible from the Admin UI and CLI.

### Creating backups

- **Via Admin UI**: Navigate to **Backups** in the main navigation. Click **Create Backup**, provide an optional custom name (e.g. `pre-v2-migration`), and confirm. The backup is generated on the server and appears immediately in the table with its size and timestamp. You can download the `.dop` file to your local computer at any time.
- **Via CLI**:
  ```bash
  # Generate a backup on the server with an automatic timestamp
  dopbase backup

  # Name the backup and download a local copy
  dopbase backup pre-deploy --output ./pre-deploy.dop
  ```

### Restoring backups

Restoring replaces current database tables with the snapshot contents, verifies
database integrity, and runs any outstanding schema migrations.

- **On a running server (Admin UI)**: On the **Backups** page, click the restore icon next to any listed backup or upload a `.dop` file with **Upload Backup**. You will be prompted to confirm the restoration. Your current administrator session is preserved during the restore.
- **During first-run setup (Admin UI)**: If you are setting up a new Dopbase server or disaster recovery host, visit `/setup`. Switch to the **Restore from Backup** tab, choose your `.dop` file, and click **Restore & initialize server**. The server decrypts and verifies the archive using its master key, populates the database, closes the initial setup window, and redirects you to sign in with the administrator credentials restored from the backup.
- **Via CLI**:
  ```bash
  # Interactive restore (prompts for confirmation and admin password)
  dopbase restore ./pre-deploy.dop

  # Non-interactive restore (for CI or automated disaster recovery)
  dopbase restore ./pre-deploy.dop --yes
  ```

### Server status requirement

Backups and restorations require a live, connected server. Before initiating `dopbase backup` or `dopbase restore`, the CLI verifies that the server endpoint is reachable and responsive (`server_status: connected (live)`). If the server is offline or stopped, the operation aborts immediately with an error instructing you to start the server (`dopbase serve`) before proceeding.

## Backup principles

- Use Dopbase's built-in `.dop` backups or a SQLite-safe backup method rather than copying a database during an unknown write state.
- Protect backups with the same care as the live database.
- Back up the master key through a separate protected process.
- Do not store the database backup and master-key backup together.
- Test restoration periodically to ensure disaster recovery readiness.
- Define retention and secure deletion rules before collecting long-term backups.

A database backup without the correct master key cannot restore plaintext secrets. A master key without the database is also insufficient. Losing either can make recovery impossible.
