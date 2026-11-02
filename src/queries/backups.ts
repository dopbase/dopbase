import { defineQueryOptions } from "@pinia/colada";
import { fetchBackups } from "~/services/backups.api";
import { keys } from "./keys";

/** The backups stored on the server, newest first. */
export const backupsQuery = defineQueryOptions({
  key: keys.backups,
  query: ({ signal }) => fetchBackups(signal),
});
