import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RouteLocationNormalized } from "vue-router";
import { redirectLegacyEnvironment } from "./router.legacy";
import * as environmentsApi from "~/services/environments.api";

vi.mock("~/services/environments.api");

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
