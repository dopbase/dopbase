import { apiRequest } from "./http.client";

export interface RunnerToken {
  id: string;
  environmentId: string;
  name: string;
  createdAt: string;
  expiresAt: string | null;
  lastUsedAt: string | null;
  revokedAt: string | null;
}

export interface CreateTokenRequest {
  name: string;
  role: string;
  expiresIn?: string;
}

export interface CreatedTokenResponse {
  token: RunnerToken;
  /** Shown exactly once. Only its hash is persisted server-side. */
  plaintextToken: string;
}

const base = (environmentId: string): string =>
  `/api/v1/environments/${encodeURIComponent(environmentId)}/tokens`;

export async function listTokens(
  environmentId: string,
  signal?: AbortSignal,
): Promise<RunnerToken[]> {
  const { data } = await apiRequest<RunnerToken[]>(base(environmentId), {
    signal,
  });
  return data;
}

export async function createToken(
  environmentId: string,
  request: CreateTokenRequest,
): Promise<CreatedTokenResponse> {
  const { data } = await apiRequest<CreatedTokenResponse>(base(environmentId), {
    method: "POST",
    body: request,
  });
  return data;
}

export interface CreatedWorkspaceTokenResponse {
  token: WorkspaceToken;
  plaintextToken: string;
}

export interface WorkspaceToken {
  id: string;
  workspaceId: string;
  name: string;
  createdAt: string;
  expiresAt: string | null;
  lastUsedAt: string | null;
  revokedAt: string | null;
}

export interface CreateWorkspaceTokenRequest {
  name: string;
  role: string;
  expiresIn?: string;
}

const workspaceBase = (workspaceId: string): string =>
  `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/tokens`;

export async function listWorkspaceTokens(
  workspaceId: string,
  signal?: AbortSignal,
): Promise<WorkspaceToken[]> {
  const { data } = await apiRequest<WorkspaceToken[]>(
    workspaceBase(workspaceId),
    { signal },
  );
  return data;
}

export async function createWorkspaceToken(
  workspaceId: string,
  request: CreateWorkspaceTokenRequest,
): Promise<CreatedWorkspaceTokenResponse> {
  const { data } = await apiRequest<CreatedWorkspaceTokenResponse>(
    workspaceBase(workspaceId),
    { method: "POST", body: request },
  );
  return data;
}

export async function revokeToken(tokenId: string): Promise<unknown> {
  const { data } = await apiRequest<unknown>(
    `/api/v1/tokens/${encodeURIComponent(tokenId)}/revoke`,
    { method: "POST" },
  );
  return data;
}
