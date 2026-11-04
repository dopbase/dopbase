import type {
  Capability,
  Grant,
  GroupDetail,
  GroupMember,
  GroupSummary,
  UserAccess,
} from "~/services/access.api";
import type { PersonalToken } from "~/services/personal-tokens.api";
import type { Role } from "~/services/roles.api";
import type { User } from "~/services/users.api";
import { useAuthStore } from "~/stores/auth.store";

/**
 * Fixtures for the People & access pages: one instance with root, a lead
 * (Admin), maya (Member), reader (custom Viewer) and review-bot (AI agent
 * owned by maya). Each builder takes overrides for the case at hand.
 */

const AT = "2026-10-01T00:00:00Z";

export function userFixture(overrides: Partial<User> = {}): User {
  return {
    id: "usr_maya",
    email: "maya@example.com",
    role: "member",
    roleId: "rol_member",
    roleName: "Member",
    ownerId: null,
    createdAt: AT,
    updatedAt: AT,
    hasPassword: true,
    ssoLinked: false,
    status: "active",
    lastSignInAt: null,
    groups: [],
    ...overrides,
  };
}

export const root = userFixture({
  id: "usr_root",
  email: "root@example.com",
  role: "root",
  roleId: null,
  roleName: "Root",
});
export const lead = userFixture({
  id: "usr_lead",
  email: "lead@example.com",
  role: "admin",
  roleId: "rol_admin",
  roleName: "Admin",
});
export const maya = userFixture();
export const reader = userFixture({
  id: "usr_reader",
  email: "reader@example.com",
  roleId: "rol_viewer",
  roleName: "Viewer",
});
export const reviewBot = userFixture({
  id: "usr_bot",
  email: "review-bot",
  role: "ai_agent",
  roleId: "rol_ai_agent",
  roleName: "AI agent",
  ownerId: "usr_maya",
  hasPassword: false,
});

export function roleFixture(overrides: Partial<Role> = {}): Role {
  return {
    id: "rol_member",
    name: "Member",
    description: "",
    builtin: true,
    capabilities: ["tokens.personal"],
    maxLevel: "project_manager",
    signIn: "interactive",
    requiresOwner: false,
    tokenScope: "data",
    tokenMaxLevel: "read",
    tokenDefaultTtlDays: 30,
    tokenMaxTtlDays: 365,
    rank: 30,
    members: 1,
    assignable: true,
    createdAt: AT,
    updatedAt: AT,
    ...overrides,
  };
}

export const adminRole = roleFixture({
  id: "rol_admin",
  name: "Admin",
  capabilities: [
    "users.view",
    "users.manage",
    "groups.manage",
    "projects.create",
    "tokens.personal",
  ],
  rank: 20,
});
export const memberRole = roleFixture();
export const agentRole = roleFixture({
  id: "rol_ai_agent",
  name: "AI agent",
  maxLevel: "read",
  signIn: "token_only",
  requiresOwner: true,
  tokenMaxTtlDays: 90,
  rank: 50,
});
export const viewerRole = roleFixture({
  id: "rol_viewer",
  name: "Viewer",
  builtin: false,
  maxLevel: "read",
  rank: 100,
});

export function grantFixture(overrides: Partial<Grant> = {}): Grant {
  return {
    id: "grt_pay",
    principalType: "user",
    principalId: "usr_maya",
    principalLabel: "maya@example.com",
    scopeType: "project",
    scopeId: "prj_pay",
    scopeLabel: "payments",
    projectId: "prj_pay",
    level: "write",
    grantedBy: "usr_lead",
    createdAt: AT,
    ...overrides,
  };
}

export function accessFixture(overrides: Partial<UserAccess> = {}): UserAccess {
  return {
    userId: "usr_maya",
    email: "maya@example.com",
    roleName: "Member",
    maxLevel: "project_manager",
    groups: [],
    grants: [grantFixture()],
    ...overrides,
  };
}

export function groupSummaryFixture(
  overrides: Partial<GroupSummary> = {},
): GroupSummary {
  return {
    id: "grp_backend",
    name: "Backend",
    description: "Payments service team",
    memberCount: 0,
    createdBy: { id: "usr_lead", email: "lead@example.com" },
    createdAt: AT,
    updatedAt: AT,
    members: [],
    grants: [],
    ...overrides,
  };
}

export function memberFixture(
  overrides: Partial<GroupMember> = {},
): GroupMember {
  return {
    id: "usr_maya",
    email: "maya@example.com",
    addedAt: "2026-10-20T09:00:00Z",
    addedBy: { id: "usr_lead", email: "lead@example.com" },
    ...overrides,
  };
}

export function groupDetailFixture(
  overrides: Partial<GroupDetail> = {},
): GroupDetail {
  const summary = groupSummaryFixture();
  return {
    group: {
      id: summary.id,
      name: summary.name,
      description: summary.description,
      memberCount: summary.memberCount,
      createdBy: summary.createdBy,
      createdAt: summary.createdAt,
      updatedAt: summary.updatedAt,
    },
    members: [memberFixture()],
    grants: [],
    ...overrides,
  };
}

export function tokenFixture(
  overrides: Partial<PersonalToken> = {},
): PersonalToken {
  return {
    id: "pat_1",
    userId: "usr_bot",
    name: "ci-deploy",
    scopeType: "all",
    scopeId: "",
    scopeLabel: "",
    level: "read",
    createdBy: "usr_maya",
    createdAt: AT,
    expiresAt: "2099-01-01T00:00:00Z",
    lastUsedAt: null,
    revokedAt: null,
    ...overrides,
  };
}

/** Signs the test in as `adminId` holding `capabilities`. */
export function signInAs(
  capabilities: Capability[],
  { adminId = "usr_lead", role = "admin" as "admin" | "member" | "root" } = {},
): void {
  useAuthStore().session = {
    adminId,
    email: `${adminId}@example.com`,
    sessionKind: "browser",
    recentAuthentication: true,
    role,
    capabilities,
  };
}

/** Lets pending promises and watchers run. */
export function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}
