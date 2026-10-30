import { apiRequest } from "./http.client";
import type { AccountRole } from "./auth.api";

export interface User {
  id: string;
  /** An AI agent's name. */
  email: string;
  role: "root" | "admin" | "member" | "ai_agent";
  /** The person who answers for an AI agent. */
  ownerId?: string | null;
  createdAt: string;
  updatedAt: string;
  hasPassword: boolean;
  ssoLinked: boolean;
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
  role: Exclude<AccountRole, "root"> = "member",
): Promise<User> {
  const { data } = await apiRequest<User>(BASE, {
    method: "POST",
    body: password ? { email, password, role } : { email, role },
  });
  return data;
}

export async function updateUser(
  id: string,
  body: {
    email?: string;
    password?: string;
    role?: Exclude<AccountRole, "root" | "ai_agent">;
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
