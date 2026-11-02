import { defineQueryOptions } from "@pinia/colada";
import { fetchStatus } from "~/services/instance.api";
import { keys } from "./keys";

/**
 * The server's health checks, last 24 hours and what's stored. The Status
 * page polls it while open; Run checks writes its own fresh answer in.
 */
export const statusQuery = defineQueryOptions({
  key: keys.status,
  query: ({ signal }) => fetchStatus(signal),
});
