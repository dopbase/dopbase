import { computed, ref, shallowRef } from "vue";
import { useRoute, type RouteLocationRaw } from "vue-router";
import { useQuery, useQueryCache } from "@pinia/colada";
import { removeGrant } from "~/services/access.api";
import type { Grant, ProjectAccess } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { projectAccessQuery, userAccessQuery } from "~/queries/access";
import { loadFailure, onlyWhen, readQuery } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { useAuthStore } from "~/stores/auth.store";
import {
  levelMeter,
  type GrantPrincipal,
  type LevelMeter,
} from "~/utils/level";

export interface AccessRow {
  grant: Grant;
  isGroup: boolean;
  /** Whether you can change or remove it here. */
  editable: boolean;
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
  const cache = useQueryCache();
  const invalidate = useInvalidate();
  const auth = useAuthStore();

  const projectRef = computed(() =>
    typeof route.params.projectRef === "string" ? route.params.projectRef : "",
  );

  const accessQuery = useQuery(() =>
    onlyWhen(projectRef.value !== "", projectAccessQuery(projectRef.value)),
  );
  // A failed reload clears the list: grants that may be out of date
  // shouldn't stay up for someone to act on.
  const loadError = loadFailure(
    accessQuery,
    (cause) =>
      errorMessage(cause, "Could not load who has access to this project."),
    { withData: true },
  );
  const access = computed<ProjectAccess | null>(() =>
    loadError.value ? null : (accessQuery.data.value ?? null),
  );
  const loading = accessQuery.isLoading;
  const actionError = ref<string | null>(null);
  /** Only project managers change access; access.report can only read it. */
  const canManage = computed(
    () => access.value?.project.level === "project_manager",
  );
  const rbac = computed(() => auth.hasFeature("rbac"));
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
          // Without RBAC only AI agents' access changes.
          editable: canManage.value && (rbac.value || grant.principalAgent),
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
  const changing = shallowRef<Grant | null>(null);
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
          await readQuery(cache, userAccessQuery(grant.principalId))
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
    await invalidate.accessChanged();
  }

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    await accessQuery.refetch();
  }

  async function remove(grant: Grant): Promise<void> {
    actionError.value = null;
    try {
      await runWithReauth(() => removeGrant(grant.id));
      await invalidate.accessChanged();
    } catch (cause) {
      actionError.value = errorMessage(cause, "Could not remove that access.");
    }
  }

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
