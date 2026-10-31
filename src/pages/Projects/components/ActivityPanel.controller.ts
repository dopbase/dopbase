import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import type { RouteLocationRaw } from "vue-router";
import * as auditApi from "~/services/audit.api";
import type { AuditEvent } from "~/services/audit.api";
import type { Environment } from "~/services";
import { errorMessage } from "~/services/api-errors";
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
  const items = ref<AuditEvent[]>([]);
  const nextCursor = ref<string | null>(null);
  const loading = ref(false);
  const loadingMore = ref(false);
  const loadError = ref<string | null>(null);
  let version = 0;
  // One request at a time: a new filter cancels the page still loading.
  let request: AbortController | null = null;

  function nextRequest(): AbortController {
    request?.abort();
    request = new AbortController();
    return request;
  }

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

  async function load(): Promise<void> {
    if (!projectId.value) return;
    const current = ++version;
    const { signal } = nextRequest();
    loading.value = true;
    loadingMore.value = false;
    loadError.value = null;
    try {
      const page = await auditApi.listAuditEvents(query(), signal);
      if (current !== version) return;
      items.value = page.items;
      nextCursor.value = page.nextCursor;
    } catch (cause) {
      if (current !== version) return;
      items.value = [];
      nextCursor.value = null;
      loadError.value = errorMessage(cause, "Could not load the activity.");
    } finally {
      if (current === version) loading.value = false;
    }
  }

  async function loadMore(): Promise<void> {
    const cursor = nextCursor.value;
    if (!cursor || loadingMore.value) return;
    const current = version;
    const { signal } = nextRequest();
    loadingMore.value = true;
    try {
      const page = await auditApi.listAuditEvents(
        { ...query(), cursor },
        signal,
      );
      if (current !== version) return;
      items.value = [...items.value, ...page.items];
      nextCursor.value = page.nextCursor;
    } catch (cause) {
      if (current === version)
        loadError.value = errorMessage(cause, "Could not load more activity.");
    } finally {
      if (current === version) loadingMore.value = false;
    }
  }

  watch(
    [projectId, () => environment.value?.id, wholeProject, action, actor],
    load,
    { immediate: true },
  );
  onUnmounted(() => {
    version++;
    request?.abort();
  });

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
