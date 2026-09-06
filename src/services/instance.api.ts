import { apiRequest } from "./http.client";

/** Safe, read-only instance status. Never exposes paths or key material. */
export interface InstanceStatus {
  version: string;
  publicUrl: string;
  initializationState: string;
  databaseHealth: string;
  keyAvailability: string;
  /** Always `"restart-required"` in v0.0.14: config changes need a restart. */
  configurationReload: string;
}
export interface StatusResponse {
  version: string;
  uptimeSeconds: number;
  initializationState: string;
  databaseHealth: string;
  keyAvailability: string;
  projects: number;
  environments: number;
  secrets: number;
  humanUsers: number;
  aiAgents: number;
  activeRunnerTokens: number;
  activeAgentTokens: number;
  backups: number;
  observedAt: string;
}

export interface FactoryResetPreview {
  users: number;
  projects: number;
  environments: number;
  secrets: number;
  runnerTokens: number;
  backups: number;
  aiAgents: number;
  agentTokens: number;
}

const BASE = "/api/v1/instance";

export async function fetchInstanceStatus(): Promise<InstanceStatus> {
  const { data } = await apiRequest<InstanceStatus>(BASE);
  return data;
}
export async function fetchStatus(): Promise<StatusResponse> {
  const { data } = await apiRequest<StatusResponse>("/api/v1/status");
  return data;
}

export async function fetchFactoryResetPreview(): Promise<FactoryResetPreview> {
  const { data } = await apiRequest<FactoryResetPreview>(
    `${BASE}/factory-reset`,
  );
  return data;
}

export async function factoryReset(
  currentPassword: string,
  confirmation: string,
  acknowledged: boolean,
): Promise<void> {
  await apiRequest(`${BASE}/factory-reset`, {
    method: "POST",
    body: { currentPassword, confirmation, acknowledged },
  });
}
