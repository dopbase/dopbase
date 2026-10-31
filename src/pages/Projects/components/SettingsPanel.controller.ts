import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import { useRouter } from "vue-router";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import { fetchProjectAccess, reaches } from "~/services/access.api";
import type { Grant } from "~/services/access.api";
import type { Environment, Project } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { isActiveToken } from "./RunDialog.controller";

export interface EnvironmentStats {
  /** Null when the count couldn't be read. */
  secrets: number | null;
  tokens: number | null;
}

export type SettingsDialog =
  | { kind: "new-environment" }
  | { kind: "rename-environment"; environment: Environment };

export type SettingsDelete =
  { kind: "project" } | { kind: "environment"; environment: Environment };

/**
 * Settings panel controller: the project's name, its environments with
 * protection and counts, and deleting either. Project managers only, so
 * every count it reads is one they may see.
 *
 * `changed` runs after anything that alters the environment list, so the
 * page and its switchers stay in step.
 */
export function useSettingsPanelController(
  project: Ref<Project | null>,
  environments: Ref<Environment[]>,
  changed: () => Promise<void> | void,
) {
  const router = useRouter();

  const name = ref("");
  const savingName = ref(false);
  const nameError = ref<string | null>(null);
  const actionError = ref<string | null>(null);
  const stats = ref<Record<string, EnvironmentStats>>({});
  const grants = ref<Grant[]>([]);
  let statsVersion = 0;
  let statsRequest: AbortController | null = null;

  watch(
    () => project.value?.name,
    (current) => {
      name.value = current ?? "";
    },
    { immediate: true },
  );

  const nameChanged = computed(
    () => name.value.trim() !== "" && name.value.trim() !== project.value?.name,
  );

  async function saveName(): Promise<void> {
    const current = project.value;
    if (!current || !nameChanged.value) return;
    savingName.value = true;
    nameError.value = null;
    try {
      const renamed = await projectsApi.renameProject(
        current.id,
        name.value.trim(),
      );
      await router.replace({
        name: "project-settings",
        params: { projectRef: renamed.name },
      });
      await changed();
    } catch (cause) {
      nameError.value = errorMessage(cause, "Could not rename the project.");
    } finally {
      savingName.value = false;
    }
  }

  /** Secret and active token counts per environment, and the grants. */
  async function loadStats(): Promise<void> {
    const current = ++statsVersion;
    statsRequest?.abort();
    const request = new AbortController();
    statsRequest = request;
    const { signal } = request;
    const list = environments.value;
    const results = await Promise.all(
      list.map(async (environment) => {
        const [secrets, tokens] = await Promise.allSettled([
          secretsApi.listSecrets(environment.id, signal),
          tokensApi.listTokens(environment.id, signal),
        ]);
        return [
          environment.id,
          {
            secrets:
              secrets.status === "fulfilled" ? secrets.value.length : null,
            tokens:
              tokens.status === "fulfilled"
                ? tokens.value.filter((token) => isActiveToken(token)).length
                : null,
          },
        ] as const;
      }),
    );
    if (current !== statsVersion) return;
    stats.value = Object.fromEntries(results);
    if (!project.value) return;
    try {
      const access = await fetchProjectAccess(project.value.name, signal);
      if (current === statsVersion) grants.value = access.grants;
    } catch {
      if (current === statsVersion) grants.value = [];
    }
  }

  watch(
    () => environments.value.map((environment) => environment.id).join(","),
    loadStats,
    { immediate: true },
  );
  onUnmounted(() => {
    statsVersion++;
    statsRequest?.abort();
  });

  const canManage = (environment: Environment): boolean =>
    reaches(environment.level ?? "manage", "manage");

  async function setProtected(
    environment: Environment,
    value: boolean,
  ): Promise<void> {
    actionError.value = null;
    try {
      await environmentsApi.setEnvironmentProtected(environment.id, value);
      await changed();
    } catch (cause) {
      actionError.value = errorMessage(
        cause,
        "Could not change the environment's protection.",
      );
    }
  }

  // ---------------------------------------------------------------------
  // Create and rename environments
  // ---------------------------------------------------------------------

  const dialog = ref<SettingsDialog | null>(null);

  async function submitName(value: string): Promise<void> {
    const current = dialog.value;
    if (!current || !project.value) return;
    if (current.kind === "new-environment") {
      await environmentsApi.createEnvironment(project.value.name, value);
    } else {
      await environmentsApi.renameEnvironment(current.environment.id, value);
    }
    await changed();
  }

  // ---------------------------------------------------------------------
  // Delete
  // ---------------------------------------------------------------------

  const deleting = ref<SettingsDelete | null>(null);
  const deleteLoading = ref(false);
  const deleteError = ref<string | null>(null);

  function count(value: number, one: string, many: string) {
    return { label: value === 1 ? one : many, count: value };
  }

  const deleteCounts = computed(() => {
    const target = deleting.value;
    if (!target) return [];
    if (target.kind === "environment") {
      const own = stats.value[target.environment.id];
      return [
        count(
          own?.secrets ?? 0,
          "secret and its history",
          "secrets and their history",
        ),
        count(own?.tokens ?? 0, "runner token", "runner tokens"),
        count(
          grants.value.filter(
            (grant) => grant.scopeId === target.environment.id,
          ).length,
          "grant on this environment",
          "grants on this environment",
        ),
      ];
    }
    const all = Object.values(stats.value);
    return [
      count(environments.value.length, "environment", "environments"),
      count(
        all.reduce((sum, entry) => sum + (entry.secrets ?? 0), 0),
        "secret",
        "secrets",
      ),
      count(
        all.reduce((sum, entry) => sum + (entry.tokens ?? 0), 0),
        "runner token",
        "runner tokens",
      ),
      count(grants.value.length, "grant", "grants"),
    ];
  });

  /** "Deletes 3 environments, 44 secrets, 3 runner tokens and 5 grants." */
  const projectDeleteSummary = computed(() => {
    const all = Object.values(stats.value);
    const parts = [
      count(environments.value.length, "environment", "environments"),
      count(
        all.reduce((sum, entry) => sum + (entry.secrets ?? 0), 0),
        "secret",
        "secrets",
      ),
      count(
        all.reduce((sum, entry) => sum + (entry.tokens ?? 0), 0),
        "runner token",
        "runner tokens",
      ),
      count(grants.value.length, "grant", "grants"),
    ].map((part) => `${part.count} ${part.label}`);
    const list = `${parts.slice(0, -1).join(", ")} and ${parts.at(-1)}`;
    return `Deletes ${list}. Audit events stay.`;
  });

  function startDelete(target: SettingsDelete): void {
    deleteError.value = null;
    deleting.value = target;
  }

  function cancelDelete(): void {
    if (!deleteLoading.value) deleting.value = null;
  }

  async function confirmDelete(): Promise<void> {
    const target = deleting.value;
    if (!target || !project.value) return;
    deleteLoading.value = true;
    deleteError.value = null;
    try {
      if (target.kind === "project") {
        await projectsApi.deleteProject(project.value.id);
        deleting.value = null;
        await router.push({ name: "projects" });
        return;
      }
      await environmentsApi.deleteEnvironment(target.environment.id);
      deleting.value = null;
      await changed();
    } catch (cause) {
      deleteError.value = errorMessage(
        cause,
        target.kind === "project"
          ? "The project could not be deleted."
          : "The environment could not be deleted.",
      );
    } finally {
      deleteLoading.value = false;
    }
  }

  return {
    name,
    nameChanged,
    savingName,
    nameError,
    saveName,
    actionError,
    stats,
    canManage,
    setProtected,
    dialog,
    submitName,
    deleting,
    deleteCounts,
    deleteLoading,
    deleteError,
    projectDeleteSummary,
    startDelete,
    cancelDelete,
    confirmDelete,
  };
}
