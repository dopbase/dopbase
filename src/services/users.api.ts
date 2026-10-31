import { apiRequest } from "./http.client";
import type { AccountRole } from "./auth.api";

export interface User {
  id: string;
  /** An AI agent's name. */
  email: string;
  /** The fixed role closest to the account's role. Prefer `roleName`. */
  role: "root" | "admin" | "member" | "ai_agent";
  /** Empty for root. */
  roleId?: string | null;
  /** `Root` for root. */
  roleName?: string;
  /** The person who answers for an AI agent. */
  ownerId?: string | null;
  createdAt: string;
  updatedAt: string;
  hasPassword: boolean;
  ssoLinked: boolean;
  /** A disabled account can't sign in, and its tokens stop working. */
  status: "active" | "disabled";
  /** The last interactive sign-in, or null if they never signed in. */
  lastSignInAt: string | null;
  /** The groups the account belongs to. */
  groups: { id: string; name: string }[];
}
const BASE = "/api/v1/users";
export async function fetchUsers(): Promise<User[]> {
  const { data } = await apiRequest<User[]>(BASE);
  return data;
}

/**
 * Leave `password` empty for an account that uses single sign-on only, and
 * for an AI agent, whose `email` is its name.
 */
export async function createUser(
  email: string,
  password: string,
  role: Exclude<AccountRole, "root"> | { roleId: string } = "member",
): Promise<User> {
  const chosen = typeof role === "string" ? { role } : role;
  const { data } = await apiRequest<User>(BASE, {
    method: "POST",
    body: password ? { email, password, ...chosen } : { email, ...chosen },
  });
  return data;
}

export async function updateUser(
  id: string,
  body: {
    email?: string;
    password?: string;
    role?: Exclude<AccountRole, "root" | "ai_agent">;
    roleId?: string;
    ownerId?: string;
  },
): Promise<User> {
  const { data } = await apiRequest<User>(`${BASE}/${encodeURIComponent(id)}`, {
    method: "PATCH",
    body,
  });
  return data;
}

export async function deleteUser(id: string): Promise<void> {
  await apiRequest(`${BASE}/${encodeURIComponent(id)}`, { method: "DELETE" });
}

/** Remove a user's single sign-on link. */
export async function unlinkUserSso(id: string): Promise<User> {
  const { data } = await apiRequest<User>(
    `${BASE}/${encodeURIComponent(id)}/sso`,
    { method: "DELETE" },
  );
  return data;
}

export async function fetchUser(id: string): Promise<User> {
  const { data } = await apiRequest<User>(`${BASE}/${encodeURIComponent(id)}`);
  return data;
}

/** Stop an account signing in; its sessions end and its tokens stop. */
export async function disableUser(id: string): Promise<User> {
  const { data } = await apiRequest<User>(
    `${BASE}/${encodeURIComponent(id)}/disable`,
    { method: "POST" },
  );
  return data;
}

export async function enableUser(id: string): Promise<User> {
  const { data } = await apiRequest<User>(
    `${BASE}/${encodeURIComponent(id)}/enable`,
    { method: "POST" },
  );
  return data;
}
