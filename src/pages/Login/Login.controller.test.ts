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
vi.mock("~/services/health.api", () => ({
  fetchHealth: vi.fn().mockResolvedValue({
    product: "dopbase",
    version: "0.2.0",
    apiVersion: "v1",
    status: "ok",
  }),
}));

beforeEach(() => {
  setActivePinia(createPinia());
  routerPush.mockReset();
  routeQuery.redirect = undefined;
  routeQuery.sso_error = undefined;
  routeQuery.password = undefined;
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

  it("maps EMAIL_INVALID to the email field", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(422, { EMAIL_INVALID: "Please use proper email" }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.fieldErrors.value.email).toBe("Enter a valid email address.");
    expect(c.formError.value).toBeNull();
  });

  it("shows the server's wording for a wrong email or password", async () => {
    // The server says the same for an unknown email and a wrong password,
    // and adds how many tries are left.
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(401, {
        AUTHENTICATION_INVALID:
          "Email or password is wrong. 2 more tries, then a 5-minute pause.",
      }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.formError.value).toBe(
      "Email or password is wrong. 2 more tries, then a 5-minute pause.",
    );
    expect(c.fieldErrors.value.email).toBeUndefined();
  });

  it("falls back to a plain message when the server sends none", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(new Error("offline"));
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.formError.value).toBe("Email or password is wrong.");
  });

  it("says how long to wait once sign-in is paused", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(429, {
        RATE_LIMITED: "Too many tries. Wait 5 minutes, then try again.",
      }),
    );
    const c = useLoginController();
    c.email.value = "a@b.c";
    c.password.value = "pw";
    await c.submit();
    expect(c.formError.value).toBe(
      "Too many tries. Wait 5 minutes, then try again.",
    );
  });

  it("names the server and its version for the page", async () => {
    const c = useLoginController();
    await c.loadMethods();
    await new Promise((resolve) => setTimeout(resolve));
    expect(c.host).toBe(window.location.host);
    expect(c.version.value).toBe("0.2.0");
  });

  it("notices Caps Lock from key events in the password field", () => {
    const c = useLoginController();
    const press = (on: boolean) =>
      ({
        getModifierState: (key: string) => key === "CapsLock" && on,
      }) as unknown as KeyboardEvent;
    c.trackCapsLock(press(true));
    expect(c.capsLock.value).toBe(true);
    c.trackCapsLock(press(false));
    expect(c.capsLock.value).toBe(false);
  });

  it("offers single sign-on only when the server has it on", async () => {
    vi.mocked(authApi.fetchAuthMethods).mockResolvedValueOnce({
      password: true,
      passwordForm: true,
      sso: { name: "Acme SSO" },
      cliBrowser: true,
    });
    const c = useLoginController();
    expect(c.methodsLoaded.value).toBe(false);
    await c.loadMethods();
    expect(c.methodsLoaded.value).toBe(true);
    expect(c.ssoName.value).toBe("Acme SSO");

    vi.mocked(authApi.fetchAuthMethods).mockRejectedValueOnce(
      new ApiError(404, { NOT_FOUND: "missing" }),
    );
    const offline = useLoginController();
    await offline.loadMethods();
    expect(offline.methodsLoaded.value).toBe(true);
    expect(offline.ssoName.value).toBeNull();
  });

  it("starts single sign-on from the same safe redirect", () => {
    routeQuery.redirect = "/projects/p/api";
    vi.mocked(authApi.ssoStartUrl).mockReturnValue("/start");
    expect(useLoginController().ssoHref.value).toBe("/start");
    expect(authApi.ssoStartUrl).toHaveBeenLastCalledWith("/projects/p/api");

    routeQuery.redirect = "https://evil.example";
    void useLoginController().ssoHref.value;
    expect(authApi.ssoStartUrl).toHaveBeenLastCalledWith(undefined);
  });

  it("explains why a single sign-on attempt failed", () => {
    routeQuery.sso_error = "account_not_found";
    expect(useLoginController().ssoError.value).toContain(
      "No Dopbase account uses that email",
    );
    routeQuery.sso_error = "something-new";
    expect(useLoginController().ssoError.value).toContain("didn't work");
  });

  it("says when password sign-in is turned off", async () => {
    vi.mocked(authApi.login).mockRejectedValueOnce(
      new ApiError(403, { PASSWORD_LOGIN_DISABLED: "turned off" }),
    );
    const c = useLoginController();
    c.ssoName.value = "Keycloak";
    c.email.value = "dev@example.com";
    c.password.value = "correct-horse-789";
    await c.submit();
    expect(c.formError.value).toContain("Sign in with Keycloak");
  });

  it("hides the password form when root chose so, except at /login?password", async () => {
    vi.mocked(authApi.fetchAuthMethods).mockResolvedValue({
      password: true,
      passwordForm: false,
      sso: { name: "Acme SSO" },
      cliBrowser: true,
    });
    const c = useLoginController();
    expect(c.showPasswordForm.value).toBe(true);
    await c.loadMethods();
    expect(c.showPasswordForm.value).toBe(false);

    routeQuery.password = "";
    const breakGlass = useLoginController();
    await breakGlass.loadMethods();
    expect(breakGlass.showPasswordForm.value).toBe(true);
  });
});
