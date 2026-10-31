import { defineComponent } from "vue";
import { createMemoryHistory, createRouter } from "vue-router";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { installRouterGuards } from "./router.guards";

const { capabilities } = vi.hoisted(() => ({
  capabilities: new Set<string>(),
}));

vi.mock("~/stores/auth.store", () => ({
  useAuthStore: () => ({
    bootstrapState: "ready",
    isAuthenticated: true,
    can: (...needed: string[]) => needed.some((cap) => capabilities.has(cap)),
  }),
}));

const Page = defineComponent({ render: () => null });

function makeRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/projects", name: "projects", component: Page },
      { path: "/backups", name: "backups", component: Page },
      { path: "/users", name: "users", component: Page },
      {
        path: "/users/:userId",
        name: "user",
        component: Page,
        meta: { page: "users" },
      },
      {
        path: "/:pathMatch(.*)*",
        name: "not-found",
        component: Page,
        meta: { public: true },
      },
      { path: "/:pathMatch(.*)*", name: "no-access", component: Page },
    ],
  });
  installRouterGuards(router);
  return router;
}

beforeEach(() => {
  capabilities.clear();
});

describe("installRouterGuards", () => {
  it("shows No access at the address of a page the account can't open", async () => {
    const router = makeRouter();

    await router.push("/backups?from=link#latest");

    expect(router.currentRoute.value.name).toBe("no-access");
    expect(router.currentRoute.value.fullPath).toBe(
      "/backups?from=link#latest",
    );
  });

  it("opens the page once the account holds its capability", async () => {
    capabilities.add("users.view");
    const router = makeRouter();

    await router.push("/users");

    expect(router.currentRoute.value.name).toBe("users");
  });

  it("opens pages that need no capability", async () => {
    const router = makeRouter();

    await router.push("/projects");

    expect(router.currentRoute.value.name).toBe("projects");
  });

  it("still sends unknown addresses to not-found", async () => {
    const router = makeRouter();

    await router.push("/workspace");

    expect(router.currentRoute.value.name).toBe("not-found");
  });

  it("gates a detail page by the page it belongs to", async () => {
    const router = makeRouter();

    await router.push("/users/usr_1");
    expect(router.currentRoute.value.name).toBe("no-access");

    capabilities.add("users.view");
    await router.push("/users/usr_2");
    expect(router.currentRoute.value.name).toBe("user");
  });
});
