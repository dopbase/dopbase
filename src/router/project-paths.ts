import type { RouteParamsRawGeneric } from "vue-router";

/** Reserved third path segment for the multi-environment overview. */
export const PROJECT_OVERVIEW_SEGMENT = "all";

export type ProjectLocationParams = {
  workspaceSlug: string;
  projectRef: string;
  environmentName?: string;
};

export function projectLocationParams(
  workspaceSlug: string,
  projectRef: string,
  environmentName?: string,
): ProjectLocationParams {
  const params: ProjectLocationParams = { workspaceSlug, projectRef };
  if (environmentName !== undefined) {
    params.environmentName = environmentName;
  }
  return params;
}

export function projectRouteParams(
  params: ProjectLocationParams,
): RouteParamsRawGeneric {
  return params as RouteParamsRawGeneric;
}
