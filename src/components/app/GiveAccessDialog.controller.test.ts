import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import {
  useGiveAccessDialog,
  type GrantPrincipal,
} from "./GiveAccessDialog.controller";
import * as accessApi from "~/services/access.api";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import type { Environment, Project } from "~/services";
import { ApiError } from "~/services/http.client";
import { grantFixture, settle } from "~/tests/people";

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  setGrant: vi.fn(),
}));
vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");

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

const maya: GrantPrincipal = {
  type: "user",
  id: "usr_maya",
  label: "maya@example.com",
  maxLevel: "project_manager",
};
const bot: GrantPrincipal = {
  type: "user",
  id: "usr_bot",
  label: "review-bot",
  maxLevel: "read",
};
const backend: GrantPrincipal = {
  type: "group",
  id: "grp_backend",
  label: "Backend",
};

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    project({ id: "prj_search", name: "search", level: "read" }),
    project({}),
    project({ id: "prj_data", name: "data" }),
  ]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
    environment({}),
    environment({ id: "env_prod", name: "prod", protected: true }),
  ]);
  vi.mocked(accessApi.setGrant).mockResolvedValue(grantFixture());
});

/** Opens the dialog the way a page does: mounted already open. */
async function openFor(
  principal: GrantPrincipal,
  existing: accessApi.Grant[] = [],
  editing: accessApi.Grant | null = null,
) {
  const dialog = useGiveAccessDialog(
    ref(true),
    ref(principal),
    ref(existing),
    ref(editing),
  );
  await settle();
  return dialog;
}

describe("useGiveAccessDialog", () => {
  it("offers only projects you manage and picks the first", async () => {
    const dialog = await openFor(maya);
    expect(dialog.manageable.value.map((item) => item.name)).toEqual([
      "payments",
      "data",
    ]);
    expect(dialog.projectId.value).toBe("prj_pay");
    expect(dialog.level.value).toBe("read");
  });

  it("gives a whole project at the chosen level", async () => {
    const dialog = await openFor(maya);
    dialog.level.value = "write";
    expect(dialog.summary.value).toBe(
      "maya@example.com will be able to read and change secrets in all of payments.",
    );
    expect(await dialog.submit()).toBe(true);
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_maya",
      scopeType: "project",
      scope: "prj_pay",
      level: "write",
    });
  });

  it("loads environments for one-environment access and picks the first", async () => {
    const dialog = await openFor(maya);
    dialog.where.value = "environment";
    await settle();
    expect(environmentsApi.listEnvironments).toHaveBeenCalledWith("payments");
    expect(dialog.environmentId.value).toBe("env_staging");
    dialog.environmentId.value = "env_prod";
    dialog.level.value = "none";
    expect(dialog.summary.value).toBe(
      "maya@example.com will no longer reach payments / prod, even through a project grant.",
    );
    await dialog.submit();
    expect(accessApi.setGrant).toHaveBeenCalledWith(
      expect.objectContaining({
        scopeType: "environment",
        scope: "env_prod",
        level: "none",
      }),
    );
  });

  it("swaps Project manager for Manage on one environment, and None back on a project", async () => {
    const dialog = await openFor(maya);
    dialog.level.value = "project_manager";
    dialog.where.value = "environment";
    await nextTick();
    expect(dialog.level.value).toBe("manage");
    expect(dialog.levels.value.map((choice) => choice.level)).not.toContain(
      "project_manager",
    );
    dialog.level.value = "none";
    dialog.where.value = "project";
    await nextTick();
    expect(dialog.level.value).toBe("metadata");
  });

  it("shows levels above an agent's role but won't give them", async () => {
    const dialog = await openFor(bot);
    expect(
      dialog.levels.value
        .filter((choice) => choice.disabled)
        .map((choice) => choice.level),
    ).toEqual(["write", "manage", "project_manager"]);
    dialog.level.value = "write";
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("Their role allows up to Read.");
    expect(accessApi.setGrant).not.toHaveBeenCalled();
  });

  it("doesn't cap a group, which has no role of its own", async () => {
    const dialog = await openFor(backend);
    expect(dialog.levels.value.every((choice) => !choice.disabled)).toBe(true);
    expect(dialog.summary.value).toBe(
      "Everyone in Backend will be able to read secrets in all of payments.",
    );
    await dialog.submit();
    expect(accessApi.setGrant).toHaveBeenCalledWith(
      expect.objectContaining({
        principalType: "group",
        principalId: "grp_backend",
      }),
    );
  });

  it("warns that a new grant replaces the one they hold there", async () => {
    const dialog = await openFor(maya, [grantFixture()]);
    expect(dialog.replaces.value?.id).toBe("grt_pay");
    expect(dialog.summary.value).toMatch(
      /This replaces their current grant there\.$/,
    );
    dialog.projectId.value = "prj_data";
    expect(dialog.replaces.value).toBeNull();
  });

  it("changes only the level of a grant being edited", async () => {
    const grant = grantFixture({
      scopeType: "environment",
      scopeId: "env_prod",
      scopeLabel: "payments/prod",
      level: "read",
    });
    const dialog = await openFor(maya, [grant], grant);
    expect(projectsApi.listProjects).not.toHaveBeenCalled();
    expect(dialog.where.value).toBe("environment");
    expect(dialog.level.value).toBe("read");
    dialog.level.value = "metadata";
    expect(dialog.summary.value).toBe(
      "maya@example.com will be able to see the secret names in payments/prod.",
    );
    await dialog.submit();
    expect(accessApi.setGrant).toHaveBeenCalledWith({
      principalType: "user",
      principalId: "usr_maya",
      scopeType: "environment",
      scope: "env_prod",
      level: "metadata",
    });
  });

  it("asks for a project when you manage none", async () => {
    vi.mocked(projectsApi.listProjects).mockResolvedValueOnce([
      project({ level: "write" }),
    ]);
    const dialog = await openFor(maya);
    expect(dialog.manageable.value).toEqual([]);
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("Choose a project.");
  });

  it("says when projects or environments won't load", async () => {
    vi.mocked(projectsApi.listProjects).mockRejectedValueOnce(new Error(""));
    const dialog = await openFor(maya);
    expect(dialog.error.value).toBe("Could not load your projects.");
    vi.mocked(projectsApi.listProjects).mockResolvedValueOnce([project({})]);
    const second = await openFor(maya);
    vi.mocked(environmentsApi.listEnvironments).mockRejectedValueOnce(
      new Error(""),
    );
    second.where.value = "environment";
    await settle();
    expect(second.error.value).toBe("Could not load its environments.");
  });

  it("shows the server's reason when the grant is refused", async () => {
    vi.mocked(accessApi.setGrant).mockRejectedValueOnce(
      new ApiError(403, {
        ACCESS_EXCEEDS_YOURS: "You can't give more access than you have.",
      }),
    );
    const dialog = await openFor(maya);
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe(
      "You can't give more access than you have.",
    );
    expect(dialog.saving.value).toBe(false);
  });

  it("loads projects again each time it opens", async () => {
    const open = ref(false);
    useGiveAccessDialog(open, ref(maya), ref([]), ref(null));
    await settle();
    expect(projectsApi.listProjects).not.toHaveBeenCalled();
    open.value = true;
    await settle();
    open.value = false;
    await nextTick();
    open.value = true;
    await settle();
    expect(projectsApi.listProjects).toHaveBeenCalledTimes(2);
  });
});
