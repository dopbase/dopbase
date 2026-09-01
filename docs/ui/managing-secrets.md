---
title: "Managing secrets"
description: "Set, reveal, replace, and delete secrets in the Dopbase Admin UI. Values stay hidden until you explicitly reveal one."
---

# Managing secrets

The Secrets tab lists the secrets of one environment. The list shows key names
and metadata. Values stay hidden until you explicitly reveal one.

## Set and delete

- **Set a secret** by key name. Dopbase encrypts the value on the server before
  it is stored. Setting an existing key replaces its value.
- **Delete** a secret with its row action, after a confirmation.

Every change is an individual record. Replacing `STRIPE_SECRET_KEY` does not
touch `DATABASE_URL`, and each change lands in the [audit log](./audit-instance).

## Revealing a value

Reveal shows the plaintext next to the key, with a countdown starting at
thirty seconds. When the countdown ends, the value hides itself. Switching
environments or leaving the page hides it immediately too.

Copying the value does not extend the timer. The page keeps the revealed
plaintext only in memory and removes it when reloaded.

## Password re-confirmation

Revealing and exporting require a recent password confirmation. If you have
not entered your password in the last ten minutes, the server asks for it
first. After the dialog confirms your password, the pending reveal or export
continues.

The ten-minute window applies to reveal, export, and the `.env` editor. After
it expires, the next sensitive action asks once more.

## Editing a whole environment as `.env`

Use **Edit as .env** to edit several values together. The editor loads every
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
