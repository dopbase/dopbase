import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useProjectAccessController } from "./ProjectAccess.controller";
import * as accessApi from "~/services/access.api";
import * as environmentsApi from "~/services/environments.api";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { mountController } from "~/tests/mount-controller";

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
vi.mock("~/services/environments.api");
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
    {
      id: "grp_backend",
      name: "Backend",
      description: "",
      memberCount: 2,
      createdAt: "",
      updatedAt: "",
    },
  ]);
  vi.mocked(usersApi.fetchUsers).mockResolvedValue([
    {
      id: "usr_root",
      email: "root@example.com",
      role: "root",
      createdAt: "",
      updatedAt: "",
      hasPassword: true,
      ssoLinked: false,
    },
    {
      id: "usr_me",
      email: "me@example.com",
      role: "admin",
      createdAt: "",
      updatedAt: "",
      hasPassword: true,
      ssoLinked: false,
    },
    {
      id: "usr_member",
      email: "member@example.com",
      role: "member",
      createdAt: "",
      updatedAt: "",
      hasPassword: true,
      ssoLinked: false,
    },
  ]);
});

describe("useProjectAccessController", () => {
  it("lists project grants before environment grants", async () => {
    signIn(["users.view"]);
    const { controller } = mountController(useProjectAccessController);
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
    const { controller } = mountController(useProjectAccessController);
    await settle();
    expect(controller.principals.value).toEqual([
      { type: "group", id: "grp_backend", label: "Backend" },
      { type: "user", id: "usr_member", label: "member@example.com" },
    ]);
  });

  it("lists nobody without the capabilities to see users and groups", async () => {
    signIn([]);
    const { controller } = mountController(useProjectAccessController);
    await settle();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(accessApi.fetchGroups).not.toHaveBeenCalled();
    expect(controller.principals.value).toEqual([]);
  });

  it("grants on the whole project or one environment", async () => {
    signIn(["users.view"]);
    vi.mocked(accessApi.setGrant).mockResolvedValue(grant({}));
    const { controller } = mountController(useProjectAccessController);
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

  it("explains a removal or protection change the server refuses", async () => {
    signIn(["users.view"]);
    vi.mocked(accessApi.removeGrant).mockRejectedValueOnce(
      new ApiError(403, { SELF_GRANT: "You can't change your own access." }),
    );
    vi.mocked(environmentsApi.setEnvironmentProtected).mockRejectedValueOnce(
      new ApiError(403, { AUTHORIZATION_DENIED: "Not allowed." }),
    );
    const { controller } = mountController(useProjectAccessController);
    await settle();
    await controller.remove(access.grants[1]);
    expect(controller.actionError.value).toBe(
      "You can't change your own access.",
    );
    await controller.setProtected("env_prod", false);
    expect(environmentsApi.setEnvironmentProtected).toHaveBeenCalledWith(
      "env_prod",
      false,
    );
    expect(controller.actionError.value).toBe("Not allowed.");
  });
});
