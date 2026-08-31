import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useDashboardLayoutController } from "./DashboardLayout.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const { routerPush } = vi.hoisted(() => ({
  routerPush: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
}));

vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
  sessionStorage.clear();
});

async function loginAdmin(): Promise<void> {
  vi.mocked(authApi.login).mockResolvedValueOnce({
    adminId: "usr_1",
    email: "a@b.c",
    sessionKind: "browser",
    token: null,
    csrfToken: "csrf_1",
  });
  await useAuthStore().login("a@b.c", "pw");
}

describe("useDashboardLayoutController", () => {
  it("exposes the signed-in email", async () => {
    await loginAdmin();
    const c = useDashboardLayoutController();
    expect(c.email.value).toBe("a@b.c");
  });

  it("logs out and routes to the login screen", async () => {
    await loginAdmin();
    vi.mocked(authApi.logout).mockResolvedValueOnce();
    const c = useDashboardLayoutController();
    await c.logout();
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
});