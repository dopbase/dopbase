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
import { setEnvironmentProtected } from "~/services/environments.api";
import { fetchUsers } from "~/services/users.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";

/** Someone a grant can be given to. */
export interface Principal {
  type: PrincipalType;
  id: string;
  label: string;
}

/** Levels a grant can give, by what it covers. */
export const PROJECT_LEVELS: Level[] = [
  "metadata",
  "read",
  "write",
  "manage",
  "project_manager",
];
export const ENVIRONMENT_LEVELS: Level[] = [
  "none",
  "metadata",
  "read",
  "write",
  "manage",
];

export const LEVEL_HINTS: Record<Level, string> = {
  none: "Hide this environment, even from a project grant",
  metadata: "See secret names and history, not values",
  read: "Reveal, export, and run with values",
  write: "Change, import, and roll back secrets",
  manage: "Rename, protect, and issue runner tokens",
  project_manager: "Everything, including who has access",
};

/** The value of the scope picker that means the whole project. */
export const WHOLE_PROJECT = "";

/**
 * Project access: the project's environments with their protection, and
 * every grant on the project and its environments. Changes run through the
 * shared password confirmation, since the server wants a recent sign-in.
 */
export function useProjectAccessController() {
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

  async function setProtected(
    environmentId: string,
    value: boolean,
  ): Promise<void> {
    actionError.value = null;
    try {
      await setEnvironmentProtected(environmentId, value);
      await load();
    } catch (cause) {
      actionError.value = errorMessage(
        cause,
        "Could not change the environment's protection.",
      );
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
    loading,
    loadError,
    actionError,
    principals,
    projectRef,
    load,
    give,
    remove,
    setProtected,
  };
}

export type ProjectAccessController = ReturnType<
  typeof useProjectAccessController
>;
