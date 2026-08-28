# Run an application

`dopbase run` starts a child process with secrets from the selected project and environment.

::: warning Planned interface
Project selection, environment selection, and process exit behavior are not final.
:::

Use `--` to separate Dopbase arguments from the application command:

```bash
dopbase run -- npm run dev
```

The same pattern works with other commands:

```bash
dopbase run -- cargo run
dopbase run -- python app.py
```

## Data flow

```text
Dopbase server
      ↓ encrypted connection
Dopbase client
      ↓ process environment
Application
```

The client retrieves the allowed values and injects them into the child process environment. It does not need to create a `.env` file.

## Expected safety rules

- Stop if the server cannot be reached or authentication fails.
- Do not print secret values while preparing the environment.
- Pass the child process exit status back to the calling shell.
- Avoid keeping plaintext values after the child process starts.
- Make project and environment selection visible before execution.

Applications still need to avoid printing their own environment variables. Dopbase cannot prevent a child process from logging a value after receiving it.
