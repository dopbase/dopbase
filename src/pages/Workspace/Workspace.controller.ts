import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import type { AffectedCounts, Environment, Project } from "~/services";

/**
 * Workspace controller: projects/environments rail data, URL-derived
 * selection, and project/environment CRUD.
 *
 * The selected project is addressed by its unique name and the selected
 * environment by its immutable id — both live exclusively in the URL.
 * Mutating actions throw on failure so calling dialogs can render errors;
 * navigation happens only after success.
 */
export function useWorkspaceController() {
  const route = useRoute();
  const router = useRouter();

  const projects = ref<Project[] | null>(null);
  const projectsError = ref<string | null>(null);
  const environments = ref<Environment[] | null>(null);
  const environmentsLoading = ref(false);
  const environmentsError = ref<string | null>(null);

  const projectRef = computed(() =>
    typeof route.params.projectRef === "string"
      ? route.params.projectRef
      : null,
  );
  const environmentId = computed(() =>
    typeof route.params.environmentId === "string"
      ? route.params.environmentId
      : null,
  );
  const activeTab = computed(() =>
    route.name === "environment-tokens" ? "tokens" : "secrets",
  );

  const project = computed(
    () =>
      projects.value?.find(
        (candidate) =>
          candidate.name === projectRef.value ||
          candidate.id === projectRef.value,
      ) ?? null,
  );
  const selectedEnvironment = computed(
    () =>
      environments.value?.find(
        (candidate) => candidate.id === environmentId.value,
      ) ?? null,
  );

  async function loadProjects(): Promise<void> {
    projectsError.value = null;
    try {
      projects.value = await projectsApi.listProjects();
    } catch {
      projectsError.value = "Could not load projects.";
      projects.value = null;
    }
  }

  async function loadEnvironments(): Promise<void> {
    if (!projectRef.value) {
      environments.value = null;
      return;
    }
    environmentsLoading.value = true;
    environmentsError.value = null;
    try {
      environments.value = await environmentsApi.listEnvironments(
        projectRef.value,
      );
    } catch {
      environmentsError.value = "Could not load environments.";
      environments.value = null;
    } finally {
      environmentsLoading.value = false;
    }
  }

  watch(projectRef, loadEnvironments, { immediate: true });
  onMounted(loadProjects);

  // Landing on /workspace with existing projects opens the first project.
  watch(projects, (list) => {
    if (list && list.length > 0 && !projectRef.value) {
      router.replace({
        name: "project",
        params: { projectRef: list[0].name },
      });
    }
  });

  // Opening a project without an environment selects its first one.
  watch([environments, environmentId], ([list, id]) => {
    if (route.name === "project" && list && list.length > 0 && !id) {
      router.replace({
        name: "environment",
        params: {
          projectRef: projectRef.value as string,
          environmentId: list[0].id,
        },
      });
    }
  });

  function selectProject(refName: string): void {
    if (refName === projectRef.value) return;
    router.push({ name: "project", params: { projectRef: refName } });
  }

  function selectEnvironment(id: string): void {
    if (!projectRef.value || id === environmentId.value) return;
    router.push({
      name: "environment",
      params: { projectRef: projectRef.value, environmentId: id },
    });
  }

  function switchTab(tab: "secrets" | "tokens"): void {
    if (!projectRef.value || !environmentId.value) return;
    router.push({
      name: tab === "tokens" ? "environment-tokens" : "environment",
      params: {
        projectRef: projectRef.value,
        environmentId: environmentId.value,
      },
    });
  }

  async function createProject(name: string): Promise<void> {
    const created = await projectsApi.createProject(name);
    await loadProjects();
    router.push({ name: "project", params: { projectRef: created.name } });
  }

  async function renameProject(name: string): Promise<void> {
    if (!projectRef.value) return;
    const updated = await projectsApi.renameProject(projectRef.value, name);
    await loadProjects();
    if (environmentId.value) {
      router.replace({
        name: "environment",
        params: {
          projectRef: updated.name,
          environmentId: environmentId.value,
        },
      });
    } else {
      router.replace({
        name: "project",
        params: { projectRef: updated.name },
      });
    }
  }

  async function deleteProject(): Promise<AffectedCounts> {
    if (!projectRef.value) throw new Error("No project selected.");
    const affected = await projectsApi.deleteProject(projectRef.value);
    await loadProjects();
    router.replace({ name: "workspace" });
    return affected;
  }

  async function createEnvironment(name: string): Promise<void> {
    if (!projectRef.value) return;
    const created = await environmentsApi.createEnvironment(
      projectRef.value,
      name,
    );
    await loadEnvironments();
    router.push({
      name: "environment",
      params: { projectRef: projectRef.value, environmentId: created.id },
    });
  }

  async function renameEnvironment(id: string, name: string): Promise<void> {
    await environmentsApi.renameEnvironment(id, name);
    await loadEnvironments();
  }

  async function deleteEnvironment(id: string): Promise<AffectedCounts> {
    const affected = await environmentsApi.deleteEnvironment(id);
    await loadEnvironments();
    if (environmentId.value === id && projectRef.value) {
      router.replace({
        name: "project",
        params: { projectRef: projectRef.value },
      });
    }
    return affected;
  }

  /** Affected-count preview for the destructive environment dialog. */
  async function describeEnvironmentDeletion(
    id: string,
  ): Promise<Array<{ label: string; count: number }>> {
    const [secrets, tokens] = await Promise.all([
      secretsApi.listSecrets(id),
      tokensApi.listTokens(id),
    ]);
    return [
      { label: "secrets", count: secrets.length },
      { label: "runner tokens", count: tokens.length },
    ];
  }

  return {
    projects,
    projectsError,
    loadProjects,
    environments,
    environmentsLoading,
    environmentsError,
    projectRef,
    environmentId,
    activeTab,
    project,
    selectedEnvironment,
    selectProject,
    selectEnvironment,
    switchTab,
    createProject,
    renameProject,
    deleteProject,
    createEnvironment,
    renameEnvironment,
    deleteEnvironment,
    describeEnvironmentDeletion,
  };
}

export type WorkspaceController = ReturnType<typeof useWorkspaceController>;
