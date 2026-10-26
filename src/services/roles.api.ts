import { apiRequest } from "./http.client";
import type { Capability, Level } from "./access.api";

export type SignIn = "interactive" | "token_only";
export type TokenScope = "none" | "data" | "audit";

/** A role: what holders may do on the instance and how deep into data. */
export interface Role {
  id: string;
  name: string;
  description: string;
  /** Built-in roles can't change or be deleted. */
  builtin: boolean;
  capabilities: Capability[];
  maxLevel: Level;
  signIn: SignIn;
  requiresOwner: boolean;
  tokenScope: TokenScope;
  tokenMaxLevel: Level;
  tokenDefaultTtlDays: number;
  tokenMaxTtlDays: number;
  rank: number;
  /** Accounts holding the role. */
  members: number;
  /** Whether you may give this role to someone. */
  assignable: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface RoleSettings {
  name: string;
  description?: string;
  capabilities: Capability[];
  maxLevel: Level;
  /** Only when creating; it never changes. */
  signIn?: SignIn;
  tokenScope?: TokenScope;
  tokenMaxLevel?: Level;
  tokenDefaultTtlDays?: number;
  tokenMaxTtlDays?: number;
  rank?: number;
}

/** Capabilities only root holds. No role can be given them. */
export const ROOT_ONLY_CAPABILITIES: readonly Capability[] = [
  "roles.manage",
  "grants.all_projects",
  "sso.manage",
  "instance.backup",
  "instance.reset",
  "password.breakglass",
];

const BASE = "/api/v1/roles";

export async function fetchRoles(): Promise<Role[]> {
  const { data } = await apiRequest<Role[]>(BASE);
  return data;
}

export async function createRole(settings: RoleSettings): Promise<Role> {
  const { data } = await apiRequest<Role>(BASE, {
    method: "POST",
    body: settings,
  });
  return data;
}

export async function updateRole(
  id: string,
  settings: Partial<RoleSettings>,
): Promise<Role> {
  const { data } = await apiRequest<Role>(`${BASE}/${encodeURIComponent(id)}`, {
    method: "PATCH",
    body: settings,
  });
  return data;
}

export async function deleteRole(id: string): Promise<void> {
  await apiRequest(`${BASE}/${encodeURIComponent(id)}`, { method: "DELETE" });
}
