# Product status

Dopbase is in the pre-release design and implementation stage. The documentation is public so users and contributors can see the intended behavior before the first stable release.

## What this means

Examples marked as planned describe the v0.1 interface. They are a product contract under development, not proof that a downloadable release already exists.

The following areas are part of the planned v0.1 scope:

- One executable containing the server and command-line client
- SQLite storage for self-hosted installations
- Projects, environments, and individually managed secrets
- Encryption before persistence
- A REST API and admin interface
- `.env` import and export
- Process injection through `dopbase run`
- Human authentication, service tokens, and audit records

## Details that are not final

Command names express the intended workflow, but flags and configuration keys may change. The public Dopbase Cloud hostname, installation packages, authentication flow, API paths, encryption implementation, and release compatibility policy are not final.

Pages avoid inventing these details. Where a value has not been decided, the documentation uses a placeholder or says that the behavior is planned.

## Before using Dopbase

Do not treat a pre-release build as production-ready secret storage until the project publishes a supported release, threat model, security review, backup procedure, and upgrade policy.

Follow the [public roadmap](/about/roadmap) for the planned sequence of work.
