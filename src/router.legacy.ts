import type { NavigationGuardWithThis } from "vue-router";
import { listEnvironments, showEnvironment } from "~/services/environments.api";
import { showProject } from "~/services/projects.api";

/** Project tabs that sit where an environment name would. */
const PROJECT_TABS: Record<string, string> = {
  access: "project-access",
  settings: "project-settings",
};

/** Whether the viewer reaches a project `p` page called `name`. */
async function projectPOwns(name: string): Promise<boolean> {
  try {
    if (PROJECT_TABS[name]) {
      await showProject("p");
      return true;
    }
    const environments = await listEnvironments("p");
    return environments.some((environment) => environment.name === name);
  } catch {
    return false;
  }
}

/**
 * `/projects/p/<x>` is both an old link to project `<x>` and, for a
 * project named `p`, its Access or Settings tab or its environment `<x>`.
 * The page of `p` wins when the viewer can open it; anything else is the
 * old link.
 */
export const redirectLegacyProject: NavigationGuardWithThis<undefined> = async (
  to,
) => {
  const ref = String(to.params.projectRef);
  const kept = { query: to.query, hash: to.hash, replace: true };
  if (!(await projectPOwns(ref))) {
    return { name: "project", params: { projectRef: ref }, ...kept };
  }
  const tab = PROJECT_TABS[ref];
  return tab
    ? { name: tab, params: { projectRef: "p" }, ...kept }
    : {
        name: "environment",
        params: { projectRef: "p", environmentName: ref },
        ...kept,
      };
};

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
