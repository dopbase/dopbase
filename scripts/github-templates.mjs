import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const SECTION_DEFINITIONS = [
  { marker: "<!-- pr-template:summary -->", heading: "Summary" },
  { marker: "<!-- pr-template:compatibility -->", heading: "Compatibility" },
  { marker: "<!-- pr-template:verification -->", heading: "Verification" },
  { marker: "<!-- pr-template:checklist -->", heading: "Checklist" },
];

const RELEASE_SECTION_HEADINGS = [
  "Added",
  "Improvement",
  "Fixed",
  "Security",
  "Note",
];

export const REQUIRED_CHECKLIST_ITEMS = [
  "I kept this pull request focused on one problem.",
  "I added or updated tests for behavior changes, or explained why tests are not needed.",
  "I updated the documentation and `CHANGELOG.md` when public behavior changed, or explained why they are not needed.",
];

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function stripHtmlComments(value) {
  let previous;
  let current = value;
  do {
    previous = current;
    current = current.replace(/<!--[\s\S]*?-->/g, "");
  } while (current !== previous);
  return current;
}

function occurrenceCount(value, search) {
  return value.split(search).length - 1;
}

export function validatePullRequestBody(body) {
  const errors = [];
  const locations = [];
  let previousLocation = -1;

  if (body.trim() === "") {
    return ["The pull request body is empty. Use the repository template."];
  }

  for (const section of SECTION_DEFINITIONS) {
    const count = occurrenceCount(body, section.marker);
    if (count !== 1) {
      errors.push(
        `The ${section.heading} template marker must appear exactly once.`,
      );
      locations.push(-1);
      continue;
    }

    const location = body.indexOf(section.marker);
    locations.push(location);
    if (location < previousLocation) {
      errors.push(`The ${section.heading} section is out of order.`);
    }
    previousLocation = location;
  }

  if (locations.some((location) => location === -1)) return errors;

  for (let index = 0; index < SECTION_DEFINITIONS.length; index += 1) {
    const section = SECTION_DEFINITIONS[index];
    const start = locations[index] + section.marker.length;
    const end = locations[index + 1] ?? body.length;
    const content = body.slice(start, end);
    const headingPattern = new RegExp(
      `^##\\s+${escapeRegex(section.heading)}\\s*$`,
      "m",
    );

    if (!headingPattern.test(content)) {
      errors.push(`The ${section.heading} heading is missing.`);
      continue;
    }

    if (section.heading === "Checklist") continue;

    const visibleContent = stripHtmlComments(content)
      .replace(headingPattern, "")
      .trim();
    if (!/[A-Za-z0-9]/.test(visibleContent)) {
      errors.push(`The ${section.heading} section needs an answer.`);
    }
  }

  const checklistStart = locations.at(-1);
  const checklist = body.slice(checklistStart);
  for (const item of REQUIRED_CHECKLIST_ITEMS) {
    const checkedItem = new RegExp(
      `^-\\s*\\[[xX]\\]\\s*${escapeRegex(item)}\\s*$`,
      "m",
    );
    if (!checkedItem.test(checklist)) {
      errors.push(`Check this item before merging: ${item}`);
    }
  }

  return errors;
}

function parseVersion(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(value);
  if (!match) return null;
  return match.slice(1).map((component) => BigInt(component));
}

function compareVersions(left, right) {
  for (let index = 0; index < 3; index += 1) {
    if (left[index] < right[index]) return -1;
    if (left[index] > right[index]) return 1;
  }
  return 0;
}

export function findPreviousVersion(currentVersion, tags) {
  const current = parseVersion(currentVersion);
  if (!current) {
    throw new Error(
      `Release version ${currentVersion} must match MAJOR.MINOR.PATCH.`,
    );
  }

  const candidates = tags
    .map((tag) => ({ tag: tag.trim(), parsed: parseVersion(tag.trim()) }))
    .filter(
      (candidate) =>
        candidate.parsed && compareVersions(candidate.parsed, current) < 0,
    )
    .sort((left, right) => compareVersions(right.parsed, left.parsed));

  if (candidates.length === 0) {
    throw new Error(
      `No earlier semantic-version tag exists before ${currentVersion}.`,
    );
  }
  return candidates[0].tag;
}

export function extractChangelog(changelog, version) {
  if (!parseVersion(version)) {
    throw new Error(`Release version ${version} must match MAJOR.MINOR.PATCH.`);
  }

  const lines = changelog.replaceAll("\r\n", "\n").split("\n");
  const headingPattern = new RegExp(
    `^## ${escapeRegex(version)} - \\d{4}-\\d{2}-\\d{2}$`,
  );
  const matches = lines
    .map((line, index) => (headingPattern.test(line) ? index : -1))
    .filter((index) => index !== -1);

  if (matches.length !== 1) {
    throw new Error(
      `CHANGELOG.md must contain exactly one dated section for ${version}.`,
    );
  }

  const start = matches[0];
  const nextHeading = lines.findIndex(
    (line, index) => index > start && line.startsWith("## "),
  );
  const end = nextHeading === -1 ? lines.length : nextHeading;
  const section = lines.slice(start, end).join("\n").trim();
  const notes = lines
    .slice(start + 1, end)
    .join("\n")
    .trim();
  if (notes === "") {
    throw new Error(`The ${version} changelog section has no release notes.`);
  }
  return section;
}

export function parseChangelogRelease(changelog, version) {
  const section = extractChangelog(changelog, version);
  const lines = section.split("\n").slice(1);
  const headings = lines
    .map((line, index) => {
      const match = /^### (.+)$/.exec(line);
      return match ? { title: match[1], index } : null;
    })
    .filter(Boolean);
  const summaryEnd = headings[0]?.index ?? lines.length;
  const summary = lines.slice(0, summaryEnd).join("\n").trim();

  if (summary === "") {
    throw new Error(`The ${version} changelog section needs a summary.`);
  }
  if (/\n\s*\n/.test(summary)) {
    throw new Error(`The ${version} release summary must be one paragraph.`);
  }
  if (summary.replace(/\s+/g, " ").length > 500) {
    throw new Error(
      `The ${version} release summary must be 500 characters or fewer.`,
    );
  }

  const releaseSections = new Map();
  for (let index = 0; index < headings.length; index += 1) {
    const heading = headings[index];
    if (!RELEASE_SECTION_HEADINGS.includes(heading.title)) {
      throw new Error(
        `The ${version} changelog uses unsupported section ${heading.title}.`,
      );
    }
    if (releaseSections.has(heading.title)) {
      throw new Error(
        `The ${version} changelog repeats the ${heading.title} section.`,
      );
    }

    const end = headings[index + 1]?.index ?? lines.length;
    const content = lines
      .slice(heading.index + 1, end)
      .join("\n")
      .trim();
    if (content === "") {
      throw new Error(
        `The ${version} changelog ${heading.title} section is empty.`,
      );
    }
    releaseSections.set(heading.title, content);
  }

  return { summary, sections: releaseSections };
}

export function renderReleaseNotes({
  template,
  changelog,
  currentVersion,
  previousVersion,
  repository,
}) {
  if (!parseVersion(previousVersion)) {
    throw new Error(
      `Previous version ${previousVersion} must match MAJOR.MINOR.PATCH.`,
    );
  }
  if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error(`Repository ${repository} must use the owner/name form.`);
  }

  const release = parseChangelogRelease(changelog, currentVersion);
  const optionalSections = RELEASE_SECTION_HEADINGS.filter((heading) =>
    release.sections.has(heading),
  )
    .map((heading) => `## ${heading}\n\n${release.sections.get(heading)}`)
    .join("\n\n");
  const replacements = new Map([
    ["{{SUMMARY}}", release.summary],
    [
      "{{OPTIONAL_SECTIONS}}",
      optionalSections === "" ? "" : `${optionalSections}\n\n`,
    ],
    ["{{PREVIOUS_VERSION}}", previousVersion],
    ["{{VERSION}}", currentVersion],
    ["{{REPOSITORY}}", repository],
  ]);
  let rendered = template;
  for (const [placeholder, value] of replacements) {
    if (!rendered.includes(placeholder)) {
      throw new Error(`Release template is missing ${placeholder}.`);
    }
    rendered = rendered.replaceAll(placeholder, value);
  }
  if (/{{[A-Z_]+}}/.test(rendered)) {
    throw new Error(
      "Release notes contain an unresolved template placeholder.",
    );
  }
  return `${rendered.trim()}\n`;
}

function option(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) {
    throw new Error(`Missing required option ${name}.`);
  }
  return process.argv[index + 1];
}

function main() {
  const command = process.argv[2];
  if (command === "validate-pr") {
    const errors = validatePullRequestBody(process.env.PR_BODY ?? "");
    if (errors.length > 0) {
      console.error("Pull request template validation failed:");
      for (const error of errors) console.error(`- ${error}`);
      process.exitCode = 1;
    } else {
      console.log("Pull request body follows the repository template.");
    }
    return;
  }

  if (command === "previous-version") {
    const currentVersion = option("--version");
    const commit = option("--commit");
    const tags = execFileSync("git", ["tag", "--merged", `${commit}^`], {
      encoding: "utf8",
    }).split("\n");
    console.log(findPreviousVersion(currentVersion, tags));
    return;
  }

  if (command === "render-release") {
    const currentVersion = option("--version");
    const previousVersion = option("--previous-version");
    const repository = option("--repository");
    const output = option("--output");
    const rendered = renderReleaseNotes({
      template: readFileSync(".github/RELEASE_TEMPLATE.md", "utf8"),
      changelog: readFileSync("CHANGELOG.md", "utf8"),
      currentVersion,
      previousVersion,
      repository,
    });
    writeFileSync(output, rendered);
    return;
  }

  throw new Error(
    "Usage: github-templates.mjs <validate-pr|previous-version|render-release>",
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
