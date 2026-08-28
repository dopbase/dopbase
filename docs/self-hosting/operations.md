# Operations

Running a secrets manager means protecting both availability and confidentiality. A working process is not enough by itself.

## Network and TLS

Localhost is the expected development default. Any server reachable over a network needs authenticated encryption in transit, a deliberate bind address, and firewall rules that expose only the required interface.

Production TLS and reverse-proxy guidance will be published after the server configuration is implemented.

## Monitoring

Monitor service availability, database health, disk space, backup completion, authentication failures, and abnormal audit activity. Monitoring output must not contain request bodies or secret values.

## Logging

Application logs may include timestamps, operation names, request identifiers, status codes, and safe resource identifiers. They must never include plaintext secrets, credentials, decrypted request bodies, or authentication tokens.

## Upgrades

A supported release needs documented database migrations, version compatibility, rollback limits, and backup requirements. Do not automate upgrades until those guarantees are published.

## Incident response

An incident plan should cover a stolen database, exposed master key, leaked service token, unavailable server, failed migration, and corrupted backup. Each case has different recovery and rotation requirements.

## Production readiness checklist

- A supported Dopbase release is installed.
- TLS and network exposure are reviewed.
- The database and master key use separate protected storage.
- Backups are automated and restoration has been tested.
- Logs and monitoring have been checked for secret leakage.
- Upgrade and incident procedures have named owners.
