---
title: "Import and export"
description: "Import a .env file into a Dopbase environment and export values back out, with parsing done in your browser."
---

# Import and export

Most projects start with a `.env` file somewhere. Import brings it into
Dopbase; export gets values back out when something needs the file form.

## Importing a `.env` file

1. Open the environment that should receive the values and choose **Import**.
2. Pick the file. Parsing happens in your browser; nothing is uploaded yet.
   Lines that cannot be parsed are skipped and counted, with the reasons
   listed.
3. Review on the next page. It lists the key names the file contains. Values
   are never rendered, not even during review.
4. Choose a mode:
   - **Merge** adds new keys and updates existing ones. Keys already on the
     server but absent from the file are left alone. This is the default.
   - **Replace** makes the environment match the file exactly, including
     removing keys the file does not have.
5. **Validate** runs a dry run on the server. You get the result grouped into
   added, updated, unchanged, and deleted keys. Nothing is stored yet.
6. **Apply** performs the import. Review the deleted-keys group before
   continuing.

The review process has two constraints:

- Large files use a full review page instead of a popup.
- The parsed file lives in memory only. Reload the review page or navigate
  away and the browser discards it. Select the file again to restart. The
  server receives no changes until you apply the import.

## Exporting a `.env` file

Choose **Export** on an environment and Dopbase downloads a file named after
the project and environment:

```text
payment-service_production.env
```

Export requires a recent password confirmation, like reveal. The downloaded
file contains plaintext values, and nothing about the download encrypts it.
Treat the file the way you would treat any credential on disk: move it where
it belongs, then delete it.

## Which tool when

The CLI does the same jobs from a terminal:

```bash
dopbase import payment-service/staging .env.staging
dopbase export payment-service/staging --output .env.staging
```

The command reference covers the CLI flags, including `--dry-run`,
`--replace`, and `--stdout`. See [CLI commands](/cli/commands). Choose the
interface that fits the workflow. Both write the same records and create audit
events.
