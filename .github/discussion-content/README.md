# Dopbase community launch

This directory contains the eight launch posts, a publication manifest, and sanitized
CLI transcripts. The public discussion board is
<https://github.com/dopbase/dopbase/discussions>.

## Verification

The tutorials were replayed with the installed Dopbase 0.1.8 binary on macOS ARM64
on 2026-10-02. The lab used independent temporary server and client directories,
ports 18840 and 19000, and dummy values. Both servers stopped afterward. The
original workspace and personal Dopbase data were not used.

`verification.json` records the commands, output, and exit codes. It includes
login and guided-init transcripts with terminal redraws; the public posts show
only the final prompts and selected answers. Setup tokens, credentials, project
IDs, and local paths are redacted. Demo environment IDs and timestamps remain
visible so related outputs can be followed across commands.

The installer was read, but its execution was rejected by automatic approval
review. Its output block is explicitly an expected excerpt checked against the
public script. Linux and remote HTTPS connections were not replayed. The embedded
Admin UI returned HTTP 200 and the lab initialized accounts through the released
bootstrap API; browser setup instructions come from the versioned documentation.

## Complete the web setup

The authenticated account has Write access. The supported GitHub GraphQL API can
publish and edit discussions but does not expose category, section, or discussion
pin mutations. Browser-control tools were unavailable during this launch.

Open the discussion board and use the pencil next to Categories. Create the
following sections and configure these categories:

| Section                 | Category            | Format          | Description                                                              |
| ----------------------- | ------------------- | --------------- | ------------------------------------------------------------------------ |
| Project updates         | 📣 Announcements    | Announcement    | Releases, project updates, and community news from Dopbase maintainers.  |
| Learn and get help      | 🙋 Help & questions | Q&A             | Get help with installation, configuration, the CLI, and self-hosting.    |
| Learn and get help      | 📚 Tutorials        | Open discussion | Share practical Dopbase guides, examples, and walkthroughs.              |
| Share and shape Dopbase | 💡 Feature requests | Open discussion | Describe a problem and discuss what Dopbase could do better.             |
| Share and shape Dopbase | 💬 Feedback         | Open discussion | Tell us what works, what feels confusing, and what needs improvement.    |
| Share and shape Dopbase | 🛠 Showcase          | Open discussion | Share something you built or a workflow you use with Dopbase.            |
| Share and shape Dopbase | 👋 General          | Open discussion | Introductions, project conversations, and questions about the community. |

Keep Announcements and General. Rename Q&A to Help & questions, Ideas to Feature
requests, and Show and tell to Showcase. Add Tutorials and Feedback. Polls had no
posts when inspected; confirm it is still empty before deleting it.

The starter posts use existing categories until this step is complete: Tutorials
and Feedback use General, Feature requests uses Ideas, and Showcase uses Show and
tell. The existing welcome remains discussion #59. No discussion was deleted.

After renaming, inspect the actual category URLs. The help and feature forms use
the current `q-a` and `ideas` slugs. If those slugs change, rename the corresponding
form files under `.github/DISCUSSION_TEMPLATE/` to match. Update the issue chooser
and feature-request links to the actual slugs. The new form filenames already
match `tutorials` and `feedback`.

## Publication

Authenticate GitHub CLI with repository write access. Preview without changing
files or posts:

```bash
python3 .github/discussion-content/publish.py
```

After category setup, relocate the posts and refresh category links:

```bash
python3 .github/discussion-content/publish.py --apply
```

For an initial launch before category setup, explicitly allow the documented
mapping:

```bash
python3 .github/discussion-content/publish.py --apply --allow-existing-categories
```

The helper reuses discussion #59 and the IDs in `manifest.json`, resolves links,
and saves after each creation so an interrupted run can resume. Edit the Markdown
files to update post bodies. The manifest records desired and actual categories.

## Pins and form checks

Globally pin the welcome, installation tutorial, and feature-request starter.
Within Tutorials, pin the configuration tutorial. Within Feedback, pin its
starter prompt. Set each post's `pin_verified` value in the manifest only after
checking the web interface.

Preview each of the four category forms in GitHub. Help uses Q&A and should
support accepted answers. Tutorials accepts community posts and requires the
version tested, command/output pairs, verification, and cleanup. Forms must be
on the default branch to appear. Announcements keeps its maintainer posting
restriction.

Run `bun run test:github`, `bun run test:docs`, and `bun run build:docs` for changes
to these assets. Check Markdown links and YAML schema before publishing.

## First month

Review discussions on Tuesdays and Fridays. Request missing details, answer
questions, link related threads, and mark accepted answers once the problem is
resolved. Publish the next guide only after replaying its commands and capturing
its output.

| Target date | Follow-up                                                                                                |
| ----------- | -------------------------------------------------------------------------------------------------------- |
| 2026-10-09  | Clone development into staging; show listings and prove later changes stay separate.                     |
| 2026-10-16  | Create an expiring runner token; redact it and show allowed access plus an access-denied case.           |
| 2026-10-23  | Create an encrypted backup and verify restoration into a separate instance with its matching master key. |
| 2026-10-30  | Review unanswered questions, response times, participation, and recurring setup problems.                |

These dates are a maintainer checklist, not scheduled publications. Choose later
tutorials from the questions people actually ask. Keep release announcements
linked to real releases.
