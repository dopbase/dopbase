import type { NavigationGuardWithThis } from "vue-router";
import { showEnvironment } from "~/services/environments.api";

const ROUTE_FOR_TAIL: Record<string, string> = {
  "": "environment",
  tokens: "environment-tokens",
  import: "environment-import",
};

/**
 * Environment links before 0.2.0 used the immutable id, as in
 * `/projects/p/payments/e/env_123/tokens`. Look the environment up and move
 * to its address by name. If it's gone or out of reach, land on the
 * project instead.
 */
export const redirectLegacyEnvironment: NavigationGuardWithThis<
  undefined
> = async (to) => {
  const projectRef = String(to.params.projectRef);
  const tail = typeof to.params.tail === "string" ? to.params.tail : "";
  try {
    const environment = await showEnvironment(String(to.params.environmentId));
    return {
      name: ROUTE_FOR_TAIL[tail] ?? "environment",
      params: {
        projectRef: environment.projectName,
        environmentName: environment.name,
      },
      query: to.query,
      replace: true,
    };
  } catch {
    return { name: "project", params: { projectRef }, replace: true };
  }
};
