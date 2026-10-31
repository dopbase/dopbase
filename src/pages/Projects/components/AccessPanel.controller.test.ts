import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAccessPanelController } from "./AccessPanel.controller";
import * as accessApi from "~/services/access.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { mountController } from "~/tests/mount-controller";
import { groupSummaryFixture, root, userFixture } from "~/tests/people";

vi.mock("vue-router", () => ({
  useRoute: () => ({ params: { projectRef: "payments" } }),
}));

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchProjectAccess: vi.fn(),
  fetchGroups: vi.fn(),
  setGrant: vi.fn(),
  removeGrant: vi.fn(),
}));
vi.mock("~/services/users.api");

const grant = (overrides: Partial<accessApi.Grant>): accessApi.Grant => ({
  id: "grt_1",
  principalType: "user",
  principalId: "usr_member",
  principalLabel: "member@example.com",
  scopeType: "project",
  scopeId: "prj_pay",
  scopeLabel: "payments",
  projectId: "prj_pay",
  level: "read",
  grantedBy: null,
  createdAt: "",
  ...overrides,
});

const access: accessApi.ProjectAccess = {
  project: {
    id: "prj_pay",
    name: "payments",
    level: "project_manager",
    createdAt: "",
    updatedAt: "",
  },
  environments: [
    {
      id: "env_prod",
      projectId: "prj_pay",
      projectName: "payments",
      name: "prod",
      protected: true,
      level: "manage",
      createdAt: "",
      updatedAt: "",
    },
  ],
  grants: [
    grant({
      id: "grt_env",
      scopeType: "environment",
      scopeId: "env_prod",
      scopeLabel: "payments/prod",
      level: "none",
    }),
    grant({ id: "grt_project" }),
  ],
};

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

function signIn(capabilities: accessApi.Capability[]): void {
  useAuthStore().session = {
    adminId: "usr_me",
    email: "me@example.com",
    sessionKind: "browser",
    recentAuthentication: true,
    role: "admin",
    capabilities,
  };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(accessApi.fetchProjectAccess).mockResolvedValue(access);
  vi.mocked(accessApi.fetchGroups).mockResolvedValue([
    groupSummaryFixture({ memberCount: 2 }),
  ]);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    root,
    userFixture({ id: "usr_me", email: "me@example.com", role: "admin" }),
    userFixture({ id: "usr_member", email: "member@example.com" }),
  ]);
});

describe("useAccessPanelController", () => {
  it("lists project grants before environment grants", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();
    expect(accessApi.fetchProjectAccess).toHaveBeenCalledWith(
      "payments",
      expect.any(AbortSignal),
    );
    expect(controller.grants.value.map((item) => item.id)).toEqual([
      "grt_project",
      "grt_env",
    ]);
  });

  it("offers groups and users but never root or yourself", async () => {
    signIn(["users.view", "groups.manage"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();
    expect(controller.principals.value).toEqual([
      { type: "group", id: "grp_backend", label: "Backend" },
      { type: "user", id: "usr_member", label: "member@example.com" },
    ]);
  });

  it("lists nobody without the capabilities to see users and groups", async () => {
    signIn([]);
    const { controller } = mountController(useAccessPanelController);
    await settle();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(accessApi.fetchGroups).not.toHaveBeenCalled();
    expect(controller.principals.value).toEqual([]);
  });

  it("grants on the whole project or one environment", async () => {
    signIn(["users.view"]);
    vi.mocked(accessApi.setGrant).mockResolvedValue(grant({}));
    const { controller } = mountController(useAccessPanelController);
    await settle();
    const member = {
      type: "user" as const,
      id: "usr_member",
      label: "member@example.com",
    };
    await controller.give(member, "", "write");
    expect(accessApi.setGrant).toHaveBeenLastCalledWith({
      principalType: "user",
      principalId: "usr_member",
      scopeType: "project",
      scope: "prj_pay",
      level: "write",
    });
    await controller.give(member, "env_prod", "metadata");
    expect(accessApi.setGrant).toHaveBeenLastCalledWith({
      principalType: "user",
      principalId: "usr_member",
      scopeType: "environment",
      scope: "env_prod",
      level: "metadata",
    });
  });

  it("explains a removal the server refuses", async () => {
    signIn(["users.view"]);
    vi.mocked(accessApi.removeGrant).mockRejectedValueOnce(
      new ApiError(403, { SELF_GRANT: "You can't change your own access." }),
    );
    const { controller } = mountController(useAccessPanelController);
    await settle();
    await controller.remove(access.grants[1]);
    expect(controller.actionError.value).toBe(
      "You can't change your own access.",
    );
  });
});

describe("useAccessPanelController rows and dialog", () => {
  it("shows where each grant applies and filters by who", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();

    expect(
      controller.rows.value.map((row) => [row.where, row.whereProtected]),
    ).toEqual([
      ["Whole project", false],
      ["prod", true],
    ]);
    expect(controller.canManage.value).toBe(true);

    controller.filter.value = "nobody";
    expect(controller.rows.value).toEqual([]);
  });

  it("says what a new grant will allow before giving it", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();

    controller.openGive();
    controller.principalKey.value = "user:usr_member";
    controller.where.value = "environment";
    controller.environmentId.value = "env_prod";
    controller.level.value = "write";

    expect(controller.dialogSummary.value).toBe(
      "member@example.com will be able to read and change secrets in payments / prod. This replaces their current grant there.",
    );
    await expect(controller.submitDialog()).resolves.toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_member",
      scopeType: "environment",
      scope: "env_prod",
      level: "write",
    });
    expect(controller.dialog.value).toBeNull();
  });

  it("offers Project manager only on the whole project", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();
    controller.openGive();

    expect(controller.levels.value).toContain("project_manager");
    controller.level.value = "project_manager";
    controller.where.value = "environment";
    await settle();

    expect(controller.levels.value).not.toContain("project_manager");
    expect(controller.level.value).toBe("read");
  });

  it("changes the level of an existing grant in place", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useAccessPanelController);
    await settle();
    const projectGrant = controller.rows.value[0]!.grant;

    controller.openChange(projectGrant);
    controller.level.value = "manage";
    await controller.submitDialog();

    expect(accessApi.setGrant).toHaveBeenCalledWith(
      expect.objectContaining({
        principalId: "usr_member",
        scopeType: "project",
        scope: "prj_pay",
        level: "manage",
      }),
    );
  });
});
