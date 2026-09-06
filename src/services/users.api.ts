import { apiRequest } from "./http.client";
import type { AccountRole } from "./auth.api";

export interface User {
  id: string;
  email: string;
  role: "root" | "admin" | "member";
  createdAt: string;
  updatedAt: string;
}
const BASE = "/api/v1/users";
export async function fetchUsers(): Promise<User[]> {
  const { data } = await apiRequest<User[]>(BASE);
  return data;
}

export async function createUser(
  email: string,
  password: string,
  role: Exclude<AccountRole, "root" | "ai_agent"> = "member",
): Promise<User> {
  const { data } = await apiRequest<User>(BASE, {
    method: "POST",
    body: { email, password, role },
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
