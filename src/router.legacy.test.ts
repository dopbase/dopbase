import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RouteLocationNormalized } from "vue-router";
import {
  redirectLegacyEnvironment,
  redirectLegacyProject,
} from "./router.legacy";
import type { Environment } from "~/services";
import * as environmentsApi from "~/services/environments.api";
import * as projectsApi from "~/services/projects.api";

vi.mock("~/services/environments.api");
vi.mock("~/services/projects.api");

function legacy(
  tail: string,
  query: Record<string, string> = {},
): RouteLocationNormalized {
  return {
    params: { projectRef: "billing", environmentId: "env_1", tail },
    query,
  } as unknown as RouteLocationNormalized;
}

async function redirect(to: RouteLocationNormalized) {
  return (
    redirectLegacyEnvironment as unknown as (
      to: RouteLocationNormalized,
    ) => Promise<unknown>
  )(to);
}

beforeEach(() => {
  vi.mocked(environmentsApi.showEnvironment).mockReset();
  vi.mocked(projectsApi.showProject).mockReset();
  vi.mocked(environmentsApi.listEnvironments).mockReset();
});

describe("redirectLegacyProject", () => {
  function at(projectRef: string): RouteLocationNormalized {
    return {
      params: { projectRef },
      query: { q: "1" },
      hash: "#top",
    } as unknown as RouteLocationNormalized;
  }

  async function enter(to: RouteLocationNormalized) {
    return (
      redirectLegacyProject as unknown as (
        to: RouteLocationNormalized,
      ) => Promise<unknown>
    )(to);
  }

  const kept = { query: { q: "1" }, hash: "#top", replace: true };

  const oldLink = (projectRef: string) => ({
    name: "project",
    params: { projectRef },
    ...kept,
  });

  function projectP(environments: string[] | null): void {
    if (environments === null) {
      vi.mocked(projectsApi.showProject).mockRejectedValue(new Error("404"));
      vi.mocked(environmentsApi.listEnvironments).mockRejectedValue(
        new Error("404"),
      );
      return;
    }
    vi.mocked(projectsApi.showProject).mockResolvedValue(
      {} as Awaited<ReturnType<typeof projectsApi.showProject>>,
    );
    vi.mocked(environmentsApi.listEnvironments).mockResolvedValue(
      environments.map((name) => ({ name }) as Environment),
    );
  }

  it.each(["billing", "access", "settings"])(
    "sends the old link to project %s when there's no project p",
    async (ref) => {
      projectP(null);

      await expect(enter(at(ref))).resolves.toEqual(oldLink(ref));
    },
  );

  it.each([
    ["access", "project-access"],
    ["settings", "project-settings"],
  ])("opens project p's %s tab", async (tab, name) => {
    projectP(["staging"]);

    await expect(enter(at(tab))).resolves.toEqual({
      name,
      params: { projectRef: "p" },
      ...kept,
    });
    expect(projectsApi.showProject).toHaveBeenCalledWith("p");
  });

  it("opens project p's environment by that name", async () => {
    projectP(["staging"]);

    await expect(enter(at("staging"))).resolves.toEqual({
      name: "environment",
      params: { projectRef: "p", environmentName: "staging" },
      ...kept,
    });
    expect(environmentsApi.listEnvironments).toHaveBeenCalledWith("p");
  });

  it("keeps an old link working when p has no environment by that name", async () => {
    projectP(["staging"]);

    await expect(enter(at("billing"))).resolves.toEqual(oldLink("billing"));
  });
});

describe("redirectLegacyEnvironment", () => {
  it.each([
    ["", "environment"],
    ["tokens", "environment-tokens"],
    ["import", "environment-import"],
  ])("sends the id-based %s address to the named one", async (tail, name) => {
    vi.mocked(environmentsApi.showEnvironment).mockResolvedValue({
      id: "env_1",
      projectId: "prj_1",
      projectName: "billing",
      name: "prod",
      createdAt: "",
      updatedAt: "",
    });

    await expect(redirect(legacy(tail, { imported: "1" }))).resolves.toEqual({
      name,
      params: { projectRef: "billing", environmentName: "prod" },
      query: { imported: "1" },
      replace: true,
    });
    expect(environmentsApi.showEnvironment).toHaveBeenCalledWith("env_1");
  });

  it("lands on the project when the environment can't be found", async () => {
    vi.mocked(environmentsApi.showEnvironment).mockRejectedValue(
      new Error("gone"),
    );

    await expect(redirect(legacy(""))).resolves.toEqual({
      name: "project",
      params: { projectRef: "billing" },
      replace: true,
    });
  });
});
