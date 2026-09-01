---
title: "Secrets management for developers"
description: "Learn how Dopbase gives developers an open-source way to store application secrets by project and environment and load them into processes."
---

# Secrets management for developers

Dopbase is an open-source secrets manager for developers. It stores application configuration such as database URLs, API keys, and service credentials across development, staging, and production.

The product has three parts:

- A server that stores encrypted secrets and exposes an API and admin interface.
- A command-line client that connects to a Dopbase server.
- The planned Dopbase Cloud service, which will provide a managed server.

Both the server and client ship in the same `dopbase` executable. You choose the role through commands such as `dopbase serve` and `dopbase client connect`.

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

- Follow the [quick start](./quick-start) to install Dopbase and run it locally.
- Learn how the [server and client](./server-client) divide responsibilities.
- Manage secrets in the browser with the [Admin UI](/ui/).
- Read the [self-hosting guide](/self-hosting/) before operating a server.
- Review the [security model](/reference/security) before storing real credentials.
