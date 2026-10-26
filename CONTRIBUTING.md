# Contributing to Dopbase

Thanks for taking the time to work on Dopbase. Check the code, tests, and current documentation before changing public behavior.

## Before you start

Search existing issues and [discussions](https://github.com/dopbase/dopbase/discussions)
before opening a new thread. Ask setup questions in Help & questions and discuss
new capabilities in Feature requests. Use Feedback for your experience with
existing behavior, Tutorials for tested walkthroughs, and Showcase for working
examples.

Report reproducible bugs in Issues. For a substantial feature, architecture
change, or new dependency, discuss the problem before starting a large pull
request. Accepted work gets an issue linked to its discussion. Existing feature
issues remain valid; a discussion is not required for every change.

Community tutorials should include the Dopbase version and OS tested,
prerequisites, commands paired with output, verification steps, and cleanup.
Explain variable IDs and timestamps, redact credentials, and label examples
that were not executed.

Security vulnerabilities do not belong in public issues. Follow [SECURITY.md](./SECURITY.md) instead.

All project interactions must follow the [Code of Conduct](./CODE_OF_CONDUCT.md).
Conduct concerns should be reported privately through the process described
there.

## Repository layout

| Path    | Purpose                                           |
| ------- | ------------------------------------------------- |
| `app/`  | Rust service, command-line application, and tests |
| `src/`  | Vue application, frontend tests, and shared setup |
| `docs/` | Public VitePress documentation                    |

The Rust service and Vue application are still scaffolding. Product behavior should stay consistent with the public documentation, but implementation findings may require the documentation to change.

## Set up the project

You need:

- [Bun](https://bun.sh/) for the frontend and documentation
- A Rust toolchain with Rust 2024 edition support

Install the JavaScript dependencies:

```bash
bun install
```

Development scripts use `ui`, `app`, and `docs` as targets:

```bash
bun run dev
bun run dev:ui
bun run dev:app
bun run dev:docs
```

Fresh development instances enter setup automatically. Open the printed link
or supply `DOPBASE_ROOT_EMAIL` to generate credentials. Use a separate
`DOPBASE_DATA_DIR` for development and keep it the same across starts.
`bun run dev` starts the Admin UI and Rust app together. Use a targeted command
when you need only one part of the repository.

## Make a change

Keep each pull request focused on one problem. Match the surrounding code, add tests where behavior changes, and update public documentation when a user-facing interface changes.

GitHub fills new pull requests with the repository template. Complete every
section and check each checklist item. The template check must pass before the
pull request can merge.

Do not commit generated output from `dist/`, `docs/.vitepress/dist/`, coverage reports, editor settings, local databases, credentials, or `.env` files.

Use commit messages that explain the change in plain language. A pull request should explain:

- What problem it solves
- How the solution works
- How it was tested
- Any compatibility, security, or migration concerns
- What the change looks like when it runs

### Show a demo

A pull request that adds a feature or improves existing behavior needs a
screenshot or a short video in the Demo section. Reviewers use it to see the
change without checking out the branch. This applies to the CLI as much as the
web UI: a terminal screenshot of the new command or flag counts.

Drag the image or video into the pull request description and GitHub uploads
it. A Markdown image, an `<img>` or `<video>` tag, or a link to a `.png`,
`.jpg`, `.gif`, `.webp`, `.mp4`, `.mov`, or `.webm` file also works.

Changes to documentation, tests, CI, dependencies, or internal refactors that
users can't see don't need a demo. Write `Not needed:` followed by the reason,
for example `Not needed: documentation only.`

The template check fails when the Demo section has neither media nor a
`Not needed:` reason. It can't tell a feature from a refactor, so reviewers
may still ask for a demo when the reason doesn't fit the change.

Screenshots, videos, and logs must not show credentials, tokens, secret values,
or private endpoints. Use test data and blur or crop anything else.

### Add a database migration

Generate migrations with the sqlx CLI from `app/`, so the version is the UTC
creation time instead of the next free number:

```bash
cargo install sqlx-cli --no-default-features --features sqlite
cd app
sqlx migrate add -r <name>
```

Use a lowercase `snake_case` name, keep one logical change per pair, and make
the down file reverse exactly that change.

Do not edit or rename a migration after it ships in a release: sqlx checksums
applied migrations and refuses to start when one changes. Databases created by
0.1.9 or earlier are moved onto the timestamped versions automatically at
startup; see [`app/migrations/README.md`](./app/migrations/README.md).

### Add a single sign-on preset

The providers listed on the Single sign-on page come from
[`app/src/modules/sso/presets.toml`](./app/src/modules/sso/presets.toml). To
add a provider, add a `[[preset]]` block to that file. The database stores the
preset ID as plain text and the Admin UI reads the list from the server, so no
other file changes.

```toml
[[preset]]
id = "pocket-id"
label = "Pocket ID"
button_label = "Pocket ID"
issuer = "https://<your-domain>"
hint = "Create an OIDC client and paste the redirect URI shown above."
```

`id` uses lowercase letters, digits, and dashes. Saved providers refer to it,
so don't change it after a release. `label` names the provider in the list,
`button_label` is the default text of the sign-in button, `issuer` is an
example issuer URL, and `hint` is the setup note under the list. The list
follows the file order, with `generic` last.

Three optional fields handle provider quirks:

- `email_verified_claim` names the ID token claim that proves the email
  belongs to the person signing in. The default is `email_verified`; Microsoft
  Entra uses `xms_edov`.
- `prompt_login = false` stops Dopbase from sending `prompt=login` when
  someone confirms a sensitive action, for providers that reject it, such as
  Google. `max_age=0` is still sent.
- `reject_issuer_segments` lists issuer path segments to refuse, and
  `reject_issuer_message` tells root why. Microsoft refuses `/common/`, where
  any tenant can get tokens.

The comment at the top of the file repeats these rules. A misspelled key fails
the build's tests instead of being ignored, and a saved provider whose preset
was removed from the file keeps working and behaves like `generic`. Run
`cargo test sso_presets` from `app/` to check the file, then rebuild: the file
is compiled into the binary.

### Check capabilities and levels, never role names

Roles are data, so code never asks which role an account has. Ask what it may
do instead, through `app/src/services/authx.rs`:

- An instance action, such as managing people or reading the audit log,
  checks a capability with `require_capability` or `Access::has`.
- An action on a project or environment checks the caller's level with
  `projects::service::authorize` or `environments::service::authorize`. They
  answer 404 for anything the caller can't see, so its existence doesn't leak.

```rust
let (admin_id, email) = require_account(identity)?;
let env = environments::service::authorize(state, identity, id, Level::Write).await?;
```

A new kind of power is a new `Capability` variant, given to the built-in roles
that should have it in a migration. If only root should ever hold it, add it to
`Capability::ROOT_ONLY`. In the Admin UI, gate on `auth.can(...)` and the
`level` that project and environment responses carry, never on `role`.

## Check your work

Run the complete repository checks when your change crosses several targets:

```bash
bun run format:check
bun run lint
bun run typecheck
bun run test
bun run build
```

`test` covers the Admin UI and Rust app. Documentation, installer, and GitHub
template tests have separate commands.

Frontend:

```bash
bun run format:repo:check
bun run lint:ui
bun run typecheck:ui
bun run test:ui
bun run build:ui
```

The frontend temporarily uses two TypeScript compilers. Native TypeScript 7
checks the TypeScript project graph, while the TypeScript 6 compatibility
package powers `vue-tsc` and ESLint until their Vue/compiler integrations
support the TypeScript 7 API. Run `bun run typecheck:ui` to execute both checks.

Rust:

```bash
bun run format:app:check
bun run lint:app
bun run typecheck:app
bun run test:app
```

Changes that affect migrations or multiple backend modules must also pass the
isolated Docker verification:

```bash
docker build -f app/Dockerfile.test -t dopbase-backend-test .
docker run --rm --name dopbase-backend-test-run dopbase-backend-test
docker image rm dopbase-backend-test
docker image prune -f --filter label=dopbase.test=true
```

The container builds the real Vue Admin UI, embeds it in the release binary,
then launches that binary from a clean temporary directory. It verifies the UI,
health API, OpenAPI document, Swagger UI, and default storage layout without
mounting or modifying a host database.

Documentation:

```bash
bun run test:docs
bun run build:docs
```

Repository utilities:

```bash
bun run test:github
actionlint .github/workflows/release.yml
shellcheck scripts/release/*.sh
```

If an existing unrelated failure prevents a check from passing, describe the failure and the command output in the pull request. Do not hide or silently skip it.

## Publish a release

Releases use annotated semantic-version tags. Before tagging, update the
version in `package.json` and `app/Cargo.toml`, refresh `app/Cargo.lock`, and
keep draft release notes under the planned version in `CHANGELOG.md`, such as
`## 0.1.9 - Unreleased`. Replace `Unreleased` with the release date before
publishing; release generation requires a dated heading. Merge those changes
into `main`, then create and push the tag
from the release commit:

Start each release section with one short summary paragraph. Add only the
sections that apply: `Added` for new features, `Improvement` for improvements,
`Fixed` for bug fixes, `Security` for security work, and `Note` for other
information. Each section must contain at least one entry.

```bash
git switch main
git pull --ff-only
version=0.1.0
git tag -a "$version" -m "$version"
git push origin "$version"
```

Pushing the tag starts the GitHub release workflow. It verifies that the tag
matches both package versions, builds the Linux and macOS tar.gz archives, creates
`checksums.txt`, and publishes the release only after all four targets succeed.
Release scripts live in `scripts/release/`.

A manual Release workflow run executes all checks and builds without publishing.
Tag runs publish only after all uploads are verified.
Published assets are preserved on reruns.

The release body comes from `.github/RELEASE_TEMPLATE.md`. It includes the
summary, the populated optional sections, and a full changelog link comparing
the previous version with the new one.

## Review expectations

Maintainers may ask for a smaller change, clearer tests, documentation, or a different interface. Review focuses on correctness, security, maintainability, and keeping Dopbase understandable for someone running it themselves.

Be patient and respectful. Security-sensitive changes may need additional review.

## Contribution license

Dopbase is licensed under the [Apache License 2.0](./LICENSE). Unless you state otherwise, any contribution you intentionally submit for inclusion in Dopbase is provided under that license, as described in section 5.

The project does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.
