import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, type RouteLocationRaw } from "vue-router";
import {
  fetchProjectAccess,
  fetchUserAccess,
  removeGrant,
} from "~/services/access.api";
import type { Grant, ProjectAccess } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import {
  levelMeter,
  type GrantPrincipal,
  type LevelMeter,
} from "~/utils/level";

export interface AccessRow {
  grant: Grant;
  isGroup: boolean;
  /** "Whole project", or the environment's name. */
  where: string;
  whereProtected: boolean;
  meter: LevelMeter;
}

/**
 * Access panel controller: every grant on the project and its
 * environments. Giving access opens the shared Give access page; changing
 * a level opens the shared Change access dialog. Removals run through the
 * shared password confirmation, since the server wants a recent sign-in.
 * Protection lives on the Settings tab.
 */
export function useAccessPanelController() {
  const route = useRoute();
  const { runWithReauth } = useReauthentication();

  const access = ref<ProjectAccess | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  let request: AbortController | null = null;

  const projectRef = computed(() =>
    typeof route.params.projectRef === "string" ? route.params.projectRef : "",
  );
  /** Only project managers change access; access.report can only read it. */
  const canManage = computed(
    () => access.value?.project.level === "project_manager",
  );
  const filter = ref("");

  /** Where Give access goes: the shared page, with this project fixed. */
  const giveTo = computed<RouteLocationRaw>(() => ({
    name: "project-access-give",
    params: { projectRef: projectRef.value },
  }));

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
  // Change level
  // ---------------------------------------------------------------------

  /** The grant the Change access dialog is open for. */
  const changing = ref<Grant | null>(null);
  /** Who holds it, with their role's cap once it's known. */
  const changePrincipal = ref<GrantPrincipal | null>(null);

  /**
   * Opens Change level. A person's or agent's role cap loads first, so the
   * dialog never offers a level the server would refuse. Without the right
   * to read it, the dialog opens uncapped and the server still checks.
   */
  async function openChange(grant: Grant): Promise<void> {
    actionError.value = null;
    const principal: GrantPrincipal = {
      type: grant.principalType,
      id: grant.principalId,
      label: grant.principalLabel,
    };
    if (grant.principalType === "user") {
      try {
        principal.maxLevel = (
          await fetchUserAccess(grant.principalId)
        ).maxLevel;
      } catch {
        // Not allowed to see their role; the server enforces it on save.
      }
    }
    changePrincipal.value = principal;
    changing.value = grant;
  }

  function closeChange(): void {
    changing.value = null;
  }

  async function changed(): Promise<void> {
    changing.value = null;
    await load();
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
  onMounted(load);
  onUnmounted(() => request?.abort());

  return {
    access,
    grants,
    rows,
    filter,
    canManage,
    giveTo,
    changing,
    changePrincipal,
    openChange,
    closeChange,
    changed,
    loading,
    loadError,
    actionError,
    projectRef,
    load,
    remove,
  };
}

export type AccessPanelController = ReturnType<typeof useAccessPanelController>;
