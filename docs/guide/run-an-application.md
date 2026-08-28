# Run an application

`dopbase run` starts a child process with secrets from one explicit environment.

::: warning Planned interface
The behavior on this page defines the intended v0.1 command and is not yet
available in a stable release.
:::

Use `--` to separate Dopbase arguments from the application command:

```bash
dopbase run payment-service/development -- npm run dev
```

The same pattern works with other commands:

```bash
dopbase run payment-service/development -- cargo run
dopbase run env_01ABCDEF -- python app.py
```

The environment may be an immutable `env_...` ID or a readable
`project/environment` reference. An environment already belongs to one project,
so no active project selection is needed.

## Deployment configuration

Application servers should use an immutable environment ID and a runner token
scoped to that environment:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN=<environment-runner-token>
dopbase run env_01ABCDEF -- ./payment-service
```

The environment can instead come from deployment-time configuration:

```bash
export DOPBASE_ENV=env_01ABCDEF
dopbase run -- ./payment-service
```

An explicit positional environment takes precedence over `DOPBASE_ENV`. If
neither is present, Dopbase fails rather than using the current directory or a
remembered environment.

Use different environment IDs and runner tokens for production and staging.
See [target projects and environments](/cli/environment-targeting) for a full
two-server example.

## Data flow

```text
Dopbase server
      ↓ authenticated encrypted connection
Dopbase client
      ↓ selected environment values
Application process
```

The client retrieves the allowed values and injects them into the child process.
It does not create a `.env` file.

Managed values override same-named variables inherited from the parent process.
Dopbase removes its own authentication variables before starting the child so
the application does not receive the credential used to retrieve its secrets.

## Runtime behavior

Before starting the application, Dopbase writes the resolved project,
environment, immutable ID, and loaded key count to standard error. Values are
never printed.

Dopbase then:

- Stops before launch if connection, authentication, authorization, or secret
  retrieval fails.
- Forwards operating-system signals to the child process.
- Returns the child's exit status to the calling shell.
- Avoids retaining plaintext values after the child starts where practical.

Applications still need to avoid printing their own environment variables.
Dopbase cannot prevent a child process from logging a value after receiving it.
