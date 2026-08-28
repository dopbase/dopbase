# CLI overview

The Dopbase command-line interface starts a server, selects a remote server, manages secrets, and runs applications with those secrets.

Both server and client commands ship in the same planned `dopbase` executable:

```bash
# Server role
dopbase serve

# Client role
dopbase client connect http://localhost:8376
dopbase login
dopbase projects
dopbase env
dopbase set
dopbase get
dopbase import
dopbase export
dopbase run
```

::: warning Pre-release command reference
The command groups express the intended v0.1 interface. Flags, output, configuration locations, and compatibility guarantees are not final.
:::

## Typical sequence

1. Start a server or obtain a Dopbase Cloud endpoint.
2. Select it with `dopbase client connect`.
3. Authenticate with `dopbase login` or a service token.
4. Select a project and environment.
5. Manage secrets or run an application.

## Output and secrets

Commands should be useful in both a terminal and automation. Human-readable output must never include a secret value unless the user explicitly requests a reveal or export operation.

Errors should identify the server, project, environment, and failed operation when safe. They must not include request bodies, tokens, plaintext secrets, or decrypted values.

## Exit behavior

The final exit-code contract is not yet published. Client commands are expected to return a nonzero status for invalid configuration, connection failures, authentication failures, authorization failures, and rejected server operations.
