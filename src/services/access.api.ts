import { apiRequest } from "./http.client";
import type { Environment } from "./environments.api";
import type { Project } from "./projects.api";

/**
 * How far into a project or environment an account reaches. Each level
 * includes the ones before it.
 */
export type Level =
  "none" | "metadata" | "read" | "write" | "manage" | "project_manager";

export const LEVELS: readonly Level[] = [
  "none",
  "metadata",
  "read",
  "write",
  "manage",
  "project_manager",
];

/** Whether `level` reaches at least `needed`. */
export function reaches(level: Level | undefined, needed: Level): boolean {
  return LEVELS.indexOf(level ?? "none") >= LEVELS.indexOf(needed);
}

/** An instance-level power a role can hold. */
export type Capability =
  | "users.view"
  | "users.manage"
  | "roles.manage"
  | "groups.manage"
  | "grants.manage"
  | "grants.all_projects"
  | "projects.create"
  | "audit.read"
  | "audit.read_all"
  | "audit.verify"
  | "access.report"
  | "tokens.personal"
  | "tokens.revoke_others"
  | "agents.view"
  | "agents.view_all"
  | "agents.manage"
  | "sso.manage"
  | "instance.view"
  | "instance.backup"
  | "instance.reset"
  | "password.breakglass";

/**
 * A part of Dopbase the server can turn off. The session lists the ones
 * that are on.
 */
export type Feature = "sso" | "rbac" | "rbac.roles";

/** What to set on the server to turn a feature on. */
export const FEATURE_SETTINGS: Record<Feature, string> = {
  sso: "DOPBASE_SSO=true",
  rbac: "DOPBASE_RBAC=true",
  "rbac.roles": "DOPBASE_RBAC=true and DOPBASE_RBAC_ROLES=true",
};

export type PrincipalType = "user" | "group";
/** `audit` is for audit tokens only, never a grant. */
export type ScopeType = "all" | "project" | "environment" | "audit";

export interface Grant {
  id: string;
  principalType: PrincipalType;
  principalId: string;
  /** The user's email or the group's name. */
  principalLabel: string;
  /** True when the principal is an AI agent. */
  principalAgent: boolean;
  scopeType: ScopeType;
  scopeId: string;
  /** `project` or `project/environment`. Empty for all projects. */
  scopeLabel: string;
  projectId: string;
  level: Level;
  grantedBy: string | null;
  createdAt: string;
}

export interface SetGrantRequest {
  principalType: PrincipalType;
  principalId: string;
  scopeType: ScopeType;
  /** Project id or name, or environment id. Left out for all projects. */
  scope?: string;
  level: Level;
}

export interface ProjectAccess {
  project: Project;
  environments: Environment[];
  grants: Grant[];
}

export interface GroupRef {
  id: string;
  name: string;
}

export interface UserAccess {
  userId: string;
  email: string;
  roleName: string;
  maxLevel: Level;
  groups: GroupRef[];
  grants: Grant[];
}

/** An account named in a response: who did something. */
export interface AccountRef {
  id: string;
  email: string;
}

export interface Group {
  id: string;
  name: string;
  description: string;
  memberCount: number;
  /** Null when the account that created it is gone. */
  createdBy: AccountRef | null;
  createdAt: string;
  updatedAt: string;
}

/** A group in the list, with enough to show who's in it and what it reaches. */
export interface GroupSummary extends Group {
  members: AccountRef[];
  grants: Grant[];
}

export interface GroupMember {
  id: string;
  email: string;
  addedAt: string;
  addedBy: AccountRef | null;
}

export interface GroupDetail {
  group: Group;
  members: GroupMember[];
  grants: Grant[];
}

export async function setGrant(request: SetGrantRequest): Promise<Grant> {
  const { data } = await apiRequest<Grant>("/api/v1/grants", {
    method: "PUT",
    body: request,
  });
  return data;
}

export async function removeGrant(id: string): Promise<void> {
  await apiRequest(`/api/v1/grants/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}

export async function fetchProjectAccess(
  projectRef: string,
  signal?: AbortSignal,
): Promise<ProjectAccess> {
  const { data } = await apiRequest<ProjectAccess>(
    `/api/v1/projects/${encodeURIComponent(projectRef)}/access`,
    { signal },
  );
  return data;
}

export async function fetchUserAccess(
  id: string,
  signal?: AbortSignal,
): Promise<UserAccess> {
  const { data } = await apiRequest<UserAccess>(
    `/api/v1/users/${encodeURIComponent(id)}/access`,
    { signal },
  );
  return data;
}

const GROUPS = "/api/v1/groups";

export async function fetchGroups(
  signal?: AbortSignal,
): Promise<GroupSummary[]> {
  const { data } = await apiRequest<GroupSummary[]>(GROUPS, { signal });
  return data;
}

export async function fetchGroup(
  id: string,
  signal?: AbortSignal,
): Promise<GroupDetail> {
  const { data } = await apiRequest<GroupDetail>(
    `${GROUPS}/${encodeURIComponent(id)}`,
    { signal },
  );
  return data;
}

export async function createGroup(
  name: string,
  description: string,
): Promise<GroupDetail> {
  const { data } = await apiRequest<GroupDetail>(GROUPS, {
    method: "POST",
    body: { name, description },
  });
  return data;
}

export async function updateGroup(
  id: string,
  body: { name?: string; description?: string },
): Promise<GroupDetail> {
  const { data } = await apiRequest<GroupDetail>(
    `${GROUPS}/${encodeURIComponent(id)}`,
    { method: "PATCH", body },
  );
  return data;
}

export async function deleteGroup(id: string): Promise<void> {
  await apiRequest(`${GROUPS}/${encodeURIComponent(id)}`, {
    method: "DELETE",
  });
}

export async function addGroupMember(
  id: string,
  userId: string,
): Promise<GroupDetail> {
  const { data } = await apiRequest<GroupDetail>(
    `${GROUPS}/${encodeURIComponent(id)}/members/${encodeURIComponent(userId)}`,
    { method: "PUT" },
  );
  return data;
}

export async function removeGroupMember(
  id: string,
  userId: string,
): Promise<GroupDetail> {
  const { data } = await apiRequest<GroupDetail>(
    `${GROUPS}/${encodeURIComponent(id)}/members/${encodeURIComponent(userId)}`,
    { method: "DELETE" },
  );
  return data;
}
