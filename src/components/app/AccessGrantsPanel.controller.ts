import { computed, ref, type Ref } from "vue";
import type { RouteLocationRaw } from "vue-router";
import { removeGrant } from "~/services/access.api";
import type { Grant } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatLevel } from "~/utils/format";
import { levelMeter, type LevelMeter } from "~/utils/level";
import type { GrantPrincipal } from "./GiveAccessDialog.controller";

export interface GrantRow {
  grant: Grant;
  project: string;
  /** The environment's name, or null for the whole project. */
  environment: string | null;
  /** "All projects" for an instance-wide grant. */
  allProjects: boolean;
  /** Held directly, or through the group named here. */
  viaGroup: { id: string; name: string } | null;
  meter: LevelMeter;
  /** Where "Open project" goes. */
  projectLink: RouteLocationRaw | null;
}

/**
 * The Access tab on a user's, AI agent's or group's page: every grant they
 * hold, where it comes from, and the level. Direct grants can change level
 * or be removed; grants through a group are changed on the group.
 */
export function useAccessGrantsPanel(
  principal: Ref<GrantPrincipal>,
  grants: Ref<Grant[]>,
  changed: () => Promise<void> | void,
) {
  const { runWithReauth } = useReauthentication();
  const filter = ref("");

  const rows = computed<GrantRow[]>(() =>
    grants.value
      .filter((grant) => grant.scopeType !== "audit")
      .map((grant) => {
        const allProjects = grant.scopeType === "all";
        const [project = grant.scopeLabel, environment = null] =
          grant.scopeType === "environment"
            ? grant.scopeLabel.split("/")
            : [grant.scopeLabel];
        const viaGroup =
          grant.principalType === "group" &&
          !(
            principal.value.type === "group" &&
            grant.principalId === principal.value.id
          )
            ? { id: grant.principalId, name: grant.principalLabel }
            : null;
        return {
          grant,
          project: allProjects ? "All projects" : project,
          environment,
          allProjects,
          viaGroup,
          meter: levelMeter(grant.level),
          projectLink: allProjects
            ? null
            : environment
              ? {
                  name: "environment",
                  params: { projectRef: project, environmentName: environment },
                }
              : { name: "project", params: { projectRef: project } },
        };
      })
      .sort(
        (a, b) =>
          Number(b.allProjects) - Number(a.allProjects) ||
          a.project.localeCompare(b.project) ||
          (a.environment ?? "").localeCompare(b.environment ?? ""),
      ),
  );

  const visibleRows = computed(() => {
    const needle = filter.value.trim().toLowerCase();
    return needle
      ? rows.value.filter((row) =>
          `${row.project}/${row.environment ?? ""}`
            .toLowerCase()
            .includes(needle),
        )
      : rows.value;
  });

  /**
   * "An environment grant overrides the project grant, so maya sees names
   * only in payments/prod." One note per environment that overrides.
   */
  const overrides = computed(() =>
    rows.value
      .filter((row) => row.environment)
      .filter((row) =>
        rows.value.some(
          (other) =>
            other.project === row.project &&
            !other.environment &&
            !other.allProjects,
        ),
      )
      .map((row) => {
        const who = principal.value.label;
        const what =
          row.grant.level === "none"
            ? "can't reach"
            : `gets ${formatLevel(row.grant.level)} in`;
        return `An environment grant overrides the project grant, so ${who} ${what} ${row.project}/${row.environment}.`;
      }),
  );

  // Give and change
  const dialogOpen = ref(false);
  const editing = ref<Grant | null>(null);

  function openGive(): void {
    editing.value = null;
    dialogOpen.value = true;
  }

  function openChange(grant: Grant): void {
    editing.value = grant;
    dialogOpen.value = true;
  }

  async function saved(): Promise<void> {
    dialogOpen.value = false;
    await changed();
  }

  // Remove
  const removing = ref<GrantRow | null>(null);
  const removeBusy = ref(false);
  const removeError = ref<string | null>(null);

  function askRemove(row: GrantRow): void {
    removeError.value = null;
    removing.value = row;
  }

  function cancelRemove(): void {
    if (!removeBusy.value) removing.value = null;
  }

  async function confirmRemove(): Promise<void> {
    const row = removing.value;
    if (!row) return;
    removeBusy.value = true;
    removeError.value = null;
    try {
      await runWithReauth(() => removeGrant(row.grant.id));
      removing.value = null;
      await changed();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      removeError.value = errorMessage(cause, "Could not remove that access.");
    } finally {
      removeBusy.value = false;
    }
  }

  return {
    filter,
    rows,
    visibleRows,
    overrides,
    dialogOpen,
    editing,
    openGive,
    openChange,
    saved,
    removing,
    removeBusy,
    removeError,
    askRemove,
    cancelRemove,
    confirmRemove,
  };
}
