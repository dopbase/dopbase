import { onMounted, reactive, ref } from "vue";
import * as auditApi from "~/services/audit.api";
import type { AuditEvent } from "~/services/audit.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import type { Environment, Project } from "~/services";

export const AUDIT_PAGE_SIZE = 25;

/**
 * Audit screen controller: cursor-paginated event listing with filters for
 * action, project, environment, and actor. Changing any filter reloads
 * from the first page; "Load more" appends the next cursor page.
 */
export function useAuditController() {
  const items = ref<AuditEvent[]>([]);
  const nextCursor = ref<string | null>(null);
  const loading = ref(false);
  const loadingMore = ref(false);
  const loadError = ref<string | null>(null);
  const hasLoaded = ref(false);

  const filters = reactive({
    action: "",
    projectId: "",
    environmentId: "",
    actor: "",
  });

  const projects = ref<Project[]>([]);
  const environments = ref<Environment[]>([]);

  function currentQuery(): auditApi.AuditQuery {
    return {
      limit: AUDIT_PAGE_SIZE,
      action: filters.action || undefined,
      projectId: filters.projectId || undefined,
      environmentId: filters.environmentId || undefined,
      actor: filters.actor || undefined,
    };
  }

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      const page = await auditApi.listAuditEvents(currentQuery());
      items.value = page.items;
      nextCursor.value = page.nextCursor;
      hasLoaded.value = true;
    } catch {
      loadError.value = "Could not load audit events.";
    } finally {
      loading.value = false;
    }
  }

  async function loadMore(): Promise<void> {
    if (!nextCursor.value || loadingMore.value) return;
    loadingMore.value = true;
    try {
      const page = await auditApi.listAuditEvents({
        ...currentQuery(),
        cursor: nextCursor.value,
      });
      items.value.push(...page.items);
      nextCursor.value = page.nextCursor;
    } catch {
      loadError.value = "Could not load more audit events.";
    } finally {
      loadingMore.value = false;
    }
  }

  async function loadFilterOptions(): Promise<void> {
    try {
      const [projectList, environmentList] = await Promise.all([
        projectsApi.listProjects(),
        environmentsApi.listEnvironments(),
      ]);
      projects.value = projectList;
      environments.value = environmentList;
    } catch {
      // Filter dropdowns stay empty; the log itself still renders.
    }
  }

  onMounted(() => {
    load();
    loadFilterOptions();
  });

  return {
    items,
    nextCursor,
    loading,
    loadingMore,
    loadError,
    hasLoaded,
    filters,
    projects,
    environments,
    load,
    loadMore,
  };
}

export type AuditController = ReturnType<typeof useAuditController>;
