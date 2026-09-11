import { computed, onMounted, reactive, ref } from "vue";
import * as auditApi from "~/services/audit.api";
import type { AuditEvent } from "~/services/audit.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import type { Environment, Project } from "~/services";
import { formatDateTime, formatRelativeTime } from "~/utils/format";

export const AUDIT_PAGE_SIZE = 25;

/**
 * Audit screen controller: cursor-paginated event listing with filters for
 * action, project, environment, and actor. Changing any filter reloads
 * from the first page. "Load more" appends the next cursor page.
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
  let requestVersion = 0;

  const projectOptions = computed(() => [
    { label: "All projects", value: "" },
    ...projects.value.map((project) => ({
      label: project.name,
      value: project.id,
    })),
  ]);
  const environmentOptions = computed(() => [
    { label: "All environments", value: "" },
    ...environments.value.map((environment) => ({
      label: `${environment.projectName}/${environment.name}`,
      value: environment.id,
    })),
  ]);
  const projectName = (id: string | null): string =>
    projects.value.find((project) => project.id === id)?.name ?? id ?? "—";
  const environmentName = (id: string | null): string => {
    if (!id) return "—";
    const match = environments.value.find(
      (environment) => environment.id === id,
    );
    return match ? `${match.projectName}/${match.name}` : id;
  };

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
    const version = ++requestVersion;
    const query = currentQuery();
    loading.value = true;
    loadError.value = null;
    try {
      const page = await auditApi.listAuditEvents(query);
      if (version !== requestVersion) return;
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
    const version = requestVersion;
    const cursor = nextCursor.value;
    const query = currentQuery();
    loadingMore.value = true;
    try {
      const page = await auditApi.listAuditEvents({
        ...query,
        cursor,
      });
      if (version !== requestVersion || nextCursor.value !== cursor) return;
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
      // Filter dropdowns stay empty while the log itself still renders.
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
    projectOptions,
    environmentOptions,
    projectName,
    environmentName,
    formatDateTime,
    formatRelativeTime,
    load,
    loadMore,
  };
}

export type AuditController = ReturnType<typeof useAuditController>;
