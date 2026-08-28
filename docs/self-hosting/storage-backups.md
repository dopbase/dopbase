# Storage and backups

Dopbase Community is planned to use SQLite by default. A small installation may have a data layout similar to:

```text
dopbase_data/
└── dopbase.db
```

The exact path and configuration are not final.

## Database contents

The database is expected to contain encrypted secret values, encrypted data keys, nonces, encryption-version metadata, users, projects, environments, tokens, and audit records.

It must not contain the master encryption key required to decrypt those records.

## Backup principles

- Use a SQLite-safe backup method rather than copying a database during an unknown write state.
- Protect backups with the same care as the live database.
- Back up the master key through a separate protected process.
- Do not store the database backup and master-key backup together.
- Test restoration instead of assuming a copied file is usable.
- Define retention and secure deletion rules before collecting long-term backups.

A database backup without the correct master key cannot restore plaintext secrets. A master key without the database is also insufficient. Losing either can make recovery impossible.

## Restore procedure

The supported backup and restore commands will be documented after the storage implementation is stable. Until then, no file-copy example should be treated as a production procedure.
