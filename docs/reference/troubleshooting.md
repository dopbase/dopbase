# Troubleshooting

Dopbase is pre-release, so stable diagnostic commands and error codes are not available yet. The checks below follow the intended server/client model.

## The client cannot connect

1. Confirm the server process is running.
2. Confirm the active endpoint is the one you intended to use.
3. Check the scheme, hostname, port, firewall, and TLS configuration.
4. Do not assume the client will fall back to another endpoint.

For local development, the planned default is `http://localhost:8376`.

## Authentication fails

Connecting and logging in are separate. Select the server first, then authenticate with that endpoint. A token issued by one server should not be assumed to work on another.

## An application cannot see a variable

Check the selected project and environment, confirm the key exists there, and verify that the application was started through `dopbase run`. The final inspection commands are not yet defined.

## A secret appeared in logs

Treat the value as exposed. Remove or restrict the log, rotate the credential at its source, update Dopbase, and review where else the log was shipped or retained. Do not copy the value into a public issue.

## A database or key is missing

Do not overwrite the remaining material. Recovery requires both a usable database backup and the correct separately stored master key. Follow the supported restore procedure when it becomes available.
