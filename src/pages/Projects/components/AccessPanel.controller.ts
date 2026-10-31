import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  fetchGroups,
  fetchProjectAccess,
  removeGrant,
  setGrant,
} from "~/services/access.api";
import type {
  Grant,
  Level,
  PrincipalType,
  ProjectAccess,
} from "~/services/access.api";
import { fetchUsers } from "~/services/users.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import { formatLevel } from "~/utils/format";
import {
  ENVIRONMENT_LEVELS,
  grantSentence,
  LEVEL_HINTS,
  levelMeter,
  PROJECT_LEVELS,
  type LevelMeter,
} from "~/utils/level";

/** Someone a grant can be given to. */
export interface Principal {
  type: PrincipalType;
  id: string;
  label: string;
}

// The level lists and wording are shared with the people pages.
export { ENVIRONMENT_LEVELS, LEVEL_HINTS, PROJECT_LEVELS };

export interface AccessRow {
  grant: Grant;
  isGroup: boolean;
  /** "Whole project", or the environment's name. */
  where: string;
  whereProtected: boolean;
  meter: LevelMeter;
}

export type AccessDialog = { mode: "give" } | { mode: "change"; grant: Grant };

/** The value of the scope picker that means the whole project. */
export const WHOLE_PROJECT = "";

/**
 * Access panel controller: every grant on the project and its
 * environments, and the Give access / Change level dialog. Changes run
 * through the shared password confirmation, since the server wants a
 * recent sign-in. Protection lives on the Settings tab.
 */
export function useAccessPanelController() {
  const route = useRoute();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const access = ref<ProjectAccess | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const principals = ref<Principal[]>([]);
  let request: AbortController | null = null;

  const projectRef = computed(() =>
    typeof route.params.projectRef === "string" ? route.params.projectRef : "",
  );
  /** Only project managers change access; access.report can only read it. */
  const canManage = computed(
    () => access.value?.project.level === "project_manager",
  );
  const filter = ref("");

  /** Project grants first, then each environment's, by environment name. */
  const grants = computed<Grant[]>(() =>
    [...(access.value?.grants ?? [])].sort(
      (a, b) =>
        Number(a.scopeType === "environment") -
          Number(b.scopeType === "environment") ||
        a.scopeLabel.localeCompare(b.scopeLabel) ||
        a.principalLabel.localeCompare(b.principalLabel),
    ),
  );

  const rows = computed<AccessRow[]>(() => {
    const query = filter.value.trim().toLowerCase();
    const environments = access.value?.environments ?? [];
    return grants.value
      .filter((grant) => grant.principalLabel.toLowerCase().includes(query))
      .map((grant) => {
        const environment = environments.find(
          (candidate) => candidate.id === grant.scopeId,
        );
        return {
          grant,
          isGroup: grant.principalType === "group",
          where:
            grant.scopeType === "project"
              ? "Whole project"
              : (environment?.name ?? grant.scopeLabel),
          whereProtected: environment?.protected ?? false,
          meter: levelMeter(grant.level),
        };
      });
  });

  // ---------------------------------------------------------------------
  // Give access / change level
  // ---------------------------------------------------------------------

  const dialog = ref<AccessDialog | null>(null);
  const principalKey = ref("");
  const where = ref<"project" | "environment">("project");
  const environmentId = ref("");
  const level = ref<Level>("read");
  const saving = ref(false);
  const dialogError = ref<string | null>(null);

  const levels = computed(() =>
    where.value === "project" ? PROJECT_LEVELS : ENVIRONMENT_LEVELS,
  );

  function openGive(): void {
    principalKey.value = principals.value[0]
      ? `${principals.value[0].type}:${principals.value[0].id}`
      : "";
    where.value = "project";
    environmentId.value = access.value?.environments[0]?.id ?? "";
    level.value = "read";
    dialogError.value = null;
    dialog.value = { mode: "give" };
  }

  function openChange(grant: Grant): void {
    principalKey.value = `${grant.principalType}:${grant.principalId}`;
    where.value = grant.scopeType === "environment" ? "environment" : "project";
    environmentId.value =
      grant.scopeType === "environment"
        ? grant.scopeId
        : (access.value?.environments[0]?.id ?? "");
    level.value = grant.level;
    dialogError.value = null;
    dialog.value = { mode: "change", grant };
  }

  // Keep the level valid when the scope changes what is allowed.
  watch(where, () => {
    if (!levels.value.includes(level.value)) level.value = "read";
  });

  /** The person or group the dialog is about, picked or fixed. */
  const dialogPrincipal = computed<Principal | null>(() => {
    if (dialog.value?.mode === "change") {
      const grant = dialog.value.grant;
      return {
        type: grant.principalType,
        id: grant.principalId,
        label: grant.principalLabel,
      };
    }
    return (
      principals.value.find(
        (candidate) =>
          `${candidate.type}:${candidate.id}` === principalKey.value,
      ) ?? null
    );
  });

  const dialogSummary = computed(() => {
    const principal = dialogPrincipal.value;
    if (!principal || !access.value) return "";
    const project = access.value.project.name;
    const environment = access.value.environments.find(
      (candidate) => candidate.id === environmentId.value,
    );
    const target =
      where.value === "project"
        ? `all of ${project}`
        : `${project} / ${environment?.name ?? "this environment"}`;
    const replaces = grants.value.some(
      (grant) =>
        grant.principalId === principal.id &&
        (where.value === "project"
          ? grant.scopeType === "project"
          : grant.scopeId === environmentId.value),
    );
    const sentence = grantSentence(
      principal.label,
      principal.type === "group",
      level.value,
      target,
    );
    return replaces
      ? `${sentence} This replaces their current grant there.`
      : sentence;
  });

  async function submitDialog(): Promise<boolean> {
    const principal = dialogPrincipal.value;
    if (!principal) {
      dialogError.value = "Choose who gets access.";
      return false;
    }
    if (where.value === "environment" && !environmentId.value) {
      dialogError.value = "Choose an environment.";
      return false;
    }
    saving.value = true;
    dialogError.value = null;
    try {
      await give(
        principal,
        where.value === "project" ? WHOLE_PROJECT : environmentId.value,
        level.value,
      );
      dialog.value = null;
      return true;
    } catch (cause) {
      dialogError.value = errorMessage(cause, "Could not give that access.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  async function load(): Promise<void> {
    request?.abort();
    const current = new AbortController();
    request = current;
    loading.value = true;
    loadError.value = null;
    try {
      const result = await fetchProjectAccess(projectRef.value, current.signal);
      if (!current.signal.aborted) access.value = result;
    } catch (cause) {
      if (current.signal.aborted) return;
      access.value = null;
      loadError.value = errorMessage(
        cause,
        "Could not load who has access to this project.",
      );
    } finally {
      if (request === current) loading.value = false;
    }
  }

  /**
   * People and groups to pick from. Each list needs its own capability, so
   * a project manager without them sees only what they may list.
   */
  async function loadPrincipals(): Promise<void> {
    const [users, groups] = await Promise.allSettled([
      auth.can("users.view") ? fetchUsers() : Promise.resolve([]),
      auth.can("users.view", "groups.manage")
        ? fetchGroups()
        : Promise.resolve([]),
    ]);
    principals.value = [
      ...(groups.status === "fulfilled" ? groups.value : []).map((group) => ({
        type: "group" as const,
        id: group.id,
        label: group.name,
      })),
      ...(users.status === "fulfilled" ? users.value : [])
        .filter(
          (user) => user.role !== "root" && user.id !== auth.session?.adminId,
        )
        .map((user) => ({
          type: "user" as const,
          id: user.id,
          label: user.email,
        })),
    ];
  }

  async function give(
    principal: Principal,
    scope: string,
    level: Level,
  ): Promise<void> {
    await runWithReauth(async () => {
      await setGrant({
        principalType: principal.type,
        principalId: principal.id,
        scopeType: scope === WHOLE_PROJECT ? "project" : "environment",
        scope: scope === WHOLE_PROJECT ? access.value!.project.id : scope,
        level,
      });
    });
    await load();
  }

  async function remove(grant: Grant): Promise<void> {
    actionError.value = null;
    try {
      await runWithReauth(() => removeGrant(grant.id));
      await load();
    } catch (cause) {
      actionError.value = errorMessage(cause, "Could not remove that access.");
    }
  }

  watch(projectRef, load);
  onMounted(() => {
    void load();
    void loadPrincipals();
  });
  onUnmounted(() => request?.abort());

  return {
    access,
    grants,
    rows,
    filter,
    canManage,
    dialog,
    principalKey,
    where,
    environmentId,
    level,
    levels,
    saving,
    dialogError,
    dialogSummary,
    openGive,
    openChange,
    submitDialog,
    formatLevel,
    loading,
    loadError,
    actionError,
    principals,
    projectRef,
    load,
    give,
    remove,
  };
}

export type AccessPanelController = ReturnType<typeof useAccessPanelController>;
