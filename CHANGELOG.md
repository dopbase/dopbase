# Changelog

All notable changes to Dopbase are documented in this file.

## Unreleased

Dopbase now supports more self-hosted connection formats, creates a recovery
archive before host-side factory resets, and uses clearer copy across the CLI,
Admin UI, API documentation, and public guides.

### Added

- `dopbase client connect` accepts explicit HTTP URLs and bare IPv4 or IPv6
  addresses with optional ports. Bare IP addresses use HTTP by default.
- `dopbase admin factory-reset` saves the complete data directory as a ZIP
  before removing it. Pass `--no-backup` to reset without creating the archive.

### Improvement

- CLI errors, prompts, and help text provide clearer recovery steps and use
  consistent terms.
- Frontend copy, OpenAPI descriptions, source comments, and public
  documentation use shorter, more direct language.

### Security

- The CLI warns before connecting over HTTP because the transport does not
  protect credentials or secrets. HTTPS remains the recommended protocol.

## 0.1.3 - 2026-09-09

Dopbase 0.1.3 improves CLI output and credential handling, keeps API routes in
one shared catalog, expands the AI agent documentation, and trims release
dependencies.

### Improvement

- CLI list and detail commands now use readable tables, labeled fields, and
  concise result messages by default. `--json` keeps the existing
  machine-readable response shapes.
- Password and secret prompts show masked input. Interactive
  `dopbase secret set --stdin` explains how to finish entering a value, while
  piped input remains unchanged.
- `dopbase login --token` can save a runner token in the encrypted local
  credential store. `dopbase run --token <TOKEN>` provides a one-off override.
- Backend route paths now come from one shared catalog used by the server,
  OpenAPI annotations, and CLI requests.
- OpenAPI labels and metadata are clearer, and wide documentation tables can be
  expanded for easier reading.
- Added guides for AI agent access and supported environment variables.
- Reduced application dependencies and tightened release build settings.

### Security

- Runtime authentication now resolves `--token`, then `DOPBASE_TOKEN`, then
  the saved credential. Command-line tokens remain intended for one-off use
  because process inspection and shell history may expose them.

## 0.1.2 - 2026-09-09

Dopbase 0.1.2 adds an offline, root-authorized factory reset, switches new
environments to short random IDs, removes native Windows packaging, and
standardizes pull request and release notes.

### Improvement

- Added `dopbase admin factory-reset` for root-authorized, offline instance
  resets from the server host.
- New environments receive random six-digit IDs such as `env_482731`. Existing
  IDs continue to work.
- The Admin UI shows the selected environment ID with a copy button and focuses
  the name field when a project or environment dialog opens.
- `dopbase server logs --watch` and `-w` replace `--follow` and `-f`. The old
  flags are no longer accepted.
- Pull request descriptions now follow a checked repository template. Release
  notes use the matching changelog section and link to the complete comparison
  between versions.

### Note

- Removed native Windows release archives and the PowerShell installer. Windows
  users must run Dopbase in a Linux container with Docker.

## 0.1.1 - 2026-09-08

### Added

- Added `dopbase server logs` for reading, following, and clearing background
  server output.
- Added command-specific help and examples when required CLI arguments or
  subcommands are missing.
- Published a CLI cheat sheet, architecture documentation, and monthly public
  security summaries.

### Changed

- Moved local server commands under `dopbase server`: `start` runs in the
  foreground, while `up`, `down`, `status`, and `logs` manage a background
  server. The old `serve`, `serve --background`, and `stop` commands are no
  longer supported.
- Fixed the SQLite database location at `<data-dir>/dopbase.db` and replaced
  `--bind-address` and `--database-url` with separate host and port settings.
  The matching legacy configuration keys and environment variables are no
  longer supported.
- Updated the command reference and examples for the new server commands and
  made wide documentation tables scroll on smaller screens.

### Fixed

- Prevented server status checks from creating missing database or lock files.

### Security

- Required HTTPS for remote client and public server URLs while keeping HTTP
  available for loopback addresses. Client requests no longer follow redirects.
- Rate-limited password re-verification, restricted post-login redirects to
  same-origin paths, and removed deleted secret data from SQLite files after
  restores and factory resets.
- Added content security, frame, content-type, referrer, and API cache-control
  response headers.

## 0.1.0 - 2026-09-06

### Added

- Static root/admin/member permissions, browser-based user administration,
  metadata-only AI service accounts with expiring bearer tokens, and the
  authenticated `/api/v1/status` summary.
- Console user-role selection, AI account creation/deletion, instance counts,
  and root-only Danger Zone visibility.
- User guides for account management, role permissions, instance status,
  and factory-reset confirmations.

### Changed

- Replaced the boxed server startup banner with plain startup lines and
  updated documentation examples to match.

## 0.0.15 - 2026-09-06

### Added

- PocketBase-inspired component system for the Admin UI: solid borderless
  buttons, filled fields with inline labels, borderless badges, and new
  toggle and checkbox primitives, all carrying the Dopbase purple accent.
- The "Edit as .env" view is now a full editor shell with a filename tab and
  unsaved-changes dot, active-line gutter highlight, a status bar with cursor
  position and problem counts, a problems strip, and Cmd/Ctrl+S to save.

### Fixed

- Equalized `DbInput` and `DbSelect` field heights.
- Selecting a project now reliably opens its first environment instead of
  getting stuck on a stale environment left over from the previous project.

## 0.0.14 - 2026-09-05

### Added

- Encrypted backup creation, upload, download, deletion, and restoration through
  the Admin UI, REST API, and CLI.
- First-run disaster recovery from a backup archive using the one-time setup
  token and the source server's master key.
- An encrypted, server-scoped CLI run cache for explicitly requested offline
  secret injection.

### Changed

- Rekey cross-server backup imports to the target server's master key while
  preserving secret values and the active administrator session.
- Expanded backup, restore, storage, audit, and CLI documentation, including a
  dedicated disaster-recovery guide.
- Simplified frontend and binary build command names and refreshed the repository
  banner.

### Security

- Validate backup authentication, manifests, SQLite integrity, schema migrations,
  archive sizes, and setup authorization before modifying live data.
- Apply restores through disposable database connections so cancellation cannot
  return unsafe connection state to the application pool.

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
