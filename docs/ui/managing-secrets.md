---
title: "Admin UI: manage secrets"
description: "Set, reveal, replace, and delete secrets in the Dopbase Admin UI. Values stay hidden until you explicitly reveal one."
---

# Managing secrets

The Secrets tab lists the secrets of one environment. The list shows key names
and metadata. Values stay hidden until you explicitly reveal one. Type in
**Filter keys** to narrow the list.

Every row has a **⋯** menu with **Reveal**, **Copy value**, **History**,
**Edit value** and, last and in red, **Delete**. You only see the items your
level allows.

## An empty environment

What an empty environment shows depends on the rest of the project and on
your level. It goes away once the first secret is saved.

In a new project, where no environment has secrets yet, anyone who can write
sees the setup timeline from **Projects** carried on: the project is done,
adding secrets is the current step, and running the app waits until there's a
secret. One box takes a `.env` file you drop or pick, opens the `.env` view
with **Paste .env**, or starts **Add secret**. Below it is the
`dopbase import` command for this environment, and the last step shows
`dopbase login` and `dopbase run`.

When another environment in the project already has secrets, the page offers
to start from it instead. **Copy from another environment** picks the one with
the most secrets, which you can change. **Keys only** is the default: every
key comes over with an empty value, so staging never ends up pointing at dev's
database by accident. **Keys and values** copies the values on the server and
shows only when you can read the source. **Review keys** lists the source's
keys with their versions, all ticked. Untick what doesn't belong and choose
**Copy**. Keys the environment already has are left alone. The same box for a
`.env` file and the import command sit underneath.

Someone who can see the environment but not write to it is told so and sent
to a project manager. If they can open the project's **Access** tab, **See who
has access** takes them there to find one.

## Set and delete

- **Set a secret** by key name. Dopbase encrypts the value on the server before
  it is stored. Setting an existing key replaces its value.
- **Delete** a secret from its row menu, after a confirmation.

Every change is an individual record. Replacing `STRIPE_SECRET_KEY` does not
touch `DATABASE_URL`, and each change lands in the [audit log](./audit-instance).

Deleting a secret also deletes its version history.

## History and rollback

Each save creates a new version. Choose **History** in a row's menu, or **See history**
in the update dialog, to list the versions of that secret, newest first. Each
row shows the change (created, updated, imported, or rolled back from an
earlier version), who made it, and when. The current version is marked
`current`. Versions past the
[retention limit](/guide/projects-environments-secrets#versions-and-rollback)
are marked `expired`: their value is no longer stored, so they cannot be
revealed or restored.

To check an old value, use the eye button on its row. It asks for your
password like any reveal, and hides the value again after thirty seconds.

**Roll back** saves the value of that version as a new version. The dialog
asks you to confirm first and shows the version number the result gets.
History is never changed, so you can roll back a rollback. If someone changed
the secret while the dialog was open, Dopbase refuses the rollback, reloads
the history, and asks you to check it and try again.

Apps get the restored value the next time they fetch secrets. Processes
already started with `dopbase run` keep the old value until they restart.

## Revealing a value

Click the dots in the Value column, or choose **Reveal** in the row menu.
Reveal shows the plaintext under the key, with a countdown starting at
thirty seconds. When the countdown ends, the value hides itself. Switching
environments or leaving the page hides it immediately too.

Copying the value does not extend the timer. The page keeps the revealed
plaintext only in memory and removes it when reloaded.

**Copy value** puts the value on your clipboard without showing it. It counts
as a reveal: it asks for your password when needed, and the audit log records
it. Some browsers block copying after the password prompt; reveal the value
and copy it from there instead.

## Password re-confirmation

Revealing, exporting, importing, and rolling back require a recent password
confirmation.
This includes import previews and saving changes from the `.env` editor. If
you have not entered your password in the last ten minutes, the server asks
for it first. After confirmation, the pending action continues. Cancelling
the prompt leaves your draft available to try again.

The server checks the window again when you apply an import. If it expires
after previewing, you must confirm your password again before saving. A
successful preview creates an audit event without changing secrets or layout.

## Editing a whole environment as `.env`

Switch the view from **Table** to **.env** to edit several values together. The editor loads every
secret in the environment as `.env` text:

```text
# Payment provider keys
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...

DATABASE_URL=postgres://...
```

The editor provides:

- Syntax highlighting with a line-number gutter, and malformed lines flagged
  both inline and in a list below the editor.
- Comments and blank lines are yours to keep. Dopbase remembers the layout
  (ordering and comments) separately from the values, so they survive reloads
  and deploys.
- A change summary before saving, grouped by added, updated, unchanged, and
  deleted keys. Dopbase stores the changes only after you accept the summary.

The editor content is wiped when you close it, switch environments, or leave
the page. Closing the browser tab counts.

Secrets set through the editor behave exactly like secrets set one at a time.
They are individual records, and the CLI sees them the same way.
