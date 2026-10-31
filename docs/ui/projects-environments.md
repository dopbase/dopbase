---
title: "Admin UI: projects and environments"
description: "Browse projects and environments in the Dopbase Admin UI, switch between them, run an app with an environment's values, and manage tokens, access, activity and settings."
---

# Projects and environments

Projects is the main screen of the Admin UI. It shows the model described
in [projects, environments, and secrets](/guide/projects-environments-secrets)
as a tree: each project is a folder, and its environments sit inside it.

You see only the projects and environments you have access to, and only the
buttons your access allows. Someone who reads names only, for example, sees the
secrets list without **Reveal**, **Edit value**, or **Add secret**. See
[project access](./project-access) for how access is given.

## The project tree

Open a project to list its environments. Each row shows whether the
environment is protected, your level there, and when it last changed. Click a
row to open its secrets. A project with more than six environments lists six
and a **Show all** link. Type in the filter box to find a project or an
environment by name; a match inside a project opens it.

- **Create a project** with **New project**, if your role can create
  projects. Names are unique on the server, so pick the application's name:
  `payment-service`, not `backend`. You become the new project's project
  manager. The empty screen also shows the matching
  `dopbase project create` command.
- The **⋯** menu on a project you manage has **New environment**,
  **Access** (the project's [Access](./project-access) page), **Rename** and
  **Delete project**.
- Renaming a project or an environment breaks links that used the old name,
  because names are part of the address.
- Deleting shows what goes with it before you confirm: every environment and
  secret for a project, the secret and runner token counts for an
  environment. Deletion cannot be undone.

Adding environments needs project manager on the project. Renaming, deleting,
and runner tokens need manage on the environment. A lock and an amber
**protected** chip mark a protected environment, which only a grant on the
environment itself or a project manager reaches.

## The project page

Opening an environment shows one bar across the top:

- **Where you are.** `payments / prod` is also the switcher. Click the project
  to change project, or the environment (or press **E**) to change
  environment. The environment menu shows your level in each one and
  highlights the one you're on. Project managers also get **New
  environment** at the bottom of it, so there's no trip to Settings to add
  one. On a phone both open as a sheet from the bottom of the screen, with
  rows big enough to tap and a filter when the list is long. Long names are
  shortened in the bar; hover one to read it in full.
- **Protection.** A protected environment gets an amber chip and an amber
  line across the top of the page. Nothing else in the console uses amber,
  so production never looks like dev.
- **Your access.** Your level here, so a missing button makes sense. On a
  phone it sits with Run on a row of its own under the switchers.
- **Run.** Opens how to run an app with this environment's values.

Below it are the tabs. Secrets, Tokens and Activity belong to the
environment you're on; Access and Settings to the whole project, and keep
the environment you last opened in the bar.

| Tab      | Shows                                              | Who sees it                                     |
| -------- | -------------------------------------------------- | ----------------------------------------------- |
| Secrets  | The environment's secrets                          | Everyone who reaches the environment            |
| Compare  | Keys across this and up to 3 other environments    | Everyone who reaches the environment            |
| Tokens   | Runner tokens for this environment                 | Manage on the environment                       |
| Access   | Who reaches the project ([more](./project-access)) | Project manager, or `access.report` to read     |
| Activity | The audit trail for this project                   | Everyone; you see what your audit access covers |
| Settings | Name, environments, danger zone                    | Project manager                                 |

A link to a tab you can't use opens Secrets instead.

Runner token names follow the same rule as environment names: lowercase
letters, numbers and single hyphens, starting with a letter, such as
`deploy-prod`. The dialog that shows a new token gives the
`export DOPBASE_TOKEN` line with the whole token, ready to paste. It's the
only time the token is shown.

### Run

**Run** gives the exact command to run an app with this environment's values,
for the **CLI**, **Docker**, or **GitHub Actions**. People sign in with
`dopbase login` and run by name (`dopbase run payments/prod -- npm start`).
Run `dopbase env default payments/prod` once and a plain
`dopbase run -- npm start` picks that environment. Docker and GitHub Actions
use the environment's immutable ID with a runner token in `DOPBASE_TOKEN`, so
renaming the environment never breaks a deploy. If you manage the environment,
it also names the runner token deploys used last, and when it expires.

### Compare

Compare lines up this environment's keys with up to three others in the
project. The environment open in the bar is the base: open `payments / prod`
and every column is read against prod, switch to staging and staging becomes
the base. Compare stays open as you switch, and the environment you left takes
the new base's place in the columns.

**Compare with** starts with one other environment and remembers your choice
for the project in this browser. **Add environment** adds more, up to three;
at three it goes away until you remove one with its ×.

Values never appear. Each cell says **Set** (with its version) or **Not set**
for the base, and **Same**, **Differs**, **Not set** or **Set** for the
others. The server compares values only where you have Read on both
environments. With **Names only** somewhere, that column says which keys
exist but not whether they match. **Differences** shows only keys missing
somewhere or different from the base, and **All keys** shows the rest too.

Compare doesn't show or change values that are already set, and a note above
the table says so. Seeing or editing a value happens on the **Secrets** tab,
where revealing asks for your password and is recorded in the audit log.

Click a **Not set** cell to fill it, where you can write. **Match prod** (or
whichever environment has the key) copies that value on the server, so nobody
sees it, and the audit log records it as `secret.copied`. **Use its own
value** lets you type one instead. Either way it lands as a new version that
History can roll back.

### Activity

Activity lists who did what in this environment: reveals, changes, tokens,
access. Remove the environment chip to see the whole project, or add an
**Action** or **Actor** filter. **Open in audit log** opens the full
[audit log](./audit-instance) with the same filters.

### Settings

- **General** renames the project. CLI paths and links use the name, so
  renaming breaks scripts that use the old one.
- **Environments** lists each one with a **Protected** switch, its secret
  count, and its active runner tokens. Its **⋯** menu renames, protects or
  deletes it. **New environment** adds one.
- **Danger zone** deletes the project, after saying how many environments,
  secrets, runner tokens and grants go with it. Audit events stay.

Deleting an environment lists its secrets, runner tokens and grants first,
and asks you to type its name.

## Addresses you can share

Selection lives in the URL, not in some hidden app state:

```text
/projects/payment-service/production/secrets
/projects/payment-service/production/tokens
/projects/payment-service/production/activity
/projects/payment-service/access
/projects/payment-service/settings
```

Bookmark a production environment, paste a link to a teammate, or put the URL
in a runbook. The link opens the same place later, unless someone renames the
project or the environment.

`/projects/payment-service` opens the project's first environment. Links
from before 0.2.0, such as `/projects/p/payment-service/e/env_482731`, still
work: Dopbase looks the environment up and moves you to its new address.

## Runner tokens

The **Tokens** tab lists the environment's runner tokens. A runner token lets
a deployed app read this environment's values and nothing else. For your own
scripts or an AI agent, use a personal token from your account menu instead.

- **Create** one with **New runner token**. Name it after the app or
  pipeline that uses it, such as `deploy-prod`, and pick when it expires:
  7 days, 30 days, 90 days, a year, or a custom number of hours or days up to
  three years. The dialog says what the token can read and until when.
- **Copy it** from the next dialog. Dopbase shows the plaintext once, with
  the `export DOPBASE_TOKEN=...` and `dopbase run` lines to use it. Choose
  **I've stored it** only after saving it somewhere safe.
- **Watch expiry** in the table. Tokens that expire within a week show it in
  amber; expired and revoked tokens are greyed out.
- Each row's **⋯** menu copies the name, opens Activity, or revokes the
  token. Revocation is immediate.

A runner token can read and export the values of its environment. Use
`dopbase run` to inject them or `dopbase export` to write a file. It cannot
change secrets or reach another environment. Give each deployed workload its
own token. See [identity and access](/reference/identity) for the full rules.
