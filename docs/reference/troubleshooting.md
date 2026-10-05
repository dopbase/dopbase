---
title: "Troubleshooting"
description: "Fix common Dopbase problems: client connection failures, authentication errors, and server or storage issues."
---

# Troubleshooting

## Startup requires setup

Fresh `server start` normally enters web setup. If the web UI is disabled,
supply `DOPBASE_ROOT_EMAIL` to generate credentials or run explicit CLI setup
using the same `--data-dir`, `--config`, and master-key settings as startup.
Blank email values count as unset. Invalid nonblank values on fresh storage fail
before creating files; existing instances ignore bootstrap email values.

A pending factory reset stops automatic startup without deleting more data.
Run `dopbase server setup` with the same instance options to finish it. A damaged
database or mismatched master key is an error, not a reason to initialize again.

If setup says the instance is already initialized, do not reset it to recover a
password. Stop its server and use `dopbase admin reset-password EMAIL` with the
matching master key. This also recovers credentials lost after initialization
committed. Subsequent startup never generates a replacement password. See
[server setup](/cli/setup).

## The client cannot connect

1. Run `dopbase client status` and confirm the effective server and its source.
2. Confirm the server process is running.
3. Check the scheme, hostname, port, firewall, and TLS configuration.
4. Do not assume the client will fall back to another endpoint.

For local development, the default is `http://localhost:8840` when no endpoint
is configured or overridden. If a configured remote endpoint is unavailable,
Dopbase does not fall back to that local default.

## Authentication fails

Connecting and logging in are separate. Run `dopbase client status` to verify the
resolved endpoint and authentication source. A saved token is used only for the
normalized server that issued it. A token issued by one server is not reused on
another.

If the encrypted session is missing or damaged, run `dopbase logout` and then
`dopbase login` to create a new `session` and `session-key`. For externally
managed automation, provide a scoped token through `DOPBASE_TOKEN`.

If status shows an unknown email for an encrypted session created by an older
CLI, run `dopbase login` again to refresh the cached identity.

## An application cannot see a variable

Check the environment reference passed to `dopbase run`, then inspect its safe
metadata and keys:

```bash
dopbase env show payment-service/staging
dopbase secret list payment-service/staging
```

Confirm that the intended key exists and that the application was started with
the same readable reference or immutable environment ID. These commands do not
reveal secret values.

If no environment was passed and `DOPBASE_ENV` is unset, configure the active
server's default with:

```bash
dopbase env default payment-service/staging
```

## `dopbase run` cannot reach the server

After a successful live run, Dopbase can use the latest encrypted cache for the
same server, environment, and credential. The fallback warning includes when
the cache was fetched and how old it is. If no usable cache exists, Dopbase does
not inject variables or start the child.

An old cache cannot be unlocked after logout or token rotation. A damaged cache
or missing `run-cache-key` also fails closed. Start with:

```bash
dopbase cache list
dopbase cache clean --dry-run
```

If Dopbase reports a damaged or unreadable cache, it leaves the file unchanged
and prints its full path. First retry with the credential that created it. If
the file is damaged or no longer needed, move that one cache document out of
`run-cache/`, then run `dopbase run` while the server is available to create a
fresh document. Do not remove `run-cache-key` unless every local runtime cache
can be discarded. Do not remove the separate `session-key` unless you also
intend to invalidate the saved CLI login.

## A secret appeared in logs

Treat the value as exposed. Remove or restrict the log, rotate the credential at its source, update Dopbase, and review where else the log was shipped or retained. Do not copy the value into a public issue.

## A database or key is missing

Do not overwrite the remaining material. Recovery requires both a usable database backup and the correct separately stored master key. Restore the database using [`dopbase restore`](/cli/commands#restore) or follow the [backup restoration guide](/self-hosting/storage-backups#restoring-backups).
