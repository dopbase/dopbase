import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useLoginController } from "./Login.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { browserSession } from "~/tests/browser-session";

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

  it("signs in and routes to projects", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    const c = useLoginController();
    c.email.value = "A@B.C";
    c.password.value = "pw";
    await c.submit();
    expect(authApi.login).toHaveBeenCalledTimes(1);
    expect(routerPush).toHaveBeenCalledWith({ name: "projects" });
    const store = useAuthStore();
    expect(store.isAuthenticated).toBe(true);
  });

  it.each(["/projects/p/acme/e/env_1", "/audit"])(
    "routes to the safe internal redirect target %s after sign-in",
    async (redirect) => {
      routeQuery.redirect = redirect;
      vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
      const c = useLoginController();
      c.email.value = "a@b.c";
      c.password.value = "pw";
      await c.submit();
      expect(routerPush).toHaveBeenCalledWith(redirect);
    },
  );

  it("ignores cross-origin and protocol-relative redirect targets", async () => {
    vi.mocked(authApi.login).mockResolvedValue(browserSession());
    for (const redirect of [
      "https://evil.example.test/phish",
      "//evil.example.test/phish",
      "javascript:alert(1)",
    ]) {
      routerPush.mockReset();
      routeQuery.redirect = redirect;
      const c = useLoginController();
      c.email.value = "a@b.c";
      c.password.value = "pw";
      await c.submit();
      expect(routerPush).toHaveBeenCalledWith({ name: "projects" });
    }
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
