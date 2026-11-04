import { computed } from "vue";
import type { Ref } from "vue";
import { useQuery } from "@pinia/colada";
import type { Environment } from "~/services";
import { reaches } from "~/services/access.api";
import { onlyWhen } from "~/queries/helpers";
import { environmentsQuery } from "~/queries/projects";

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
 * It reads the project's environment list only while an environment is
 * empty, from the same cache entry as the project page. Any secret change in
 * this tab refreshes that list, and a copy older than a few seconds is
 * refreshed when an environment turns up empty, so counts changed elsewhere
 * are seen. Older servers send no counts; their environments read as empty,
 * which falls back to the checklist.
 */
export function useEmptyEnvironmentController(
  environmentId: Ref<string>,
  projectName: Ref<string>,
  active: Ref<boolean>,
) {
  const list = useQuery(() =>
    onlyWhen(active.value, environmentsQuery(projectName.value)),
  );

  /** Null until loaded. */
  const sources = computed<CopySource[] | null>(() => {
    const environments: Environment[] | undefined = list.data.value;
    // Without the list, the checklist still gets someone started.
    if (!environments) return list.error.value ? [] : null;
    const target = environmentId.value;
    return environments
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
        canCopyValues: !environment.level || reaches(environment.level, "read"),
      }));
  });

  const view = computed<"loading" | "first-run" | "start">(() => {
    if (sources.value === null) return "loading";
    return sources.value.length === 0 ? "first-run" : "start";
  });

  return { sources, view };
}
