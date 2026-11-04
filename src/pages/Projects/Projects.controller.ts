import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useQuery, useQueryCache } from "@pinia/colada";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import type { AuthMethods } from "~/services/auth.api";
import type { AffectedCounts, Environment, Project } from "~/services";
import { reaches } from "~/services/access.api";
import { useAuthStore } from "~/stores/auth.store";
import { authMethodsQuery } from "~/queries/auth";
import { loadFailure, onlyWhen, readQuery, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { environmentsQuery, projectsQuery } from "~/queries/projects";
import { secretsQuery } from "~/queries/secrets";
import { runnerTokensQuery } from "~/queries/tokens";
import { formatRelativeTime } from "~/utils/format";
import { levelMeter, type LevelMeter } from "~/utils/level";

/** Environments a closed-up project lists before "Show all". */
export const ENVIRONMENT_PREVIEW = 6;

export interface EnvironmentRow {
  environment: Environment;
  meter: LevelMeter | null;
  updated: string;
  canManage: boolean;
}

export interface ProjectRow {
  project: Project;
  environmentCount: number;
  expanded: boolean;
  environments: EnvironmentRow[];
  /** Environments left out until "Show all". */
  hiddenCount: number;
  canManage: boolean;
}

export type NameDialogState =
  | { kind: "new-project" }
  | { kind: "rename-project"; project: Project }
  | { kind: "new-environment"; project: Project }
  | { kind: "rename-environment"; environment: Environment };

export type DeleteState =
  | { kind: "project"; project: Project }
  | { kind: "environment"; environment: Environment };

/**
 * Projects controller: the project tree on /projects. Loads every project
 * and every environment the viewer reaches in two requests, filters them,
 * opens and closes projects like an accordion, and owns the create,
 * rename and delete dialogs.
 *
 * Mutations reload the tree after they succeed. Dialog actions throw on
 * failure so the dialog can show the error.
 */
export function useProjectsController() {
  const router = useRouter();
  const auth = useAuthStore();
  const cache = useQueryCache();
  const invalidate = useInvalidate();

  const projectList = useQuery(projectsQuery);
  const environmentList = useQuery(() => environmentsQuery(null));
  const projectData = shownData<Project[]>(projectList);
  const environmentData = shownData<Environment[]>(environmentList);
  /** Null until both lists have loaded, so the tree never shows half. */
  const projects = computed(() =>
    projectData.value && environmentData.value ? projectData.value : null,
  );
  const environments = computed(() => environmentData.value ?? []);
  const projectsFailure = loadFailure(projectList, "Could not load projects.");
  const environmentsFailure = loadFailure(
    environmentList,
    "Could not load projects.",
  );
  const loadError = computed(
    () => projectsFailure.value ?? environmentsFailure.value,
  );
  const filter = ref("");
  const expanded = ref(new Set<string>());
  const showingAll = ref(new Set<string>());

  async function load(): Promise<void> {
    await Promise.all([projectList.refetch(), environmentList.refetch()]);
  }

  // Open the first project the first time the tree shows, cached or not.
  let opened = false;
  watch(
    projects,
    (list) => {
      if (opened || list === null) return;
      opened = true;
      if (list[0]) expanded.value = new Set([list[0].id]);
    },
    { immediate: true },
  );

  // ---------------------------------------------------------------------------
  // Setup checklist
  // ---------------------------------------------------------------------------

  /**
   * Root on a server with no projects is finishing setup, so the empty
   * state carries on the setup steps instead of a plain invitation.
   */
  const showSetupChecklist = computed(
    () => auth.isRoot && projects.value?.length === 0,
  );
  const accountEmail = computed(() => auth.session?.email ?? "");
  // Best effort. The sign-in step falls back to generic copy without it.
  const signInMethodsQuery = useQuery(() =>
    onlyWhen(showSetupChecklist.value, authMethodsQuery),
  );
  const signInMethods = shownData<AuthMethods>(signInMethodsQuery);

  // The server checks every action; these only hide what it would refuse.
  // Responses without a level come from servers that predate grants.
  const canCreateProjects = computed(() => auth.can("projects.create"));
  const managesProject = (project: Project): boolean =>
    reaches(project.level ?? "project_manager", "project_manager");
  const managesEnvironment = (environment: Environment): boolean =>
    reaches(environment.level ?? "manage", "manage");

  function environmentRow(environment: Environment): EnvironmentRow {
    return {
      environment,
      meter: environment.level ? levelMeter(environment.level) : null,
      updated: formatRelativeTime(environment.updatedAt),
      canManage: managesEnvironment(environment),
    };
  }

  const rows = computed<ProjectRow[]>(() => {
    const query = filter.value.trim().toLowerCase();
    const result: ProjectRow[] = [];
    for (const project of projects.value ?? []) {
      const own = environments.value
        .filter((environment) => environment.projectId === project.id)
        .sort((a, b) => a.name.localeCompare(b.name));
      const nameMatches = project.name.toLowerCase().includes(query);
      const envMatches = own.filter((environment) =>
        environment.name.toLowerCase().includes(query),
      );
      if (query && !nameMatches && envMatches.length === 0) continue;

      // A filter that matches environments opens their project and lists
      // only those; one that matches the project name lists them all.
      const listed = query && !nameMatches ? envMatches : own;
      const isExpanded =
        (query !== "" && envMatches.length > 0) ||
        expanded.value.has(project.id);
      const showAll = query !== "" || showingAll.value.has(project.id);
      const shown = showAll ? listed : listed.slice(0, ENVIRONMENT_PREVIEW);
      result.push({
        project,
        environmentCount: own.length,
        expanded: isExpanded,
        environments: isExpanded ? shown.map(environmentRow) : [],
        hiddenCount: isExpanded ? listed.length - shown.length : 0,
        canManage: managesProject(project),
      });
    }
    return result;
  });

  function toggleProject(projectId: string): void {
    const next = new Set(expanded.value);
    if (next.has(projectId)) next.delete(projectId);
    else next.add(projectId);
    expanded.value = next;
  }

  function showAllEnvironments(projectId: string): void {
    showingAll.value = new Set(showingAll.value).add(projectId);
  }

  function openEnvironment(environment: Environment): void {
    void router.push({
      name: "environment",
      params: {
        projectRef: environment.projectName,
        environmentName: environment.name,
      },
    });
  }

  function openProjectAccess(project: Project): void {
    void router.push({
      name: "project-access",
      params: { projectRef: project.name },
    });
  }

  // -------------------------------------------------------------------------
  // Create and rename
  // -------------------------------------------------------------------------

  const nameDialog = ref<NameDialogState | null>(null);

  const nameDialogCopy = computed(() => {
    const dialog = nameDialog.value;
    switch (dialog?.kind) {
      case "rename-project":
        return {
          title: "Rename project",
          label: "Project name",
          initialName: dialog.project.name,
          submitLabel: "Rename",
          hint: "Names are unique on this server. Example: payment-service",
          placeholder: "e.g. payment-service",
        };
      case "new-environment":
        return {
          title: `New environment in ${dialog.project.name}`,
          label: "Environment name",
          initialName: "",
          submitLabel: "Create",
          hint: "Example: development, staging, production",
          placeholder: "e.g. production",
        };
      case "rename-environment":
        return {
          title: "Rename environment",
          label: "Environment name",
          initialName: dialog.environment.name,
          submitLabel: "Rename",
          hint: "Links to the old name stop working.",
          placeholder: "e.g. production",
        };
      default:
        return {
          title: "New project",
          label: "Project name",
          initialName: "",
          submitLabel: "Create",
          hint: "Names are unique on this server. Example: payment-service",
          placeholder: "e.g. payment-service",
        };
    }
  });

  async function submitName(name: string): Promise<void> {
    const dialog = nameDialog.value;
    if (!dialog) return;
    switch (dialog.kind) {
      case "new-project": {
        const created = await projectsApi.createProject(name);
        void router.push({
          name: "project",
          params: { projectRef: created.name },
        });
        return;
      }
      case "rename-project":
        await projectsApi.renameProject(dialog.project.id, name);
        break;
      case "new-environment": {
        const created = await environmentsApi.createEnvironment(
          dialog.project.name,
          name,
        );
        expanded.value = new Set(expanded.value).add(dialog.project.id);
        void router.push({
          name: "environment",
          params: {
            projectRef: dialog.project.name,
            environmentName: created.name,
          },
        });
        return;
      }
      case "rename-environment":
        await environmentsApi.renameEnvironment(dialog.environment.id, name);
        break;
    }
    await invalidate.projectTreeChanged();
  }

  // -------------------------------------------------------------------------
  // Delete
  // -------------------------------------------------------------------------

  const deleting = ref<DeleteState | null>(null);
  const deleteCounts = ref<Array<{ label: string; count: number }>>([]);
  const deleteLoading = ref(false);
  const deleteError = ref<string | null>(null);
  let deleteRequest = 0;

  async function startDelete(target: DeleteState): Promise<void> {
    const current = ++deleteRequest;
    deleting.value = target;
    deleteError.value = null;
    deleteCounts.value = [];
    if (target.kind !== "environment") return;
    const id = target.environment.id;
    try {
      // Counts for a destructive confirmation are read fresh.
      const [secrets, tokens] = await Promise.all([
        readQuery(cache, secretsQuery(id), { fresh: true }),
        readQuery(cache, runnerTokensQuery(id), { fresh: true }),
      ]);
      if (current !== deleteRequest || deleting.value === null) return;
      deleteCounts.value = [
        { label: "secrets", count: secrets.length },
        { label: "runner tokens", count: tokens.length },
      ];
    } catch {
      // The preview is best effort. Deletion still confirms by name.
    }
  }

  function cancelDelete(): void {
    if (!deleteLoading.value) deleting.value = null;
  }

  async function confirmDelete(): Promise<AffectedCounts | null> {
    const target = deleting.value;
    if (!target) return null;
    deleteLoading.value = true;
    deleteError.value = null;
    try {
      const affected =
        target.kind === "project"
          ? await projectsApi.deleteProject(target.project.id)
          : await environmentsApi.deleteEnvironment(target.environment.id);
      deleting.value = null;
      await (target.kind === "project"
        ? invalidate.projectDeleted(target.project.id)
        : invalidate.environmentDeleted(target.environment.id));
      return affected;
    } catch {
      deleteError.value =
        target.kind === "project"
          ? "The project could not be deleted."
          : "The environment could not be deleted.";
      return null;
    } finally {
      deleteLoading.value = false;
    }
  }

  return {
    projects,
    loadError,
    load,
    filter,
    rows,
    toggleProject,
    showAllEnvironments,
    openEnvironment,
    openProjectAccess,
    canCreateProjects,
    showSetupChecklist,
    accountEmail,
    signInMethods,
    nameDialog,
    nameDialogCopy,
    submitName,
    deleting,
    deleteCounts,
    deleteLoading,
    deleteError,
    startDelete,
    cancelDelete,
    confirmDelete,
  };
}

export type ProjectsController = ReturnType<typeof useProjectsController>;
