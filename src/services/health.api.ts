import { apiRequest } from "./http.client";

export interface HealthResponse {
  product: string;
  version: string;
  apiVersion: string;
  status: string;
}

const BASE = "/api/v1/health";

/** Public health check; identifies the product, binary, and API version. */
export async function fetchHealth(): Promise<HealthResponse> {
  const { data } = await apiRequest<HealthResponse>(BASE);
  return data;
}
