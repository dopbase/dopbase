import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useRoleController } from "./Role.controller";
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
vi.mock("vue-router", async () => {
  const { reactive } = await import("vue");
  const live = reactive({
    name: "role-new",
    params: {} as Record<string, string>,
    query: {} as Record<string, string>,
  });
  return { useRoute: () => live, useRouter: () => router, __route: live };
});

vi.mock("~/services/roles.api", async (original) => ({
  ...(await original<typeof rolesApi>()),
  fetchRoles: vi.fn(),
  createRole: vi.fn(),
  updateRole: vi.fn(),
}));

type LiveRoute = {
  name: string;
  params: Record<string, string>;
  query: Record<string, string>;
};

async function at(name: string, params = {}, query = {}): Promise<void> {
  const mod = (await import("vue-router")) as unknown as { __route: LiveRoute };
  Object.assign(mod.__route, { name, params, query });
}

beforeEach(async () => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  await at("role-new");
  vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
    adminRole,
    memberRole,
    agentRole,
    viewerRole,
  ]);
  vi.mocked(rolesApi.createRole).mockResolvedValue(viewerRole);
  vi.mocked(rolesApi.updateRole).mockResolvedValue(viewerRole);
});

describe("useRoleController", () => {
  it("starts a new role that signs in and holds personal tokens", async () => {
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.isNew.value).toBe(true);
    expect(controller.readOnly.value).toBe(false);
    expect(controller.title.value).toBe("New role");
    expect(controller.settings.value).toEqual({
      name: "",
      description: "",
      capabilities: ["tokens.personal"],
      maxLevel: "read",
      tokenScope: "data",
      tokenMaxLevel: "read",
      tokenMaxTtlDays: 365,
      tokenDefaultTtlDays: 30,
      signIn: "interactive",
    });
  });

  it("creates the role and goes back to the list", async () => {
    const { controller } = mountController(useRoleController);
    await settle();
    controller.name.value = " Viewer ";
    controller.maxLevel.value = "read";
    controller.toggle("audit.read", true);
    controller.toggle("users.view", true);
    expect(await controller.save()).toBe(true);
    expect(rolesApi.createRole).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "Viewer",
        // Checklist order, not the order they were ticked.
        capabilities: ["users.view", "audit.read", "tokens.personal"],
        signIn: "interactive",
      }),
    );
    expect(router.push).toHaveBeenCalledWith({ name: "roles" });
  });

  it("asks for a name first", async () => {
    const { controller } = mountController(useRoleController);
    await settle();
    expect(await controller.save()).toBe(false);
    expect(controller.nameError.value).toBe("Enter a role name.");
    expect(rolesApi.createRole).not.toHaveBeenCalled();
  });

  it("sends no token settings that matter once tokens are off", async () => {
    const { controller } = mountController(useRoleController);
    await settle();
    controller.toggle("tokens.personal", false);
    expect(controller.holdsTokens.value).toBe(false);
    expect(controller.settings.value.tokenScope).toBe("none");
  });

  it("keeps the default lifetime within the longest one", async () => {
    const { controller } = mountController(useRoleController);
    await settle();
    controller.tokenLifetime.value = "30";
    expect(controller.settings.value.tokenMaxTtlDays).toBe(30);
    expect(controller.settings.value.tokenDefaultTtlDays).toBe(30);
  });

  it("pulls a role's 90-day default down when its longest lifetime drops to 30", async () => {
    await at("role", { roleId: "rol_viewer" });
    vi.mocked(rolesApi.fetchRoles).mockResolvedValue([
      roleFixture({
        ...viewerRole,
        tokenDefaultTtlDays: 90,
        tokenMaxTtlDays: 365,
      }),
    ]);
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.settings.value.tokenDefaultTtlDays).toBe(90);
    controller.tokenLifetime.value = "30";
    expect(controller.settings.value.tokenDefaultTtlDays).toBe(30);
  });

  it("duplicates a role from ?from= with its settings and a new name", async () => {
    await at("role-new", {}, { from: "rol_admin" });
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.title.value).toBe("New role from Admin");
    expect(controller.name.value).toBe("Admin copy");
    expect(controller.capabilities.value).toEqual(
      expect.arrayContaining([
        "users.manage",
        "groups.manage",
        "projects.create",
      ]),
    );
    expect(controller.isNew.value).toBe(true);
  });

  it("copies an agent role's sign-in so the duplicate stays token only", async () => {
    await at("role-new", {}, { from: "rol_ai_agent" });
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.signIn.value).toBe("token_only");
    // 90 days is a choice; the role's lifetime stays as it was.
    expect(controller.tokenLifetime.value).toBe("90");
  });

  it("opens a built-in role read only and won't save it", async () => {
    await at("role", { roleId: "rol_member" });
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.readOnly.value).toBe(true);
    expect(controller.title.value).toBe("Member");
    controller.toggle("users.view", true);
    expect(controller.capabilities.value).not.toContain("users.view");
    expect(await controller.save()).toBe(false);
    expect(rolesApi.updateRole).not.toHaveBeenCalled();
  });

  it("duplicates from a built-in role's page", async () => {
    await at("role", { roleId: "rol_member" });
    const { controller } = mountController(useRoleController);
    await settle();
    controller.duplicate();
    expect(router.push).toHaveBeenCalledWith({
      name: "role-new",
      query: { from: "rol_member" },
    });
  });

  it("saves a custom role without sending its sign-in", async () => {
    await at("role", { roleId: "rol_viewer" });
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.name.value).toBe("Viewer");
    controller.maxLevel.value = "write";
    expect(await controller.save()).toBe(true);
    const [id, sent] = vi.mocked(rolesApi.updateRole).mock.calls[0]!;
    expect(id).toBe("rol_viewer");
    expect(sent).toMatchObject({ name: "Viewer", maxLevel: "write" });
    expect(sent).not.toHaveProperty("signIn");
  });

  it("says when the role no longer exists", async () => {
    await at("role", { roleId: "rol_gone" });
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.notFound.value).toBe(true);
  });

  it("shows the server's reason and stays on the page", async () => {
    vi.mocked(rolesApi.createRole).mockRejectedValueOnce(
      new ApiError(422, {
        ROLE_TOKEN_SCOPE_INVALID: "Token settings need Hold personal tokens.",
      }),
    );
    const { controller } = mountController(useRoleController);
    await settle();
    controller.name.value = "Viewer";
    expect(await controller.save()).toBe(false);
    expect(controller.saveError.value).toBe(
      "Token settings need Hold personal tokens.",
    );
    expect(controller.saving.value).toBe(false);
    expect(router.push).not.toHaveBeenCalled();
  });

  it("says why roles couldn't load", async () => {
    vi.mocked(rolesApi.fetchRoles).mockRejectedValueOnce(
      new ApiError(403, { ACCESS_DENIED: "Only root manages roles." }),
    );
    const { controller } = mountController(useRoleController);
    await settle();
    expect(controller.loadError.value).toBe("Only root manages roles.");
  });
});
