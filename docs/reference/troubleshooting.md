---
title: "Troubleshooting"
description: "Fix common Dopbase problems: client connection failures, authentication errors, and server or storage issues."
---

# Troubleshooting

## The client cannot connect

1. Run `dopbase config` and confirm the effective server and its source.
2. Confirm the server process is running.
3. Check the scheme, hostname, port, firewall, and TLS configuration.
4. Do not assume the client will fall back to another endpoint.

For local development, the default is `http://localhost:8840` when no endpoint
is configured or overridden. If a configured remote endpoint is unavailable,
Dopbase does not fall back to that local default.

## Authentication fails

Connecting and logging in are separate. Run `dopbase config` to verify the
resolved endpoint and authentication source. A saved token is used only for the
normalized server that issued it; a token issued by one server is not reused on
another.

If the operating system credential store is unavailable, provide a scoped token
through `DOPBASE_TOKEN` instead of expecting `login` to write plaintext
credentials.

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

## A secret appeared in logs

Treat the value as exposed. Remove or restrict the log, rotate the credential at its source, update Dopbase, and review where else the log was shipped or retained. Do not copy the value into a public issue.

## A database or key is missing

Do not overwrite the remaining material. Recovery requires both a usable database backup and the correct separately stored master key. Follow the supported restore procedure when it becomes available.
