# Command reference

This page tracks the planned command surface. Detailed flags and machine-readable output will be documented when their interfaces stabilize.

| Command                               | Purpose                                           | Status           |
| ------------------------------------- | ------------------------------------------------- | ---------------- |
| `dopbase serve`                       | Start a self-hosted server                        | Planned for v0.1 |
| `dopbase client connect <server-url>` | Select the active server                          | Planned for v0.1 |
| `dopbase login`                       | Authenticate with the active server               | Planned for v0.1 |
| `dopbase init`                        | Create or associate a project                     | Planned for v0.1 |
| `dopbase projects`                    | List and select projects                          | Planned for v0.1 |
| `dopbase env`                         | List and select environments                      | Planned for v0.1 |
| `dopbase set`                         | Create or update a secret                         | Planned for v0.1 |
| `dopbase get`                         | Read secret metadata or explicitly reveal a value | Planned for v0.1 |
| `dopbase import`                      | Import structured secrets from `.env`             | Planned for v0.1 |
| `dopbase export`                      | Export secrets to `.env`                          | Planned for v0.1 |
| `dopbase run -- <command>`            | Inject secrets into a child process               | Planned for v0.1 |

## Authentication

Human users authenticate through `dopbase login`. CI systems, servers, containers, and deployment automation are expected to use service tokens or machine identities.

```bash
export DOPBASE_TOKEN=dbs_xxxxxxxxxxxxxxxxx
dopbase run -- npm start
```

The environment-variable name and token prefix above are provisional until the authentication interface is implemented.

## Reveal and export operations

Reading metadata should not reveal a value by default. Commands that reveal or export plaintext secrets need an explicit user action and a corresponding audit event.

## Automation

Stable exit codes, non-interactive authentication, structured output, and redaction rules are required before the CLI can be considered ready for production automation.
