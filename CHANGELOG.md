# Changelog

All notable changes to Dopbase are documented in this file.

## Unreleased

## 0.0.13 - 2026-09-04

### Added

- Encrypted, server-scoped CLI sessions with cached identity metadata and safe
  offline status reporting.
- Server-scoped default environments for `dopbase run`, with explicit argument
  and `DOPBASE_ENV` precedence.
- Single-instance server locking and branded foreground and background startup
  banners.

### Changed

- Made server switching validate and confirm the destination, coordinate with
  matching local servers, and clear obsolete sessions and environment defaults.
- Required interactive password confirmation before CLI secret reveal and export
  operations return plaintext values.
- Refreshed Dopbase branding, CLI documentation, and frontend/backend build
  commands.

## 0.0.12 - 2026-09-01

### Added

- Revision-protected secret import previews that reject stale replace operations.
- Windows x64 release archives and a checksum-verifying PowerShell installer.
- Session-history retention with automatic cleanup after 30 days.

### Changed

- Serialized concurrent secret writes and token revocations so versions and audit
  records remain consistent.
- Made project, environment, authentication, and audit mutations transactional.
- Moved password hashing and verification to bounded blocking workers.
- Hardened daemon PID ownership, private-file writes, and rate-limiter storage.
- Cancelled stale Admin UI requests and queued concurrent reauthentication work.
- Expanded release verification across Linux, macOS, and Windows artifacts.

## 0.0.8 - 2026-08-30

### Added

- Configurable server host and port through CLI flags, `server.toml`, and
  environment variables, including automatic public URL derivation for
  loopback addresses.
- Background server operation with PID and log files, graceful shutdown via
  `dopbase stop`, stale PID cleanup, and startup failure reporting.
- Informational `dopbase update` checks for newer GitHub releases.
- Configuration and CLI controls for enabling the Swagger UI and OpenAPI
  document when required.
- A dedicated Admin UI page for reviewing imported secrets.

### Changed

- The default server port is now **8840** instead of 8376.
- Swagger UI and OpenAPI documents are disabled by default.
- Expanded generated endpoint documentation and moved HTTP error coverage into
  integration tests.
- Refined workspace and dialog presentation throughout the Admin UI.
- Streamlined backend dependencies and adopted shared Rust formatting rules.

## 0.0.7 - 2026-08-29

### Added

- Public documentation for CLI commands, REST API endpoints, security,
  encryption keys, backups, audit events, and troubleshooting.
- Operational guides for installation, configuration, project workflows, and
  self-hosting.
- Initial release changelog and project branding.

### Changed

- Updated the roadmap, open-source status, development guidance, and usage
  documentation to reflect the implemented v0.1 command and API surface.

## 0.0.6 - 2026-08-29

### Added

- Complete Admin UI routing and boot flow with authentication and dashboard
  layouts.
- Instance setup, administrator login, account and session management, audit
  events, and instance status screens.
- Project and environment navigation with secret and runner token management.
- Dotenv parsing, syntax highlighting, import/export workflows, and environment
  file editing.
- Typed frontend API clients, session state, reauthentication flows, shared UI
  components, and application icons.
- Frontend tests covering authentication, workspace workflows, secret
  management, dotenv handling, formatting, and HTTP behavior.

### Changed

- Replaced the starter interface with the Dopbase design theme and application
  shell.

## 0.0.5 - 2026-08-29

### Added

- CLI commands for serving Dopbase and managing authentication, projects,
  environments, secrets, and runner tokens.
- A CLI API client, local client configuration, and dotenv parsing and
  serialization.
- End-to-end backend workflow and application verification tests.
- The composed Dopbase binary and supporting database service integration.

## 0.0.4 - 2026-08-29

### Added

- Versioned REST API modules for health checks, authentication, instance
  bootstrap, projects, environments, secrets, runner tokens, audit events, and
  instance status.
- Contracts, validation errors, persistence, business logic, HTTP handlers,
  OpenAPI schemas, and route assembly for each API module.

## 0.0.3 - 2026-08-29

### Added

- Layered server configuration with shared constants, resource limits, error
  codes, and token prefixes.
- Authentication extractors, secure token utilities, request rate limiting,
  envelope encryption, and shared audit recording.
- Stable HTTP response envelopes and error responses.
- Shared authentication, resource, and secret models.
- Application state, server lifecycle management, router composition, and an
  embedded fallback UI.

## 0.0.2 - 2026-08-29

### Added

- Cross-platform release automation and a release installer for Linux and
  macOS on x64 and ARM64.
- Installer tests, a backend test container, and a combined development runner.
- Initial database migrations for instance metadata, administrators, sessions,
  projects, environments, secrets, runner tokens, audit events, and environment
  layouts.
- Migration operation documentation and the v0.1 implementation roadmap.

## 0.0.1 - 2026-08-28

### Added

- Initial Rust backend and Vue Admin UI project scaffold.
- Project README, contribution guidelines, license, security policy, and legal
  notices.
- Initial public documentation site, product boundaries, roadmap, CLI and API
  plans, self-hosting guidance, and reference documentation.
