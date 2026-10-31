import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import { useProjectController } from "./Project.controller";
import * as projectsApi from "~/services/projects.api";
import * as environmentsApi from "~/services/environments.api";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

const { route, push, replace } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    route: reactive({
      name: "environment" as string,
      params: {} as Record<string, string>,
    }),
    push: vi.fn(),
    replace: vi.fn(),
  };
});

vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => ({ push, replace }),
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

const payments = {
  id: "prj_1",
  name: "payments",
  level: "write" as const,
  createdAt: "",
  updatedAt: "",
};

function environment(
  id: string,
  name: string,
  extra: Partial<Environment> = {},
): Environment {
  return {
    id,
    projectId: "prj_1",
    projectName: "payments",
    name,
    createdAt: "",
    updatedAt: "",
    ...extra,
  };
}

const dev = environment("env_1", "dev", { level: "manage" });
const prod = environment("env_2", "prod", {
  level: "read",
  protected: true,
});

function at(name: string, params: Record<string, string>): void {
  route.name = name;
  route.params = params;
}

async function loaded() {
  const { controller } = mountController(useProjectController);
  await vi.waitFor(() => expect(controller.environments.value).not.toBeNull());
  await vi.waitFor(() => expect(controller.projects.value).not.toBeNull());
  return controller;
}

beforeEach(() => {
  vi.resetAllMocks();
  capabilities.clear();
  at("environment", { projectRef: "payments", environmentName: "dev" });
  vi.mocked(projectsApi.listProjects).mockResolvedValue([payments]);
  vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([dev, prod]);
});

describe("useProjectController", () => {
  it("resolves the environment named in the address", async () => {
    const c = await loaded();

    expect(environmentsApi.listEnvironments).toHaveBeenCalledWith(
      "payments",
      expect.any(AbortSignal),
    );
    expect(c.environment.value?.id).toBe("env_1");
    expect(c.meter.value).toEqual({ label: "Manage", filled: 4, strong: true });
  });

  it("opens the first environment when the address names none", async () => {
    at("project", { projectRef: "payments" });
    await loaded();

    expect(replace).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "payments", environmentName: "dev" },
    });
  });

  it("says when the named environment isn't there", async () => {
    at("environment", { projectRef: "payments", environmentName: "qa" });
    const c = await loaded();

    expect(c.environment.value).toBeNull();
    expect(c.environmentMissing.value).toBe(true);
  });

  it("shows Tokens only where you manage the environment", async () => {
    const c = await loaded();
    expect(c.tabs.value.map((tab) => tab.key)).toEqual([
      "secrets",
      "tokens",
      "activity",
    ]);

    route.params = { projectRef: "payments", environmentName: "prod" };
    await nextTick();
    expect(c.tabs.value.map((tab) => tab.key)).toEqual(["secrets", "activity"]);
  });

  it("shows Access and Settings to project managers", async () => {
    vi.mocked(projectsApi.listProjects).mockResolvedValue([
      { ...payments, level: "project_manager" },
    ]);
    const c = await loaded();

    expect(c.tabs.value.map((tab) => tab.key)).toEqual([
      "secrets",
      "tokens",
      "access",
      "activity",
      "settings",
    ]);
    expect(c.tabs.value.find((tab) => tab.key === "settings")?.to).toEqual({
      name: "project-settings",
      params: { projectRef: "payments" },
    });
  });

  it("shows Access read-only to people with access.report", async () => {
    capabilities.add("access.report");
    const c = await loaded();

    expect(c.tabs.value.map((tab) => tab.key)).toContain("access");
    expect(c.tabs.value.map((tab) => tab.key)).not.toContain("settings");
  });

  it("keeps the last environment on project-wide tabs", async () => {
    at("environment", { projectRef: "payments", environmentName: "prod" });
    const first = await loaded();
    expect(first.environment.value?.name).toBe("prod");

    at("project-access", { projectRef: "payments" });
    const c = await loaded();

    expect(c.activeTab.value).toBe("access");
    expect(c.projectLevel.value).toBe(true);
    expect(c.environment.value?.name).toBe("prod");
    expect(c.environmentMissing.value).toBe(false);
  });

  it("remembers the environment by id, so a rename keeps it", async () => {
    at("environment", { projectRef: "payments", environmentName: "prod" });
    await loaded();
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      dev,
      { ...prod, name: "production" },
    ]);

    at("project-settings", { projectRef: "payments" });
    const c = await loaded();

    expect(c.environment.value?.name).toBe("production");
  });

  it("keeps Access and Settings before the project has environments", async () => {
    vi.mocked(projectsApi.listProjects).mockResolvedValue([
      { ...payments, level: "project_manager" },
    ]);
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([]);
    at("project-settings", { projectRef: "payments" });
    const c = await loaded();

    expect(c.tabs.value.map((tab) => tab.key)).toEqual(["access", "settings"]);
  });

  it("says when the project's own details didn't load", async () => {
    vi.mocked(projectsApi.listProjects).mockRejectedValue(new Error("down"));
    const { controller: c } = mountController(useProjectController);

    await vi.waitFor(() =>
      expect(c.projectsError.value).toBe(
        "Could not load this project's details.",
      ),
    );
  });

  it("falls back to Secrets when Settings is opened without Project manager", async () => {
    at("project-settings", { projectRef: "payments" });
    await loaded();

    await vi.waitFor(() =>
      expect(replace).toHaveBeenCalledWith(
        expect.objectContaining({ name: "environment" }),
      ),
    );
  });

  it("opens this environment's activity", async () => {
    const c = await loaded();

    c.openActivity();

    expect(push).toHaveBeenCalledWith({
      name: "environment-activity",
      params: { projectRef: "payments", environmentName: "dev" },
    });
  });

  it("falls back to Secrets when a Tokens link is opened without Manage", async () => {
    at("environment-tokens", {
      projectRef: "payments",
      environmentName: "prod",
    });
    await loaded();

    await vi.waitFor(() =>
      expect(replace).toHaveBeenCalledWith({
        name: "environment",
        params: { projectRef: "payments", environmentName: "prod" },
      }),
    );
  });

  it("keeps the Tokens tab when switching to an environment you manage", async () => {
    at("environment-tokens", {
      projectRef: "payments",
      environmentName: "dev",
    });
    const c = await loaded();

    c.switchEnvironment(environment("env_3", "staging", { level: "manage" }));
    expect(push).toHaveBeenLastCalledWith({
      name: "environment-tokens",
      params: { projectRef: "payments", environmentName: "staging" },
    });

    c.switchEnvironment(prod);
    expect(push).toHaveBeenLastCalledWith({
      name: "environment",
      params: { projectRef: "payments", environmentName: "prod" },
    });
  });

  it("switches project by name", async () => {
    const c = await loaded();

    c.switchProject({ ...payments, id: "prj_2", name: "search" });

    expect(push).toHaveBeenCalledWith({
      name: "project",
      params: { projectRef: "search" },
    });
  });

  it("creates an environment and opens it", async () => {
    const qa = environment("env_9", "qa");
    vi.mocked(environmentsApi.createEnvironment).mockResolvedValue(qa);
    const c = await loaded();
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue([
      dev,
      prod,
      qa,
    ]);
    // The new environment is in the list by the time its page opens.
    let listed: string[] = [];
    push.mockImplementation(() => {
      listed = (c.environments.value ?? []).map((item) => item.name);
    });

    await c.createEnvironment("qa");

    expect(listed).toContain("qa");
    expect(environmentsApi.createEnvironment).toHaveBeenCalledWith(
      "payments",
      "qa",
    );
    expect(push).toHaveBeenCalledWith({
      name: "environment",
      params: { projectRef: "payments", environmentName: "qa" },
    });
  });

  it("surfaces a failed environment load", async () => {
    vi.mocked(environmentsApi.listEnvironments).mockRejectedValue(
      new Error("down"),
    );
    const { controller: c } = mountController(useProjectController);

    await vi.waitFor(() =>
      expect(c.loadError.value).toBe(
        "Could not load this project's environments.",
      ),
    );
  });
});
