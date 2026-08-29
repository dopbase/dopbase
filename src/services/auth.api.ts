import { apiRequest } from "./http.client";

/** Session kinds supported by the server. Browser UI always sends `"browser"`. */
export type SessionKind = "browser" | "cli";

export interface LoginRequest {
  email: string;
  password: string;
  sessionKind: SessionKind;
}

export interface LoginResponse {
  adminId: string;
  email: string;
  sessionKind: SessionKind;
  /** Bearer token, only present for CLI sessions. */
  token: string | null;
  /** CSRF token, only present for browser sessions. */
  csrfToken: string | null;
}

export interface SessionResponse {
  adminId: string;
  email: string;
  sessionKind: SessionKind;
  /** True when the password was confirmed within the last ten minutes. */
  recentAuthentication: boolean;
}

const BASE = "/api/v1/auth";

export async function login(request: LoginRequest): Promise<LoginResponse> {
  const { data } = await apiRequest<LoginResponse>(`${BASE}/login`, {
    method: "POST",
    body: {
      email: request.email,
      password: request.password,
      sessionKind: request.sessionKind,
    },
    anonymous: true,
  });
  return data;
}

export async function logout(): Promise<void> {
  await apiRequest(`${BASE}/logout`, { method: "POST" });
}

export async function fetchSession(): Promise<SessionResponse> {
  const { data } = await apiRequest<SessionResponse>(`${BASE}/session`);
  return data;
}

export async function reauthenticate(password: string): Promise<void> {
  await apiRequest(`${BASE}/reauthenticate`, {
    method: "POST",
    body: { password },
  });
}

export async function changePassword(
  currentPassword: string,
  newPassword: string,
): Promise<void> {
  await apiRequest(`${BASE}/change-password`, {
    method: "POST",
    body: { currentPassword, newPassword },
  });
}
