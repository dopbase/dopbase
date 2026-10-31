import { describe, expect, it } from "vitest";
import { router } from "./router";

describe("project routes", () => {
  it.each([
    ["/projects", "projects"],
    ["/projects/billing", "project"],
    ["/projects/billing/access", "project-access"],
    ["/projects/billing/settings", "project-settings"],
    ["/projects/billing/prod/activity", "environment-activity"],
    ["/projects/billing/prod/compare", "environment-compare"],
    ["/projects/billing/prod/secrets", "environment"],
    ["/projects/billing/prod/tokens", "environment-tokens"],
    ["/projects/billing/prod/import", "environment-import"],
    ["/projects/p/billing/e/env_1", "legacy-environment"],
    ["/projects/p/billing/e/env_1/tokens", "legacy-environment"],
    ["/projects/p/billing/e/env_1/import", "legacy-environment"],
    ["/status", "status"],
    // Legacy workspace URLs fall through to the not-found page.
    ["/workspace", "not-found"],
  ])("resolves %s", (path, routeName) => {
    expect(router.resolve(path).name).toBe(routeName);
  });

  it("reads the environment name from the address", () => {
    expect(router.resolve("/projects/billing/prod/secrets").params).toEqual({
      projectRef: "billing",
      environmentName: "prod",
    });
  });
});

describe("renamed routes", () => {
  it.each([
    ["/instance", { name: "status" }],
    [
      "/projects/p/billing",
      { name: "project", params: { projectRef: "billing" } },
    ],
    [
      "/projects/p/billing/access",
      { name: "project-access", params: { projectRef: "billing" } },
    ],
  ])("sends %s to its new address", (path, expected) => {
    const resolved = router.resolve(path);
    const redirect = resolved.matched[0]?.redirect;
    const target =
      typeof redirect === "function"
        ? (redirect as unknown as (to: typeof resolved) => unknown)(resolved)
        : redirect;
    expect(target).toEqual(expected);
  });
});
