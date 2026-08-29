import { apiRequest } from "./http.client";
import type { AffectedCounts } from "./projects.api";

export interface Environment {
  id: string;
  projectId: string;
  projectName: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

const BASE = "/api/v1/environments";

/**
 * Lists environments, optionally scoped to one project. `project` accepts a
 * project name or immutable id.
 */
export async function listEnvironments(
  project?: string,
): Promise<Environment[]> {
  const query =
    project === undefined ? "" : `?project=${encodeURIComponent(project)}`;
  const { data } = await apiRequest<Environment[]>(`${BASE}${query}`);
  return data;
}

export async function createEnvironment(
  projectRef: string,
  name: string,
): Promise<Environment> {
  const { data } = await apiRequest<Environment>(
    `/api/v1/projects/${encodeURIComponent(projectRef)}/environments`,
    { method: "POST", body: { name } },
  );
  return data;
}

export async function showEnvironment(id: string): Promise<Environment> {
  const { data } = await apiRequest<Environment>(
    `${BASE}/${encodeURIComponent(id)}`,
  );
  return data;
}

export async function renameEnvironment(
  id: string,
  name: string,
): Promise<Environment> {
  const { data } = await apiRequest<Environment>(
    `${BASE}/${encodeURIComponent(id)}`,
    { method: "PATCH", body: { name } },
  );
  return data;
}

export async function deleteEnvironment(id: string): Promise<AffectedCounts> {
  const { data } = await apiRequest<{ affected: AffectedCounts }>(
    `${BASE}/${encodeURIComponent(id)}`,
    { method: "DELETE" },
  );
  return data.affected;
}
