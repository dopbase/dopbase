import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useDashboardLayoutController } from "./DashboardLayout.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { browserSession } from "~/tests/browser-session";

const { routerPush, routerReplace, route } = await vi.hoisted(async () => {
  const { reactive } = await import("vue");
  return {
    routerPush: vi.fn(),
    routerReplace: vi.fn(),
    route: reactive({
      path: "/projects/p/billing/e/env_1",
      fullPath: "/projects/p/billing/e/env_1",
    }),
  };
});

vi.mock("vue-router", () => ({
  useRoute: () => route,
  useRouter: () => ({ push: routerPush, replace: routerReplace }),
}));

vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
  routerReplace.mockReset();
  route.path = "/projects/p/billing/e/env_1";
  route.fullPath = "/projects/p/billing/e/env_1";
  sessionStorage.clear();
});

async function loginAdmin(): Promise<void> {
  vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
  await useAuthStore().login("a@b.c", "pw");
}

describe("useDashboardLayoutController", () => {
  it("exposes the signed-in email", async () => {
    await loginAdmin();
    const c = useDashboardLayoutController();
    expect(c.email.value).toBe("a@b.c");
  });

  it("shows a member only the sections their capabilities open", () => {
    useAuthStore().session = {
      adminId: "usr_2",
      email: "maya@example.com",
      sessionKind: "browser",
      recentAuthentication: false,
      role: "member",
      roleName: "Member",
      capabilities: ["tokens.personal"],
    };
    const c = useDashboardLayoutController();

    expect(c.sections.value.map((section) => section.label)).toEqual([
      "Workspace",
      "Instance",
    ]);
    expect(c.roleName.value).toBe("Member");
    expect(c.showPersonalTokens.value).toBe(true);
  });

  it("names the built-in role when the session has no role name", () => {
    useAuthStore().session = {
      adminId: "usr_1",
      email: "root@example.com",
      sessionKind: "browser",
      recentAuthentication: false,
      role: "root",
    };
    const c = useDashboardLayoutController();

    expect(c.roleName.value).toBe("Root Administrator");
    // Root has every capability but no personal tokens.
    expect(c.showPersonalTokens.value).toBe(false);
    expect(c.sections.value).toHaveLength(3);
  });

  it("highlights the sidebar item for the current path", async () => {
    const c = useDashboardLayoutController();
    expect(c.activePage.value).toBe("projects");

    route.path = "/status";
    await nextTick();
    expect(c.activePage.value).toBe("status");

    route.path = "/account";
    await nextTick();
    expect(c.activePage.value).toBeNull();
  });

  it("logs out and routes to the login screen with loggingOut state", async () => {
    await loginAdmin();
    let resolveLogout!: () => void;
    const logoutPromise = new Promise<void>((resolve) => {
      resolveLogout = resolve;
    });
    vi.mocked(authApi.logout).mockReturnValueOnce(logoutPromise);
    const c = useDashboardLayoutController();
    expect(c.loggingOut.value).toBe(false);
    const callPromise = c.logout();
    expect(c.loggingOut.value).toBe(true);
    resolveLogout();
    await callPromise;
    expect(c.loggingOut.value).toBe(false);
    expect(authApi.logout).toHaveBeenCalledTimes(1);
    expect(routerPush).toHaveBeenCalledWith({ name: "login" });
    expect(useAuthStore().isAuthenticated).toBe(false);
  });

  it("still routes to login when the logout API fails", async () => {
    await loginAdmin();
    vi.mocked(authApi.logout).mockRejectedValueOnce(
      new ApiError(403, {
        AUTHORIZATION_DENIED: "A valid CSRF token is required.",
      }),
    );
    const c = useDashboardLayoutController();
    await expect(c.logout()).resolves.toBeUndefined();
    expect(routerPush).toHaveBeenCalledWith({ name: "login" });
    const store = useAuthStore();
    expect(store.session).toBeNull();
    expect(store.csrfToken).toBeNull();
  });

  it("returns to the interrupted page after signing in again", async () => {
    const c = useDashboardLayoutController();

    await c.signInAgain();

    expect(routerReplace).toHaveBeenCalledWith({
      name: "login",
      query: { redirect: "/projects/p/billing/e/env_1" },
    });
  });

  it("closes the navigation sheet when the route changes", async () => {
    const c = useDashboardLayoutController();
    c.navOpen.value = true;

    route.fullPath = "/backups";
    await nextTick();

    expect(c.navOpen.value).toBe(false);
  });
});
