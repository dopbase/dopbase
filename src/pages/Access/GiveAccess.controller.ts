import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import {
  fetchGroup,
  fetchUserAccess,
  reaches,
  setGrant,
} from "~/services/access.api";
import type { Grant, Level, ScopeType } from "~/services/access.api";
import type { Environment, Project } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatLevel, formatList } from "~/utils/format";
import {
  grantSentence,
  levelChoices,
  type GrantPrincipal,
} from "~/utils/level";

export type GiveAccessKind = "user" | "agent" | "group";
export type GrantWhere = "project" | "environment";

const KIND_FOR_ROUTE: Record<string, GiveAccessKind> = {
  "user-access-give": "user",
  "agent-access-give": "agent",
  "group-access-give": "group",
};

export interface ProjectChoice {
  project: Project;
  environmentCount: number;
  /** The level they hold on the whole project already, if any. */
  current: Level | null;
}

export interface EnvironmentChoice {
  environment: Environment;
  /** The level they hold on this environment already, if any. */
  current: Level | null;
}

/** One grant the page will save. */
export interface PlannedGrant {
  scopeType: ScopeType;
  scopeId: string;
  /** `payments` or `payments / staging`. */
  label: string;
  /** The level it replaces, when they already hold a grant there. */
  replaces: Level | null;
}

/** "4 environments · has Write now" under a project's name. */
export function projectMeta(choice: ProjectChoice): string {
  return [
    `${choice.environmentCount} ${choice.environmentCount === 1 ? "environment" : "environments"}`,
    choice.current ? `has ${formatLevel(choice.current)} now` : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

/**
 * Give access page for a person, AI agent or group, reached from their
 * Access tab. Pick a project you manage, the whole project or some of its
 * environments, and one level. Each place becomes one grant; a grant they
 * already hold there is replaced.
 *
 * The server checks it all again: you need Project manager on the project,
 * and nobody can be given more than their role allows.
 */
export function useGiveAccessController() {
  const route = useRoute();
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const kind = computed<GiveAccessKind>(
    () => KIND_FOR_ROUTE[String(route.name)] ?? "user",
  );
  const principalId = computed(() =>
    String(
      route.params.userId ?? route.params.agentId ?? route.params.groupId ?? "",
    ),
  );

  const principal = ref<GrantPrincipal | null>(null);
  /** The grants the principal holds directly, not through a group. */
  const grants = ref<Grant[]>([]);
  const projects = ref<Project[] | null>(null);
  const environments = ref<Environment[]>([]);
  const loadError = ref<string | null>(null);

  const projectFilter = ref("");
  const environmentFilter = ref("");
  const projectId = ref("");
  const where = ref<GrantWhere>("project");
  const environmentIds = ref<string[]>([]);
  const level = ref<Level>("read");
  const saving = ref(false);
  const error = ref<string | null>(null);

  async function loadPrincipal(): Promise<void> {
    const id = principalId.value;
    if (kind.value === "group") {
      const detail = await fetchGroup(id);
      principal.value = { type: "group", id, label: detail.group.name };
      grants.value = detail.grants.filter(
        (grant) => grant.principalType === "group" && grant.principalId === id,
      );
      return;
    }
    const access = await fetchUserAccess(id);
    principal.value = {
      type: "user",
      id,
      label: access.email,
      maxLevel: access.maxLevel,
    };
    grants.value = access.grants.filter(
      (grant) => grant.principalType === "user" && grant.principalId === id,
    );
  }

  async function load(): Promise<void> {
    loadError.value = null;
    principal.value = null;
    projects.value = null;
    try {
      const [, projectList, environmentList] = await Promise.all([
        loadPrincipal(),
        projectsApi.listProjects(),
        environmentsApi.listEnvironments(),
      ]);
      projects.value = projectList;
      environments.value = environmentList;
      // With one project to choose from, choose it.
      if (manageable.value.length === 1)
        projectId.value = manageable.value[0]!.project.id;
    } catch (cause) {
      loadError.value = errorMessage(
        cause,
        kind.value === "group"
          ? "Could not load the group."
          : "Could not load their access.",
      );
    }
  }

  onMounted(load);

  function currentOn(scopeType: ScopeType, scopeId: string): Level | null {
    return (
      grants.value.find(
        (grant) => grant.scopeType === scopeType && grant.scopeId === scopeId,
      )?.level ?? null
    );
  }

  /** Projects you can give access in: the ones you manage. */
  const manageable = computed<ProjectChoice[]>(() =>
    (projects.value ?? [])
      .filter((project) =>
        reaches(project.level ?? "project_manager", "project_manager"),
      )
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((project) => ({
        project,
        environmentCount: environments.value.filter(
          (environment) => environment.projectId === project.id,
        ).length,
        current: currentOn("project", project.id),
      })),
  );

  const visibleProjects = computed(() => {
    const needle = projectFilter.value.trim().toLowerCase();
    return needle
      ? manageable.value.filter((choice) =>
          choice.project.name.toLowerCase().includes(needle),
        )
      : manageable.value;
  });

  const project = computed(
    () =>
      manageable.value.find((choice) => choice.project.id === projectId.value)
        ?.project ?? null,
  );

  const environmentChoices = computed<EnvironmentChoice[]>(() =>
    environments.value
      .filter((environment) => environment.projectId === projectId.value)
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((environment) => ({
        environment,
        current: currentOn("environment", environment.id),
      })),
  );

  const visibleEnvironments = computed(() => {
    const needle = environmentFilter.value.trim().toLowerCase();
    return needle
      ? environmentChoices.value.filter((choice) =>
          choice.environment.name.toLowerCase().includes(needle),
        )
      : environmentChoices.value;
  });

  function chooseProject(id: string): void {
    if (projectId.value === id) return;
    projectId.value = id;
    environmentIds.value = [];
    environmentFilter.value = "";
  }

  function toggleEnvironment(id: string, on: boolean): void {
    const rest = environmentIds.value.filter((candidate) => candidate !== id);
    environmentIds.value = on ? [...rest, id] : rest;
  }

  // Keep the level valid when the scope changes what is allowed.
  watch(where, (scope) => {
    // Project manager only exists on a whole project.
    if (scope === "environment" && level.value === "project_manager")
      level.value = "manage";
    if (scope === "project" && level.value === "none") level.value = "metadata";
  });

  const levels = computed(() =>
    levelChoices(where.value, principal.value?.maxLevel),
  );

  /** What saving would write, one entry per grant. */
  const planned = computed<PlannedGrant[]>(() => {
    const chosen = project.value;
    if (!chosen) return [];
    if (where.value === "project")
      return [
        {
          scopeType: "project",
          scopeId: chosen.id,
          label: chosen.name,
          replaces: currentOn("project", chosen.id),
        },
      ];
    return environmentChoices.value
      .filter((choice) => environmentIds.value.includes(choice.environment.id))
      .map((choice) => ({
        scopeType: "environment" as const,
        scopeId: choice.environment.id,
        label: `${chosen.name} / ${choice.environment.name}`,
        replaces: choice.current,
      }));
  });

  const summary = computed(() => {
    const chosen = project.value;
    const who = principal.value;
    if (!who || !chosen) return "";
    const target =
      where.value === "project"
        ? `all of ${chosen.name}`
        : `${chosen.name} / ${formatList(
            planned.value.map((grant) => grant.label.split(" / ")[1]!),
          )}`;
    return grantSentence(who.label, who.type === "group", level.value, target);
  });

  /**
   * A whole-project grant below Project manager skips protected
   * environments; say which ones it leaves out.
   */
  const protectedNote = computed(() => {
    if (where.value !== "project" || level.value === "project_manager")
      return "";
    const names = environmentChoices.value
      .filter((choice) => choice.environment.protected)
      .map((choice) => choice.environment.name);
    if (names.length === 0) return "";
    return `${formatList(names)} ${names.length === 1 ? "is" : "are"} protected, so this grant doesn't reach ${names.length === 1 ? "it" : "them"}. Give access to ${names.length === 1 ? "it" : "them"} on its own.`;
  });

  const backTo = computed<RouteLocationRaw>(() => {
    switch (kind.value) {
      case "agent":
        return { name: "agent", params: { agentId: principalId.value } };
      case "group":
        return { name: "group-access", params: { groupId: principalId.value } };
      default:
        return { name: "user-access", params: { userId: principalId.value } };
    }
  });

  async function submit(): Promise<boolean> {
    error.value = null;
    const who = principal.value;
    if (!who) return false;
    if (!project.value) {
      error.value = "Choose a project.";
      return false;
    }
    if (planned.value.length === 0) {
      error.value = "Choose at least one environment.";
      return false;
    }
    if (levels.value.find((choice) => choice.level === level.value)?.disabled) {
      error.value = `Their role allows up to ${formatLevel(who.maxLevel ?? "none")}.`;
      return false;
    }
    saving.value = true;
    const todo = [...planned.value];
    let saved = 0;
    try {
      // One grant per place. Setting a grant replaces the one there, so
      // retrying after a sign-in prompt is safe.
      await runWithReauth(async () => {
        for (const grant of todo.slice(saved)) {
          await setGrant({
            principalType: who.type,
            principalId: who.id,
            scopeType: grant.scopeType,
            scope: grant.scopeId,
            level: level.value,
          });
          saved += 1;
        }
      });
      await router.push(backTo.value);
      return true;
    } catch (cause) {
      const reason = errorMessage(cause, "Could not give that access.");
      error.value =
        saved > 0
          ? `Saved ${saved} of ${todo.length}. ${todo[saved]!.label} failed: ${reason}`
          : reason;
      // Show what is held now, so a second try only redoes the rest.
      if (saved > 0) await loadPrincipal().catch(() => undefined);
      return false;
    } finally {
      saving.value = false;
    }
  }

  function cancel(): void {
    void router.push(backTo.value);
  }

  return {
    kind,
    principal,
    projects,
    loadError,
    load,
    projectFilter,
    manageable,
    visibleProjects,
    projectId,
    project,
    chooseProject,
    where,
    environmentChoices,
    environmentFilter,
    visibleEnvironments,
    environmentIds,
    toggleEnvironment,
    level,
    levels,
    planned,
    summary,
    protectedNote,
    backTo,
    saving,
    error,
    submit,
    cancel,
  };
}

export type GiveAccessController = ReturnType<typeof useGiveAccessController>;
