import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  principalMeta,
  projectMeta,
  useGiveAccessController,
} from "./GiveAccess.controller";
import * as accessApi from "~/services/access.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as usersApi from "~/services/users.api";
import type { Environment, Project } from "~/services";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  accessFixture,
  grantFixture,
  groupDetailFixture,
  groupSummaryFixture,
  lead,
  maya,
  reader,
  reviewBot,
  root,
  settle,
  signInAs,
} from "~/tests/people";

const { route, push } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    route: reactive({
      name: "user-access-give" as string,
      params: { userId: "usr_maya" } as Record<string, string>,
    }),
    push: vi.fn(),
  };
});

vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => ({ push }),
}));
vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  fetchUserAccess: vi.fn(),
  fetchGroup: vi.fn(),
  fetchGroups: vi.fn(),
  fetchProjectAccess: vi.fn(),
  setGrant: vi.fn(),
}));
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/users.api");

const project = (overrides: Partial<Project>): Project => ({
  id: "prj_pay",
  name: "payments",
  level: "project_manager",
  createdAt: "",
  updatedAt: "",
  ...overrides,
});
const environment = (overrides: Partial<Environment>): Environment => ({
  id: "env_staging",
  projectId: "prj_pay",
  projectName: "payments",
  name: "staging",
  protected: false,
  level: "manage",
  createdAt: "",
  updatedAt: "",
  ...overrides,
});

const prodGrant = grantFixture({
  id: "grt_prod",
  scopeType: "environment",
  scopeId: "env_prod",
  scopeLabel: "payments/prod",
  level: "metadata",
});

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  route.name = "user-access-give";
  route.params = { userId: "usr_maya" };
  vi.mocked(accessApi.fetchUserAccess).mockResolvedValue(
    accessFixture({
      grants: [
        grantFixture(),
        prodGrant,
        // Held through a group: not theirs to replace.
        grantFixture({
          id: "grt_group",
          principalType: "group",
          principalId: "grp_backend",
          scopeId: "prj_data",
          scopeLabel: "data",
          projectId: "prj_data",
        }),
      ],
    }),
  );
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    project({ id: "prj_search", name: "search", level: "read" }),
    project({}),
    project({ id: "prj_data", name: "data" }),
  ]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
    environment({}),
    environment({ id: "env_prod", name: "prod", protected: true }),
    environment({ id: "env_qa", name: "qa" }),
    environment({ id: "env_data", projectId: "prj_data", name: "main" }),
  ]);
  vi.mocked(accessApi.setGrant).mockResolvedValue(grantFixture());
});

async function open() {
  const { controller } = mountController(useGiveAccessController);
  await settle();
  return controller;
}

describe("useGiveAccessController", () => {
  it("offers only projects you manage, with what they hold directly", async () => {
    const page = await open();
    expect(page.principal.value).toEqual({
      type: "user",
      id: "usr_maya",
      label: "maya@example.com",
      maxLevel: "project_manager",
    });
    expect(
      page.manageable.value.map((choice) => [
        choice.project.name,
        choice.environmentCount,
        choice.current,
      ]),
    ).toEqual([
      ["data", 1, null],
      ["payments", 3, "write"],
    ]);
    // Nothing is chosen until you pick, unless there is only one.
    expect(page.projectId.value).toBe("");
    page.projectFilter.value = "pay";
    expect(page.visibleProjects.value.map((c) => c.project.name)).toEqual([
      "payments",
    ]);
    expect(page.manageable.value.map(projectMeta)).toEqual([
      "1 environment",
      "3 environments · has Write now",
    ]);
  });

  it("chooses the project when you manage only one", async () => {
    vi.mocked(projectsApi.listProjects).mockResolvedValueOnce([project({})]);
    const page = await open();
    expect(page.project.value?.name).toBe("payments");
  });

  it("gives a whole project and goes back to the Access tab", async () => {
    const page = await open();
    page.chooseProject("prj_data");
    page.level.value = "write";
    expect(page.planned.value).toEqual([
      {
        scopeType: "project",
        scopeId: "prj_data",
        label: "data",
        replaces: null,
      },
    ]);
    expect(page.summary.value).toBe(
      "maya@example.com will be able to read and change secrets in all of data.",
    );
    expect(await page.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_maya",
      scopeType: "project",
      scope: "prj_data",
      level: "write",
    });
    expect(push).toHaveBeenCalledWith({
      name: "user-access",
      params: { userId: "usr_maya" },
    });
  });

  it("gives several environments one grant each and says what it replaces", async () => {
    const page = await open();
    page.chooseProject("prj_pay");
    page.where.value = "environment";
    page.toggleEnvironment("env_staging", true);
    page.toggleEnvironment("env_prod", true);
    page.level.value = "read";
    expect(
      page.planned.value.map((grant) => [grant.label, grant.replaces]),
    ).toEqual([
      ["payments / prod", "metadata"],
      ["payments / staging", null],
    ]);
    expect(page.summary.value).toBe(
      "maya@example.com will be able to read secrets in payments / prod and staging.",
    );
    expect(await page.submit()).toBe(true);
    expect(
      vi
        .mocked(accessApi.setGrant)
        .mock.calls.map(([request]) => request.scope),
    ).toEqual(["env_prod", "env_staging"]);
  });

  it("filters environments, and clears the choice and filter when the project changes", async () => {
    const page = await open();
    page.chooseProject("prj_pay");
    page.where.value = "environment";
    page.environmentFilter.value = "Q";
    expect(
      page.visibleEnvironments.value.map((c) => c.environment.name),
    ).toEqual(["qa"]);
    page.toggleEnvironment("env_qa", true);
    page.chooseProject("prj_data");
    expect(page.environmentIds.value).toEqual([]);
    expect(page.environmentFilter.value).toBe("");
    expect(await page.submit()).toBe(false);
    expect(page.error.value).toBe("Choose at least one environment.");
  });

  it("swaps Project manager for Manage on environments, and None back on a project", async () => {
    const page = await open();
    page.chooseProject("prj_pay");
    page.level.value = "project_manager";
    page.where.value = "environment";
    await settle();
    expect(page.level.value).toBe("manage");
    page.level.value = "none";
    page.where.value = "project";
    await settle();
    expect(page.level.value).toBe("metadata");
  });

  it("names the protected environments a whole-project grant skips", async () => {
    const page = await open();
    page.chooseProject("prj_pay");
    page.level.value = "read";
    expect(page.protectedNote.value).toBe(
      "prod is protected, so this grant doesn't reach it. Give access to it on its own.",
    );
    page.level.value = "project_manager";
    expect(page.protectedNote.value).toBe("");
  });

  it("shows levels above an agent's role but won't give them", async () => {
    route.name = "agent-access-give";
    route.params = { agentId: "usr_bot" };
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValueOnce(
      accessFixture({
        userId: "usr_bot",
        email: "review-bot",
        maxLevel: "read",
        grants: [],
      }),
    );
    const page = await open();
    page.chooseProject("prj_pay");
    page.level.value = "write";
    expect(await page.submit()).toBe(false);
    expect(page.error.value).toBe("Their role allows up to Read.");
    expect(accessApi.setGrant).not.toHaveBeenCalled();
    page.cancel();
    expect(push).toHaveBeenCalledWith({
      name: "agent",
      params: { agentId: "usr_bot" },
    });
  });

  it("starts from a level the role allows, and keeps to it across scopes", async () => {
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValueOnce(
      accessFixture({ maxLevel: "metadata", grants: [] }),
    );
    const page = await open();
    expect(page.level.value).toBe("metadata");
    page.chooseProject("prj_pay");
    page.where.value = "environment";
    await settle();
    expect(page.level.value).toBe("metadata");
    page.where.value = "project";
    await settle();
    expect(page.level.value).toBe("metadata");
    expect(await page.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith(
      expect.objectContaining({ level: "metadata" }),
    );
  });

  it("says so when every grant saved but the page couldn't go back", async () => {
    push.mockRejectedValueOnce(new Error("guard"));
    const page = await open();
    page.chooseProject("prj_pay");
    expect(await page.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledOnce();
    expect(page.error.value).toBe(
      "Access given, but this page couldn't go back. Open their Access tab to see it.",
    );
    expect(page.saving.value).toBe(false);
  });

  it("gives a group access as a group, without a role cap", async () => {
    route.name = "group-access-give";
    route.params = { groupId: "grp_backend" };
    vi.mocked(accessApi.fetchGroup).mockResolvedValueOnce(groupDetailFixture());
    const page = await open();
    expect(page.principal.value).toEqual({
      type: "group",
      id: "grp_backend",
      label: "Backend",
    });
    page.chooseProject("prj_pay");
    page.level.value = "project_manager";
    expect(page.summary.value).toMatch(/^Everyone in Backend will be able to/);
    await page.submit();
    expect(accessApi.setGrant).toHaveBeenCalledWith(
      expect.objectContaining({
        principalType: "group",
        level: "project_manager",
      }),
    );
    expect(push).toHaveBeenCalledWith({
      name: "group-access",
      params: { groupId: "grp_backend" },
    });
  });

  it("asks for a project before saving", async () => {
    const page = await open();
    expect(await page.submit()).toBe(false);
    expect(page.error.value).toBe("Choose a project.");
  });

  it("says when their access won't load", async () => {
    vi.mocked(accessApi.fetchUserAccess).mockRejectedValueOnce(new Error(""));
    const page = await open();
    expect(page.loadError.value).toBe("Could not load their access.");
  });

  it("says how far it got when one grant is refused", async () => {
    vi.mocked(accessApi.setGrant)
      .mockResolvedValueOnce(grantFixture())
      .mockRejectedValueOnce(
        new ApiError(403, {
          ACCESS_EXCEEDS_YOURS: "You can't give more access than you have.",
        }),
      );
    const page = await open();
    page.chooseProject("prj_pay");
    page.where.value = "environment";
    page.toggleEnvironment("env_qa", true);
    page.toggleEnvironment("env_staging", true);
    expect(await page.submit()).toBe(false);
    expect(page.error.value).toBe(
      "Saved 1 of 2. payments / staging failed: You can't give more access than you have.",
    );
    expect(page.saving.value).toBe(false);
    expect(push).not.toHaveBeenCalled();
  });
});

describe("useGiveAccessController from a project", () => {
  beforeEach(() => {
    route.name = "project-access-give";
    route.params = { projectRef: "payments" };
    signInAs(["users.view", "groups.manage"]);
    vi.mocked(accessApi.fetchProjectAccess).mockResolvedValue({
      project: project({}),
      environments: [
        environment({}),
        environment({ id: "env_prod", name: "prod", protected: true }),
      ],
      grants: [
        grantFixture({ scopeId: "prj_pay", level: "write" }),
        prodGrant,
        grantFixture({
          id: "grt_bot",
          principalId: "usr_bot",
          principalLabel: "review-bot",
          scopeType: "environment",
          scopeId: "env_staging",
          scopeLabel: "payments/staging",
          level: "metadata",
        }),
      ],
    });
    vi.mocked(usersApi.fetchUsers).mockResolvedValue([
      root,
      lead,
      reviewBot,
      reader,
      maya,
    ]);
    vi.mocked(accessApi.fetchGroups).mockResolvedValue([
      groupSummaryFixture({ memberCount: 1 }),
    ]);
  });

  it("lists groups, people, then AI agents with what they hold here, never root or you", async () => {
    const page = await open();
    expect(projectsApi.listProjects).not.toHaveBeenCalled();
    expect(page.project.value?.name).toBe("payments");
    expect(
      page.principals.value.map((choice) => [
        choice.label,
        choice.group,
        principalMeta(choice),
      ]),
    ).toEqual([
      ["Backend", "groups", "1 member"],
      ["maya@example.com", "people", "Member · has Write now"],
      ["reader@example.com", "people", "Viewer"],
      ["review-bot", "agents", "AI agent · has 1 environment"],
    ]);
    page.principalFilter.value = "REV";
    expect(page.visiblePrincipals.value.map((c) => c.label)).toEqual([
      "review-bot",
    ]);
  });

  it("lists nobody without the capabilities to see users and groups", async () => {
    signInAs([]);
    const page = await open();
    expect(usersApi.fetchUsers).not.toHaveBeenCalled();
    expect(accessApi.fetchGroups).not.toHaveBeenCalled();
    expect(page.principals.value).toEqual([]);
  });

  it("caps levels at the chosen person's role and goes back to the project", async () => {
    vi.mocked(accessApi.fetchUserAccess).mockResolvedValueOnce(
      accessFixture({ maxLevel: "manage" }),
    );
    const page = await open();
    expect(await page.submit()).toBe(false);
    expect(page.error.value).toBe("Choose who gets access.");

    await page.choosePrincipal("user:usr_maya");
    expect(accessApi.fetchUserAccess).toHaveBeenCalledWith("usr_maya");
    expect(page.principal.value?.maxLevel).toBe("manage");
    expect(
      page.levels.value.find((choice) => choice.level === "project_manager")
        ?.disabled,
    ).toBe(true);

    page.where.value = "environment";
    page.toggleEnvironment("env_prod", true);
    page.level.value = "read";
    expect(
      page.planned.value.map((grant) => [grant.label, grant.replaces]),
    ).toEqual([["payments / prod", "metadata"]]);
    expect(await page.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_maya",
      scopeType: "environment",
      scope: "env_prod",
      level: "read",
    });
    expect(push).toHaveBeenCalledWith({
      name: "project-access",
      params: { projectRef: "payments" },
    });
  });

  it("keeps the last pick when an earlier role loads late", async () => {
    let finishMaya: (value: accessApi.UserAccess) => void = () => {};
    vi.mocked(accessApi.fetchUserAccess)
      .mockImplementationOnce(
        () => new Promise((resolve) => (finishMaya = resolve)),
      )
      .mockResolvedValueOnce(
        accessFixture({ userId: "usr_bot", maxLevel: "read" }),
      );
    const page = await open();
    const first = page.choosePrincipal("user:usr_maya");
    await page.choosePrincipal("user:usr_bot");
    finishMaya(accessFixture({ maxLevel: "manage" }));
    await first;
    expect(page.principal.value).toEqual({
      type: "user",
      id: "usr_bot",
      label: "review-bot",
      maxLevel: "read",
    });
  });

  it("gives a group access without loading a role", async () => {
    const page = await open();
    await page.choosePrincipal("group:grp_backend");
    expect(accessApi.fetchUserAccess).not.toHaveBeenCalled();
    page.level.value = "project_manager";
    expect(page.summary.value).toBe(
      "Everyone in Backend will be able to manage, and give others access to, all of payments.",
    );
  });

  it("says when the project's access won't load", async () => {
    vi.mocked(accessApi.fetchProjectAccess).mockRejectedValueOnce(
      new Error(""),
    );
    const page = await open();
    expect(page.loadError.value).toBe(
      "Could not load who has access to this project.",
    );
  });
});
