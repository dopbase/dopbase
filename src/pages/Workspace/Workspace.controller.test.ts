import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspaceController } from "./Workspace.controller";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import * as secretsApi from "~/services/secrets.api";
import * as tokensApi from "~/services/tokens.api";

const { routerPush, routerReplace, routeParams } = vi.hoisted(() => ({
  routerPush: vi.fn(),
  routerReplace: vi.fn(),
  routeParams: {} as Record<string, string | undefined>,
}));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
  useRoute: () => ({ params: routeParams, name: "environment" }),
}));

vi.mock("~/services/projects.api");
vi.mock("~/services/environments.api");
vi.mock("~/services/secrets.api");
vi.mock("~/services/tokens.api");

const project = {
  id: "prj_1",
  name: "app",
  createdAt: "2026-08-28T00:00:00Z",
  updatedAt: "2026-08-28T00:00:00Z",
};

beforeEach(() => {
  vi.mocked(projectsApi.listProjects).mockResolvedValue([project]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([]);
  routeParams.projectRef = undefined;
  routeParams.environmentId = undefined;
  routerPush.mockReset();
  routerReplace.mockReset();
});

describe("useWorkspaceController", () => {
  it("surfaces project loading failures", async () => {
    vi.mocked(projectsApi.listProjects).mockRejectedValueOnce(
      new Error("down"),
    );
    const c = useWorkspaceController();
    await c.loadProjects();
    expect(c.projectsError.value).toBe("Could not load projects.");
    expect(c.projects.value).toBeNull();
  });

  it("createProject reloads the list and navigates to the project", async () => {
    vi.mocked(projectsApi.createProject).mockResolvedValueOnce({
      ...project,
      name: "fresh",
    });
    const c = useWorkspaceController();
    await c.createProject("fresh");
    expect(routerPush).toHaveBeenCalledWith({
      name: "project",
      params: { projectRef: "fresh" },
    });
  });

  it("deleteProject navigates back to the workspace", async () => {
    routeParams.projectRef = "app";
    vi.mocked(projectsApi.deleteProject).mockResolvedValueOnce({
      projects: 1,
      environments: 2,
      secrets: 5,
      tokens: 1,
    });
    const c = useWorkspaceController();
    const affected = await c.deleteProject();
    expect(affected.secrets).toBe(5);
    expect(routerReplace).toHaveBeenCalledWith({ name: "workspace" });
  });

  it("selectEnvironment puts the environment in the URL", () => {
    routeParams.projectRef = "app";
    const c = useWorkspaceController();
    c.selectEnvironment("env_1");
    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "app", environmentId: "env_1" },
    });
  });

  it("createEnvironment navigates to the created environment", async () => {
    routeParams.projectRef = "app";
    vi.mocked(environmentsApi.createEnvironment).mockResolvedValueOnce({
      id: "env_9",
      projectId: "prj_1",
      projectName: "app",
      name: "staging",
      createdAt: "",
      updatedAt: "",
    });
    const c = useWorkspaceController();
    await c.createEnvironment("staging");
    expect(routerPush).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "app", environmentId: "env_9" },
    });
  });

  it("deleteEnvironment returns to the project route", async () => {
    routeParams.projectRef = "app";
    routeParams.environmentId = "env_1";
    vi.mocked(environmentsApi.deleteEnvironment).mockResolvedValueOnce({
      projects: 0,
      environments: 1,
      secrets: 3,
      tokens: 0,
    });
    const c = useWorkspaceController();
    await c.deleteEnvironment("env_1");
    expect(routerReplace).toHaveBeenCalledWith({
      name: "project",
      params: { projectRef: "app" },
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
        lastUsedAt: null,
        revokedAt: null,
      },
    ]);
    const c = useWorkspaceController();
    await expect(c.describeEnvironmentDeletion("env_1")).resolves.toEqual([
      { label: "secrets", count: 2 },
      { label: "runner tokens", count: 1 },
    ]);
  });
});
