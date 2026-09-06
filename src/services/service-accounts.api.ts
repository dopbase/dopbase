import { apiRequest } from "./http.client";

export interface ServiceAccount {
  id: string;
  name: string;
  role: "ai_agent";
  createdAt: string;
  updatedAt: string;
}

export interface AgentToken {
  id: string;
  serviceAccountId: string;
  name: string;
  createdAt: string;
  expiresAt: string | null;
  lastUsedAt: string | null;
  revokedAt: string | null;
}

export interface CreatedAgentToken {
  token: AgentToken;
  plaintextToken: string;
}

const BASE = "/api/v1/service-accounts";

export async function fetchServiceAccounts(): Promise<ServiceAccount[]> {
  const { data } = await apiRequest<ServiceAccount[]>(BASE);
  return data;
}

export async function createServiceAccount(
  name: string,
): Promise<ServiceAccount> {
  const { data } = await apiRequest<ServiceAccount>(BASE, {
    method: "POST",
    body: { name },
  });
  return data;
}

export async function deleteServiceAccount(id: string): Promise<void> {
  await apiRequest(`${BASE}/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function fetchAgentTokens(id: string): Promise<AgentToken[]> {
  const { data } = await apiRequest<AgentToken[]>(
    `${BASE}/${encodeURIComponent(id)}/tokens`,
  );
  return data;
}

export async function createAgentToken(
  id: string,
  name: string,
  expiresAt?: string,
): Promise<CreatedAgentToken> {
  const { data } = await apiRequest<CreatedAgentToken>(
    `${BASE}/${encodeURIComponent(id)}/tokens`,
    { method: "POST", body: { name, expiresAt } },
  );
  return data;
}

export async function revokeAgentToken(
  id: string,
  tokenId: string,
): Promise<AgentToken> {
  const { data } = await apiRequest<AgentToken>(
    `${BASE}/${encodeURIComponent(id)}/tokens/${encodeURIComponent(tokenId)}/revoke`,
    { method: "POST" },
  );
  return data;
}
