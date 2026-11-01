import { computed, ref, type Ref } from "vue";
import type { RouteLocationRaw } from "vue-router";
import { LEVELS, removeGrant } from "~/services/access.api";
import type { Grant } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatList } from "~/utils/format";
import { levelMeter, type LevelMeter } from "~/utils/level";
import type { GrantPrincipal } from "~/utils/level";

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
  /** A direct grant on a project or environment, which you can change or remove. */
  editable: boolean;
  /** Whether the row's menu has anything in it. Rows without one show none. */
  hasActions: boolean;
}

/** An environment grant that replaces the project grant there. */
export interface EnvironmentOverride {
  key: string;
  project: string;
  environment: string;
  /** What the project grant gives. */
  from: LevelMeter;
  /** What the environment gets instead, or null when it's blocked. */
  to: LevelMeter | null;
}

/** `payments` or `payments/prod`, as the table shows a grant. */
function rowLabel(row: GrantRow): string {
  return row.environment ? `${row.project}/${row.environment}` : row.project;
}

/**
 * The Access tab on a user's, AI agent's or group's page: every grant they
 * hold, where it comes from, and the level. Direct grants can change level
 * or be removed; grants through a group are changed on the group.
 * Giving new access happens on its own page.
 */
export function useAccessGrantsPanel(
  principal: Ref<GrantPrincipal>,
  grants: Ref<Grant[]>,
  changed: () => Promise<void> | void,
  readOnly: Ref<boolean> = ref(false),
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
        const projectLink: RouteLocationRaw | null = allProjects
          ? null
          : environment
            ? {
                name: "environment",
                params: { projectRef: project, environmentName: environment },
              }
            : { name: "project", params: { projectRef: project } };
        const editable = !readOnly.value && !viaGroup && !allProjects;
        return {
          grant,
          project: allProjects ? "All projects" : project,
          environment,
          allProjects,
          viaGroup,
          meter: levelMeter(grant.level),
          projectLink,
          editable,
          hasActions: editable || viaGroup !== null || projectLink !== null,
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
   * Environments whose own grant replaces the project grant, with both
   * levels so the note can show the change: payments/prod goes from Write
   * to Names only. One entry per environment that overrides.
   */
  const overrides = computed<EnvironmentOverride[]>(() =>
    rows.value.flatMap((row) => {
      if (!row.environment) return [];
      const projectLevels = rows.value
        .filter(
          (other) =>
            other.project === row.project &&
            !other.environment &&
            !other.allProjects,
        )
        .map((other) => other.grant.level);
      if (projectLevels.length === 0) return [];
      const from = projectLevels.reduce((a, b) =>
        LEVELS.indexOf(b) > LEVELS.indexOf(a) ? b : a,
      );
      return [
        {
          key: row.grant.id,
          project: row.project,
          environment: row.environment,
          from: levelMeter(from),
          to: row.grant.level === "none" ? null : row.meter,
        },
      ];
    }),
  );

  // Change level
  const editing = ref<Grant | null>(null);

  function openChange(grant: Grant): void {
    editing.value = grant;
  }

  async function saved(): Promise<void> {
    editing.value = null;
    await changed();
  }

  // Select. Only rows you can remove have a checkbox, and ids that leave
  // the table after a reload drop out of the selection on their own.
  const selectedIds = ref<string[]>([]);

  const selectedRows = computed(() =>
    rows.value.filter(
      (row) => row.editable && selectedIds.value.includes(row.grant.id),
    ),
  );

  /** Whether the table shows checkboxes at all. */
  const canSelect = computed(() => rows.value.some((row) => row.editable));

  const visibleSelectable = computed(() =>
    visibleRows.value.filter((row) => row.editable),
  );

  /** The header checkbox: every row in view, some of them, or none. */
  const allVisibleSelected = computed(
    () =>
      visibleSelectable.value.length > 0 &&
      visibleSelectable.value.every((row) =>
        selectedIds.value.includes(row.grant.id),
      ),
  );
  const someVisibleSelected = computed(
    () =>
      !allVisibleSelected.value &&
      visibleSelectable.value.some((row) =>
        selectedIds.value.includes(row.grant.id),
      ),
  );

  function isSelected(row: GrantRow): boolean {
    return selectedIds.value.includes(row.grant.id);
  }

  function toggleRow(row: GrantRow, on: boolean): void {
    const rest = selectedIds.value.filter((id) => id !== row.grant.id);
    selectedIds.value = on && row.editable ? [...rest, row.grant.id] : rest;
  }

  /** Select or clear the rows in view; rows hidden by the filter keep theirs. */
  function toggleAllVisible(on: boolean): void {
    const inView = visibleSelectable.value.map((row) => row.grant.id);
    const rest = selectedIds.value.filter((id) => !inView.includes(id));
    selectedIds.value = on ? [...rest, ...inView] : rest;
  }

  function clearSelection(): void {
    selectedIds.value = [];
  }

  // Remove one grant from its menu, or every selected one at once.
  const removing = ref<GrantRow[] | null>(null);
  const removeBusy = ref(false);
  const removeError = ref<string | null>(null);

  function askRemove(row: GrantRow): void {
    removeError.value = null;
    removing.value = [row];
  }

  function askRemoveSelected(): void {
    if (selectedRows.value.length === 0) return;
    removeError.value = null;
    removing.value = [...selectedRows.value];
  }

  function cancelRemove(): void {
    if (!removeBusy.value) removing.value = null;
  }

  const removeTitle = computed(() => {
    const list = removing.value ?? [];
    return list.length === 1
      ? `Remove access to ${rowLabel(list[0]!)}?`
      : `Remove ${list.length} grants?`;
  });

  const removeDescription = computed(() => {
    const list = removing.value ?? [];
    const who = principal.value.label;
    return list.length === 1
      ? `${who} loses this access on their next request. Other grants they hold stay.`
      : `${who} loses access to ${formatList(list.map(rowLabel))} on their next request. Other grants they hold stay.`;
  });

  async function confirmRemove(): Promise<void> {
    const todo = removing.value;
    if (!todo || todo.length === 0) return;
    removeBusy.value = true;
    removeError.value = null;
    let done = 0;
    try {
      // One call per grant. A retry after a sign-in prompt starts after the
      // ones already removed.
      await runWithReauth(async () => {
        for (const row of todo.slice(done)) {
          await removeGrant(row.grant.id);
          done += 1;
        }
      });
      const gone = todo.map((row) => row.grant.id);
      selectedIds.value = selectedIds.value.filter((id) => !gone.includes(id));
      removing.value = null;
      await changed();
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      const reason = errorMessage(cause, "Could not remove that access.");
      if (done === 0) {
        removeError.value = reason;
        return;
      }
      // Keep the rest in the dialog, so confirming again only tries those.
      const gone = todo.slice(0, done).map((row) => row.grant.id);
      selectedIds.value = selectedIds.value.filter((id) => !gone.includes(id));
      removing.value = todo.slice(done);
      removeError.value = `Removed ${done} of ${todo.length}. ${rowLabel(todo[done]!)} failed: ${reason}`;
      await Promise.resolve(changed()).catch(() => undefined);
    } finally {
      removeBusy.value = false;
    }
  }

  return {
    filter,
    rows,
    visibleRows,
    overrides,
    editing,
    openChange,
    saved,
    canSelect,
    selectedRows,
    allVisibleSelected,
    someVisibleSelected,
    isSelected,
    toggleRow,
    toggleAllVisible,
    clearSelection,
    removing,
    removeTitle,
    removeDescription,
    removeBusy,
    removeError,
    askRemove,
    askRemoveSelected,
    cancelRemove,
    confirmRemove,
  };
}
