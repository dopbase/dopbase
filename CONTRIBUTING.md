# Contributing to Dopbase

Thanks for taking the time to work on Dopbase. Check the code, tests, and current documentation before changing public behavior.

## Before you start

Search the existing issues before opening a new one. For a substantial feature, architecture change, or new dependency, open an issue first and describe the problem you want to solve. Early discussion can prevent a large pull request from heading in a direction the project cannot use.

Security vulnerabilities do not belong in public issues. Follow [SECURITY.md](./SECURITY.md) instead.

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

Screenshots are useful for visible interface changes. Logs and screenshots must not contain credentials or private endpoints.

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
bun run test:installer
```

If an existing unrelated failure prevents a check from passing, describe the failure and the command output in the pull request. Do not hide or silently skip it.

## Publish a release

Releases use annotated semantic-version tags. Before tagging, update the
version in `package.json` and `app/Cargo.toml`, refresh `app/Cargo.lock`, and
move the release notes from `Unreleased` to a dated version section in
`CHANGELOG.md`. Merge those changes into `main`, then create and push the tag
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
matches the Rust package version, builds the Linux and macOS archives, creates
`checksums.txt`, and publishes the release only after all four targets succeed.
The release body comes from `.github/RELEASE_TEMPLATE.md`. It includes the
summary, the populated optional sections, and a full changelog link comparing
the previous version with the new one.

## Review expectations

Maintainers may ask for a smaller change, clearer tests, documentation, or a different interface. Review focuses on correctness, security, maintainability, and keeping Dopbase understandable for someone running it themselves.

Be patient and respectful. Security-sensitive changes may need additional review.

## Contribution license

Dopbase is licensed under the [Apache License 2.0](./LICENSE). Unless you state otherwise, any contribution you intentionally submit for inclusion in Dopbase is provided under that license, as described in section 5.

The project does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.
