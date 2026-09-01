import { apiRequest } from "./http.client";

export interface RunnerToken {
  id: string;
  environmentId: string;
  name: string;
  createdAt: string;
  lastUsedAt: string | null;
  revokedAt: string | null;
}

export interface CreateTokenRequest {
  name: string;
  role: string;
}

export interface CreatedTokenResponse {
  token: RunnerToken;
  /** Shown exactly once; only its hash is persisted server-side. */
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

export async function revokeToken(tokenId: string): Promise<RunnerToken> {
  const { data } = await apiRequest<RunnerToken>(
    `/api/v1/tokens/${encodeURIComponent(tokenId)}/revoke`,
    { method: "POST" },
  );
  return data;
}
