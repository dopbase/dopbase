import { computed, ref, watch, type Ref } from "vue";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import { reaches, setGrant } from "~/services/access.api";
import type { Grant, Level, PrincipalType } from "~/services/access.api";
import type { Environment, Project } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatLevel } from "~/utils/format";
import {
  ENVIRONMENT_LEVELS,
  grantSentence,
  LEVEL_HINTS,
  PROJECT_LEVELS,
} from "~/utils/level";

/** Who the dialog gives access to. */
export interface GrantPrincipal {
  type: PrincipalType;
  id: string;
  /** An email, an agent's name, or a group's name. */
  label: string;
  /** The most their role allows; groups have no cap of their own. */
  maxLevel?: Level;
}

export type GrantWhere = "project" | "environment";

export interface LevelChoice {
  level: Level;
  label: string;
  hint: string;
  /** Above what their role allows, or Project manager on one environment. */
  disabled: boolean;
}

/**
 * Give access dialog, starting from a person, AI agent or group: pick a
 * project you manage, the whole project or one environment, and a level.
 * Levels above the principal's role are shown but can't be picked. With
 * `editing`, the grant's place is fixed and only the level changes.
 *
 * The server checks it all again: you need Project manager on the project,
 * and nobody can be given more than their role allows.
 */
export function useGiveAccessDialog(
  open: Ref<boolean>,
  principal: Ref<GrantPrincipal>,
  existing: Ref<Grant[]>,
  editing: Ref<Grant | null>,
) {
  const { runWithReauth } = useReauthentication();

  const projects = ref<Project[] | null>(null);
  const environments = ref<Environment[]>([]);
  const projectId = ref("");
  const where = ref<GrantWhere>("project");
  const environmentId = ref("");
  const level = ref<Level>("read");
  const saving = ref(false);
  const error = ref<string | null>(null);
  let loadVersion = 0;

  /** Only projects you can give access in. */
  const manageable = computed(() =>
    (projects.value ?? []).filter((project) =>
      reaches(project.level ?? "project_manager", "project_manager"),
    ),
  );
  const project = computed(
    () =>
      manageable.value.find((candidate) => candidate.id === projectId.value) ??
      null,
  );

  async function loadProjects(): Promise<void> {
    projects.value = null;
    try {
      projects.value = await projectsApi.listProjects();
    } catch (cause) {
      projects.value = [];
      error.value = errorMessage(cause, "Could not load your projects.");
    }
  }

  async function loadEnvironments(id: string): Promise<void> {
    const current = ++loadVersion;
    environments.value = [];
    const target = manageable.value.find((candidate) => candidate.id === id);
    if (!target) return;
    try {
      const list = await environmentsApi.listEnvironments(target.name);
      if (current !== loadVersion) return;
      environments.value = list;
      if (!list.some((candidate) => candidate.id === environmentId.value))
        environmentId.value = list[0]?.id ?? "";
    } catch (cause) {
      if (current === loadVersion)
        error.value = errorMessage(cause, "Could not load its environments.");
    }
  }

  watch(
    open,
    async (isOpen) => {
      if (!isOpen) return;
      error.value = null;
      saving.value = false;
      const grant = editing.value;
      if (grant) {
        // Changing a grant: its place is fixed.
        projectId.value = grant.projectId;
        where.value =
          grant.scopeType === "environment" ? "environment" : "project";
        environmentId.value =
          grant.scopeType === "environment" ? grant.scopeId : "";
        level.value = grant.level;
        return;
      }
      where.value = "project";
      environmentId.value = "";
      level.value = "read";
      await loadProjects();
      projectId.value = manageable.value[0]?.id ?? "";
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  watch([projectId, where], ([id, scope]) => {
    if (editing.value) return;
    if (scope === "environment") void loadEnvironments(id);
    // Project manager only exists on a whole project.
    if (scope === "environment" && level.value === "project_manager")
      level.value = "manage";
    if (scope === "project" && level.value === "none") level.value = "metadata";
  });

  const levels = computed<LevelChoice[]>(() => {
    const cap = principal.value.maxLevel;
    return (
      where.value === "project" ? PROJECT_LEVELS : ENVIRONMENT_LEVELS
    ).map((candidate) => ({
      level: candidate,
      label: formatLevel(candidate),
      hint: LEVEL_HINTS[candidate],
      disabled: cap !== undefined && !reaches(cap, candidate),
    }));
  });

  /** The grant this would replace, if they already have one there. */
  const replaces = computed(() => {
    if (editing.value) return editing.value;
    return (
      existing.value.find(
        (grant) =>
          grant.principalId === principal.value.id &&
          (where.value === "project"
            ? grant.scopeType === "project" && grant.scopeId === projectId.value
            : grant.scopeType === "environment" &&
              grant.scopeId === environmentId.value),
      ) ?? null
    );
  });

  const target = computed(() => {
    const name =
      editing.value?.scopeLabel ?? project.value?.name ?? "this project";
    if (editing.value)
      return editing.value.scopeType === "environment"
        ? editing.value.scopeLabel
        : `all of ${name}`;
    if (where.value === "project") return `all of ${name}`;
    const environment = environments.value.find(
      (candidate) => candidate.id === environmentId.value,
    );
    return `${name} / ${environment?.name ?? "this environment"}`;
  });

  const summary = computed(() => {
    const sentence = grantSentence(
      principal.value.label,
      principal.value.type === "group",
      level.value,
      target.value,
    );
    return replaces.value && !editing.value
      ? `${sentence} This replaces their current grant there.`
      : sentence;
  });

  async function submit(): Promise<boolean> {
    error.value = null;
    const grant = editing.value;
    const scopeType = grant ? grant.scopeType : where.value;
    const scope = grant
      ? grant.scopeId
      : where.value === "project"
        ? projectId.value
        : environmentId.value;
    if (!scope) {
      error.value =
        where.value === "project"
          ? "Choose a project."
          : "Choose an environment.";
      return false;
    }
    if (levels.value.find((choice) => choice.level === level.value)?.disabled) {
      error.value = `Their role allows up to ${formatLevel(principal.value.maxLevel ?? "none")}.`;
      return false;
    }
    saving.value = true;
    try {
      await runWithReauth(async () => {
        await setGrant({
          principalType: principal.value.type,
          principalId: principal.value.id,
          scopeType,
          scope,
          level: level.value,
        });
      });
      return true;
    } catch (cause) {
      error.value = errorMessage(cause, "Could not give that access.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  return {
    projects,
    manageable,
    project,
    projectId,
    where,
    environments,
    environmentId,
    level,
    levels,
    replaces,
    summary,
    saving,
    error,
    submit,
  };
}
