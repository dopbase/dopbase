---
title: "Dopbase Cloud"
description: "Dopbase Cloud is the planned managed deployment of the Dopbase server for teams that want the same model without operating it."
---

# Dopbase Cloud

Dopbase Cloud is the planned managed deployment of the Dopbase server. It is for teams that want the same client and project model without operating the service themselves.

::: warning Not yet available
The public Cloud endpoint, service regions, availability commitments, and account model have not been announced.
:::

## The same client model

The CLI connects to Cloud through the same command used for self-hosting:

```bash
dopbase client connect <dopbase-cloud-url>
dopbase login
```

Commands such as `import`, `secret set`, `export`, and `run` then use the active
Cloud endpoint and an explicit environment reference.

## Managed responsibilities

Dopbase Cloud is intended to handle server operation, database maintenance, backups, updates, monitoring, recovery, and managed encryption-key infrastructure.

The final service capabilities and guarantees will be documented when Cloud is available.

## Independent deployments

A self-hosted server remains independent. Cloud does not register, control, monitor, or silently receive secrets from a self-hosted installation.
