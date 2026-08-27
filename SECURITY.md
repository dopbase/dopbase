# Security policy

Dopbase handles application credentials, so a vulnerability can affect systems beyond the Dopbase server itself. Please report suspected security problems privately.

## Supported versions

Dopbase has no stable release and is not supported for production use. Security reports against the current source are still welcome, but the project does not yet provide release support periods, backports, or fixed response times.

This section will be replaced with a version support table when supported releases exist.

## Report a vulnerability

Use GitHub private vulnerability reporting:

1. Open the repository's **Security** tab.
2. Select **Report a vulnerability**.
3. Complete the private advisory form.

Do not open a public issue, discussion, or pull request for an undisclosed vulnerability. Do not include live credentials, customer data, private service URLs, or secrets from a system you do not own.

Include enough information to reproduce and assess the report safely:

- A description of the vulnerability and its impact
- The affected commit, branch, or version
- Required configuration and preconditions
- Reproduction steps or a minimal proof of concept
- Relevant logs with all secrets removed
- Any mitigation or fix you have already tested

## What happens next

Maintainers will review the report privately, confirm the affected behavior, and request more information when needed. If the issue is accepted, remediation and disclosure will be coordinated through the private advisory.

The project will credit reporters who want public credit unless legal, privacy, or safety concerns prevent it. Please allow time for investigation and a fix before publishing details.

## What belongs in a public issue

Feature requests, configuration questions, documentation corrections, and ordinary bugs without a security impact can use the public issue tracker. If you are unsure whether a problem is security-sensitive, use private vulnerability reporting.
