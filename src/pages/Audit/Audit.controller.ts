import { computed, onMounted, reactive, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import * as auditApi from "~/services/audit.api";
import type { AuditEvent, ChainVerifyReport } from "~/services/audit.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import { fetchUsers } from "~/services/users.api";
import type { User } from "~/services/users.api";
import type { Environment, Project } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { useAuthStore } from "~/stores/auth.store";
import { formatDateTime, formatRelativeTime } from "~/utils/format";

export const AUDIT_PAGE_SIZE = 25;

/**
 * Every action the server records, by area. The list filter matches one
 * action exactly, so the Action chip offers these.
 */
export const AUDIT_ACTIONS: readonly string[] = [
  "secret.created",
  "secret.updated",
  "secret.deleted",
  "secret.revealed",
  "secret.version_revealed",
  "secret.rolled_back",
  "secret.imported",
  "secret.import_previewed",
  "secret.exported",
  "secret.runtime_accessed",
  "secret.copied",
  "project.created",
  "project.renamed",
  "project.deleted",
  "project.initialized",
  "environment.created",
  "environment.renamed",
  "environment.deleted",
  "environment.protected",
  "environment.unprotected",
  "token.created",
  "token.revoked",
  "personal_token.created",
  "personal_token.revoked",
  "login.succeeded",
  "login.failed",
  "logout.succeeded",
  "cli_login.approved",
  "cli_login.denied",
  "sso_provider.created",
  "sso_provider.updated",
  "sso_provider.verified",
  "sso_provider.enabled",
  "sso_provider.disabled",
  "sso_provider.deleted",
  "sso_provider.policy_changed",
  "sso_identity.linked",
  "sso_identity.unlinked",
  "sso_login.failed",
  "admin.bootstrapped",
  "admin.created",
  "admin.updated",
  "admin.deleted",
  "admin.reauthenticated",
  "admin.password_changed",
  "user.disabled",
  "user.enabled",
  "grant.set",
  "grant.removed",
  "group.created",
  "group.updated",
  "group.deleted",
  "group_member.added",
  "group_member.removed",
  "role.created",
  "role.updated",
  "role.deleted",
  "backup.created",
  "backup.downloaded",
  "backup.uploaded",
  "backup.restored",
  "backup.deleted",
  "master_key.downloaded",
  "backup.restored_bootstrap",
  "audit.pruned",
];

/** Which credential made a request, when it wasn't a signed-in session. */
const CREDENTIALS: Record<string, string> = {
  personal_token: "personal token",
  runner: "runner token",
};

export interface FilterOption {
  value: string;
  label: string;
}

/** The chain line under an event: whether the hash chain vouches for it. */
export interface ChainLine {
  tone: "ok" | "crit" | "neutral";
  text: string;
}

/** `9f2c…a71e`: enough of a hash to compare by eye. */
export function shortHash(hash: string): string {
  return hash.length > 12 ? `${hash.slice(0, 4)}…${hash.slice(-4)}` : hash;
}

/**
 * Audit screen controller: cursor-paginated events, filtered by Action,
 * Actor and Where. Filters live in the address (`?action=…&actor=…&
 * projectId=…&environmentId=…`), so other pages link straight to a
 * filtered log and a chip change can be shared. Opening an event shows
 * it in a drawer and checks the hash chain once per visit.
 */
export function useAuditController() {
  const auth = useAuthStore();
  const route = useRoute();
  const router = useRouter();

  const items = ref<AuditEvent[]>([]);
  const nextCursor = ref<string | null>(null);
  const loading = ref(false);
  const loadingMore = ref(false);
  const loadError = ref<string | null>(null);
  const hasLoaded = ref(false);

  const fromQuery = (key: string): string => {
    const value = route.query[key];
    return typeof value === "string" ? value : "";
  };
  const filters = reactive({
    action: fromQuery("action"),
    projectId: fromQuery("projectId"),
    environmentId: fromQuery("environmentId"),
    actor: fromQuery("actor"),
  });

  const projects = ref<Project[]>([]);
  const environments = ref<Environment[]>([]);
  const people = ref<User[]>([]);
  let requestVersion = 0;

  const projectName = (id: string | null): string =>
    projects.value.find((project) => project.id === id)?.name ?? id ?? "";
  const environmentName = (id: string | null): string => {
    if (!id) return "";
    const match = environments.value.find(
      (environment) => environment.id === id,
    );
    return match ? `${match.projectName}/${match.name}` : id;
  };
  /** `payments/prod`, `payments`, or empty for instance events. */
  function where(event: AuditEvent): string {
    if (event.environmentId) return environmentName(event.environmentId);
    if (event.projectId) return projectName(event.projectId);
    return "";
  }
  function actorName(event: AuditEvent): string {
    return event.actorLabel ?? event.actorId ?? event.actorType;
  }
  function credential(event: AuditEvent): string | null {
    return CREDENTIALS[event.actorType] ?? null;
  }

  // Filters

  const actionOptions = computed<FilterOption[]>(() => {
    const known = new Set(AUDIT_ACTIONS);
    const extra = [filters.action, ...items.value.map((item) => item.action)]
      .filter((action) => action && !known.has(action))
      .filter((action, index, all) => all.indexOf(action) === index);
    return [...AUDIT_ACTIONS, ...extra].map((action) => ({
      value: action,
      label: action,
    }));
  });

  /**
   * People you can see, then anyone in the loaded events, then the actor
   * in the address so a link from another page always shows its chip.
   */
  const actorOptions = computed<FilterOption[]>(() => {
    const options = new Map<string, string>();
    for (const person of people.value) options.set(person.id, person.email);
    for (const event of items.value) {
      if (event.actorId && !options.has(event.actorId))
        options.set(event.actorId, actorName(event));
    }
    if (filters.actor && !options.has(filters.actor))
      options.set(filters.actor, filters.actor);
    return [...options].map(([value, label]) => ({ value, label }));
  });

  const whereOptions = computed<FilterOption[]>(() => {
    const options: FilterOption[] = [];
    for (const project of projects.value) {
      options.push({ value: `project:${project.id}`, label: project.name });
      for (const environment of environments.value) {
        if (environment.projectId !== project.id) continue;
        options.push({
          value: `environment:${environment.id}`,
          label: `${project.name}/${environment.name}`,
        });
      }
    }
    const chosen = whereValue.value;
    if (chosen && !options.some((option) => option.value === chosen)) {
      options.push({
        value: chosen,
        label: filters.environmentId
          ? environmentName(filters.environmentId)
          : projectName(filters.projectId),
      });
    }
    return options;
  });

  const whereValue = computed<string | null>(() => {
    if (filters.environmentId) return `environment:${filters.environmentId}`;
    if (filters.projectId) return `project:${filters.projectId}`;
    return null;
  });

  function applyFilters(): void {
    const query: Record<string, string> = {};
    for (const [key, value] of Object.entries(filters)) {
      if (value) query[key] = value;
    }
    void router.replace({ query });
    void load();
  }

  function setAction(value: string | null): void {
    filters.action = value ?? "";
    applyFilters();
  }

  function setActor(value: string | null): void {
    filters.actor = value ?? "";
    applyFilters();
  }

  function setWhere(value: string | null): void {
    const [kind, id = ""] = (value ?? "").split(":");
    if (kind === "environment") {
      filters.environmentId = id;
      filters.projectId =
        environments.value.find((environment) => environment.id === id)
          ?.projectId ?? "";
    } else {
      filters.environmentId = "";
      filters.projectId = kind === "project" ? id : "";
    }
    applyFilters();
  }

  // Loading

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
      // A newer filter already replaced this request.
      if (version === requestVersion)
        loadError.value = "Could not load audit events.";
    } finally {
      if (version === requestVersion) loading.value = false;
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
      if (version === requestVersion)
        loadError.value = "Could not load more audit events.";
    } finally {
      loadingMore.value = false;
    }
  }

  async function loadFilterOptions(): Promise<void> {
    const [projectList, environmentList, accounts] = await Promise.all([
      projectsApi.listProjects().catch(() => [] as Project[]),
      environmentsApi.listEnvironments().catch(() => [] as Environment[]),
      auth.can("users.view")
        ? fetchUsers().catch(() => [] as User[])
        : Promise.resolve([] as User[]),
    ]);
    projects.value = projectList;
    environments.value = environmentList;
    people.value = accounts;
  }

  /** What the viewer's audit capabilities let this screen show. */
  const scopeDescription = computed(() => {
    if (auth.can("audit.read_all")) {
      return "Every event on this server. Values never appear here.";
    }
    if (auth.can("audit.read")) {
      return "Events on this server's settings and on the projects you reach. Values never appear here.";
    }
    return "Your own actions on this server. Values never appear here.";
  });

  // Chain verification

  const canVerify = computed(() => auth.can("audit.verify"));
  const chain = ref<ChainVerifyReport | null>(null);
  const verifying = ref(false);
  const verifyError = ref<string | null>(null);

  async function verifyChain(): Promise<void> {
    if (!canVerify.value || verifying.value) return;
    verifying.value = true;
    verifyError.value = null;
    try {
      chain.value = await auditApi.verifyAuditChain();
    } catch (cause) {
      verifyError.value = errorMessage(cause, "Could not check the chain.");
    } finally {
      verifying.value = false;
    }
  }

  /** The whole log's result, for the alert under the header. */
  const chainSummary = computed<ChainLine | null>(() => {
    const report = chain.value;
    if (!report) return null;
    if (report.broken) {
      return {
        tone: "crit",
        text: `The chain breaks at #${report.broken.seq}: ${report.broken.reason}`,
      };
    }
    if (!report.ok) {
      return {
        tone: "crit",
        text: `${report.unsequenced} ${report.unsequenced === 1 ? "record was" : "records were"} written outside the chain.`,
      };
    }
    const head = report.headSeq === null ? "" : ` up to #${report.headSeq}`;
    return {
      tone: "ok",
      text: `Chain verified${head}. ${report.records} ${report.records === 1 ? "record" : "records"} checked.`,
    };
  });

  /** Whether the chain vouches for one event. */
  function chainLine(event: AuditEvent): ChainLine | null {
    if (!canVerify.value) return null;
    if (verifyError.value) return { tone: "crit", text: verifyError.value };
    const report = chain.value;
    if (!report) {
      return verifying.value
        ? { tone: "neutral", text: "Checking the chain…" }
        : null;
    }
    if (event.seq === null || event.seq === undefined) {
      return {
        tone: "neutral",
        text: "This event is older than the hash chain, so the chain can't vouch for it.",
      };
    }
    if (report.broken && report.broken.seq <= event.seq) {
      return {
        tone: "crit",
        text: `Chain broken at #${report.broken.seq}: ${report.broken.reason}`,
      };
    }
    if (!report.broken && !report.ok) {
      return {
        tone: "crit",
        text: "The chain check failed. Records were written outside it.",
      };
    }
    const upTo = report.broken ? report.broken.seq - 1 : report.headSeq;
    const hash = event.recordHash
      ? ` · sha256 ${shortHash(event.recordHash)}`
      : "";
    return { tone: "ok", text: `Chain verified up to #${upTo}${hash}` };
  }

  // The event drawer

  const selected = ref<AuditEvent | null>(null);

  function openEvent(event: AuditEvent): void {
    selected.value = event;
    if (!chain.value && !verifying.value) void verifyChain();
  }

  function closeEvent(): void {
    selected.value = null;
  }

  /** What the drawer's code block shows. Records never hold values. */
  function eventPayload(event: AuditEvent): string {
    const payload: Record<string, unknown> = {
      id: event.id,
      action: event.action,
      actorType: event.actorType,
      actor: event.actorId,
    };
    if (event.projectId) payload.project = event.projectId;
    if (event.environmentId) payload.environment = event.environmentId;
    if (event.resourceType) payload.resourceType = event.resourceType;
    if (event.resourceId) payload.resource = event.resourceId;
    if (Object.keys(event.metadata).length > 0)
      payload.metadata = event.metadata;
    payload.createdAt = event.createdAt;
    return JSON.stringify(payload, null, 2);
  }

  /** `203.0.113.24 · dopbase-cli 0.2.0`, or empty without an address. */
  function origin(event: AuditEvent): string {
    const client = event.metadata.client;
    return [event.ipAddress, typeof client === "string" ? client : null]
      .filter(Boolean)
      .join(" · ");
  }

  onMounted(() => {
    void load();
    void loadFilterOptions();
  });

  return {
    items,
    nextCursor,
    loading,
    loadingMore,
    loadError,
    hasLoaded,
    scopeDescription,
    filters,
    projects,
    environments,
    actionOptions,
    actorOptions,
    whereOptions,
    whereValue,
    setAction,
    setActor,
    setWhere,
    projectName,
    environmentName,
    where,
    actorName,
    credential,
    formatDateTime,
    formatRelativeTime,
    load,
    loadMore,
    loadFilterOptions,
    canVerify,
    chain,
    verifying,
    verifyError,
    verifyChain,
    chainSummary,
    chainLine,
    selected,
    openEvent,
    closeEvent,
    eventPayload,
    origin,
  };
}

export type AuditController = ReturnType<typeof useAuditController>;
