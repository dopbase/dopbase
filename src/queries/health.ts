import { defineQueryOptions } from "@pinia/colada";
import { fetchHealth } from "~/services/health.api";
import { keys } from "./keys";

/**
 * The server's version and status, shown before anyone signs in. Dropped as
 * soon as no screen shows it.
 */
export const publicHealthQuery = defineQueryOptions({
  key: keys.public.health,
  query: ({ signal }) => fetchHealth(signal),
  gcTime: 0,
});
