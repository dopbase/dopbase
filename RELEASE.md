# Releasing Dopbase

This runbook is the source of truth for preparing and publishing a Dopbase
release on GitHub. It is written for maintainers and AI agents.

## Release rules

- Use a stable semantic version in `MAJOR.MINOR.PATCH` form, such as `0.1.7`.
- Release branches are named `release/<version>`.
- Tags contain the version only, such as `0.1.7`; do not prefix them with `v`.
- Create the release branch from the latest `origin/main`.
- Keep unrelated changes out of the release pull request.
- A repository owner must manually review and approve the pull request.
- Publish only commits that have been merged into `main`.
- Treat pushing the tag as the publishing action: it starts
  `.github/workflows/release.yml`, which builds the artifacts and publishes the
  GitHub release.

## Information required before starting

The release request must identify the target version. If it does not, stop and
ask the owner for it; do not choose a major, minor, or patch increment without
approval.

Before changing files, fetch `main` and all tags, confirm that the target tag
does not exist, and check that the working tree is clean. Preserve any existing
work instead of overwriting or discarding it.

```bash
git fetch origin main --tags
git status --short
git tag --list <version>
```

The release may proceed when the requested version matches
`MAJOR.MINOR.PATCH`, the tag is absent, and existing work is safely accounted
for.

## 1. Create the release branch

Create the branch directly from the fetched `origin/main`:

```bash
git switch --create release/<version> origin/main
```

If `release/<version>` already exists, inspect it and ask the owner whether to
continue it. Do not recreate, reset, or overwrite the branch.

This step is complete when `git branch --show-current` prints
`release/<version>` and its starting commit is the current `origin/main`.

## 2. Update the product version and documentation

Update the version in these authoritative files:

- `package.json`
- `app/Cargo.toml`

Refresh `app/Cargo.lock` after changing `app/Cargo.toml`:

```bash
cargo check --manifest-path app/Cargo.toml
```

Search the repository for the previous product version and inspect every match.
Update user-facing documentation that describes the current release, including
`README.md` and relevant files under `docs/`. Leave historical examples,
changelog entries, dependency versions, test fixtures, protocol versions, and
format versions unchanged unless they actually refer to the product release.

```bash
rg -n '<previous-version>' README.md docs package.json app/Cargo.toml app/Cargo.lock
```

This step is complete when `package.json`, the `app` package in
`app/Cargo.toml`, and the `app` package entry in `app/Cargo.lock` all contain the
target version, and every documentation match has been reviewed.

## 3. Prepare the changelog

Find the latest semantic-version tag earlier than the target version. Then
review both the commit list and the complete diff from that tag through the
current release branch:

```bash
previous_version="$(node scripts/github-templates.mjs previous-version --version <version> --commit HEAD)"
git log --first-parent --oneline "${previous_version}..HEAD"
git diff --stat "${previous_version}..HEAD"
git diff "${previous_version}..HEAD"
```

Use the diff as the source of truth. Account for every user-visible change,
including compatibility, migration, and security effects; do not copy commit
subjects blindly.

Add a new section at the top of `CHANGELOG.md` in this form:

```markdown
## <version> - YYYY-MM-DD

One short summary paragraph, no more than 500 characters.

### Added

- A user-visible change.
```

Use the current release date. Include only applicable headings from this list:
`Added`, `Improvement`, `Fixed`, `Security`, and `Note`. Every included heading
must have content. The automated release notes are generated from this section,
so do not use other heading names.

This step is complete when every user-visible change in the comparison is
represented once, the summary and headings satisfy the release-note format,
and historical changelog sections remain unchanged.

## 4. Verify the release preparation

Run the same checks used by the GitHub release workflow, plus the documentation
build:

```bash
bun install --frozen-lockfile
bun run test:github
bunx vitest run --passWithNoTests
bun run test:installer
bun run build
bun run docs:build
cargo fmt --manifest-path app/Cargo.toml -- --check
cargo clippy --manifest-path app/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path app/Cargo.toml --locked --all-targets
cargo test --manifest-path app/Cargo.toml --locked --all-targets --all-features
```

Also confirm the three release versions agree and the new changelog section
exists:

```bash
test "$(jq -r '.version' package.json)" = "<version>"
test "$(cargo metadata --manifest-path app/Cargo.toml --locked --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "app") | .version')" = "<version>"
grep --fixed-strings "## <version> -" CHANGELOG.md
```

Resolve release-related failures before continuing. If an unrelated existing
failure remains, record the exact command and failure in the pull request and
ask the owner whether the release may proceed.

## 5. Commit and open the pull request

Review the final diff, then create one release-preparation commit:

```bash
git diff --check
git diff --stat
git diff
git status --short
git add -- <each-changed-release-file>
git commit -m "chore(release): prepare <version>"
git push --set-upstream origin release/<version>
```

Stage only files actually changed for the release. Open a pull request from
`release/<version>` to `main` using `.github/pull_request_template.md`. Include
the version, changelog summary, compatibility impact, and every verification
command and result. The pull request is ready when its template check and all
required CI checks pass.

## 6. Owner review and merge

A repository owner manually reviews the release pull request, requests any
needed corrections, and approves it. Apply corrections on the release branch
and repeat the affected checks.

The owner merges the approved pull request into `main`. Do not tag an unmerged
release branch or bypass required review and CI.

This step is complete when the pull request is merged and the release commit is
reachable from `origin/main`.

## 7. Tag the merged release

Update local `main`, identify the exact merged commit, and create an annotated
tag from that commit:

```bash
git switch main
git pull --ff-only origin main
git tag -a <version> -m "<version>"
git show --no-patch <version>
```

Before pushing, confirm the tag points to the intended commit and that its
versions and changelog entry match `<version>`. Pushing the tag is irreversible
release publication and requires explicit owner approval:

```bash
git push origin <version>
```

## 8. Verify the GitHub release

Watch the `Release` GitHub Actions workflow triggered by the tag. It must:

- verify the tag, package versions, and changelog;
- pass the frontend, installer, Rust, and release-binary checks;
- build macOS and Linux archives for AMD64 and ARM64;
- generate `checksums.txt`;
- generate notes from `CHANGELOG.md` and `.github/RELEASE_TEMPLATE.md`; and
- publish the GitHub release and all five assets.

The release is complete only when the workflow succeeds and the GitHub release
named `Dopbase <version>` is visible with its notes, four ZIP archives, and
`checksums.txt`. Report the release URL and workflow result to the owner.

If the workflow fails, diagnose the failure and prepare a normal reviewed fix.
Never move, delete, or replace a published tag unless the owner explicitly
authorizes recovery after reviewing the impact.

## release instruction

When asked to **release Dopbase `<version>`**, follow this runbook in order and
report each completion criterion. Pause for owner review before merge, and
pause again for explicit approval before pushing the release tag. A request to
prepare a release authorizes preparation of the branch and pull request; it
does not by itself authorize merging or publishing.
