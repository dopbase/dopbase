import { defineInfiniteQueryOptions } from "@pinia/colada";
import { listAuditEvents, type AuditPage } from "~/services/audit.api";
import { keys } from "./keys";

/** What an audit feed is narrowed to; `null` leaves a filter off. */
export interface AuditFilters {
  limit: number;
  projectId: string | null;
  environmentId: string | null;
  action: string | null;
  actor: string | null;
}

/**
 * The audit log, newest first, a page at a time. Every write refreshes it.
 *
 * It doesn't refresh on window focus: that would refetch every page already
 * loaded, one after another.
 */
export const auditEventsQuery = defineInfiniteQueryOptions(
  (filters: AuditFilters) => ({
    key: keys.audit.events({ ...filters }),
    initialPageParam: null as string | null,
    // A parked feed has no pages to read a cursor from.
    getNextPageParam: (page: AuditPage | undefined) => page?.nextCursor ?? null,
    query: ({ signal, pageParam }) =>
      listAuditEvents(
        {
          limit: filters.limit,
          projectId: filters.projectId ?? undefined,
          environmentId: filters.environmentId ?? undefined,
          action: filters.action ?? undefined,
          actor: filters.actor ?? undefined,
          ...(pageParam ? { cursor: pageParam } : {}),
        },
        signal,
      ),
    refetchOnWindowFocus: false,
  }),
);
