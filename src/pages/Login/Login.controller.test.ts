import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useLoginController } from "./Login.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const { routerPush, routeQuery } = vi.hoisted(() => ({
  routerPush: vi.fn(),
  routeQuery: {} as Record<string, unknown>,
}));

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
  useRoute: () => ({ query: routeQuery }),
}));

vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
  routeQuery.redirect = undefined;
});

describe("useLoginController", () => {
  it("blocks empty submissions with field errors", async () => {
    const c = useLoginController();
    await c.submit();
    expect(c.fieldErrors.value.email).toBeTruthy();
    expect(c.fieldErrors.value.password).toBeTruthy();
    expect(authApi.login).not.toHaveBeenCalled();
  });

  it("rejects malformed emails locally", async () => {
    const c = useLoginController();
    c.email.value = "not-an-email";
    c.password.value = "pw";
    await c.submit();
    expect(c.fieldErrors.value.email).toBe("Enter a valid email address.");
    expect(authApi.login).not.toHaveBeenCalled();
  });

  it("signs in and routes to the workspace", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    const c = useLoginController();
    c.email.value = "A@B.C";
    c.password.value = "pw";
    await c.submit();
    expect(authApi.login).toHaveBeenCalledTimes(1);
    expect(routerPush).toHaveBeenCalledWith({ name: "workspace" });
    const store = useAuthStore();
    expect(store.isAuthenticated).toBe(true);
  });

  it("honors the redirect query after login", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    routeQuery.redirect = "/audit";
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(routerPush).toHaveBeenCalledWith("/audit");
  });

  it("maps EMAIL_INVAILD to the email field", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(422, { EMAIL_INVAILD: "Please use proper email" }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.fieldErrors.value.email).toBe("Enter a valid email address.");
    expect(c.formError.value).toBeNull();
  });

  it("keeps invalid credentials generic", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_INVALID: "wrong" }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.formError.value).toBe("The email or password is incorrect.");
    expect(c.fieldErrors.value.email).toBeUndefined();
  });

  it("maps rate limiting to a retry message", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(429, { RATE_LIMITED: "slow down" }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.formError.value).toContain("Too many login attempts");
  });
});
