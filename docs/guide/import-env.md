# Import a `.env` file

Dopbase treats `.env` as an import and export format. It does not store the file as one opaque object.

::: warning Planned interface
Import behavior, conflict handling, and command flags are still being specified.
:::

## Import

From your application directory:

```bash
dopbase import .env
```

Given this file:

```dotenv
DATABASE_URL=postgres://...
REDIS_URL=redis://...
API_KEY=abc123
```

Dopbase creates or updates three individually manageable secrets. The server encrypts their values before persistence.

## Before importing

Confirm that the client points to the intended server and that you selected the correct project and environment. Importing development values into production is easy to do and hard to notice from key names alone.

Do not paste secret values into command output, issue reports, or screenshots while troubleshooting an import.

## Export

`dopbase export` is planned for workflows that still require a `.env` file. Exporting writes plaintext credentials to disk, so the command should make that consequence explicit and create files with restrictive permissions where the platform supports them.

Prefer [`dopbase run`](./run-an-application) when an application only needs secrets in its process environment.
