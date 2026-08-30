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

Connecting does not authenticate. A successful server change removes the old
saved credential, so sign in separately:

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

## Cloud uses the same command

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

The Cloud URL has not been published. Dopbase Cloud follows the same client and
REST model as a self-hosted server.

## Machine-global state

The selected endpoint is stored in the user's global Dopbase configuration,
not in an application repository. Login credentials are stored separately in
the operating system credential store. No active project or environment is
saved.

Use `dopbase config` to inspect the effective endpoint and authentication
status without displaying token contents. Read [client configuration](./configuration)
for the file format, precedence rules, and multi-instance behavior.

## Safety behavior

Once a remote endpoint is configured, an outage causes client commands to fail.
Dopbase does not silently switch to localhost, another self-hosted server, or
Cloud. The implicit local default applies only when no server is configured or
overridden.
