---
title: "Import and export secret files"
description: "Move secrets between Dopbase and dotenv, JSON, or YAML files."
---

# Import and export secret files

Dopbase accepts dotenv, JSON, and YAML files. It parses each file into
individual secret records instead of storing the file as one opaque object.

## Create a project from a secret file

Use `init` when the project does not exist yet:

```bash
dopbase init payment-service development --from .env
dopbase init another-service development --from secrets.json
dopbase init worker development --from secrets.yml
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

Dotenv input accepts blank lines, comments, quoted values, and empty values. It
does not expand variables or execute substitutions. JSON and YAML must contain
one flat object whose keys and values are strings. Nested objects, arrays,
numbers, booleans, null values, duplicate keys, and empty keys are rejected.
Every format must contain at least one secret. Dopbase validates the complete
input before changing server state.

The format is inferred from `.json`, `.yaml`, and `.yml`. Every other filename,
including `.env.production`, defaults to dotenv. Use `--format` to override the
filename or when reading from stdin:

```bash
dopbase import payment-service/staging secrets.data --format json
cat secrets.yml | dopbase import payment-service/staging - --format yaml
cat .env | dopbase init worker development --from - --format dotenv
```

Stdin imports need an existing valid login or `DOPBASE_TOKEN`. The CLI cannot
use the same stdin stream for both secret data and an interactive login.

## Export

Export requires an explicit file or stdout destination:

```bash
dopbase export payment-service/staging --output .env.staging
dopbase export payment-service/staging --output secrets.json
dopbase export payment-service/staging --stdout --format yaml
```

File output uses the filename rules above. Stdout defaults to dotenv. Pass
`--format` to choose another format. JSON and YAML keys are sorted so repeated
exports have stable output.

Export refuses to overwrite an existing file without `--force` and uses
restrictive permissions where supported. Export reveals plaintext values, so
it is permission-controlled and audited. Every CLI export requires an
interactive password confirmation. There is no non-interactive bypass.

Prefer [`dopbase run`](./run-an-application) when an application only needs
secrets in its process environment.

The Admin UI supports dotenv files. It parses the file locally, shows a key
review and a dry-run summary before applying, and downloads dotenv exports.
See [import and export](/ui/import-export).

## Handle values safely

Import and export output never echoes secret values as status information.
Do not paste `.env` contents, revealed output, private service URLs, or tokens
into logs, screenshots, issues, or shell history.
