# Admin UI v0.1 implementation plan

This document breaks the first complete Dopbase administration experience into
ordered implementation phases. Complete and verify each phase before starting
the next one.

The target is one self-hosted executable with restart-only server
configuration, one human administrator, project and environment management,
encrypted secrets, runner tokens, audit records, and an embedded Vue Admin UI.

::: warning Pre-release plan
This is an implementation checklist, not a claim that the described behavior
is available. Update the public reference documentation when implementation
findings require an interface change.
:::

## Fixed decisions

- Server process configuration loads once at startup. Changes require a full
  graceful stop and fresh start; there is no file watcher, SIGHUP reload, or UI
  reload button.
- Client configuration remains separate and is read by each CLI invocation.
- SQLite remains the self-hosted database.
- The Admin UI and REST API are served from the same origin by the Dopbase
  executable.
- v0.1 supports exactly one human admin plus environment-scoped runner tokens.
- A first admin must prove access with a one-time setup token printed when an
  uninitialized server starts.
- Password recovery is an offline CLI operation that verifies the configured
  master key. The master key is never entered in the browser or sent over HTTP.
- If the master key is lost, encrypted secrets and offline password recovery
  are unavailable.
- Projects are the Admin UI landing workspace. v0.1 has no generic metrics
  dashboard, organizations, invitations, or additional human users.

## Phase 1: server configuration and lifecycle

### Goal

Give `serve` and offline operator commands one configuration interface and a
predictable restart lifecycle.

### Implement

- [ ] Define `~/.dopbase/server.toml`, with `--config <path>` available to both
      `dopbase serve` and offline admin commands.
- [ ] Add the initial schema:

```toml
version = 1
bind_address = "127.0.0.1:8376"
database_url = "sqlite://./dopbase.db"
shutdown_grace_seconds = 10

[master_key]
provider = "file"
path = "~/.dopbase/master.key"
```

- [ ] Resolve settings in this order: CLI argument, environment variable,
      server TOML, local default.
- [ ] Parse and validate configuration once in the composition root, then pass
      an immutable configuration value through application state.
- [ ] Reject unsupported versions, invalid bind addresses, non-SQLite database
      URLs, invalid grace periods, and incomplete key-provider settings before
      starting HTTP.
- [ ] Handle SIGINT and SIGTERM by stopping new requests, draining active
      requests for at most `shutdown_grace_seconds`, closing SQLite, and
      exiting.
- [ ] Do not implement partial reload. Database-backed users, projects,
      environments, secrets, tokens, and audits remain live because they are
      application data rather than process configuration.

### Verify before continuing

- [ ] Unit tests cover every configuration source and precedence combination.
- [ ] Invalid configuration fails without opening the database or network
      listener.
- [ ] A shutdown integration test proves that requests drain and SQLite closes.
- [ ] Restarting proves the updated configuration is loaded from scratch.

## Phase 2: database and master-key foundation

### Goal

Create the first durable schema and establish the key invariant required by
secrets, bootstrap, and offline recovery.

### Implement

- [ ] Add the initial SQLite migration for:
  - Instance metadata and encrypted master-key verification record
  - Single admin user
  - Browser and CLI sessions
  - Projects and environments
  - Current encrypted secret values
  - Environment runner tokens
  - Audit events
- [ ] Enable SQLite foreign keys, WAL mode, busy timeout, and transactional
      migrations before serving requests.
- [ ] Enforce unique project names, environment names within a project, and
      secret keys within an environment.
- [ ] Generate prefixed immutable public IDs for projects, environments,
      tokens, sessions, and audit records.
- [ ] For a brand-new database only, atomically generate a 256-bit master key at
      the configured file path with owner-only permissions.
- [ ] If an existing database has a missing, unreadable, or incorrect key, fail
      startup before binding HTTP. Never generate a replacement key for an
      existing database.
- [ ] Encrypt a fixed verification payload and store it in instance metadata so
      startup and recovery can verify the configured key.
- [ ] Implement envelope encryption using ChaCha20-Poly1305: a random data key
      per stored value, a separately wrapped data key, unique nonces, and
      authenticated environment/key/version metadata.
- [ ] Keep only the current secret value in v0.1. Increment its version counter
      on update; history and rollback remain v0.2.

### Verify before continuing

- [ ] Tests use isolated temporary SQLite databases and key files only.
- [ ] Key generation succeeds for a new database and never overwrites an
      existing key.
- [ ] Existing-database tests fail safely for missing, wrong, or malformed keys.
- [ ] Encryption round-trip and tamper tests cover ciphertext, wrapped keys,
      nonces, and authenticated metadata.
- [ ] Logs, errors, and debug formatting contain no key material or plaintext.

## Phase 3: bootstrap, authentication, and recovery

### Goal

Securely claim an empty instance, authenticate the single admin, and provide an
operator-only recovery path.

### Server interfaces

```text
GET  /api/v1/health
GET  /api/v1/bootstrap/status
POST /api/v1/bootstrap/admin
POST /api/v1/auth/login
POST /api/v1/auth/logout
GET  /api/v1/auth/session
POST /api/v1/auth/reauthenticate
POST /api/v1/auth/change-password
```

### Implement

- [ ] Organize bootstrap and auth as vertical-slice Rust modules: route table,
      thin controller, business service, private SQLite repository, typed
      models, typed errors, and OpenAPI registration.
- [ ] Return successful JSON through the shared response envelope and map
      validation, conflict, authentication, authorization, and internal errors
      to stable safe responses.
- [ ] When no admin exists, generate a cryptographically random setup token in
      memory. Print it once in dedicated startup output and exclude it from
      structured request logs.
- [ ] Keep the token valid until setup succeeds or the process restarts. Compare
      it in constant time and rate-limit failed claims.
- [ ] Make bootstrap status public but reveal only `setupRequired` or `ready`.
- [ ] Create the first admin and close bootstrap in one transaction so two
      concurrent claims cannot create multiple admins.
- [ ] Normalize the email to lowercase and require a valid email plus a
      12–128-character password.
- [ ] Hash passwords with Argon2id using a documented OWASP baseline and store
      the PHC-formatted hash.
- [ ] Automatically create a browser session after successful bootstrap.
- [ ] Store opaque session-token hashes in SQLite. Browser sessions use an
      HttpOnly, SameSite Strict cookie, Secure outside localhost, with an
      eight-hour idle and 24-hour absolute expiry.
- [ ] Require a server-issued CSRF header for cookie-authenticated mutations.
      Bearer-authenticated CLI requests do not use CSRF.
- [ ] Let CLI login receive an opaque bearer session token for OS credential
      storage without changing the browser cookie behavior.
- [ ] Rate-limit login by normalized email and client address and return the same
      error for unknown email and wrong password.
- [ ] Track recent password authentication in the session for ten minutes.
- [ ] Implement `dopbase admin reset-password <email> [--config <path>]`.
      Require the server to be stopped, acquire exclusive database access,
      verify the master key, securely prompt twice, update the password hash,
      revoke all human sessions, preserve runner tokens, and record
      `admin.password_reset`.
- [ ] Provide no HTTP forgot-password or reset endpoint.

### Verify before continuing

- [ ] Bootstrap token tests cover invalid values, rate limiting, restart
      rotation, successful invalidation, and concurrent claims.
- [ ] Login tests cover generic failures, rate limiting, cookie attributes,
      bearer sessions, CSRF, idle/absolute expiry, logout, and password change.
- [ ] Recovery tests cover a running server, wrong key, missing key, unknown
      email, successful reset, session revocation, runner-token preservation,
      and audit creation.
- [ ] OpenAPI includes every route and realistic response status.

## Phase 4: project, environment, secret, token, and audit interfaces

### Goal

Complete the authenticated server behavior needed by the v0.1 Admin UI and CLI.

### Implement

- [ ] Add vertical-slice modules and OpenAPI definitions for projects,
      environments, secrets, tokens, audit, and safe instance status.
- [ ] Implement project create/list/show/rename/delete. Deletion reports affected
      counts and removes environments, secrets, and scoped tokens in one
      transaction.
- [ ] Implement environment create/list/show/rename/delete with equivalent
      transactional behavior.
- [ ] Implement secret metadata listing, secure set/update, deletion, reveal,
      merge import, replace import, and export.
- [ ] Validate an entire import before mutation. Default import merges; replace
      explicitly reports keys that will be removed.
- [ ] Require password reauthentication within the previous ten minutes for
      reveal and export.
- [ ] Implement runner-token create/list/revoke. Scope each token to one
      environment and permit value retrieval for `dopbase run` without mutate,
      export, or cross-environment access.
- [ ] Display each new runner token exactly once and persist only its hash.
- [ ] Implement paginated audit listing with filters for time, action, project,
      environment, actor, and safe resource identifiers.
- [ ] Record bootstrap, login, password, project, environment, secret, reveal,
      export, token, and recovery events. Never record request bodies, secret
      values, usable tokens, or password material.
- [ ] Add authenticated instance status containing version, public endpoint,
      initialization state, database health, safe key-availability status, and
      `configurationReload: "restartRequired"`. Do not expose filesystem paths
      or key sources.

### Verify before continuing

- [ ] CRUD tests cover validation, uniqueness, authorization, transaction
      rollback, affected counts, and cascades.
- [ ] Import tests cover merge, replace, dry-run calculations, duplicate keys,
      invalid files, empty values, and all-or-nothing behavior.
- [ ] Reveal/export tests cover recent-password expiry and safe auditing.
- [ ] Runner-token tests prove exact environment scope and forbidden mutations.
- [ ] A redaction test scans HTTP errors, logs, and audit payloads for fixture
      values and tokens.

## Phase 5: Vue foundation and design system

### Goal

Replace the starter interface with the routing, data flow, localization, and
visual primitives required by every Admin UI screen.

### Visual direction

- Obsidian `#0D0B14`
- Paper `#F8F7FB`
- Dopbase Violet `#863BFF`
- Signal Cyan `#47BFFF`
- Lilac `#EDE6FF`
- Danger `#D6455D`
- IBM Plex Sans for interface copy and Fira Code for IDs, keys, timestamps, and
  status text

The signature element is a narrow violet-to-cyan keyline through the project
and environment rail. Its nodes show the selected environment. Keep the main
workspace quiet, dense, and operational rather than using generic dashboard
cards.

### Implement

- [ ] Replace the starter component with a router view and app boot state.
- [ ] Use hash history and global guards that resolve bootstrap and session
      state before rendering protected routes.
- [ ] Add Pinia stores only for shared instance, session, project-navigation,
      and toast state. Keep HTTP transport in stateless services.
- [ ] Follow one-screen-per-folder architecture: typed interface, controller,
      thin page template, colocated tests, and private components only when one
      screen needs them.
- [ ] Create accessible shared Text, Button, Input, Dialog, Toast, Table, Badge,
      and navigation primitives after checking that no reusable implementation
      already exists.
- [ ] Add component stories for every new shared primitive and demonstrate
      normal, loading, disabled, invalid, destructive, keyboard-focus, and
      responsive states.
- [ ] Route all user-facing text through typed i18n keys in both English and
      Chinese.
- [ ] Correct the stale locale storage key, supported locale list, comments,
      and fallback behavior in the existing i18n module.
- [ ] Implement responsive behavior: fixed rail and table workspace on desktop;
      compact instance header, navigation drawer, and readable stacked rows on
      mobile.
- [ ] Respect reduced motion, visible focus, semantic landmarks, screen-reader
      labels, and WCAG AA contrast.

### Verify before continuing

- [ ] Router tests cover boot, setup-required, unauthenticated, authenticated,
      expired-session, and failed-status states without protected-content flash.
- [ ] Store and transport tests cover successful and failed envelopes without
      leaking internal response details into UI copy.
- [ ] Component stories and keyboard tests cover every shared primitive.
- [ ] English and Chinese keys have compile-time and test parity.

## Phase 6: setup and login experience

### Goal

Implement the complete first-run and returning-admin flows.

### Layout

Use a split screen on desktop: a dark instance-seal panel on the left and the
form on the right. The seal shows only the endpoint, server reachability, and
safe master-key availability. On mobile it becomes a compact header above the
form.

### Implement

- [ ] `/setup` contains setup token, email, password, and password confirmation
      in one form with visually distinct instance-claim and admin-account
      sections.
- [ ] Submitting setup disables duplicate submission, maps validation errors to
      their fields, starts the returned session, and routes to the empty
      projects workspace.
- [ ] `/login` contains email and password with generic authentication errors.
- [ ] Add a “Lost access?” disclosure explaining that the operator must stop
      the server and run `dopbase admin reset-password`. Do not render a reset
      form or request the master key.
- [ ] Add account password change with current password confirmation and session
      rotation.
- [ ] Add logout from the account area and workspace rail.

### Verify before continuing

- [ ] Controller tests cover validation, pending state, server errors, session
      establishment, routing, and duplicate-submit prevention.
- [ ] Accessibility tests cover form labels, descriptions, errors, focus
      movement, password-manager compatibility, and keyboard submission.
- [ ] Setup never renders after bootstrap is closed, including direct-route
      navigation and stale browser tabs.

## Phase 7: projects and environment workspace

### Goal

Make the main Admin UI useful without introducing a separate dashboard.

### Layout

```text
┌ instance + project rail ┬ project / environment workspace ┐
│ Projects                │ Project A / production           │
│ ├ development           │ KEY             UPDATED  ACTION  │
│ ├ staging               │ DATABASE_URL    2h       Reveal  │
│ └ production            │ API_KEY         1d       Reveal  │
│                         │                                  │
│ Audit / Instance        │ Import / Add secret / Export     │
│ admin@email / Logout    │                                  │
└─────────────────────────┴──────────────────────────────────┘
```

### Implement

- [ ] The authenticated landing route opens the projects workspace.
- [ ] Empty state explains the project/environment model and offers “Create
      project” and “Import `.env`” actions.
- [ ] Build project and environment create, rename, and delete interactions.
      Destructive dialogs require typing the resource name and display affected
      counts.
- [ ] Render the project/environment hierarchy in the keyline rail and preserve
      the selected environment in the URL, never global machine configuration.
- [ ] List secret key metadata without fetching values.
- [ ] Add secure prompt-based create/update, delete, and recent-password reveal.
- [ ] Keep revealed plaintext only in component memory, auto-hide after 30
      seconds, and clear it on route change. Copying does not extend the timer.
- [ ] Add `.env` import with local file parsing, key/count preview, merge by
      default, explicit replace mode, and no value rendering.
- [ ] Add export with recent-password confirmation and an explicit browser
      download.

### Verify before continuing

- [ ] Controller tests cover URL selection, empty states, CRUD refresh, failed
      mutations, destructive confirmations, and stale-resource handling.
- [ ] Reveal tests use fake timers and prove clearing on timeout and navigation.
- [ ] Import tests prove previews and errors never render secret values.
- [ ] Browser storage contains no secret values, setup tokens, runner tokens, or
      master-key material.

## Phase 8: runner tokens, audit, and instance status

### Goal

Complete the remaining operational screens without broadening v0.1 into team
management.

### Implement

- [ ] Add contextual runner-token management under each environment.
- [ ] Display a created token in a single-use dialog with copy and explicit
      acknowledgment; closing it makes the plaintext unavailable.
- [ ] Add token metadata listing and revocation with confirmation.
- [ ] Add an audit screen with pagination and filters. Use readable project and
      environment context while retaining immutable IDs in detail views.
- [ ] Add a read-only instance screen for version, endpoint, database health,
      key availability, and restart-only configuration status.
- [ ] Link the instance screen to server configuration and backup documentation.
      Do not allow editing startup settings from the browser.
- [ ] Keep account management limited to the single admin's email display,
      password change, session status, and logout.

### Verify before continuing

- [ ] Token plaintext cannot be recovered after its creation dialog closes.
- [ ] Audit filters, pagination, loading, and empty states work on desktop and
      mobile.
- [ ] The instance screen exposes no database path, key path, private
      configuration, or credential.
- [ ] No members, invitations, organizations, or role-management routes are
      reachable in v0.1.

## Phase 9: integration, hardening, and documentation

### Goal

Verify the system as one executable and make its operational contract clear.

### Implement and verify

- [ ] Embed the production Vue assets into the Rust executable with SPA fallback
      that does not intercept `/api/v1` routes.
- [ ] Exercise fresh start → setup → project → environment → secret → reveal →
      runner token → `dopbase run` → audit as an end-to-end flow.
- [ ] Exercise graceful stop, configuration change, restart, login, and data
      continuity.
- [ ] Exercise forgotten-password recovery with correct and incorrect master
      keys and verify every prior human session is rejected afterward.
- [ ] Run Rust formatting, check, Clippy, unit tests, SQLite integration tests,
      migration tests, and OpenAPI contract tests.
- [ ] Run frontend Vitest, TypeScript, ESLint, component stories, accessibility
      checks, responsive browser checks, and the production build.
- [ ] Run documentation formatting, link checking, stale-interface searches,
      and the VitePress production build.
- [ ] Update serve, identity, security, encryption-key, operations, API, audit,
      troubleshooting, and product-status documentation with the implemented
      behavior and recovery warnings.
- [ ] Confirm logs, screenshots, test failures, and support examples contain no
      real token, setup credential, key material, or secret value.

## Completion criteria

The v0.1 Admin UI is complete when a new operator can start one executable,
claim it with the terminal setup token, create the single admin, manage projects
and environments, import and edit encrypted secrets, create a scoped runner
token, run an application against the selected environment, inspect the audit
trail, restart after configuration changes, and recover a forgotten admin
password offline using the same protected master key.
