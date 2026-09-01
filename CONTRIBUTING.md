# Contributing to Dopbase

Thanks for taking the time to work on Dopbase. Check the code, tests, and current documentation before changing public behavior.

## Before you start

Search the existing issues before opening a new one. For a substantial feature, architecture change, or new dependency, open an issue first and describe the problem you want to solve. Early discussion can prevent a large pull request from heading in a direction the project cannot use.

Security vulnerabilities do not belong in public issues. Follow [SECURITY.md](./SECURITY.md) instead.

## Repository layout

| Path     | Purpose                                   |
| -------- | ----------------------------------------- |
| `app/`   | Rust service and command-line application |
| `src/`   | Vue application                           |
| `docs/`  | Public VitePress documentation            |
| `tests/` | Frontend tests and shared test setup      |

The Rust service and Vue application are still scaffolding. Product behavior should stay consistent with the public documentation, but implementation findings may require the documentation to change.

## Set up the project

You need:

- [Bun](https://bun.sh/) for the frontend and documentation
- A Rust toolchain with Rust 2024 edition support

Install the JavaScript dependencies:

```bash
bun install
```

Start the Vue development server:

```bash
bun run dev
```

Run the Rust application:

```bash
cargo run --manifest-path app/Cargo.toml
```

Start the documentation site:

```bash
bun run docs:dev
```

## Make a change

Keep each pull request focused on one problem. Match the surrounding code, add tests where behavior changes, and update public documentation when a user-facing interface changes.

Do not commit generated output from `dist/`, `docs/.vitepress/dist/`, coverage reports, editor settings, local databases, credentials, or `.env` files.

Use commit messages that explain the change in plain language. A pull request should explain:

- What problem it solves
- How the solution works
- How it was tested
- Any compatibility, security, or migration concerns

Screenshots are useful for visible interface changes. Logs and screenshots must not contain credentials or private endpoints.

## Check your work

Run the checks that apply to your change.

Frontend:

```bash
bun run typecheck
bunx vitest run --passWithNoTests
bun run build
bunx prettier --check .
```

The frontend temporarily uses two TypeScript compilers. Native TypeScript 7
checks the TypeScript project graph, while the TypeScript 6 compatibility
package powers `vue-tsc` and ESLint until their Vue/compiler integrations
support the TypeScript 7 API. Run `bun run typecheck` to execute both checks.

The repository does not contain a discoverable frontend test file yet, so `--passWithNoTests` keeps the scaffold check explicit without treating the absence of tests as a failure. Vitest will run normally as soon as `*.test.*` or `*.spec.*` files are added.

Rust:

```bash
cargo fmt --manifest-path app/Cargo.toml -- --check
cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path app/Cargo.toml
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
bun run docs:build
```

If an existing unrelated failure prevents a check from passing, describe the failure and the command output in the pull request. Do not hide or silently skip it.

## Publish a release

Releases use annotated semantic-version tags. Before tagging, update the
version in `app/Cargo.toml`, refresh `app/Cargo.lock`, and add the release notes
to `CHANGELOG.md`. Merge those changes into `main`, then create and push the
tag from the release commit:

```bash
git switch main
git pull --ff-only
git tag -a v0.0.12 -m "v0.0.12"
git push origin v0.0.12
```

Pushing the tag starts the GitHub release workflow. It verifies that the tag
matches the Rust package version, builds the Linux, macOS, and Windows archives, creates
`checksums.txt`, and publishes the release only after every target succeeds.

## Review expectations

Maintainers may ask for a smaller change, clearer tests, documentation, or a different interface. Review focuses on correctness, security, maintainability, and keeping Dopbase understandable for someone running it themselves.

Be patient and respectful. Security-sensitive changes may need additional review.

## Contribution license

Dopbase is licensed under the [Apache License 2.0](./LICENSE). Unless you state otherwise, any contribution you intentionally submit for inclusion in Dopbase is provided under that license, as described in section 5.

The project does not currently require a Contributor License Agreement or Developer Certificate of Origin sign-off.
