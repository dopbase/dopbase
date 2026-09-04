---
title: "CLI overview"
description: "The Dopbase CLI starts a server, selects a remote server, manages projects and environments, and runs applications with their secrets."
---

# CLI overview

The Dopbase command-line interface starts a server, selects a remote server,
manages projects and environments, and runs applications with their secrets.

Both server and client commands ship in the same `dopbase` executable:

```bash
# Server role
dopbase serve

# Client role
dopbase status
dopbase login
dopbase init payment-service development --from .env
dopbase env create payment-service staging
dopbase import payment-service/staging .env.staging
dopbase run payment-service/development -- npm start
```

## No hidden project or environment

Without configuration, the CLI connects to `http://localhost:8840`. Connecting
to another endpoint saves only that server in the user's machine-global config.
Dopbase does not write repository configuration and does not save an active
project or environment.

Commands accept an environment directly. Use a readable reference such as
`payment-service/staging` in a terminal or an immutable `env_...` ID in a
deployment. Because each environment belongs to one project, no separate
project selection is required.

Read [target projects and environments](./environment-targeting) for the full
development, staging, and production workflow.

## Typical sequence

1. Start the default local server, or obtain another Dopbase endpoint.
2. Use implicit localhost or select another server with `dopbase client connect`.
3. Authenticate with `dopbase login` or a scoped runner token.
4. Bootstrap a project with `init`, or create project and environment resources.
5. Pass an environment reference to secret, import, export, or run commands.

## Output and secrets

Commands should work in both a terminal and automation. Human-readable and
structured output must never include a secret value unless the user explicitly
requests reveal or export behavior.

Errors identify the server, project, environment, and failed operation when
safe. They must not include request bodies, tokens, plaintext secrets, or
decrypted values.

## Exit behavior

Client commands return a nonzero status for invalid configuration, connection
failures, authentication failures, authorization failures, and rejected server
operations. `dopbase run` returns the child process exit status after the child
has started.
