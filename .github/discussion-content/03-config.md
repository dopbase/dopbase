# Configure Dopbase: servers, CLI connections, and defaults

Tested with Dopbase **0.1.8 on macOS ARM64**, using separate local demo instances on ports 18840 and 19000. Linux uses the same CLI commands but was not replayed for this guide. Generated IDs, timestamps, and file paths will differ. Paths and setup tokens are redacted below.

Complete [installation](https://github.com/dopbase/dopbase/discussions/61) and [the import exercise](https://github.com/dopbase/dopbase/discussions/62) first. This guide uses their `discussion-demo/development` environment. Keep the server on port 18840 running.

In a new client terminal:

```bash
unset DOPBASE_URL DOPBASE_TOKEN DOPBASE_ENV
export DOPBASE_DATA_DIR="$HOME/.dopbase-discussion-client"
```

These shell commands print nothing on success.

## Know which settings you are changing

| File                                          | Purpose                                                    |
| --------------------------------------------- | ---------------------------------------------------------- |
| `<server-data-dir>/server.toml`               | Local server listener and other server settings            |
| `<client-data-dir>/config.toml`               | The CLI's selected server and optional default environment |
| `<client-data-dir>/session` and `session-key` | Separately stored encrypted login and its key              |

Without `--data-dir` or `DOPBASE_DATA_DIR`, the default data directory is `~/.dopbase`. A fresh client with no server override uses `http://localhost:8840`. This demo deliberately uses a separate client directory and port 18840.

Inspect the current endpoint:

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:18840
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           tutorial@example.com
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

This captured status is from just after installation. Your identity should still be authenticated; your environment field may differ if you already saved a default.

## Save and inspect a run default

```bash
dopbase env default discussion-demo/development
```

```text
Set discussion-demo/development as the default environment.
```

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:18840
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           tutorial@example.com
Environment:     env_856343 (default)
```

The saved default is an immutable environment ID associated with this server. You can now omit the environment argument.

After the import tutorial, `APP_NAME` has been updated. Run:

```bash
dopbase run -- sh -c 'test "$APP_NAME" = "discussion-demo-updated" && echo "Environment loaded"'
```

Captured output:

```text
Dopbase: discussion-demo/development (env_856343), 3 key(s), source=live
Environment loaded
```

The saved default selected development without a positional environment argument. The assertion matched the updated value imported in the previous tutorial.

## Override the default for one command

Create an environment with a different dummy value:

```bash
dopbase env create discussion-demo/staging
```

```text
Created environment discussion-demo/staging (env_955793).
```

Set it through stdin:

```bash
printf %s discussion-staging | dopbase secret set discussion-demo/staging APP_NAME --stdin
```

```text
Saved APP_NAME in discussion-demo/staging.
Version:  1
```

`printf %s` supplies the exact value without a trailing newline.

```bash
dopbase run discussion-demo/staging -- \
  sh -c 'test "$APP_NAME" = "discussion-staging" && echo "Staging selected"'
```

```text
Dopbase: discussion-demo/staging (env_955793), 1 key(s), source=live
Staging selected
```

The explicit environment overrides the saved development default for this command. The saved default itself stays unchanged.

Clear it:

```bash
dopbase env default --clear
```

```text
Cleared the default environment.
```

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:18840
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           tutorial@example.com
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

## Check a failed connection

The following failure was captured with no service listening on port 19999:

```bash
dopbase client connect http://localhost:19999
```

```text
Warning: Plain HTTP does not encrypt traffic to http://localhost:19999. Credentials and secrets could be exposed. Use HTTPS whenever possible.
Error: Could not connect to Dopbase at http://localhost:19999.
Check that the server is running and verify the active endpoint with `dopbase client status`.
```

Check the saved endpoint afterward:

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:18840
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           tutorial@example.com
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

It is still port 18840 and the login remains saved. Failed validation does not switch the active endpoint. If port 19999 is occupied on your machine, select another unused port for this exercise.

## Start and select another local server

In a third terminal, start a separate instance:

```bash
dopbase --data-dir "$HOME/.dopbase-discussion-custom-server" server start --port 19000
```

Captured startup excerpt; setup-token lines and the final timestamped log are omitted:

```text
Dopbase
Secure, Simple and Private
Version 0.1.8

Public URL: http://localhost:19000
Bind:       127.0.0.1:19000
Admin UI:   http://localhost:19000
API:        http://localhost:19000/api/v1
Config:     <LAB_DIR>/server-custom
```

Open `http://localhost:19000` and claim this new instance using its own setup token. It has a separate account and database. For the recorded demo, the same dummy account email was used on both instances.

Switch in the client terminal:

```bash
dopbase client connect http://localhost:19000
```

Captured interaction, with redraws removed:

```text
Warning: Plain HTTP does not encrypt traffic to http://localhost:19000. Credentials and secrets could be exposed. Use HTTPS whenever possible.
Change active Dopbase server?
Current: http://localhost:18840
New:     http://localhost:19000

This will:
- delete the saved CLI session and session key
- clear the saved default environment
? Continue? Yes
Connected to http://localhost:19000.
Previous server:              http://localhost:18840
Background server stopped:    no
CLI session removed:          yes
Default environment cleared:  no
Run `dopbase login` to authenticate with the new server.
```

The client uses a different data directory from both servers, so neither foreground server is stopped. If a foreground server shares your client data directory, stop it before switching. A matching managed background server can be stopped after confirmation.

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:19000
Server status:   connected (live)
Server source:   config
Authentication:  none
Identity:        none
Email:           none
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

The new endpoint is reachable, but the previous login was removed. Sign in with the account created on port 19000:

```bash
dopbase login
```

Captured interaction, with redraws removed:

```text
Dopbase login
Server: http://localhost:19000

? Email: tutorial@example.com
? Password: ********
Logged in to http://localhost:19000.
```

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:19000
Server status:   connected (live)
Server source:   config
Authentication:  encrypted_session
Identity:        admin
Email:           tutorial@example.com
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

## Connect to a remote instance

For an existing reachable HTTPS deployment, the command is `dopbase client connect https://dopbase.example.com`, followed by `dopbase login`. Replace that example URL with your endpoint. This remote example was not replayed; the URL is illustrative.

The same confirmation and login steps apply. Select the final endpoint rather than a redirect. `DOPBASE_URL` must be unset before changing the saved connection.

## Precedence and cleanup

Client endpoint selection follows `--server`, then `DOPBASE_URL`, then `config.toml`, then localhost. For `run`, the positional environment wins over `DOPBASE_ENV`, which wins over a default saved for that server. Server options take precedence over environment variables, which take precedence over `server.toml`.

Stop both demo servers with Ctrl+C in their own terminals. The demo data stays in the separate directories. Run `unset DOPBASE_DATA_DIR` in the client terminal when returning to your usual setup; it prints no output.

Read the [configuration reference](https://github.com/dopbase/dopbase/blob/0.1.8/docs/cli/configuration.md) for the schema. Which connection or environment setting would you like another example for?
