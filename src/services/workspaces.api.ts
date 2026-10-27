import { apiRequest } from "./http.client";

export interface Workspace {
  id: string;
  name: string;
  rootPath: string;
  createdAt: string;
  updatedAt: string;
}
export interface CreateWorkspaceInput {
  name: string;
  rootPath?: string;
}
export interface UpdateWorkspaceInput {
  name: string;
  rootPath: string;
}
const BASE = "/api/v1/workspaces";
export async function listWorkspaces(signal?: AbortSignal): Promise<Workspace[]> {
  return (await apiRequest<Workspace[]>(BASE, { signal })).data;
}
export async function createWorkspace(input: CreateWorkspaceInput): Promise<Workspace> {
  return (await apiRequest<Workspace>(BASE, { method: "POST", body: input })).data;
}
export async function updateWorkspace(
  id: string,
  input: UpdateWorkspaceInput,
): Promise<Workspace> {
  return (await apiRequest<Workspace>(`${BASE}/${encodeURIComponent(id)}`, { method: "PATCH", body: input })).data;
}
export async function deleteWorkspace(id: string): Promise<void> {
  await apiRequest(`${BASE}/${encodeURIComponent(id)}`, { method: "DELETE" });
}
