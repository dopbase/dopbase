# Changelog

All notable changes to Dopbase are documented in this file.

## Unreleased

### Added

- Access per project and environment. Accounts now point to a row in the new
  `roles` table instead of a fixed role name. A role holds capabilities, such
  as `users.manage` or `audit.read`, and a ceiling on the access its grants
  can give. Admin, Member, Auditor, and AI agent are built in. Root is a flag
  on the account and holds every capability. Grants give a user or a group one
  level (`metadata`, `read`, `write`, `manage`, or `project_manager`) on all
  projects, one project, or one environment. A grant on an environment
  overrides one on its project, and `none` on an environment cuts it out of a
  project grant. `GET /api/v1/auth/session` reports `roleName`,
  `capabilities`, and `maxLevel`, and backups include the new tables.
- A project's **Access** page, opened from the shield next to it, lists who
  reaches the project and lets its project managers give or remove access
  for users and groups. The `PUT /api/v1/grants`,
  `DELETE /api/v1/grants/{id}`, `GET /api/v1/projects/{ref}/access`, and
  `GET /api/v1/users/{id}/access` endpoints back it, and the Users page shows
  where each person's access comes from.
- Groups. The new **Groups** page creates groups and adds or removes members,
  and a grant to a group reaches everyone in it. Leaving a group takes its
  access away on the next request.
- Personal tokens (`dpp_`). A token acts as its account from a script or an
  AI agent on another machine. It reaches at most what the account reaches
  right now, within its own project or environment and level (`metadata`,
  `read`, or `write`), and always expires within the role's limit. Create
  them on the Account page, with `dopbase token personal create`, or through
  `POST /api/v1/users/me/tokens`, and use them as `DOPBASE_TOKEN`.
- Custom roles. Root creates them on the new **Roles** page or with
  `POST /api/v1/roles`: pick capabilities, how deep into project data holders
  reach, and what their personal tokens may do. A Viewer that reads values
  but never changes them, or a token-only role for CI bots, needs no code.
  Built-in roles never change. People are given a role by its id
  (`roleId` on `POST` and `PATCH /api/v1/users`), and only roles within your
  own can be given. User responses add `roleId` and `roleName`.
- Audit tokens. The Auditor role's personal tokens read the audit log and no
  project data, for a SIEM or another log collector. Create one on the
  Account page or with `dopbase token personal create --audit`.
- `dopbase access` shows what you, or the token in `DOPBASE_TOKEN`, can reach
  and where it comes from. `dopbase access --project <ref>` lists who reaches
  a project.
- Protected environments. Turn protection on from the Access page or with
  `PATCH /api/v1/environments/{id}` and `{"protected": true}`, and a project
  grant below project manager stops reaching the environment. Only a grant
  on the environment itself or a project manager gets in.

### Changed

- `dopbase client status` shows the account's role, such as Member, Auditor,
  or Root, on a new **Role** line in place of **Identity**. It used to say
  `admin` for every saved login, whatever the role. The role comes from the
  server, so it reads `unknown (server offline)` when the server can't be
  reached. **Authentication** now says how the CLI signed in, for example
  `dopbase login (saved session)` or `personal token (DOPBASE_TOKEN)`. The
  JSON output adds `role` and keeps `authentication` and `identity` as they
  were.
- Service accounts are now accounts with the AI agent role. The upgrade turns
  each one into an AI agent owned by root that still reads secret names on
  every project, and its `dpa_` tokens keep working as personal tokens. New
  agents are added on the Users page with a name and no password, start with
  no access, and never reach more than the person who owns them. An owner
  can't be deleted while they own agents; give the agents another owner
  first. The `/api/v1/service-accounts` endpoints are gone: use
  `POST /api/v1/users` with `"role": "ai_agent"` and
  `/api/v1/users/{id}/tokens`.
- A personal token that reaches values can reveal, export, and run without a
  password prompt, and a `write` token can change secrets. A `metadata` token
  is refused with `Your access to this environment does not allow this.`

- Lists of projects and environments show only what your grants reach, and
  anything else answers 404. Reading secret names needs `metadata`, values
  need `read`, changes need `write`, and runner tokens, renaming, or deleting
  an environment need `manage`. Renaming or deleting a project and adding
  environments need `project_manager`. The Admin UI hides what your level
  does not allow.
- Creating a project needs the `projects.create` capability, which admins
  have and members do not. The creator becomes the project's project manager.
- New members start with no access until someone shares a project with them.
  Admins that root creates start as project managers of every project, and
  admins that other admins create start with none. Existing accounts keep
  what they had: the upgrade gives every admin `project_manager` and every
  member `write` on all projects.
- Nobody can change, delete, or unlink an account whose role or access goes
  beyond their own, give a role they do not hold, or change their own
  grants.
- The audit log shows each reader what their role allows. Roles with the
  new `audit.read_all` capability, Auditor and root, read every event.
  `audit.read` alone, which admins have, shows settings events and events on
  the projects the reader reaches. Everyone else sees their own actions, so
  the Audit page is open to all people and members no longer get a 403.
- The built-in Auditor role gains `audit.read_all` and `tokens.personal`.
- Factory reset deletes custom roles.
- Signing in with a password refuses disabled accounts and accounts that use
  tokens only, instead of issuing a session the server then rejects. Keeping
  password sign-in while single sign-on is required now follows the
  `password.breakglass` capability, and verifying a provider follows
  `sso.manage`; both are root's alone.
- `dopbase client status` reports a `dpp_` token as `personal_token`.
- The documentation covers roles, grants, groups, protected environments,
  personal tokens, AI agent accounts, and audit visibility, with new Project
  access, Groups, and Roles pages. "Identity and tokens" is now "Identity and
  access".
- User management, the audit log, instance status, backups, single sign-on
  settings, and factory reset now check a capability instead of a role. A
  refused request says `Your role does not allow this operation.` where it
  said `This operation requires an administrator.`
- `PATCH /api/v1/environments/{id}` answers `ENVIRONMENT_UPDATED` instead of
  `ENVIRONMENT_RENAMED`, and its `name` is optional.
- The **Projects** and **Audit** pages no longer read the session store
  themselves. Their controllers now decide who can create projects, manage a
  project or environment, and how much of the audit log the viewer sees. The
  frontend architecture guide spells out the rule: `ui` components hold no
  business logic, every screen splits into a page and a controller, and a
  component one page uses lives in that page's folder. Nothing changes for
  people using the console.
- New Dopbase logo: two stacked Ds in lavender, replacing the D in a ring. The
  console shows it on its own in the sidebar, the sign-in and setup pages, and
  the not-found page. The favicon is now the violet app icon, the docs site
  switches between the lavender and violet mark with its theme, the README
  shows the full logo with the wordmark, and the social preview image uses the
  new banner. The icon set now matches the Figma file: it adds icons for
  agents, filters, activity, menus, external links and the command key, and
  drops 33 icons nothing used.
- The console sidebar now groups its pages into **Workspace** (Projects, Audit
  log), **People & access** (Users, Groups, Roles) and **Instance** (Status,
  Backups, Single sign-on), and hides a group when your role can open none of
  its pages. Your email and real role name sit at the bottom, replacing the
  "admin console" and "member console" label, and open a menu with Account,
  Personal tokens, Docs and Sign out. Instance is now **Status** at `/status`;
  `/instance` redirects there. A link to a page your role can't open now
  shows a **No access** screen that names the missing capability, instead of
  sending you to Projects. The sidebar and the router read the same
  capability map, so the nav never offers a page the router would refuse.

### Fixed

- Release builds on macOS 27 no longer fail with `mis-aligned LINKEDIT string
  pool` while loading `sqlx-macros`. The release profile now strips only the
  final binary and leaves proc-macro libraries unstripped.

## 0.1.10 - 2026-10-26

[Dopbase](https://dopbase.com) 0.1.10 adds single sign-on for the Admin UI
through OpenID Connect, browser approval for `dopbase login`, and secret
version history with rollback. The audit log becomes a hash chain that
`dopbase audit verify` checks, and it can be listed, exported, and pruned on a
schedule. The Admin UI works on phones. Migrations now use timestamp versions,
so a database cannot go back to 0.1.9, and the invalid email error code is
now `EMAIL_INVALID`.

### Added

- Accounts can go without a password once single sign-on is on. Administrators
  leave the password empty on the Users page, which marks such accounts, and
  the person signs in through the provider and can set a password later on the
  Account page. Root can turn on **Require single sign-on**, which refuses
  password sign-in for everyone except root, so root stays the way back in.
  Turning single sign-on off lifts it, and the settings page warns how many
  accounts have no password before you turn it off or remove it. Sensitive
  actions can be confirmed at the provider in a popup that asks for a fresh
  sign-in (`max_age=0`) and checks `auth_time`; in the CLI, an account that
  cannot use a password, because it has none or single sign-on is required,
  approves a code in the Admin UI instead of typing one, and only the owner of
  that terminal session can approve it. Administrators can unlink an account's
  identity, and people with a password can unlink their own. Root can also
  hide the password form so the sign-in page shows only the single sign-on
  button; passwords keep working in the CLI and the API, and root reaches the
  form at `/login?password`. In the API, `POST /api/v1/users` accepts no `password` while single sign-on
  is on, users report `hasPassword` and `ssoLinked`, and
  `GET /api/v1/auth/session` reports `hasPassword`, `passwordConfirmation`,
  and `ssoConfirmation`. New audit events: `sso_identity.unlinked` and
  `sso_provider.policy_changed`. Issue #121.

- Single sign-on for the Admin UI through one OpenID Connect provider, with
  presets for Google, Microsoft Entra ID, Okta, Auth0, Amazon Cognito, Clerk,
  Supabase, and Keycloak, plus an option for any other OpenID Connect
  provider, such as Authentik or Zitadel. The presets live in one file,
  `app/src/modules/sso/presets.toml`, so adding a provider takes one new entry
  and no database or Admin UI change; `CONTRIBUTING.md` lists the fields.
  `GET /api/v1/sso/provider` returns the list as `presets`, and saving an
  unknown preset fails with `SSO_PRESET_INVALID`. It needs an HTTPS public
  URL (`localhost` excepted): the settings page stays locked and says how to set one until then, and the
  server refuses setup and sign-ins without it. Issuers entered with or
  without a trailing slash are stored the way the provider publishes them.
  Root configures it on the new Single sign-on page, registers the redirect URI it shows, and
  must complete a test sign-in, which links the root account, before turning it
  on. The sign-in page then shows a **Sign in with …** button. A first sign-in
  links to an existing admin or member account only when the provider marks
  the email as verified; later sign-ins match the provider's subject
  identifier, so an email change at the provider cannot move the identity. Root
  is never linked by email, unknown emails are refused, and people can link
  from the Account page by hand. Saving a different issuer, for example when
  moving from Google to Keycloak, removes the old links and turns single
  sign-on off until the next test sign-in; people then link again by verified
  email. Entra accounts link by email only with the
  `xms_edov` claim. Sign-ins use PKCE, a nonce, and a `state` bound to the
  starting browser, and finish through a one-time handoff code, so the
  session cookie and CSRF token are set the same way as for password login.
  The client secret is encrypted with the master key, re-encrypted on key
  rotation, and restored with backups. Passwords keep working. New audit
  events: `sso_provider.*`, `sso_identity.linked`, and `sso_login.failed`;
  logins record `method: sso`. Issue #121.

- `dopbase login --browser` signs the CLI in through the Admin UI, so the
  password is never typed in the terminal. It prints a link to copy into any
  browser and a short code, then waits until the code is approved at
  `/cli/approve`. `--browser-open` also opens the link in this machine's
  browser, and `--password` forces the password prompt. The approval page
  stands on its own, with the code entered in one box per character. It uses
  the browser's current session and shows which account it will sign in, with
  a button to switch. Approving needs a recent confirmation, so an older
  session gets the usual confirm dialog instead of being signed out. With no
  session, or one that ends while the page is open, it goes to the sign-in page
  and back, without the "session expired" dialog. Plain
  `dopbase login` keeps asking for a password and switches to the browser when
  the server reports that single sign-on is on or password login is off. The
  new `GET /api/v1/auth/methods` endpoint reports which methods a server
  accepts. The approval page shows the requesting hostname and IP address.
  After an approval it counts down for 5 seconds and closes the tab. If the
  browser won't let it, because the tab wasn't opened by a script, the page
  says it can be closed by hand.
  Approvals and
  denials are audited as `cli_login.approved` and `cli_login.denied`, and the
  issued session as `login.succeeded` with `method: browser`. Codes expire
  after 10 minutes and work once, wrong codes are rate limited per session,
  and Ctrl+C cancels the pending request. Issue #30.

- `dopbase run` can pick which secrets reach the child. `--only KEY[,KEY]`
  injects only the named keys and stops before the child starts if one is not
  stored; `--allow-missing` turns that into a warning.
  `--preserve-env KEY[,KEY]` keeps the value the shell already has for the
  named keys. Both flags repeat and take comma lists. Warnings and the summary
  line go to standard error and contain names only. Issue #17.

- Secrets keep their version history. Every save (set, import, project init,
  or rollback) stores the new version encrypted, with the change type, who
  made it, and when. `dopbase secret history <ENVIRONMENT_REF> <KEY>` lists
  versions newest first with their status (`current`, `available`, or
  `expired`), pages with `--limit` and `--before`, prints the next-page
  command to standard error, and prints the API page unchanged with `--json`.
  `dopbase secret get <ENVIRONMENT_REF> <KEY> --version <N>` shows one
  version, and `--reveal` prints its value after password confirmation. The
  history never contains values; reveals are audited as
  `secret.version_revealed`. Issue #33.

- `dopbase secret rollback <ENVIRONMENT_REF> <KEY> [VERSION]` saves the value
  of an earlier version as a new version and leaves the history unchanged, so
  a rollback can be rolled back too. Without VERSION it lets you pick from the
  versions that can still be restored. It shows the current version, the one
  being restored, and the resulting version number, then asks for
  confirmation unless `--yes` is passed; without a terminal, VERSION and
  `--yes` are required. The request carries the version you saw, so a
  rollback is refused with `SECRET_VERSION_CONFLICT` if someone changed the
  secret in the meantime. Rollbacks are audited as `secret.rolled_back` with
  the source, previous, and new versions and no value.

- The secrets table in the web UI has a History button on each row, and the
  update dialog links to it. The History dialog lists versions, reveals an
  earlier value for thirty seconds after a password check, and rolls back
  after one confirmation. When the secret changed while the dialog was open,
  it reloads the history and asks you to try again.

- `GET /api/v1/environments/{id}/secrets/{key}/versions`,
  `POST .../versions/{version}/reveal`, and `POST .../rollback` serve the same
  features to scripts. Listing has the access of `secret list`; reveal and
  rollback need a project manager, the CSRF header and recent password
  confirmation for browser sessions, and refuse runner and AI agent tokens.

- `secret_history_versions` in `server.toml`, or
  `DOPBASE_SECRET_HISTORY_VERSIONS`, sets how many of the newest versions of
  each secret keep their value (2 to 1000, default 20). New instances start
  with `secret_history_versions = 20` in the generated `server.toml`. Older
  versions keep their metadata and show as `expired`. Pruning happens when a secret is
  saved, so a lower count applies to each secret on its next save. The
  `20261024010048_secret_versions` migration adds the `secret_versions` table
  and records every existing secret as
  its first version. Backups, restores, and master key rekeying carry the
  retained versions.

- The audit log is now a hash chain. Every audit record stores its position,
  the hash of the record before it, and the hash of its own contents, so a
  record that was modified, removed, or inserted after the fact stands out at
  that point in the chain. `dopbase audit verify` recomputes every hash in
  sequence order, stops at the first break, and exits with code `0` when the
  chain is intact and `2` when it is broken. It prints the head sequence and
  hash for external anchoring, and `--json` prints the full report.
  `GET /api/v1/audit-events/verify` serves the same check to scripts and is
  restricted to administrators, like the audit log itself.

  Records written before this release are never sealed after the fact:
  verification reports them as `legacy` without failing, and any record
  without a chain position that is newer than the chain fails verification.
  The database rejects updates of sealed records. The chain columns are
  additive: existing audit endpoints, filters, and clients keep their shape,
  and verification reads no secret values. Contributed by bosd in #123.
  Issue #119.

- `dopbase audit verify --anchor SEQ:HASH`, or `anchorSeq` and `anchorHash`
  on the verify endpoint, compares the log with a head saved outside the
  server and reports `matched`, `mismatch`, `missing`, or `pruned`. This
  catches what the chain alone cannot: a fully rewritten log, or deleted
  newest records. The server also logs the head as an `audit checkpoint` at
  startup, at shutdown, and after every verification.

- `dopbase audit list` shows one page of audit events, newest first, as a
  table of time, chain sequence, actor, action, resource, project, and
  environment. It takes the same `--action`, `--project`, `--environment`,
  and `--actor` filters as `dopbase audit export`, plus `--limit` (1 to 200,
  default 50) and `--cursor`. When more events match, the command for the
  next page is printed to standard error with the same filters, and `--json`
  prints the API page unchanged. Stored control characters show as `?` in
  the table, so a planted record cannot rewrite the terminal around it.
  Administrators only; no secret values are read.

- `dopbase audit export` streams audit records as JSON lines to standard
  output or, with `--output <FILE>`, to a file written with owner-only
  permissions that is never replaced without `--force`. `--action`,
  `--project`, `--environment`, and `--actor` narrow the stream, and `--limit`
  caps it; without filters every record is exported by following pagination to
  the end. Each line is the record exactly as the audit list API returns it,
  chain fields (`seq`, `prevHash`, `recordHash`) included, so any exported
  record can serve as a verification anchor. Export requires an
  administrator account and reads no secret values. Contributed by bosd in
  #124. Issue #119.

- `audit_retention_days` in `server.toml`, or `DOPBASE_AUDIT_RETENTION_DAYS`,
  sets how many days audit records are kept; unset keeps them forever. New
  instances start with `audit_retention_days = 365` in the generated
  `server.toml`. Existing instances keep their whole log until you add the
  setting. Older records are archived as JSON lines to `audit-archive/` in
  the data directory before they are deleted, and an `audit.pruned` record
  marks where the chain continues, so `dopbase audit verify` still passes.
  Retention runs at startup and then once a day, never inside the request
  that writes a record.
  Back up `audit-archive/` separately: snapshots store the database only.
  Contributed by bosd in #125. Issue #119.

- A Sign in with Google tutorial, with a diagram of the sign-in flow, that
  covers the Google Auth Platform setup, the OAuth client, the Dopbase
  settings, and the errors people run into most, such as
  `redirect_uri_mismatch`.

- A Sign in with Keycloak tutorial, with screenshots, that runs Keycloak in
  Docker next to a local Dopbase and walks from creating the realm and client
  to a member signing in. It is tested with Keycloak 26.4.7 and ends with
  what changes for production and the common errors.

- Screenshots in the documentation open full size when clicked. Escape, the
  Close button, or a click on the image closes the view.

### Improvement

- The SQL for single sign-on, sign-in, CLI login, and users now lives in each
  module's `repository.rs`, with the services calling it, as in the other
  modules. Behavior is unchanged.

- Code that hard-codes one of the four fixed roles (root, admin, member,
  ai_agent) is marked with a `TODO(rbac):` comment, so the role-based access
  control work can find every place with `rg "TODO\(rbac\)"`.
  `CONTRIBUTING.md` and `CONTEXT.md` explain the marker.

- Adding a migration file now rebuilds the binary. `sqlx::migrate!` embeds the
  migrations at compile time, and without a build script a new file was only
  picked up after some Rust source changed, so local builds and tests could run
  a stale set.

- Pull request descriptions have a Demo section. Features and improvements
  need a screenshot or video, and other changes write `Not needed:` with a
  reason. The template check enforces it, and `CONTRIBUTING.md` explains what
  counts.

- The update dialog in the web UI shows the key as a read-only field with
  the current version, and says which version the save creates.

- Audit records store the client's IP address, which was always empty
  before. Behind a reverse proxy, list it in `trusted_proxies` in
  `server.toml` or `DOPBASE_TRUSTED_PROXIES`, and the address comes from
  `X-Forwarded-For` for that proxy's connections only, so a client cannot
  choose the address it is recorded under. The audit list API returns
  `ipAddress`, and the Audit page shows it with the actor type when an
  event is expanded.

- Restoring a backup archives the current audit log to
  `audit-archive/pre-restore-<TIME>.jsonl` before replacing it, so records
  written after the snapshot are kept; the restore stops if the archive
  cannot be written. `backup.restored` names the archive, its record count,
  and the replaced chain's head.

- The admin UI now works on phones and tablets. Below 1024px the sidebar
  folds into a top bar with a menu button that opens the navigation in a
  side sheet, and the project rail moves behind a `project / environment`
  switcher at the top of the Projects page.
  Tables for secrets, tokens, backups, and audit events keep their
  key columns on small screens and show the rest as a second line under each row.
  Dialogs dock to the bottom of a phone screen and scroll inside,
  so their buttons stay in reach. Project and environment rename and delete
  buttons are always visible on touch screens, and form fields use
  16px text on phones so iOS no longer zooms in when a field is focused.

### Security

- Improved security to prevent secret value guessing when importing secrets
  in the Admin UI. The import also checks whether the session has expired.
  Reported by kta1kri.

- Fixed an account lockout bypass where failed logins for fake emails could
  reset the attempt budget. The limiter now keeps locked entries, evicts the
  keys with the fewest failures first, and scans for evictions in batches.
  Applies to password confirmation, setup, CLI login codes, and SSO callbacks.
  Reported by onurcangnc.

- A failed login now takes as long for an unknown email, or an account with
  no password, as for a wrong password: Dopbase verifies against a dummy hash.
  Before, an unknown email answered at once, which told an attacker which
  emails have accounts.

- Turning on **Require single sign-on** signs out every account except root,
  so sessions started with a password, CLI ones included, do not outlive the
  policy.

- One client can hold at most 20 pending single sign-on sign-ins and 20
  pending CLI login requests, so the anonymous endpoints cannot be used to
  fill the global caps and block everyone else.

- Redirect targets after sign-in refuse backslashes and control characters,
  which browsers can turn into a path to another site.

### Note

- **Breaking:** The error code for an invalid email is now `EMAIL_INVALID`. It
  was spelled `EMAIL_INVAILD`; API clients that match on it need the new
  spelling.

- **Breaking:** Database migrations use UTC timestamp versions
  (`YYYYMMDDHHMMSS_<name>.up.sql` and `.down.sql`), the format
  `sqlx migrate add -r` generates, so two branches no longer pick the same
  four-digit number. The existing migrations kept their names and only gained
  the timestamp; 0.1.9 databases move onto the new versions automatically at
  startup, and downgrading to 0.1.9 after this release is not supported.
  Contributed by meulatea in #140. Issue #139.

- **Breaking:** `dopbase run` no longer injects reserved names from stored
  secrets, and prints the skipped names to standard error. The list covers the Dopbase
  control variables (`DOPBASE_TOKEN`, `DOPBASE_URL`, `DOPBASE_ENV`,
  `DOPBASE_DATA_DIR`, `DOPBASE_SECRETS_PATH`) and the shell variables skip:
  `PATH`, `HOME`, `PS1`, `PS2`, and names that start
  with `XDG_` or `LC_`. Before, a stored `DOPBASE_TOKEN` was injected even
  though the inherited one was removed.
  Names that load code into the child (`LD_PRELOAD`, `LD_LIBRARY_PATH`,
  `DYLD_INSERT_LIBRARIES`, `DYLD_LIBRARY_PATH`, `NODE_OPTIONS`, `PERL5OPT`,
  `PYTHONWARNINGS`, `BASH_ENV`, `PROMPT_COMMAND`, `JAVA_TOOL_OPTIONS`,
  `PHPRC`) are skipped too unless `--only` names them; then they are
  injected with a warning. If you store `NODE_OPTIONS` or another of these on
  purpose, add it to `--only`. `NODE_VERSION`, `BROWSER`, `HOSTNAME`,
  `WINDIR`, and `USERPROFILE` are still injected, with a warning. The summary
  line counts injected keys, as `N of M key(s)` when some were left out.

- Deleting a secret, an environment, or a project also deletes the version
  history of the secrets involved, and so does a replace import for the keys
  it removes. Restoring deleted secrets is tracked in #135.

- The unused `user_agent` column is removed from `audit_events`; nothing
  ever recorded a value in it.

- Audit records name the root user's actions with the actor type `root`
  instead of `admin`, so they stand apart from other administrators. Other
  administrators keep `admin`.

### Fixed

- `GET /api/v1/auth/session` returns a `csrfToken` for browser sessions. Before,
  only the tab that signed in had a token, so an Admin UI tab opened from a
  link or in a new window could read but not change anything. The new token is
  derived from the session's own on the server, and both stay valid.

- Dialogs in the web UI stack in a fixed order: page dialogs, then the
  password confirmation, then "Session expired". Before, a dialog could
  open behind the one that was already up. When the session expires, every
  other dialog closes, so saving with an expired session leaves only
  "Session expired" and its sign-in button on screen, without an error
  banner behind it.

- The "confirm your password" dialog clears the password field whenever it
  closes. Before, a password typed and then cancelled was still filled in the
  next time the dialog opened.

## 0.1.9 - 2026-10-07

[Dopbase](https://dopbase.com) 0.1.9 adds guided server setup, optional web UI,
background server management, terminal secret editing, and runner exports for
pipelines. AI agent tokens can read CLI metadata. Run token arguments, legacy
server commands, and some setup responses change in this release.

### Added

- Runner tokens can export their assigned environment with `dopbase export`,
  using `DOPBASE_TOKEN` or an encrypted credential saved by `login --token`.
  Pipeline exports need no terminal or password prompt and require a live server
  response. Export formats, private files, and overwrite protection remain
  unchanged. Added an AWS CodeBuild example using Secrets Manager.

- `dopbase env edit <ENVIRONMENT_REF>` opens an environment in Vim, Neovim,
  Helix, nano, or a custom terminal editor. It checks the target before password
  confirmation and offers to add secrets when an environment is empty. It
  validates edits, previews key changes, removes editor files, and saves only
  after confirmation. Prints "Checking changes..." and "Saving..." while
  waiting for the server.
  Comments and ordering persist across CLI and UI sessions. `--editor` selects
  a command and `--dry-run` previews without saving. Protected editor profiles,
  private files, revision checks, and session cleanup reduce plaintext exposure.
  Custom commands require acknowledgement; comments remain unencrypted metadata.

- Disable the web UI with `web_ui = false` in `server.toml`,
  `DOPBASE_WEB_UI=false`, or `server start --no-web-ui`. It stays enabled by
  default. CLI and REST API access remain available, and API documentation uses
  its own setting. Background restart preserves launch overrides.
  `server setup --web` explicitly enables the UI for that run without changing
  saved configuration.

- `DOPBASE_ROOT_EMAIL` supplies the root email for `server setup`, with explicit
  `--email` taking precedence. CLI setup generates a password without prompts;
  web setup includes the email in its link and prefills the editable form.
  `server setup --web --email` is now supported. Invalid email defaults fail
  before setup writes files. On fresh `server start`, the variable creates root
  with a generated password shown once; initialized startup ignores it.

- `dopbase server setup` initializes a local instance through guided root-account
  prompts or `--email` with a generated password shown once. `--json` supports
  generated-password setup. `server setup --web` runs the existing web claim
  and backup-restoration flow.

- `dopbase server start --background` (or `-b`) starts a managed background
  server. `server restart` restarts it with the saved CLI and server environment
  overrides while rereading configuration.

- Server startup now creates missing server and client configuration files with
  commented examples for every supported setting. Client saves include the
  reference above active settings.

- Community discussion forms, starter tutorials with verified CLI output, and
  links from the README, documentation, and issue chooser.
- A discussion publication manifest and a maintainer checklist for categories,
  pins, and the first month of community follow-up.

### Improvement

- Linux and macOS release downloads use tar.gz archives with gzip level 9.
  Release steps run through tested Bash scripts, cache dependencies, and support
  a manual build run without publishing. Releases stay drafts until uploads are
  verified; reruns preserve published assets.

- The public shell installer shows download progress, verifies SHA-256, and
  checks the new binary with `--version` before replacing an existing install.
  It reports the installed path and current server commands. It requires
  tar/gzip and supports tar.gz releases only.

- `dopbase env list` and the list printed after `env clone` show a single
  `ENVIRONMENT` column with `project/environment` values. IDs, update times,
  and JSON output keep their existing format.

- CLI tests now cover every public command and subcommand, with shared fixtures
  for real server and terminal workflows. Removed repeated cases and duplicate
  backend test runs.

- `server start` initializes fresh instances automatically through the existing
  protected web setup flow or a root email with generated credentials. Foreground
  and background modes keep serving after setup. Background credentials return
  only to the launching command, including an optional `setup` object in JSON.
  Disabled UI settings are respected; interrupted factory resets still require
  explicit setup. Credential-output failure stops the new server and preserves
  the initialized account for offline password recovery.

### Security

- Runner exports enforce environment scope and record the runner token ID and
  secret count in the audit log without secret values. Human password
  confirmation, browser CSRF checks, and AI agent restrictions remain in place.

### Note

- **Breaking:** Removed `dopbase run -t <TOKEN>` and `--token <TOKEN>`.
  Inject `DOPBASE_TOKEN` through a CI secret store or deployment platform, or
  save a runner credential with `dopbase login --token`. Authentication now
  resolves `DOPBASE_TOKEN`, then the encrypted saved credential. An invalid
  environment credential never falls back to saved credentials. Arguments
  after `--` still belong to the child command.

- **Breaking:** With the web UI disabled and no bootstrap root email, background
  startup with `--json` writes
  `success = false` and `info.code = "SETUP_REQUIRED"` to stdout, replacing
  `error.SETUP_REQUIRED` on stderr. The notice includes `message`, `data_dir`,
  and `config_file`. Update scripts that read the previous response.

- Existing initialized installations preserve accounts and credentials. No
  database migration is required. Explicit terminal setup, web setup, and backup
  restoration keep their existing behavior. `dopbase init` retains its project
  and environment workflow.

- **Breaking:** `dopbase server up` and `dopbase server down` have been replaced by
  `server start --background` or `server start -b` and `server stop`.
  The old commands show an informational migration notice and exit with
  status 1 without changing server state. JSON mode returns `COMMAND_REPLACED`
  information on stdout. Servers started before this change need one manual
  stop/start before restart is available. Update scripts that use the previous commands.

### Fixed

- AI agent tokens now work with `project list`, `project show`, `env list`,
  `env show`, `secret list`, and `secret get` without `--reveal`. Metadata reads
  validate the token through the API instead of requiring a human session.
  Invalid, expired, and revoked tokens fail without falling back to saved
  credentials. Secret values and writes remain restricted.

- CLI documentation now lists the `env edit` fallback order correctly:
  `nvim`, `vim`, `hx`, then `nano`.

- Secret exports include values, layout, and a revision from one snapshot.
  Imports check supplied revisions during dry runs as well as apply, protecting
  edits from concurrent changes. Layout imports reject slots containing values.
  CLI response buffers are cleared after parsing, and exported JSON values are
  moved out of the response instead of copied.

- Background startup shows the short `dopbase server stop` and
  `dopbase server restart` commands. JSON output includes both
  `stop_command` and `restart_command`.

- Direct invocation of the internal supervised server mode no longer claims a
  runtime-owned file descriptor. Readiness reporting attaches only to an
  inherited write pipe, preventing reactor failures during startup rejection.

- Remote CLI exports now send an explicit content length for empty POST
  requests, avoiding HTML errors from hosts that require it. The fix also
  applies to other CLI commands that send empty POST requests.
- CLI errors now report the HTTP status when a server returns a non-JSON error
  response.

## 0.1.8 - 2026-09-26

[Dopbase](https://dopbase.com) 0.1.8 adds configurable expiry for runner and AI agent tokens and a CLI
command to clone an environment's secrets. Interactive `dopbase run` commands
now keep terminal input, and the Admin UI shows token expiry and clearer audit
details.

### Added

- Runner and AI agent tokens can now expire after a chosen number of hours or
  days, up to three years, or have no expiry. The Admin UI offers preset and
  custom durations; `dopbase token create` accepts `--expires-in`.
- `dopbase env clone PROJECT/ENVIRONMENT NEW_NAME` creates an environment in
  the same project and copies its current secrets without a temporary file.

### Improvement

- Secrets table rows now brighten on hover, and expanded audit events show their raw timestamp.
- Grouped the Users page controller's values and functions into `state` and
  `actions` to simplify its page setup.
- Added fix inconsistent width layout on frontend.
- Documentation now reads the current version from `package.json` during builds
  instead of keeping release numbers in individual pages.
- Documentation checks now run from one script and validate every Mermaid
  diagram under `docs`.

### Fixed

- UI controller tests now run unmount hooks inside component setup, removing
  lifecycle warnings from the test run.
- Token tests no longer print credential-bearing API responses on assertion failures.
- The AI agents list now shows each token's expiry date or "No expiry".
- `dopbase run` now keeps interactive commands attached to the terminal, so
  tools such as Codex and Claude accept input instead of stopping at startup.

### Note

- Existing runner tokens remain without expiry. New runner tokens also have no
  expiry unless one is set. AI agent tokens still default to 30 days.
- The repository's `scripts/install.sh` helper was removed. Use the installer
  at `https://dopbase.com/install.sh`.

## 0.1.7 - 2026-09-20

[Dopbase](https://dopbase.com) 0.1.7 adds guided `.env` setup, TOML secret files, and Docker-ready
exports. It also refreshes the Admin UI branding, demo assets, and contributor
documentation.

### Added

- Added TOML support for `dopbase init`, `dopbase import`, and
  `dopbase export`, including `.toml` filename detection and `--format toml`.
- Added an interactive `dopbase init` flow that reads `./.env`, checks the
  active server and login, summarizes detected variables, creates a new project
  and environment, and asks whether to delete the imported file.
- Added Docker, JSON, YAML, and dotenv export formats.
- Added `dopbase export project/env --format docker --stdout` for explicit
  output format selection.
- Added an export-only Docker format for passing secrets to `docker run` or
  `docker exec` through `--env-file` without dotenv quotes changing values.
- Added web-ready MP4 and GIF versions of the Dopbase demo, including three
  video sizes, a short forum preview, and a README Demo section with a GIF
  preview and full video download.
- Added a Contributor Covenant 3.0 Code of Conduct with a private reporting
  process and enforcement guidelines.

### Improvement

- Reformatted CLI help constants as multiline strings so examples are easier to
  edit.
- Automatic hide when user want to update token while secrets revealed.
- Removed third-party attribution from Admin UI source comments and design
  token documentation so the interface is described as Dopbase's own system.
- The Admin UI now uses the updated Dopbase mark in authentication, navigation,
  and not-found screens.
- Bun scripts now use `action:target` names for development, builds, tests,
  linting, type checks, formatting checks, and previews.
- Backend release verification now discovers the CLI command tree from the
  built binary and checks both supported help forms for every command.
- The README now includes the complete top-level CLI reference, environment
  variables, and common startup examples.
- Removed duplicate frontend and backend tests while preserving behavior
  coverage.

## 0.1.6 - 2026-09-12

[Dopbase](https://dopbase.com) 0.1.6 adds JSON and YAML secret files, runtime cache maintenance, and
consistent project and environment references across the CLI and Admin UI.

### Added

- `dopbase init`, `dopbase secret import`, and `dopbase secret export` support
  dotenv, JSON, and YAML files. File extensions select the format automatically,
  or `--format` can set it explicitly.
- `dopbase cache list` shows safe metadata for the active server's encrypted
  runtime cache.
- `dopbase cache clean` removes entries older than 14 days by default, with
  options for a different age, a dry run, or complete cleanup.

### Improvement

- `dopbase init` now accepts one `PROJECT_NAME/ENVIRONMENT_NAME` target.
- `dopbase env create` now accepts one `PROJECT_REF/ENVIRONMENT_NAME` target.
  The previous two-argument forms are no longer accepted.
- Environment references can use an environment ID or an environment name
  qualified by either a project name or project ID.
- CLI usage and help now distinguish new resource names from references to
  existing resources.
- Server startup and update errors now give more specific recovery steps.

### Fixed

- The Admin UI now keeps project and environment selection in the URL, and row
  actions no longer select the item underneath them.
- Expired browser sessions now show a sign-in prompt instead of repeatedly
  sending requests with an expired CSRF token.

## 0.1.5 - 2026-09-11

Dopbase now supports more self-hosted connection formats, creates a recovery
archive before host-side factory resets, and uses clearer copy across the CLI,
Admin UI, API documentation, and public guides.

### Added

- `dopbase client connect` accepts explicit HTTP URLs and bare IPv4 or IPv6
  addresses with optional ports. Bare IP addresses use HTTP by default.
- `dopbase admin factory-reset` saves the complete data directory as a ZIP
  before removing it. Pass `--no-backup` to reset without creating the archive.

### Improvement

- `dopbase run -t <TOKEN>` is now a short form of `--token <TOKEN>`.
- `dopbase -v` now prints the installed version as `v<version>`. The existing
  `-V` and `--version` flags use the same output.
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

[Dopbase](https://dopbase.com) 0.1.2 adds an offline, root-authorized factory reset, switches new
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

- Admin UI component system with solid borderless buttons, filled fields with
  inline labels, borderless badges, and new toggle and checkbox primitives,
  all using the Dopbase purple accent.
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
