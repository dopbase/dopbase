import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  CAPABILITY_GROUPS,
  CAPABILITY_LABELS,
  ROLE_CAPABILITIES,
  toRoleRow,
  useRolesController,
} from "./Roles.controller";
import * as rolesApi from "~/services/roles.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  adminRole,
  agentRole,
  memberRole,
  roleFixture,
  settle,
  viewerRole,
} from "~/tests/people";

const router = vi.hoisted(() => ({ push: vi.fn() }));
vi.mock("vue-router", () => ({ useRouter: () => router }));

vi.mock("~/services/roles.api", async (original) => ({
  ...(await original<typeof rolesApi>()),
  fetchRoles: vi.fn(),
  deleteRole: vi.fn(),
}));

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  // The server sorts by rank; a custom role can rank above a built-in.
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
    roleFixture({ ...viewerRole, rank: 5 }),
    adminRole,
    memberRole,
    agentRole,
  ]);
});

describe("capability checklist", () => {
  it("never offers root-only capabilities", () => {
    for (const capability of rolesApi.ROOT_ONLY_CAPABILITIES) {
      expect(ROLE_CAPABILITIES).not.toContain(capability);
    }
  });

  it("labels every capability it lists, each in one group", () => {
    for (const capability of ROLE_CAPABILITIES) {
      expect(CAPABILITY_LABELS[capability]).toBeTruthy();
    }
    expect(new Set(ROLE_CAPABILITIES).size).toBe(ROLE_CAPABILITIES.length);
    expect(CAPABILITY_GROUPS.map((group) => group.label)).toEqual([
      "People",
      "Projects",
      "Audit",
      "Tokens",
      "Instance",
    ]);
  });
});

describe("toRoleRow", () => {
  it("counts only capabilities a role can hold", () => {
    const row = toRoleRow(
      roleFixture({ capabilities: ["tokens.personal", "roles.manage"] }),
    );
    expect(row.capabilityCount).toBe(1);
  });

  it("says how holders sign in", () => {
    expect(toRoleRow(memberRole).signsIn).toBe("Password or SSO");
    expect(toRoleRow(agentRole).signsIn).toBe("Token only");
  });

  it("links to the role's page and shows its reach", () => {
    const row = toRoleRow(agentRole);
    expect(row.to).toEqual({
      name: "role",
      params: { roleId: "rol_ai_agent" },
    });
    expect(row.reach.label).toBe("Read");
    expect(row.holders).toBe(1);
  });
});

describe("useRolesController", () => {
  it("lists built-in roles before custom ones, keeping the server's order", async () => {
    const { controller } = mountController(useRolesController);
    await settle();
    expect(controller.rows.value.map((row) => row.role.name)).toEqual([
      "Admin",
      "Member",
      "AI agent",
      "Viewer",
    ]);
  });

  it("starts a duplicate from the role's settings", () => {
    const { controller } = mountController(useRolesController);
    controller.duplicate(adminRole);
    expect(router.push).toHaveBeenCalledWith({
      name: "role-new",
      query: { from: "rol_admin" },
    });
  });

  it("deletes a custom role and reloads the list", async () => {
    vi.mocked(rolesApi.deleteRole).mockResolvedValueOnce();
    const { controller } = mountController(useRolesController);
    await settle();
    controller.askDelete(viewerRole);
    await controller.confirmDelete();
    expect(rolesApi.deleteRole).toHaveBeenCalledWith("rol_viewer");
    expect(controller.deleting.value).toBeNull();
    expect(rolesApi.fetchRoles).toHaveBeenCalledTimes(2);
  });

  it("explains why a role still held by someone can't go", async () => {
    vi.mocked(rolesApi.deleteRole).mockRejectedValueOnce(
      new ApiError(409, {
        ROLE_IN_USE: "Give its holders another role first.",
      }),
    );
    const { controller } = mountController(useRolesController);
    controller.askDelete(viewerRole);
    await controller.confirmDelete();
    expect(controller.deleting.value).toStrictEqual(viewerRole);
    expect(controller.deleteError.value).toBe(
      "Give its holders another role first.",
    );
  });

  it("keeps showing roles when a reload fails", async () => {
    const { controller } = mountController(useRolesController);
    await settle();
    vi.mocked(rolesApi.fetchRoles).mockRejectedValueOnce(
      new ApiError(503, { UNAVAILABLE: "The server is busy." }),
    );
    await controller.load();
    expect(controller.rows.value).toHaveLength(4);
    expect(controller.error.value).toBe("The server is busy.");
  });
});
