# Install Dopbase and claim your local server

Tested with Dopbase **0.1.8 on macOS ARM64**, using separate local demo instances on ports 18840 and 19000. Linux uses the same CLI commands but was not replayed for this guide. Generated IDs, timestamps, and file paths will differ. Paths and setup tokens are redacted below.

This walkthrough starts a local server, creates your first administrator account, and connects the CLI. The demo uses separate data directories so it can run alongside an existing installation.

## Install and check the version

Install the release used here:

```bash
curl -fsSL https://dopbase.com/install.sh | DOPBASE_VERSION=0.1.8 sh
```

Expected installer excerpt on macOS ARM64, checked against the installer source. The installer itself was not executed for this walkthrough:

```text
Downloading Dopbase 0.1.8 for darwin/arm64...
Installed Dopbase 0.1.8 to <INSTALL_DIR>/dopbase
```

The script also prints quick-start links and a PATH reminder when needed. Installation defaults to `~/.local/bin`. Native releases support macOS and Linux on AMD64 and ARM64; Windows requires a Linux container.

Confirm the installed binary:

```bash
dopbase -v
```

```text
v0.1.8
```

If you see `command not found: dopbase`, add the default installation directory to this terminal's PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
dopbase -v
```

The export command prints nothing on success; the version command should print `v0.1.8`. Use your actual installation directory if you changed it.

## Start a separate demo server

In the first terminal:

```bash
dopbase --data-dir "$HOME/.dopbase-discussion-server" server start --port 18840
```

Captured startup output follows. `<LAB_DIR>/server` represents the separate server data directory used during verification; the final timestamped server log line is omitted:

```text
Dopbase
Secure, Simple and Private
Version 0.1.8

Public URL: http://localhost:18840
Bind:       127.0.0.1:18840
Admin UI:   http://localhost:18840
API:        http://localhost:18840/api/v1
Config:     <LAB_DIR>/server


Dopbase setup token (shown once):
<SETUP_TOKEN>

Or open this link to fill it in automatically:
http://localhost:18840/setup?token=<SETUP_TOKEN>
```

The server listens on loopback. Open `http://localhost:18840` or the setup link printed by your own server. Keep this terminal running.

## Claim the server in the Admin UI

The first visit opens the setup screen. Enter your one-time setup token, email address, and a password of 12 to 128 characters. After setup, the Admin UI opens the projects screen.

Use your own token and account details. The placeholders above cannot authenticate. The browser steps follow the [0.1.8 setup guide](https://github.com/dopbase/dopbase/blob/0.1.8/docs/ui/setup-and-sign-in.md); the verification lab checked the embedded UI's HTTP response and initialized the account through the released API.

## Connect the CLI

In a second terminal, give the client its own data directory. Clear process overrides so they do not select another endpoint or credential:

```bash
unset DOPBASE_URL DOPBASE_TOKEN DOPBASE_ENV
export DOPBASE_DATA_DIR="$HOME/.dopbase-discussion-client"
dopbase client connect http://localhost:18840
```

The `unset` and `export` commands print nothing on success. Type `y` at the confirmation. Captured output, with repeated terminal redraws removed:

```text
Warning: Plain HTTP does not encrypt traffic to http://localhost:18840. Credentials and secrets could be exposed. Use HTTPS whenever possible.
Change active Dopbase server?
Current: http://localhost:8840
New:     http://localhost:18840

This will:
- delete the saved CLI session and session key
- clear the saved default environment
? Continue? Yes
Connected to http://localhost:18840.
Previous server:              http://localhost:8840
Background server stopped:    no
CLI session removed:          no
Default environment cleared:  no
Run `dopbase login` to authenticate with the new server.
```

HTTP is used for this loopback-only demo. Use HTTPS for a remote deployment. The configured endpoint changes only after validation and confirmation.

Inspect the connection before signing in:

```bash
dopbase client status
```

```text
Config file:     <LAB_DIR>/client/config.toml
Server:          http://localhost:18840
Server status:   connected (live)
Server source:   config
Authentication:  none
Identity:        none
Email:           none
Environment:     none (set with `dopbase env default <ENVIRONMENT_REF>`)
```

`connected (live)` means the server responded. `Authentication: none` means the CLI has no saved login. Connecting does not sign you in.

## Sign in and verify

```bash
dopbase login
```

Captured interaction, with redraws removed and the password masked:

```text
Dopbase login
Server: http://localhost:18840

? Email: tutorial@example.com
? Password: ********
Logged in to http://localhost:18840.
```

Use the email and password you created, then check the result:

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

`encrypted_session` identifies the saved CLI credential. The example account is shown as `admin` by this version's status renderer. Nothing has selected a project environment yet.

## Stop or continue

Keep the first terminal running to follow [the import tutorial](https://github.com/dopbase/dopbase/discussions/62). To stop the demo, press Ctrl+C in the server terminal. This leaves your demo data available for another run. In the client terminal, `unset DOPBASE_DATA_DIR` returns later commands to their usual data directory and prints no output.

Which setup step needed a clearer explanation?
