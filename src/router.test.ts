import { describe, expect, it } from "vitest";
import { router } from "./router";

describe("project routes", () => {
  it.each([
    ["/projects", "projects"],
    ["/projects/p/billing", "project"],
    ["/projects/p/billing/e/env_1", "environment"],
    ["/projects/p/billing/e/env_1/tokens", "environment-tokens"],
    ["/projects/p/billing/e/env_1/import", "environment-import"],
    ["/status", "status"],
    // Legacy workspace URLs fall through to the not-found page.
    ["/workspace", "not-found"],
  ])("resolves %s", (path, routeName) => {
    expect(router.resolve(path).name).toBe(routeName);
  });
});

describe("renamed routes", () => {
  it("sends /instance to /status", () => {
    expect(router.resolve("/instance").matched[0]?.redirect).toEqual({
      name: "status",
    });
  });
});
