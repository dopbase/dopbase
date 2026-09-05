import { apiRequest } from "./http.client";

/**
 * Public bootstrap state. Only `setupRequired` or `ready` is ever revealed;
 * anything else is normalized to `ready`.
 */
export interface BootstrapStatus {
  state: string;
}

export interface BootstrapAdminRequest {
  setupToken: string;
  email: string;
  password: string;
}

export interface BootstrapAdminResponse {
  adminId: string;
  email: string;
  csrfToken: string;
}

const BASE = "/api/v1/bootstrap";

export async function fetchBootstrapStatus(): Promise<BootstrapStatus> {
  const { data } = await apiRequest<BootstrapStatus>(`${BASE}/status`);
  return data;
}

/** Claims an uninitialized server. Starts the returned browser session. */
export async function bootstrapAdmin(
  request: BootstrapAdminRequest,
): Promise<BootstrapAdminResponse> {
  const { data } = await apiRequest<BootstrapAdminResponse>(`${BASE}/admin`, {
    method: "POST",
    body: {
      setupToken: request.setupToken,
      email: request.email,
      password: request.password,
    },
    anonymous: true,
  });
  return data;
}

export interface BootstrapRestoreResponse {
  message: string;
  restored: boolean;
  key: string;
  size: number;
}

/** Restores an uninitialized server from an encrypted .dop backup file and optional master key. */
export async function bootstrapRestore(
  file: File,
  setupToken: string,
  masterKey?: File | string,
): Promise<BootstrapRestoreResponse> {
  const formData = new FormData();
  formData.append("file", file);
  formData.append("setup_token", setupToken.trim());
  if (masterKey) {
    if (typeof masterKey === "string") {
      formData.append(
        "master_key",
        new Blob([masterKey.trim()], { type: "text/plain" }),
        "master.key",
      );
    } else {
      formData.append("master_key", masterKey);
    }
  }
  const { data } = await apiRequest<BootstrapRestoreResponse>(
    `${BASE}/restore`,
    {
      method: "POST",
      body: formData,
      anonymous: true,
    },
  );
  return data;
}
