import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectScope, type EffectScope } from "vue";
import {
  type ProjectsController,
  useProjectsController,
} from "./Projects.controller";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";
import { DEFAULT_WORKSPACE_ID } from "~/constants/workspaces";

vi.mock("vue", async (importOriginal) => ({
  ...(await importOriginal<typeof import("vue")>()),
  onMounted: vi.fn(),
  onUnmounted: vi.fn(),
}));

const { routerPush, routerReplace, route } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    routerPush: vi.fn(),
    routerReplace: vi.fn(),
    route: reactive({
      params: {} as Record<string, string | undefined>,
      name: "environment",
      path: "/projects/default/app/staging",
      fullPath: "/projects/default/app/staging",
    }),
  };
});

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => route,
}));

vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/secrets.api");
vi.mock("~/services/tokens.api");
vi.mock("~/services/workspaces.api", () => ({
  listWorkspaces: vi.fn().mockResolvedValue([
    {
      id: "wsp_default",
      name: "default",
      rootPath: "/workspace",
      createdAt: "",
      updatedAt: "",
    },
  ]),
}));

const project = {
  id: "prj_1",
  name: "app",
  workspaceId: DEFAULT_WORKSPACE_ID,
  createdAt: "2026-08-28T00:00:00Z",
  updatedAt: "2026-08-28T00:00:00Z",
};

const otherProject = {
  id: "prj_2",
  name: "payment",
  workspaceId: DEFAULT_WORKSPACE_ID,
  createdAt: "2026-08-28T00:00:00Z",
  updatedAt: "2026-08-28T00:00:00Z",
};

const environment = (id: string, name: string, projectId = "prj_1") => ({
  id,
  projectId,
  projectName: projectId === "prj_1" ? "app" : "other",
  name,
  createdAt: "",
  updatedAt: "",
});

function navParams(projectRef: string, environmentName?: string) {
  return {
    workspaceSlug: "default",
    projectRef,
    ...(environmentName ? { environmentName } : {}),
  };
}

let scope: EffectScope;

function createController(): ProjectsController {
  return scope.run(() => useProjectsController())!;
}

beforeEach(() => {
  scope = effectScope(true);
  vi.mocked(projectsApi.listProjects).mockReset();
  vi.mocked(environmentsApi.listEnvironments).mockReset();
  vi.mocked(projectsApi.listProjects).mockResolvedValue([
    project,
    otherProject,
  ]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([]);
  route.params.workspaceSlug = "default";
  route.params.projectRef = undefined;
  route.params.environmentName = undefined;
  route.params.environmentId = undefined;
  route.name = "environment";
  route.path = "/projects/default/app/staging";
  route.fullPath = route.path;
  routerPush.mockReset();
  routerReplace.mockReset();
});

afterEach(() => {
  scope.stop();
});

describe("useProjectsController", () => {
  it("surfaces project loading failures", async () => {
    vi.mocked(projectsApi.listProjects).mockRejectedValueOnce(
      new Error("down"),
    );
    const c = createController();
    await c.loadProjects();
    expect(c.projectsError.value).toBe("Could not load projects.");
    expect(c.projects.value).toBeNull();
  });

  it("createProject reloads the list and navigates to the project", async () => {
    vi.mocked(projectsApi.createProject).mockResolvedValueOnce({
      ...project,
      name: "fresh",
    });
    const c = createController();
    await c.loadWorkspaces();
    await c.createProject("fresh");
    expect(projectsApi.listProjects).toHaveBeenCalledTimes(1);
    expect(routerPush).toHaveBeenCalledWith({
      name: "project",
      params: navParams("fresh"),
    });
  });

  it("deleteProject navigates back to projects", async () => {
    route.params.projectRef = "app";
    vi.mocked(projectsApi.deleteProject).mockResolvedValueOnce({
      projects: 1,
      environments: 2,
      secrets: 5,
      tokens: 1,
    });
    const c = createController();
    await c.loadProjects();
    const affected = await c.deleteProject("prj_1");
    expect(affected.secrets).toBe(5);
    expect(routerReplace).toHaveBeenCalledWith({ name: "projects" });
  });

  it("renames an inactive project without changing the active route", async () => {
    route.params.projectRef = "app";
    route.params.environmentName = "dev";
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      environment("env_1", "dev"),
    ]);
    vi.mocked(projectsApi.renameProject).mockResolvedValueOnce({
      ...otherProject,
      name: "payment-service",
    });
    const c = createController();
    await c.loadProjects();
    await vi.waitFor(() => expect(c.environments.value).not.toBeNull());

    await c.renameProject("prj_2", "payment-service");

    expect(projectsApi.renameProject).toHaveBeenCalledWith(
      "prj_2",
      "payment-service",
    );
    expect(routerReplace).not.toHaveBeenCalled();
  });

  it("updates the active project URL after renaming it", async () => {
    route.params.projectRef = "app";
    route.params.environmentName = "dev";
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      environment("env_1", "dev"),
    ]);
    vi.mocked(projectsApi.renameProject).mockResolvedValueOnce({
      ...project,
      name: "billing",
    });
    const c = createController();
    await c.loadWorkspaces();
    await c.loadProjects();
    await vi.waitFor(() => expect(c.environments.value).not.toBeNull());

    await c.renameProject("prj_1", "billing");

    expect(projectsApi.renameProject).toHaveBeenCalledWith("prj_1", "billing");
    expect(routerReplace).toHaveBeenCalledWith({
      name: "environment",
      params: navParams("billing", "dev"),
    });
  });

  it("deletes an inactive project without changing the active route", async () => {
    route.params.projectRef = "app";
    vi.mocked(projectsApi.deleteProject).mockResolvedValueOnce({
      projects: 1,
      environments: 2,
      secrets: 5,
      tokens: 1,
    });
    const c = createController();
    await c.loadProjects();

    await c.deleteProject("prj_2");

    expect(projectsApi.deleteProject).toHaveBeenCalledWith("prj_2");
    expect(routerReplace).not.toHaveBeenCalled();
  });

  it("selectEnvironment puts the environment in the URL", async () => {
    route.params.projectRef = "app";
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      environment("env_1", "dev"),
    ]);
    const c = createController();
    await c.loadWorkspaces();
    await vi.waitFor(() => expect(c.environments.value).not.toBeNull());
    c.selectEnvironment("env_1");
    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: navParams("app", "dev"),
    });
  });

  it("createEnvironment navigates to the created environment", async () => {
    route.params.projectRef = "app";
    vi.mocked(environmentsApi.createEnvironment).mockResolvedValueOnce({
      id: "env_9",
      projectId: "prj_1",
      projectName: "app",
      name: "staging",
      createdAt: "",
      updatedAt: "",
    });
    const c = createController();
    await c.loadWorkspaces();
    await c.createEnvironment("staging");
    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: navParams("app", "staging"),
    });
  });

  it("deleteEnvironment returns to the project route", async () => {
    route.params.projectRef = "app";
    route.params.environmentName = "dev";
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      environment("env_1", "dev"),
    ]);
    vi.mocked(environmentsApi.deleteEnvironment).mockResolvedValueOnce({
      projects: 0,
      environments: 1,
      secrets: 3,
      tokens: 0,
    });
    const c = createController();
    await c.loadWorkspaces();
    await c.loadProjects();
    await vi.waitFor(() => expect(c.environments.value).not.toBeNull());
    await c.deleteEnvironment("env_1");
    expect(routerReplace).toHaveBeenCalledWith({
      name: "project",
      params: navParams("app"),
    });
  });

  it("describeEnvironmentDeletion previews affected counts", async () => {
    vi.mocked(secretsApi.listSecrets).mockResolvedValueOnce([
      { key: "A", version: 1, createdAt: "", updatedAt: "" },
      { key: "B", version: 1, createdAt: "", updatedAt: "" },
    ]);
    vi.mocked(tokensApi.listTokens).mockResolvedValueOnce([
      {
        id: "tok_1",
        environmentId: "env_1",
        name: "t",
        createdAt: "",
        expiresAt: null,
        lastUsedAt: null,
        revokedAt: null,
      },
    ]);
    const c = createController();
    await expect(c.describeEnvironmentDeletion("env_1")).resolves.toEqual([
      { label: "secrets", count: 2 },
      { label: "runner tokens", count: 1 },
    ]);
  });

  it("reselects the first environment when switching projects", async () => {
    route.name = "project";
    route.params.projectRef = "app";
    let resolveOther!: (value: ReturnType<typeof environment>[]) => void;
    vi.mocked(environmentsApi.listEnvironments).mockImplementation(
      (reference?: string) =>
        reference === "app"
          ? Promise.resolve([environment("env_1", "dev")])
          : new Promise((resolve) => {
              resolveOther = resolve;
            }),
    );
    const c = createController();
    await c.loadWorkspaces();
    await c.loadProjects();
    await vi.waitFor(() => expect(c.environments.value).not.toBeNull());
    routerReplace.mockClear();

    route.params.projectRef = "payment";
    route.params.environmentName = undefined;
    await vi.waitFor(() => expect(c.environments.value).toBeNull());
    expect(routerReplace).not.toHaveBeenCalledWith(
      expect.objectContaining({ name: "environment" }),
    );

    resolveOther([environment("env_9", "prod", "prj_2")]);
    await vi.waitFor(() =>
      expect(routerReplace).toHaveBeenCalledWith({
        name: "environment",
        params: navParams("payment", "prod"),
      }),
    );
  });
});
