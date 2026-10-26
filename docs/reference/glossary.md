---
title: "Glossary"
description: "Definitions for Dopbase terms: active server, roles, grants, personal tokens, environments, projects, master key, and more."
---

# Glossary

## Active server

The self-hosted or Cloud endpoint resolved by the client. It comes from an
explicit override, the machine-global config, or the implicit local default.

## AI agent

An account that uses personal tokens and never signs in. It has a name instead
of an email, an owner who answers for it, and never reaches more than its owner.

## Client configuration

Per-user machine-global state stored in `~/.dopbase/config.toml` on macOS and
Linux. It contains the selected server but no token, project, environment, or
secret value.

## Audit event

A record of a security-relevant action. It contains safe context about the action but never the secret value.

## Capability

A power over the instance that a role holds, such as `users.manage` or
`audit.read`. Capabilities are fixed in code; which role holds them is data.

## Data encryption key

A key used to encrypt secret data. In Dopbase's envelope-encryption design, the master key encrypts this key before it is stored.

## Environment

A named set of secrets for one application context, such as development, staging, or production.

## Environment reference

An immutable environment ID such as `env_...` or a qualified
`PROJECT_REF/ENVIRONMENT_NAME` value accepted by environment-scoped CLI
commands. `PROJECT_REF` can be a project ID or name.

## Grant

A level on all projects, one project, or one environment, given to a user or
a group. The most specific grant decides, capped by the role's ceiling.

## Group

A named set of accounts. A grant to the group reaches everyone in it.

## Level

How far into a project or environment an account reaches: names only, read,
write, manage, or project manager. Each includes the ones before it.

## Machine identity

An identity used by a CI job, server, container, deployment system, or other non-interactive workload.

## Master key

Key material stored outside the Dopbase database and used to protect data encryption keys.

## Personal token

A token (`dpp_...`) that acts as one account from a script or an AI agent. It
reaches at most what its account reaches, within its own scope and level, and
always expires.

## Project

The Dopbase representation of an application or service. A project contains environments and their secrets.

## Project reference

An immutable project ID such as `prj_...` or the project's unique name.

## Protected environment

An environment that grants on its project, below project manager, don't reach.
Only a grant on the environment itself, or a project manager, does.

## Role

What an account may do on the instance (its capabilities) and how deep into
project data its grants can reach (its ceiling). Admin, Member, Auditor, and
AI agent are built in; root creates custom roles.

## Root

The account created during first-run setup. It holds every capability, can't
be deleted, and keeps password sign-in when single sign-on is required.

## Secret

An individually managed key and encrypted value, together with metadata such as version and timestamps.

## Self-hosted server

A Dopbase server operated by the user or their organization.

## Service token

A revocable credential that allows a machine identity to perform permitted operations.

## Runner token

A service token scoped to retrieving and exporting secrets from one environment.
It supports `dopbase run` and `dopbase export`, but cannot modify secrets.
