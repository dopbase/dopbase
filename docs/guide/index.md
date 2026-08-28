# Introduction

Dopbase is an open-source secrets manager for application configuration. It gives developers one place to store values such as database URLs, API keys, and service credentials across development, staging, and production.

The product has three parts:

- A server that stores encrypted secrets and exposes an API and admin interface.
- A command-line client that connects to a Dopbase server.
- An optional managed service, Dopbase Cloud, that runs the server for you.

Both the server and client ship in the same `dopbase` executable. You choose the role through commands such as `dopbase serve` and `dopbase client connect`.

::: warning Pre-release documentation
Dopbase is still being designed and built. These pages describe the planned v0.1 experience. Commands and behavior may change before the first stable release.
:::

## Why Dopbase exists

A `.env` file is convenient on one machine. It becomes harder to manage when several developers, servers, CI jobs, and deployment environments need the same values. Copies drift apart, old credentials remain active, and nobody has a reliable history of what changed.

Dopbase keeps the useful parts of `.env` while adding structured storage, access control, history, audit records, and process injection. It aims to do that without requiring PostgreSQL, Redis, Kubernetes, or a collection of supporting services.

## The working model

```text
Project
  └── Environment
        └── Secrets
```

A project represents an application or service. Environments hold the values that application needs in a particular context. Each secret is an individual record, not one line inside an opaque file.

## Where to go next

- Follow the [quick start](./quick-start) for the planned first-run workflow.
- Learn how the [server and client](./server-client) divide responsibilities.
- Read the [self-hosting guide](/self-hosting/) before operating a server.
- Review the [security model](/reference/security) before storing real credentials.
