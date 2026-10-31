import { apiRequest } from "./http.client";
import type { Capability, Level } from "./access.api";

/** Session kinds supported by the server. Browser UI always sends `"browser"`. */
export type SessionKind = "browser" | "cli";
/**
 * The legacy role label. Show `roleName` to people and gate the UI on
 * `capabilities`; roles themselves come from `GET /api/v1/roles`.
 */
export type AccountRole = "root" | "admin" | "member" | "ai_agent";

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
  role?: AccountRole;
  lastLoginAt?: string | null;
}

export interface SessionResponse {
  adminId: string;
  email: string;
  sessionKind: SessionKind;
  /** True when the password was confirmed within the last ten minutes. */
  recentAuthentication: boolean;
  role?: AccountRole;
  /** The role's display name. */
  roleName?: string;
  /** What the account may do to the instance. Gate the UI on these. */
  capabilities?: Capability[];
  /** The deepest level any grant can give the account. */
  maxLevel?: Level;
  /** What the role's personal tokens are for. */
  tokenScope?: "none" | "data" | "audit";
  lastLoginAt?: string | null;
  /** The account has a password, whether or not it may use it. */
  hasPassword?: boolean;
  /** The account can confirm sensitive actions with its password. */
  passwordConfirmation?: boolean;
  /** Provider name, when the account can confirm with single sign-on. */
  ssoConfirmation?: string | null;
  /** This tab's CSRF token, for browser sessions. */
  csrfToken?: string | null;
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

/**
 * `notifyAuthEvents: false` keeps a missing session from raising the
 * global "session expired" state, for pages that send people to sign in
 * themselves.
 */
export async function fetchSession(
  options: { notifyAuthEvents?: boolean } = {},
): Promise<SessionResponse> {
  const { data } = await apiRequest<SessionResponse>(
    `${BASE}/session`,
    options,
  );
  return data;
}

export async function reauthenticate(password: string): Promise<void> {
  await apiRequest(`${BASE}/reauthenticate`, {
    method: "POST",
    body: { password },
    notifyAuthEvents: false,
  });
}

/** `currentPassword` is empty for an account that has no password yet. */
export async function changePassword(
  currentPassword: string,
  newPassword: string,
): Promise<void> {
  await apiRequest(`${BASE}/change-password`, {
    method: "POST",
    body: { currentPassword, newPassword },
    notifyAuthEvents: false,
  });
}

/** Sign-in methods the server accepts. Public, read before login. */
export interface AuthMethods {
  password: boolean;
  /** Whether the login page shows the email and password form. */
  passwordForm: boolean;
  sso: { name: string } | null;
  cliBrowser: boolean;
}

/** Where a pending `dopbase login --browser` request came from. */
export interface CliLoginRequest {
  hostname: string | null;
  ipAddress: string | null;
  createdAt: string;
  expiresAt: string;
  /** `reauth` confirms a terminal that is already signed in. */
  purpose: "login" | "reauth";
  /** `dopbase 0.2.0 on macOS`, when the CLI said. Older CLIs don't. */
  client?: string | null;
  /** How long the CLI session an approval starts lasts. */
  sessionDays?: number;
}

export async function fetchAuthMethods(): Promise<AuthMethods> {
  const { data } = await apiRequest<AuthMethods>(`${BASE}/methods`, {
    anonymous: true,
  });
  return data;
}

export async function inspectCliLogin(
  userCode: string,
): Promise<CliLoginRequest> {
  // The page handles a missing or stale session itself, by sending the
  // person to sign in again, so no global "session expired" handling.
  const { data } = await apiRequest<CliLoginRequest>(`${BASE}/cli/inspect`, {
    method: "POST",
    body: { userCode },
    notifyAuthEvents: false,
  });
  return data;
}

export async function approveCliLogin(userCode: string): Promise<void> {
  await apiRequest(`${BASE}/cli/approve`, {
    method: "POST",
    body: { userCode },
    notifyAuthEvents: false,
  });
}

export async function denyCliLogin(userCode: string): Promise<void> {
  await apiRequest(`${BASE}/cli/deny`, {
    method: "POST",
    body: { userCode },
    notifyAuthEvents: false,
  });
}

export interface SsoExchangeResponse {
  session: LoginResponse;
  /** Admin UI path the sign-in started from, when it was a safe one. */
  redirectTo: string | null;
}

/** Whether the signed-in account has linked single sign-on. */
export interface SsoIdentity {
  available: boolean;
  providerName: string | null;
  linked: boolean;
  emailAtLink: string | null;
  linkedAt: string | null;
}

/** Full-page navigation target that starts a single sign-on login. */
export function ssoStartUrl(redirect?: string): string {
  const base = `${BASE}/sso/start`;
  return redirect ? `${base}?redirect=${encodeURIComponent(redirect)}` : base;
}

export async function exchangeSso(code: string): Promise<SsoExchangeResponse> {
  const { data } = await apiRequest<SsoExchangeResponse>(
    `${BASE}/sso/exchange`,
    { method: "POST", body: { code }, anonymous: true },
  );
  return data;
}

/** Returns the provider URL to open for linking the current account. */
export async function startSsoLink(): Promise<string> {
  const { data } = await apiRequest<{ authorizeUrl: string }>(
    `${BASE}/sso/link`,
    { method: "POST" },
  );
  return data.authorizeUrl;
}

export async function fetchSsoIdentity(): Promise<SsoIdentity> {
  const { data } = await apiRequest<SsoIdentity>(`${BASE}/sso/identity`);
  return data;
}

/** Returns the provider URL that confirms the current user again. */
export async function startSsoReauth(): Promise<string> {
  const { data } = await apiRequest<{ authorizeUrl: string }>(
    `${BASE}/sso/reauth`,
    { method: "POST" },
  );
  return data.authorizeUrl;
}

export async function unlinkSso(): Promise<void> {
  await apiRequest(`${BASE}/sso/identity`, { method: "DELETE" });
}
