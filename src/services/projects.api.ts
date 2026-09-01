import { apiRequest } from "./http.client";

export interface Project {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface AffectedCounts {
  projects: number;
  environments: number;
  secrets: number;
  tokens: number;
}

export interface SecretEntry {
  key: string;
  value: string;
}

export interface InitProjectRequest {
  projectName: string;
  environmentName: string;
  entries: SecretEntry[];
}

export interface InitProjectResponse {
  project: Project;
  environmentId: string;
  secretCount: number;
}

const BASE = "/api/v1/projects";

export async function listProjects(signal?: AbortSignal): Promise<Project[]> {
  const { data } = await apiRequest<Project[]>(BASE, { signal });
  return data;
}

export async function createProject(name: string): Promise<Project> {
  const { data } = await apiRequest<Project>(BASE, {
    method: "POST",
    body: { name },
  });
  return data;
}

/** Creates a project, its first environment, and imported secrets at once. */
export async function initProject(
  request: InitProjectRequest,
): Promise<InitProjectResponse> {
  const { data } = await apiRequest<InitProjectResponse>(`${BASE}/init`, {
    method: "POST",
    body: {
      projectName: request.projectName,
      environmentName: request.environmentName,
      entries: request.entries,
    },
  });
  return data;
}

/**
 * Fetches one project. `projectRef` accepts either the unique name or the
 * immutable `prj_…` id.
 */
export async function showProject(projectRef: string): Promise<Project> {
  const { data } = await apiRequest<Project>(
    `${BASE}/${encodeURIComponent(projectRef)}`,
  );
  return data;
}

export async function renameProject(
  projectRef: string,
  name: string,
): Promise<Project> {
  const { data } = await apiRequest<Project>(
    `${BASE}/${encodeURIComponent(projectRef)}`,
    { method: "PATCH", body: { name } },
  );
  return data;
}

export async function deleteProject(
  projectRef: string,
): Promise<AffectedCounts> {
  const { data } = await apiRequest<{ affected: AffectedCounts }>(
    `${BASE}/${encodeURIComponent(projectRef)}`,
    { method: "DELETE" },
  );
  return data.affected;
}
