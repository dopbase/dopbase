import { computed, ref, watch, type Ref } from "vue";
import type { RouteLocationRaw } from "vue-router";
import { useInfiniteQuery } from "@pinia/colada";
import type * as auditApi from "~/services/audit.api";
import type { AuditEvent } from "~/services/audit.api";
import type { Environment } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { auditEventsQuery, type AuditFilters } from "~/queries/audit";
import { loadFailure, onlyWhen } from "~/queries/helpers";
import { formatRelativeTime } from "~/utils/format";

export const ACTIVITY_PAGE_SIZE = 25;

export interface ActivityRow {
  event: AuditEvent;
  time: string;
  actor: string;
  /** The environment's name, or null for project-wide events. */
  where: string | null;
  whereProtected: boolean;
  details: string;
}

/** The most telling thing about an event, for the Details column. */
function describe(event: AuditEvent): string {
  const metadata = event.metadata ?? {};
  for (const key of ["key", "name", "email", "label"]) {
    const value = metadata[key];
    if (typeof value === "string" && value) return value;
  }
  return event.resourceId ?? "";
}

/**
 * Activity panel controller: the audit log for this project, narrowed to
 * the current environment unless you widen it, plus optional action and
 * actor filters. Pages load 25 events at a time.
 *
 * What it lists still depends on your role: the server only returns
 * events your audit access covers.
 */
export function useActivityPanelController(
  projectId: Ref<string | null>,
  environment: Ref<Environment | null>,
  environments: Ref<Environment[]>,
) {
  const wholeProject = ref(false);
  const action = ref("");
  const actor = ref("");

  const filters = computed<AuditFilters>(() => {
    const current = query();
    return {
      limit: ACTIVITY_PAGE_SIZE,
      projectId: current.projectId ?? null,
      environmentId: current.environmentId ?? null,
      action: current.action ?? null,
      actor: current.actor ?? null,
    };
  });
  // Keyed by filters: a new filter is its own feed, so a page still loading
  // for the old one is dropped and Load more always extends what's shown.
  const feed = useInfiniteQuery(() =>
    onlyWhen(projectId.value !== null, auditEventsQuery(filters.value)),
  );
  const loadingMore = ref(false);
  const moreError = ref<string | null>(null);
  const feedFailure = loadFailure(
    feed,
    (cause) => errorMessage(cause, "Could not load the activity."),
    { withData: true },
  );
  const loadError = computed(() => feedFailure.value ?? moreError.value);
  const loading = computed(() => feed.isLoading.value && !loadingMore.value);
  const pages = computed(() =>
    feedFailure.value ? [] : (feed.data.value?.pages ?? []),
  );
  const items = computed<AuditEvent[]>(() =>
    pages.value.flatMap((page) => page?.items ?? []),
  );
  const nextCursor = computed(() => pages.value.at(-1)?.nextCursor ?? null);

  watch(filters, () => {
    moreError.value = null;
  });

  function query(): auditApi.AuditQuery {
    return {
      limit: ACTIVITY_PAGE_SIZE,
      projectId: projectId.value ?? undefined,
      environmentId:
        wholeProject.value || !environment.value
          ? undefined
          : environment.value.id,
      action: action.value.trim() || undefined,
      actor: actor.value.trim() || undefined,
    };
  }

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    if (!projectId.value) return;
    moreError.value = null;
    await feed.refetch();
  }

  async function loadMore(): Promise<void> {
    if (!nextCursor.value || loadingMore.value) return;
    const shown = filters.value;
    loadingMore.value = true;
    moreError.value = null;
    try {
      await feed.loadNextPage({ throwOnError: true });
    } catch (cause) {
      // A filter change meanwhile started another feed; this page is moot.
      if (filters.value === shown)
        moreError.value =
          errorMessage(cause, "Could not load more activity.") || null;
    } finally {
      loadingMore.value = false;
    }
  }

  const rows = computed<ActivityRow[]>(() =>
    items.value.map((event) => {
      const where = environments.value.find(
        (candidate) => candidate.id === event.environmentId,
      );
      return {
        event,
        time: formatRelativeTime(event.createdAt),
        actor: event.actorLabel ?? event.actorType,
        where: where?.name ?? null,
        whereProtected: where?.protected ?? false,
        details: describe(event),
      };
    }),
  );

  /** The same filters on the full audit log page. */
  const auditLink = computed<RouteLocationRaw>(() => {
    const current = query();
    const params: Record<string, string> = {};
    for (const key of [
      "projectId",
      "environmentId",
      "action",
      "actor",
    ] as const) {
      const value = current[key];
      if (value) params[key] = value;
    }
    return { name: "audit", query: params };
  });

  return {
    wholeProject,
    action,
    actor,
    rows,
    loading,
    loadingMore,
    loadError,
    nextCursor,
    load,
    loadMore,
    auditLink,
  };
}
