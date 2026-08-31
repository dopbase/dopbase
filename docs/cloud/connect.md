---
title: "Connect to Dopbase Cloud"
description: "How the Dopbase client will connect to Dopbase Cloud, mirroring the self-hosted connect and login flow."
---

# Connect to Dopbase Cloud

The Cloud connection flow is planned to mirror self-hosting.

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

The first command validates Cloud and stores its URL in the machine-global
configuration. The second authenticates your user and saves the token in the
operating system credential store.

::: info Endpoint pending
`<dopbase-cloud-url>` is a placeholder. Do not substitute an assumed hostname or send credentials to an unofficial endpoint.
:::

## Switching from a self-hosted server

Selecting Cloud changes the endpoint used by later client commands. It does not move projects or secrets automatically.

Migration tools are planned for a later release. They will need to preserve resource identity, encryption guarantees, access controls, and auditability without revealing values in logs or intermediate files.

## Switching back

You can select a self-hosted endpoint again with `dopbase client connect`. The client should make the active endpoint visible so users can verify the destination before importing, exporting, or updating secrets.

Use `dopbase client connect local` to return to the implicit
`http://localhost:8840` default. Connecting validates the destination and clears
the credential from the previous active server; run `dopbase login` against the
new destination.

The selected Cloud URL is stored in the user's machine-global config. The login
token is stored in the operating system credential store, not in the TOML file
or an application repository. `dopbase config` displays the effective endpoint
and authentication status without revealing the token.
