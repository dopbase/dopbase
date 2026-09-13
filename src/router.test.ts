import { describe, expect, it } from "vitest";
import { router } from "./router";

describe("project routes", () => {
  it.each([
    ["/projects", "projects"],
    ["/projects/p/billing", "project"],
    ["/projects/p/billing/e/env_1", "environment"],
    ["/projects/p/billing/e/env_1/tokens", "environment-tokens"],
    ["/projects/p/billing/e/env_1/import", "environment-import"],
    // Legacy workspace URLs fall through to the not-found page.
    ["/workspace", "not-found"],
  ])("resolves %s", (path, routeName) => {
    expect(router.resolve(path).name).toBe(routeName);
  });
});
