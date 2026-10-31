import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  ENVIRONMENT_PREVIEW,
  useProjectsController,
} from "./Projects.controller";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import type { Environment, Project } from "~/services";
import { mountController } from "~/tests/mount-controller";

const { routerPush } = vi.hoisted(() => ({ routerPush: vi.fn() }));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
}));

const { capabilities } = vi.hoisted(() => ({
  capabilities: new Set<string>(),
}));

vi.mock("~/stores/auth.store", () => ({
  useAuthStore: () => ({
    can: (...needed: string[]) => needed.some((cap) => capabilities.has(cap)),
  }),
}));

vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/secrets.api");
vi.mock("~/services/tokens.api");

function project(id: string, name: string, extra: Partial<Project> = {}) {
  return { id, name, createdAt: "", updatedAt: "", ...extra };
}

function environment(
  id: string,
  name: string,
  owner: Project,
  extra: Partial<Environment> = {},
): Environment {
  return {
    id,
    projectId: owner.id,
    projectName: owner.name,
    name,
    createdAt: "",
    updatedAt: "2026-10-30T00:00:00Z",
    ...extra,
  };
}

const payments = project("prj_1", "payments", { level: "project_manager" });
const search = project("prj_2", "search", { level: "read" });

beforeEach(() => {
  vi.resetAllMocks();
  capabilities.clear();
  vi.mocked(projectsApi.listProjects).mockResolvedValue([payments, search]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
    environment("env_3", "staging", payments, { level: "write" }),
    environment("env_1", "prod", payments, {
      level: "metadata",
      protected: true,
    }),
    environment("env_2", "dev", payments, { level: "manage" }),
    environment("env_4", "dev", search, { level: "read" }),
  ]);
});

async function loaded() {
  const { controller } = mountController(useProjectsController);
  await vi.waitFor(() => expect(controller.projects.value).not.toBeNull());
  return controller;
}

describe("useProjectsController", () => {
  it("loads every project and environment in two requests", async () => {
    await loaded();

    expect(projectsApi.listProjects).toHaveBeenCalledTimes(1);
    expect(environmentsApi.listEnvironments).toHaveBeenCalledTimes(1);
    expect(vi.mocked(environmentsApi.listEnvironments).mock.calls[0]?.[0]).toBe(
      undefined,
    );
  });

  it("opens the first project and lists its environments A to Z", async () => {
    const c = await loaded();

    const [first, second] = c.rows.value;
    expect(first?.expanded).toBe(true);
    expect(first?.environmentCount).toBe(3);
    expect(first?.environments.map((row) => row.environment.name)).toEqual([
      "dev",
      "prod",
      "staging",
    ]);
    expect(second?.expanded).toBe(false);
    expect(second?.environments).toEqual([]);
    expect(second?.environmentCount).toBe(1);
  });

  it("opens and closes a project", async () => {
    const c = await loaded();

    c.toggleProject("prj_2");
    expect(c.rows.value[1]?.expanded).toBe(true);
    c.toggleProject("prj_2");
    expect(c.rows.value[1]?.expanded).toBe(false);
  });

  it("shows the viewer's level on each environment and gates its menu", async () => {
    const c = await loaded();

    const rows = c.rows.value[0]!.environments;
    expect(rows.map((row) => row.meter?.label)).toEqual([
      "Manage",
      "Names only",
      "Write",
    ]);
    expect(rows.map((row) => row.canManage)).toEqual([true, false, false]);
    expect(c.rows.value[0]?.canManage).toBe(true);
    expect(c.rows.value[1]?.canManage).toBe(false);
  });

  it("assumes full access when an older server sends no level", async () => {
    vi.mocked(projectsApi.listProjects).mockResolvedValue([
      project("prj_9", "legacy"),
    ]);
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      environment("env_9", "dev", project("prj_9", "legacy")),
    ]);
    const c = await loaded();

    expect(c.rows.value[0]?.canManage).toBe(true);
    expect(c.rows.value[0]?.environments[0]?.canManage).toBe(true);
    expect(c.rows.value[0]?.environments[0]?.meter).toBeNull();
  });

  it(`lists ${ENVIRONMENT_PREVIEW} environments until Show all`, async () => {
    const big = project("prj_1", "payments");
    vi.mocked(projectsApi.listProjects).mockResolvedValue([big]);
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue(
      Array.from({ length: 9 }, (_, index) =>
        environment(`env_${index}`, `env-${index}`, big),
      ),
    );
    const c = await loaded();

    expect(c.rows.value[0]?.environments).toHaveLength(ENVIRONMENT_PREVIEW);
    expect(c.rows.value[0]?.hiddenCount).toBe(3);

    c.showAllEnvironments("prj_1");
    expect(c.rows.value[0]?.environments).toHaveLength(9);
    expect(c.rows.value[0]?.hiddenCount).toBe(0);
  });

  it("filters by environment name and opens the projects that match", async () => {
    const c = await loaded();

    c.filter.value = "DEV";

    expect(c.rows.value.map((row) => row.project.name)).toEqual([
      "payments",
      "search",
    ]);
    expect(c.rows.value.every((row) => row.expanded)).toBe(true);
    expect(
      c.rows.value[0]?.environments.map((row) => row.environment.name),
    ).toEqual(["dev"]);
  });

  it("lists every environment when the filter matches the project name", async () => {
    const c = await loaded();

    c.filter.value = "pay";

    expect(c.rows.value.map((row) => row.project.name)).toEqual(["payments"]);
    expect(c.rows.value[0]?.environments).toHaveLength(3);
  });

  it("returns nothing when no project or environment matches", async () => {
    const c = await loaded();

    c.filter.value = "billing";

    expect(c.rows.value).toEqual([]);
  });

  it("surfaces loading failures", async () => {
    vi.mocked(projectsApi.listProjects).mockRejectedValue(new Error("down"));
    const { controller: c } = mountController(useProjectsController);

    await vi.waitFor(() =>
      expect(c.loadError.value).toBe("Could not load projects."),
    );
  });

  it("lets only accounts with projects.create create projects", async () => {
    expect((await loaded()).canCreateProjects.value).toBe(false);
    capabilities.add("projects.create");
    expect((await loaded()).canCreateProjects.value).toBe(true);
  });

  it("opens an environment by name", async () => {
    const c = await loaded();

    c.openEnvironment(c.rows.value[0]!.environments[1]!.environment);

    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "payments", environmentName: "prod" },
    });
  });
});

describe("useProjectsController dialogs", () => {
  it("creates a project and opens it", async () => {
    vi.mocked(projectsApi.createProject).mockResolvedValue(
      project("prj_3", "billing"),
    );
    const c = await loaded();
    c.nameDialog.value = { kind: "new-project" };

    await c.submitName("billing");

    expect(projectsApi.createProject).toHaveBeenCalledWith("billing");
    expect(routerPush).toHaveBeenCalledWith({
      name: "project",
      params: { projectRef: "billing" },
    });
  });

  it("creates an environment in its project and opens it", async () => {
    vi.mocked(environmentsApi.createEnvironment).mockResolvedValue(
      environment("env_5", "qa", search),
    );
    const c = await loaded();
    c.nameDialog.value = { kind: "new-environment", project: search };

    expect(c.nameDialogCopy.value.title).toBe("New environment in search");
    await c.submitName("qa");

    expect(environmentsApi.createEnvironment).toHaveBeenCalledWith(
      "search",
      "qa",
    );
    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "search", environmentName: "qa" },
    });
  });

  it("renames a project and reloads the tree", async () => {
    vi.mocked(projectsApi.renameProject).mockResolvedValue(
      project("prj_2", "search-api"),
    );
    const c = await loaded();
    c.nameDialog.value = { kind: "rename-project", project: search };

    expect(c.nameDialogCopy.value.initialName).toBe("search");
    await c.submitName("search-api");

    expect(projectsApi.renameProject).toHaveBeenCalledWith(
      "prj_2",
      "search-api",
    );
    expect(projectsApi.listProjects).toHaveBeenCalledTimes(2);
  });

  it("lets the dialog show a failed rename", async () => {
    vi.mocked(environmentsApi.renameEnvironment).mockRejectedValue(
      new Error("taken"),
    );
    const c = await loaded();
    c.nameDialog.value = {
      kind: "rename-environment",
      environment: environment("env_2", "dev", payments),
    };

    await expect(c.submitName("prod")).rejects.toThrow("taken");
  });

  it("previews what deleting an environment removes, then deletes it", async () => {
    vi.mocked(secretsApi.listSecrets).mockResolvedValue([
      { key: "A", version: 1, createdAt: "", updatedAt: "" },
    ]);
    vi.mocked(tokensApi.listTokens).mockResolvedValue([]);
    vi.mocked(environmentsApi.deleteEnvironment).mockResolvedValue({
      projects: 0,
      environments: 1,
      secrets: 1,
      tokens: 0,
    });
    const c = await loaded();
    const target = environment("env_2", "dev", payments);

    await c.startDelete({ kind: "environment", environment: target });
    expect(c.deleteCounts.value).toEqual([
      { label: "secrets", count: 1 },
      { label: "runner tokens", count: 0 },
    ]);

    await c.confirmDelete();
    expect(environmentsApi.deleteEnvironment).toHaveBeenCalledWith("env_2");
    expect(c.deleting.value).toBeNull();
    expect(projectsApi.listProjects).toHaveBeenCalledTimes(2);
  });

  it("keeps the delete dialog open with an error when deleting fails", async () => {
    vi.mocked(projectsApi.deleteProject).mockRejectedValue(new Error("no"));
    const c = await loaded();

    await c.startDelete({ kind: "project", project: search });
    await c.confirmDelete();

    expect(c.deleting.value).toEqual({ kind: "project", project: search });
    expect(c.deleteError.value).toBe("The project could not be deleted.");
  });
});
