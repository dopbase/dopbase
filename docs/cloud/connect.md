# Connect to Dopbase Cloud

The Cloud connection flow is planned to mirror self-hosting.

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

The first command selects Cloud as the active server. The second authenticates your user with that endpoint.

::: info Endpoint pending
`<dopbase-cloud-url>` is a placeholder. Do not substitute an assumed hostname or send credentials to an unofficial endpoint.
:::

## Switching from a self-hosted server

Selecting Cloud changes the endpoint used by later client commands. It does not move projects or secrets automatically.

Migration tools are planned for a later release. They will need to preserve resource identity, encryption guarantees, access controls, and auditability without revealing values in logs or intermediate files.

## Switching back

You can select a self-hosted endpoint again with `dopbase client connect`. The client should make the active endpoint visible so users can verify the destination before importing, exporting, or updating secrets.
