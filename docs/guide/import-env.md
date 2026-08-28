# Import a `.env` file

Dopbase treats `.env` as an import and export format. It does not store the
file as one opaque object.

::: warning Planned interface
The commands on this page specify intended v0.1 behavior and are not available
in a stable release yet.
:::

## Create a project from `.env`

Use `init` when the project does not exist yet:

```bash
dopbase init payment-service development --from .env
```

The file is validated first. Dopbase then creates the project, environment, and
individual secret records atomically. If validation or creation fails, no
partially imported project remains.

`init` fails if the project name already exists. To add another environment to
that project, create it explicitly and import into it:

```bash
dopbase env create payment-service staging
dopbase import payment-service/staging .env.staging
```

## Update an existing environment

Import merges by default:

```bash
dopbase import payment-service/staging .env.staging
```

Keys in the file are created or updated. Existing remote keys that are absent
from the file remain unchanged. Preview the operation without making changes:

```bash
dopbase import payment-service/staging .env.staging --dry-run
```

Use `--replace` only when the environment should exactly match the file.
Dopbase shows which keys would be deleted and requires confirmation or `--yes`.

The parser accepts blank lines, comments, quoted values, and empty values. It
does not expand variables or execute substitutions. Duplicate or invalid keys
reject the complete import before any server state changes.

## Export

Export requires an explicit file or stdout destination:

```bash
dopbase export payment-service/staging --output .env.staging
dopbase export payment-service/staging --stdout
```

File export refuses to overwrite an existing path without `--force` and uses
restrictive permissions where supported. Export reveals plaintext values, so
it is permission-controlled and audited.

Prefer [`dopbase run`](./run-an-application) when an application only needs
secrets in its process environment.

## Handle values safely

Import and export output never echoes secret values as status information.
Do not paste `.env` contents, revealed output, private service URLs, or tokens
into logs, screenshots, issues, or shell history.
