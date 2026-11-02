import { computed, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { useQuery } from "@pinia/colada";
import * as environmentsApi from "~/services/environments.api";
import type { Environment, Project } from "~/services";
import { reaches } from "~/services/access.api";
import { useAuthStore } from "~/stores/auth.store";
import { loadFailure, onlyWhen, shownData } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { environmentsQuery, projectsQuery } from "~/queries/projects";
import { levelMeter } from "~/utils/level";

export type ProjectTab =
  "secrets" | "compare" | "tokens" | "access" | "activity" | "settings";

/** Tabs that belong to one environment, and the route each one uses. */
const ENVIRONMENT_ROUTES = {
  secrets: "environment",
  compare: "environment-compare",
  tokens: "environment-tokens",
  activity: "environment-activity",
} as const;

/** Tabs that belong to the whole project. */
const PROJECT_ROUTES = {
  access: "project-access",
  settings: "project-settings",
} as const;

const TAB_FOR_ROUTE: Record<string, ProjectTab> = {
  environment: "secrets",
  "environment-compare": "compare",
  "environment-tokens": "tokens",
  "environment-activity": "activity",
  "project-access": "access",
  "project-settings": "settings",
};

/**
 * The environment each project was last opened on, by project id and
 * environment id, so Access and Settings, which have no environment in
 * their address, keep the context bar on it, even across renames.
 */
const lastEnvironment = new Map<string, string>();

/**
 * Project controller: one project's page. Secrets, Compare, Tokens and
 * Activity are addressed per environment
 * (`/projects/:projectRef/:environmentName/<tab>`);
 * Access and Settings cover the whole project
 * (`/projects/:projectRef/<tab>`) and keep the environment last opened.
 * It loads the project list for the switcher and the project's
 * environments, resolves the environment, and opens the first one when the
 * address names none.
 *
 * It decides which tabs show from the levels the server reports: Tokens
 * needs Manage on the environment, Settings needs Project manager, and
 * Access needs Project manager or `access.report`. A link to a tab you
 * can't use falls back to Secrets. The server still checks every request.
 */
export function useProjectController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const invalidate = useInvalidate();

  const projectRef = computed(() =>
    typeof route.params.projectRef === "string" ? route.params.projectRef : "",
  );

  const projectList = useQuery(projectsQuery);
  // Keyed by project, so another project starts empty and a late answer
  // for the previous one never lands here.
  const environmentList = useQuery(() =>
    onlyWhen(projectRef.value !== "", environmentsQuery(projectRef.value)),
  );
  const projects = shownData<Project[]>(projectList);
  const environments = shownData<Environment[]>(environmentList);
  const loadError = loadFailure(
    environmentList,
    "Could not load this project's environments.",
  );
  /**
   * The project list failed, so the project's own level is unknown. The
   * switcher falls back to the current project, but Access and Settings
   * need its level, so say so instead of showing a blank tab.
   */
  const projectsError = loadFailure(
    projectList,
    "Could not load this project's details.",
  );
  const activeTab = computed<ProjectTab>(
    () => TAB_FOR_ROUTE[String(route.name)] ?? "secrets",
  );
  const projectLevel = computed(() => activeTab.value in PROJECT_ROUTES);

  const project = computed(
    () =>
      projects.value?.find(
        (candidate) =>
          candidate.name === projectRef.value ||
          candidate.id === projectRef.value,
      ) ?? null,
  );
  /** The name to show before the project list arrives. */
  const projectName = computed(() => project.value?.name ?? projectRef.value);

  /** Named in the address, or for project tabs the one last opened. */
  const environmentName = computed(() => {
    if (typeof route.params.environmentName === "string")
      return route.params.environmentName;
    if (!projectLevel.value) return null;
    // Remembered by id, so a rename keeps it; a deleted one falls back.
    const list = environments.value ?? [];
    const remembered = list[0] && lastEnvironment.get(list[0].projectId);
    return (
      list.find((candidate) => candidate.id === remembered)?.name ??
      list[0]?.name ??
      null
    );
  });
  const environment = computed(
    () =>
      environments.value?.find(
        (candidate) => candidate.name === environmentName.value,
      ) ?? null,
  );
  /** The URL names an environment this project doesn't have, or you can't reach. */
  const environmentMissing = computed(
    () =>
      !projectLevel.value &&
      environments.value !== null &&
      environmentName.value !== null &&
      environment.value === null,
  );

  // A cached list is there from the start, so these watchers run at once.
  watch(
    environment,
    (current) => {
      if (current) lastEnvironment.set(current.projectId, current.id);
    },
    { immediate: true },
  );

  async function reloadProjects(): Promise<void> {
    await projectList.refetch();
  }

  async function reload(): Promise<void> {
    await environmentList.refetch();
  }

  /**
   * Refreshes after a change elsewhere on the page, such as a rename. The
   * current lists stay on screen meanwhile, and a failed refresh keeps
   * them: the change itself already succeeded.
   */
  async function refresh(): Promise<void> {
    await invalidate.projectTreeChanged();
  }

  // /projects/:projectRef opens the first environment.
  watch(
    environments,
    (list) => {
      if (route.name !== "project" || !list?.[0]) return;
      void router.replace(environmentLocation(list[0].name, "secrets"));
    },
    { immediate: true },
  );

  // With no environment open, show the project's own level instead.
  const level = computed(() =>
    environment.value
      ? environment.value.level
      : environments.value?.length === 0
        ? project.value?.level
        : undefined,
  );
  const meter = computed(() => (level.value ? levelMeter(level.value) : null));
  // Responses without a level come from servers that predate grants.
  const managesEnvironment = computed(
    () =>
      environment.value !== null &&
      reaches(environment.value.level ?? "manage", "manage"),
  );
  const managesProject = computed(() =>
    reaches(project.value?.level ?? "project_manager", "project_manager"),
  );
  const readsAccess = computed(
    () => managesProject.value || auth.can("access.report"),
  );

  function allowed(tab: ProjectTab): boolean {
    switch (tab) {
      case "tokens":
        return managesEnvironment.value;
      case "settings":
        return managesProject.value;
      case "access":
        return readsAccess.value;
      default:
        return true;
    }
  }

  // A link to a tab you can't use falls back to Secrets.
  watch(
    [environment, activeTab, () => project.value?.level],
    ([current, tab]) => {
      if (!current || allowed(tab)) return;
      if (tab === "access" && project.value === null) return;
      if (tab === "settings" && project.value === null) return;
      void router.replace(environmentLocation(current.name, "secrets"));
    },
    { immediate: true },
  );

  function environmentLocation(
    name: string,
    tab: keyof typeof ENVIRONMENT_ROUTES,
  ): RouteLocationRaw {
    return {
      name: ENVIRONMENT_ROUTES[tab],
      params: { projectRef: projectName.value, environmentName: name },
    };
  }

  function projectLocation(tab: keyof typeof PROJECT_ROUTES): RouteLocationRaw {
    return {
      name: PROJECT_ROUTES[tab],
      params: { projectRef: projectName.value },
    };
  }

  /**
   * Environment tabs need an environment to point at; project tabs don't,
   * so a project manager can still reach Access and Settings (and delete
   * the project) before it has any environments.
   */
  const tabs = computed(() => {
    const name = environment.value?.name;
    const all: { key: ProjectTab; label: string; to: RouteLocationRaw }[] = [];
    if (name) {
      all.push(
        {
          key: "secrets",
          label: "Secrets",
          to: environmentLocation(name, "secrets"),
        },
        {
          key: "compare",
          label: "Compare",
          to: environmentLocation(name, "compare"),
        },
        {
          key: "tokens",
          label: "Tokens",
          to: environmentLocation(name, "tokens"),
        },
      );
    }
    all.push({ key: "access", label: "Access", to: projectLocation("access") });
    if (name) {
      all.push({
        key: "activity",
        label: "Activity",
        to: environmentLocation(name, "activity"),
      });
    }
    all.push({
      key: "settings",
      label: "Settings",
      to: projectLocation("settings"),
    });
    return all.filter((tab) => allowed(tab.key));
  });

  function switchProject(target: Project): void {
    if (target.id === project.value?.id) return;
    void router.push({ name: "project", params: { projectRef: target.name } });
  }

  /**
   * Opens another environment. Environment tabs stay where they are when
   * you can use them there; project tabs move to its Secrets.
   */
  function switchEnvironment(target: Environment): void {
    if (target.id === environment.value?.id) return;
    const tab = activeTab.value;
    if (tab === "tokens" && reaches(target.level ?? "manage", "manage")) {
      void router.push(environmentLocation(target.name, "tokens"));
    } else if (tab === "activity" || tab === "compare") {
      // Compare re-bases on the environment you switch to.
      void router.push(environmentLocation(target.name, tab));
    } else {
      void router.push(environmentLocation(target.name, "secrets"));
    }
  }

  function openActivity(): void {
    if (environment.value)
      void router.push(environmentLocation(environment.value.name, "activity"));
  }

  /** Creates an environment from the empty-project state and opens it. */
  async function createEnvironment(name: string): Promise<void> {
    const created = await environmentsApi.createEnvironment(
      projectName.value,
      name,
    );
    await openEnvironment(created.name);
  }

  /** Opens the Secrets of an environment that was just created. */
  async function openEnvironment(name: string): Promise<void> {
    // Refresh in place first, so the new environment is in the list when
    // its page opens and the panels don't drop back to a skeleton.
    await refresh();
    await router.push(environmentLocation(name, "secrets"));
  }

  return {
    projects,
    environments,
    loadError,
    projectsError,
    reloadProjects,
    reload,
    refresh,
    projectRef,
    projectName,
    project,
    environment,
    environmentName,
    environmentMissing,
    activeTab,
    projectLevel,
    tabs,
    meter,
    managesEnvironment,
    managesProject,
    switchProject,
    switchEnvironment,
    openActivity,
    createEnvironment,
    openEnvironment,
  };
}

export type ProjectController = ReturnType<typeof useProjectController>;
