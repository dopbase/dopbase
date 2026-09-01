---
title: "Operations"
description: "Operate a Dopbase server safely: network and TLS setup, monitoring, upgrades, backups, and recovery for a secrets manager."
---

# Operations

Operating a secrets manager requires protecting both service availability and stored credentials.

## Network and TLS

Localhost is the expected development default. Any server reachable over a network needs authenticated encryption in transit, a deliberate bind address, and firewall rules that expose only the required interface.

Terminate TLS with a reviewed reverse proxy and restrict the Dopbase listener to the network interfaces that need it.

## Monitoring

Monitor service availability, database health, disk space, backup completion, authentication failures, and abnormal audit activity. Monitoring output must not contain request bodies or secret values.

## Logging

Application logs may include timestamps, operation names, request identifiers, status codes, and safe resource identifiers. They must never include plaintext secrets, credentials, decrypted request bodies, or authentication tokens.

## Upgrades

Back up the database and master key before an upgrade. Review the release notes for migration and compatibility information before replacing the executable.

## Incident response

An incident plan should cover a stolen database, exposed master key, leaked service token, unavailable server, failed migration, and corrupted backup. Each case has different recovery and rotation requirements.

## Production readiness checklist

- A supported Dopbase release is installed.
- TLS and network exposure are reviewed.
- The database and master key use separate protected storage.
- Backups are automated and restoration has been tested.
- Logs and monitoring have been checked for secret leakage.
- Upgrade and incident procedures have named owners.
