import { computed, onUnmounted, ref, watch } from "vue";
import type { Ref } from "vue";
import * as environmentsApi from "~/services/environments.api";
import { reaches } from "~/services/access.api";

/** Another environment in the project that new secrets can start from. */
export interface CopySource {
  id: string;
  name: string;
  secretCount: number;
  /** Whether you can read its values, so they can be copied too. */
  canCopyValues: boolean;
}

/**
 * Empty environment controller: decides what an empty environment shows a
 * writer. When no other environment in the project has secrets, the
 * project is new and the first-run checklist walks through it. Otherwise
 * the start panel offers to copy keys from the environment with the most.
 *
 * The project's environments are fetched fresh each time an environment
 * turns up empty, so counts changed elsewhere since the page loaded are
 * seen. Older servers send no counts; their environments read as empty,
 * which falls back to the checklist.
 */
export function useEmptyEnvironmentController(
  environmentId: Ref<string>,
  projectName: Ref<string>,
  active: Ref<boolean>,
) {
  /** Null until loaded. */
  const sources = ref<CopySource[] | null>(null);
  let request: AbortController | null = null;

  async function load(): Promise<void> {
    request?.abort();
    const current = new AbortController();
    request = current;
    sources.value = null;
    const target = environmentId.value;
    try {
      const environments = await environmentsApi.listEnvironments(
        projectName.value,
        current.signal,
      );
      if (current.signal.aborted) return;
      sources.value = environments
        .filter(
          (environment) =>
            environment.id !== target && (environment.secretCount ?? 0) > 0,
        )
        .sort(
          (a, b) =>
            (b.secretCount ?? 0) - (a.secretCount ?? 0) ||
            a.name.localeCompare(b.name),
        )
        .map((environment) => ({
          id: environment.id,
          name: environment.name,
          secretCount: environment.secretCount ?? 0,
          canCopyValues:
            !environment.level || reaches(environment.level, "read"),
        }));
    } catch {
      if (current.signal.aborted) return;
      // Without the list, the checklist still gets someone started.
      sources.value = [];
    }
  }

  watch(
    [active, environmentId],
    ([isActive]) => {
      if (isActive) void load();
      else request?.abort();
    },
    { immediate: true },
  );
  onUnmounted(() => request?.abort());

  const view = computed<"loading" | "first-run" | "start">(() => {
    if (sources.value === null) return "loading";
    return sources.value.length === 0 ? "first-run" : "start";
  });

  return { sources, view };
}
