import { describe, expect, it } from "vitest";
import { router } from "./router";

describe("project routes", () => {
  it.each([
    ["/projects", "projects"],
    ["/projects/default/billing", "project"],
    ["/projects/default/billing/all", "project-overview"],
    ["/projects/default/billing/staging", "environment"],
    ["/projects/default/billing/staging/tokens", "environment-tokens"],
    ["/projects/default/billing/staging/import", "environment-import"],
    ["/workspace", "not-found"],
  ])("resolves %s", (path, routeName) => {
    expect(router.resolve(path).name).toBe(routeName);
  });

  it("redirects removed /projects/p/... URLs to the projects index", () => {
    const location = router.resolve("/projects/p/billing");
    expect(location.matched[0]?.redirect).toEqual({ name: "projects" });
  });
});
