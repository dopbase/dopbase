import { beforeEach, describe, expect, it, vi } from "vitest";
import { useQuery } from "@pinia/colada";
import { freshPinia } from "~/tests/pinia";
import { keys } from "~/queries/keys";
import { useAccessPanelController } from "./AccessPanel.controller";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { accessFixture } from "~/tests/people";

vi.mock("vue-router", () => ({
  useRoute: () => ({ params: { projectRef: "payments" } }),
}));

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchProjectAccess: vi.fn(),
  fetchUserAccess: vi.fn(),
  removeGrant: vi.fn(),
}));

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

beforeEach(() => {
  freshPinia();
  vi.clearAllMocks();
  vi.mocked(accessApi.fetchProjectAccess).mockResolvedValue(access);
  vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
    accessFixture({ userId: "usr_member", maxLevel: "write" }),
  );
});

describe("useAccessPanelController", () => {
  it("lists project grants before environment grants", async () => {
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

  it("shows where each grant applies and filters by who", async () => {
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

  it("gives access on the shared page, with this project fixed", async () => {
    const { controller } = mountController(useAccessPanelController);
    await settle();
    expect(controller.giveTo.value).toEqual({
      name: "project-access-give",
      params: { projectRef: "payments" },
    });
  });

  it("opens Change level with the person's role cap, and reloads once saved", async () => {
    const { controller } = mountController(useAccessPanelController);
    await settle();
    const projectGrant = controller.rows.value[0]!.grant;

    await controller.openChange(projectGrant);
    expect(accessApi.fetchUserAccess).toHaveBeenCalledWith(
      "usr_member",
      expect.any(AbortSignal),
    );
    expect(controller.changePrincipal.value).toEqual({
      type: "user",
      id: "usr_member",
      label: "member@example.com",
      maxLevel: "write",
    });
    expect(controller.changing.value).toBe(projectGrant);

    await controller.changed();
    expect(controller.changing.value).toBeNull();
    expect(accessApi.fetchProjectAccess).toHaveBeenCalledTimes(2);
  });

  it("opens Change level uncapped when their role can't be read, and for groups", async () => {
    vi.mocked(accessApi.fetchUserAccess).mockRejectedValueOnce(
      new ApiError(403, { FORBIDDEN: "No." }),
    );
    const { controller } = mountController(useAccessPanelController);
    await settle();

    await controller.openChange(access.grants[1]!);
    expect(controller.changePrincipal.value?.maxLevel).toBeUndefined();
    expect(controller.changing.value).not.toBeNull();

    vi.mocked(accessApi.fetchUserAccess).mockClear();
    await controller.openChange(
      grant({
        principalType: "group",
        principalId: "grp_backend",
        principalLabel: "Backend",
      }),
    );
    expect(accessApi.fetchUserAccess).not.toHaveBeenCalled();
    expect(controller.changePrincipal.value).toEqual({
      type: "group",
      id: "grp_backend",
      label: "Backend",
    });
  });

  it("refreshes the project lists on screen after removing a grant", async () => {
    vi.mocked(accessApi.removeGrant).mockResolvedValueOnce();
    const listProjects = vi.fn().mockResolvedValue([]);
    mountController(() =>
      useQuery({ key: keys.projects, query: listProjects }),
    );
    const { controller } = mountController(useAccessPanelController);
    await settle();

    await controller.remove(access.grants[1]!);

    // What someone reaches, the viewer included, may have changed with it.
    expect(listProjects).toHaveBeenCalledTimes(2);
    expect(accessApi.fetchProjectAccess).toHaveBeenCalledTimes(2);
  });

  it("explains a removal the server refuses", async () => {
    vi.mocked(accessApi.removeGrant).mockRejectedValueOnce(
      new ApiError(403, { SELF_GRANT: "You can't change your own access." }),
    );
    const { controller } = mountController(useAccessPanelController);
    await settle();
    await controller.remove(access.grants[1]!);
    expect(controller.actionError.value).toBe(
      "You can't change your own access.",
    );
  });
});
