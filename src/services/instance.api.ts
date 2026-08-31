import { apiRequest } from "./http.client";

/** Safe, read-only instance status. Never exposes paths or key material. */
export interface InstanceStatus {
  version: string;
  publicUrl: string;
  initializationState: string;
  databaseHealth: string;
  keyAvailability: string;
  /** Always `"restart-required"` in v0.0.8: config changes need a restart. */
  configurationReload: string;
}

const BASE = "/api/v1/instance";

export async function fetchInstanceStatus(): Promise<InstanceStatus> {
  const { data } = await apiRequest<InstanceStatus>(BASE);
  return data;
}
