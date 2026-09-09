import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  extractChangelog,
  findPreviousVersion,
  parseChangelogRelease,
  renderReleaseNotes,
  validatePullRequestBody,
} from "./github-templates.mjs";

const pullRequestTemplate = readFileSync(
  new URL("../.github/pull_request_template.md", import.meta.url),
  "utf8",
);
const releaseTemplate = readFileSync(
  new URL("../.github/RELEASE_TEMPLATE.md", import.meta.url),
  "utf8",
);

function completedPullRequest() {
  return pullRequestTemplate
    .replace(
      "<!-- Explain what changed and why. Keep this focused on the problem this pull request solves. -->",
      "Add repository templates and validate them in CI.",
    )
    .replace(
      '<!-- List breaking changes, migration steps, or other compatibility effects. Write "No compatibility impact." when there are none. -->',
      "No compatibility impact.",
    )
    .replace(
      "<!-- List the commands or checks you ran. If a check was not run, say why. -->",
      "- `node --test scripts/github-templates.test.mjs`",
    )
    .replaceAll("- [ ]", "- [x]");
}

test("accepts a completed pull request template", () => {
  assert.deepEqual(validatePullRequestBody(completedPullRequest()), []);
});

test("rejects empty sections and unchecked checklist items", () => {
  const errors = validatePullRequestBody(pullRequestTemplate);
  assert(errors.includes("The Summary section needs an answer."));
  assert(errors.includes("The Compatibility section needs an answer."));
  assert(errors.includes("The Verification section needs an answer."));
  assert(
    errors.some((error) => error.startsWith("Check this item before merging:")),
  );
});

test("rejects a removed template marker", () => {
  const body = completedPullRequest().replace(
    "<!-- pr-template:compatibility -->",
    "",
  );
  assert(
    validatePullRequestBody(body).includes(
      "The Compatibility template marker must appear exactly once.",
    ),
  );
});

test("selects the greatest earlier semantic version", () => {
  assert.equal(
    findPreviousVersion("0.1.1", ["0.0.15", "notes", "0.1.0", "0.0.9"]),
    "0.1.0",
  );
  assert.equal(
    findPreviousVersion("1.0.0", ["0.9.9", "0.10.0", "0.2.10"]),
    "0.10.0",
  );
});

test("rejects a release without an earlier version", () => {
  assert.throws(
    () => findPreviousVersion("0.1.0", ["0.1.0", "draft"]),
    /No earlier semantic-version tag exists/,
  );
});

test("extracts only the requested changelog section", () => {
  const changelog = `# Changelog

## 0.2.0 - 2026-09-08

Short release summary.

### Added

- New release.

## 0.1.0 - 2026-09-01

- Earlier release.
`;
  assert.equal(
    extractChangelog(changelog, "0.2.0"),
    "## 0.2.0 - 2026-09-08\n\nShort release summary.\n\n### Added\n\n- New release.",
  );
});

test("renders the required fields and populated optional sections", () => {
  const changelog = `# Changelog

## 0.2.0 - 2026-09-08

Dopbase adds project import and improves login feedback.

### Added

- Added project import.

### Improvement

- Improved login feedback.

### Security

- Limited repeated sign-in attempts.

## 0.1.0 - 2026-09-01

Earlier release.
`;
  const notes = renderReleaseNotes({
    template: releaseTemplate,
    changelog,
    currentVersion: "0.2.0",
    previousVersion: "0.1.0",
    repository: "dopbase/dopbase",
  });

  assert(notes.startsWith("## Summary\n\nDopbase adds project import"));
  assert(notes.includes("## Added\n\n- Added project import."));
  assert(notes.includes("## Improvement\n\n- Improved login feedback."));
  assert(notes.includes("## Security\n\n- Limited repeated sign-in attempts."));
  assert(!notes.includes("## Fixed"));
  assert(!notes.includes("## Note"));
  assert.match(
    notes,
    /\*\*Full Changelog\*\*: https:\/\/github\.com\/dopbase\/dopbase\/compare\/0\.1\.0\.\.\.0\.2\.0/,
  );
  assert(!notes.includes("## 0.1.0 - 2026-09-01"));
  assert(!notes.includes("{{"));
});

test("allows a release with only the required fields", () => {
  const notes = renderReleaseNotes({
    template: releaseTemplate,
    changelog: "## 1.0.0 - 2026-09-08\n\nFirst stable release.\n",
    currentVersion: "1.0.0",
    previousVersion: "0.9.0",
    repository: "dopbase/dopbase",
  });

  assert(notes.includes("## Summary\n\nFirst stable release."));
  assert(!notes.includes("## Added"));
  assert(notes.includes("**Full Changelog**:"));
  assert(!notes.includes("\n\n\n"));
});

test("rejects missing summaries and unsupported release sections", () => {
  assert.throws(
    () =>
      parseChangelogRelease(
        "## 1.0.0 - 2026-09-08\n\n### Added\n\n- Feature.\n",
        "1.0.0",
      ),
    /needs a summary/,
  );
  assert.throws(
    () =>
      parseChangelogRelease(
        "## 1.0.0 - 2026-09-08\n\nRelease summary.\n\n### Changed\n\n- Change.\n",
        "1.0.0",
      ),
    /unsupported section Changed/,
  );
});

test("rejects empty and repeated optional release sections", () => {
  assert.throws(
    () =>
      parseChangelogRelease(
        "## 1.0.0 - 2026-09-08\n\nRelease summary.\n\n### Added\n",
        "1.0.0",
      ),
    /Added section is empty/,
  );
  assert.throws(
    () =>
      parseChangelogRelease(
        "## 1.0.0 - 2026-09-08\n\nRelease summary.\n\n### Note\n\n- First.\n\n### Note\n\n- Second.\n",
        "1.0.0",
      ),
    /repeats the Note section/,
  );
});
