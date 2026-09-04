---
title: "dopbase client connect"
description: "Select the Dopbase server used by later client commands with dopbase client connect, including the localhost default."
---

# `dopbase client connect`

`dopbase client connect` selects the server used by later client commands.

When no server has been selected, Dopbase uses the local default automatically:

```text
http://localhost:8840
```

You can therefore start a default local server and sign in without running
`connect` first:

```bash
dopbase serve
dopbase login
```

## Connect to another server

Select a self-hosted or Cloud endpoint:

```bash
dopbase client connect https://dopbase.example.com
```

The command normalizes the URL and verifies that it is a compatible Dopbase
server before changing machine-global state. If validation fails, the previous
server and credential remain active.

An actual server change requires a yes/no confirmation. The warning identifies
the current and destination endpoints and explains that Dopbase will stop the
current managed background server when present, delete the encrypted local CLI
session, and clear the saved default environment. There is no non-interactive
bypass.

If a foreground `dopbase serve` process is using the same data directory, stop
it with Ctrl+C before switching. A `serve --background` process representing
the current endpoint is stopped automatically after confirmation. Remote
servers, unrelated local servers, and browser sessions are never stopped or
revoked.

Connecting does not authenticate. Sign in separately after switching:

```bash
dopbase login
```

## Return to the local server

Use the `local` alias:

```bash
dopbase client connect local
dopbase login
```

After validating `http://localhost:8840`, Dopbase removes the configured server
override and returns to the implicit local default.

`DOPBASE_URL` must be unset before changing the saved endpoint; otherwise its
environment override would remain active instead of the newly selected server.

## Cloud uses the same command

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

The Cloud URL has not been published. Dopbase Cloud is planned to use the same
client and REST model as a self-hosted server.

## Machine-global state

The selected endpoint is stored in the user's global Dopbase configuration,
not in an application repository. Login credentials are stored separately in
the encrypted local session file. An optional `dopbase run` default is saved as
an immutable environment ID scoped to this server.

Use `dopbase status` to inspect the effective endpoint and authentication
status without displaying token contents. Read [client configuration](./configuration)
for the file format, precedence rules, and multi-instance behavior.

## Safety behavior

Once a remote endpoint is configured, an outage causes client commands to fail.
Dopbase does not silently switch to localhost, another self-hosted server, or
Cloud. The implicit local default applies only when no server is configured or
overridden.
