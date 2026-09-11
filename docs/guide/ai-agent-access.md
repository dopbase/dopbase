---
title: "AI agent access"
description: "Give an AI coding agent access to Dopbase project structure and secret metadata without exposing secret values."
---

# AI agent access

An AI agent account lets a coding assistant inspect how projects are configured
without giving it the credentials stored in those projects. The agent can see
project and environment names, secret names, versions, timestamps, editor
layouts, and instance counts. It cannot read a secret value or change Dopbase
data.

This access is useful when an agent needs configuration context to work on a
repository. It is not a credential for running the application.

## When to use an AI agent

Use an AI agent token for tasks such as:

- checking whether variables referenced by the code exist in Dopbase;
- comparing the secret names in development, staging, and production;
- finding a missing or renamed variable without exposing its value
- creating an `.env.example` file with empty placeholders
- reviewing deployment files that refer to environment variables.

An agent token cannot verify whether a stored credential works, run integration
tests that need secret values, deploy an application, or diagnose a malformed
value. Use an environment-scoped [runner token](./run-an-application) when a
process needs runtime secrets.

::: warning Metadata is still sensitive
Secret names and project structure can reveal details about your infrastructure.
AI agents can currently see every project and environment on the Dopbase
instance. Create an account only for an agent you trust.
:::

## Create an account and token

Root and admin users create AI accounts from the Admin UI:

1. Open **Users** and choose **Add AI agent**.
2. Give the account a name that identifies the tool or job using it.
3. Choose **Get token** beside the new account.
4. Confirm your password and copy the token when it appears.

Agent tokens start with `dpa_`. The Admin UI creates a token that expires after
30 days and revokes any previous active token for that account. Dopbase shows
the plaintext token once, so put it in a password manager, CI secret store, or
another protected credential store.

Do not commit the token, add it to an `.env` file in the repository, paste it
into an agent prompt, or include it in logs.

## Give the token to an agent

Dopbase reads an agent token from `DOPBASE_TOKEN`. Populate
`AI_AGENT_TOKEN` from your credential store before starting the agent, then set
the server and token in its shell environment:

```bash
export DOPBASE_URL=https://dopbase.example.com
export DOPBASE_TOKEN="$AI_AGENT_TOKEN"

dopbase client status
```

`client status` reports the identity as `ai_agent` without printing the token.
For a trusted local setup, you can save the token in Dopbase's encrypted client
session instead:

```bash
printf '%s' "$AI_AGENT_TOKEN" | dopbase login --token
```

Run this setup outside the agent conversation so the token never becomes part
of its prompt.

## Inspect configuration metadata

The agent can use normal CLI commands. JSON output is easier for automated
tools to parse:

```bash
dopbase --json project list
dopbase --json env list payment-service
dopbase --json secret list payment-service/staging
dopbase --json secret get payment-service/staging DATABASE_URL
```

These commands return identifiers, names, versions, and timestamps. They never
return the value of `DATABASE_URL` or any other secret.

For example, ask a coding agent:

```text
Find every environment variable referenced by this repository. Compare those
names with payment-service/staging in Dopbase and report missing or unused
names. Use metadata commands only. Do not attempt to reveal or change secrets.
```

The agent can inspect the repository, call `dopbase secret list`, and compare
the two sets of names. A human can then decide whether any missing secrets
should be added.

## Operations that are denied

An AI agent token cannot reveal values, change secrets, or load runtime values
into a process. These commands fail authorization when the active credential is
an agent token:

```bash
dopbase secret get payment-service/staging DATABASE_URL --reveal
printf '%s' 'replacement' | \
  dopbase secret set payment-service/staging DATABASE_URL --stdin
dopbase run payment-service/staging -- npm test
```

Do not replace the agent token with a human session to bypass these checks. If
the task needs to run an application, create a runner token for that specific
environment and follow [Run an application](./run-an-application).

## Revoke access

Return to **Users** and delete the AI account to remove the account and its
tokens. Generating another token from the Admin UI also revokes the account's
previous active token. Revoke access as soon as a token may have been exposed.

See [Identity and tokens](/reference/identity#ai-agents) for the complete
permission matrix.
