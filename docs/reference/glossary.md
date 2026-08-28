# Glossary

## Active server

The self-hosted or Cloud endpoint resolved by the client. It comes from an
explicit override, the machine-global config, or the implicit local default.

## Client configuration

Per-user machine-global state stored in `~/.dopbase/config.toml` or its Windows
equivalent. It contains the selected server but no token, project, environment,
or secret value.

## Audit event

A record of a security-relevant action. It contains safe context about the action but never the secret value.

## Data encryption key

A key used to encrypt secret data. In the planned envelope-encryption design, the master key encrypts this key before it is stored.

## Environment

A named set of secrets for one application context, such as development, staging, or production.

## Environment reference

An immutable environment ID such as `env_...` or a readable
`project/environment` name accepted by environment-scoped CLI commands.

## Machine identity

An identity used by a CI job, server, container, deployment system, or other non-interactive workload.

## Master key

Key material stored outside the Dopbase database and used to protect data encryption keys.

## Project

The Dopbase representation of an application or service. A project contains environments and their secrets.

## Secret

An individually managed key and encrypted value, together with metadata such as version and timestamps.

## Self-hosted server

A Dopbase server operated by the user or their organization.

## Service token

A revocable credential that allows a machine identity to perform permitted operations.

## Runner token

A service token scoped to retrieving secrets from one environment for
`dopbase run`. It cannot modify or export secrets.
