import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ROLE_CAPABILITIES, useRolesController } from "./Roles.controller";
import * as rolesApi from "~/services/roles.api";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/roles.api", async (original) => ({
  ...(await original<typeof rolesApi>()),
  fetchRoles: vi.fn(),
  createRole: vi.fn(),
  updateRole: vi.fn(),
  deleteRole: vi.fn(),
}));

const role = (overrides: Partial<rolesApi.Role>): rolesApi.Role => ({
  id: "rol_member",
  name: "Member",
  description: "",
  builtin: true,
  capabilities: ["tokens.personal"],
  maxLevel: "manage",
  signIn: "interactive",
  requiresOwner: false,
  tokenScope: "data",
  tokenMaxLevel: "write",
  tokenDefaultTtlDays: 30,
  tokenMaxTtlDays: 365,
  rank: 20,
  members: 2,
  assignable: true,
  createdAt: "",
  updatedAt: "",
  ...overrides,
});

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

const viewer: rolesApi.RoleSettings = {
  name: "Viewer",
  capabilities: ["tokens.personal"],
  maxLevel: "read",
  signIn: "interactive",
};

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
    role({}),
    role({ id: "rol_viewer", name: "Viewer", builtin: false }),
  ]);
});

describe("useRolesController", () => {
  it("never offers root-only capabilities", () => {
    for (const capability of rolesApi.ROOT_ONLY_CAPABILITIES) {
      expect(ROLE_CAPABILITIES).not.toContain(capability);
    }
    expect(ROLE_CAPABILITIES).toContain("audit.read_all");
  });

  it("separates custom roles from built-in ones", async () => {
    const { controller } = mountController(useRolesController);
    await settle();
    expect(controller.custom.value.map((item) => item.id)).toEqual([
      "rol_viewer",
    ]);
  });

  it("creates a role, and changes one without its sign-in", async () => {
    const { controller } = mountController(useRolesController);
    await settle();
    await controller.save(viewer, null);
    expect(rolesApi.createRole).toHaveBeenCalledWith(viewer);
    await controller.save(viewer, "rol_viewer");
    expect(rolesApi.updateRole).toHaveBeenCalledWith("rol_viewer", {
      ...viewer,
      signIn: undefined,
    });
    expect(rolesApi.fetchRoles).toHaveBeenCalledTimes(3);
  });
});
