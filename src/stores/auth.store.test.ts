import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAuthStore } from "./auth.store";
import * as authApi from "~/services/auth.api";
import * as bootstrapApi from "~/services/bootstrap.api";
import { ApiError } from "~/services/http.client";

vi.mock("~/services/auth.api");
vi.mock("~/services/bootstrap.api");

beforeEach(() => {
  setActivePinia(createPinia());
  sessionStorage.clear();
});

describe("auth store", () => {
  it("login sends a browser session kind", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    expect(authApi.login).toHaveBeenCalledWith({
      email: "a@b.c",
      password: "pw",
      sessionKind: "browser",
    });
  });

  it("login stores the session and persists the CSRF token", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    expect(store.session).toEqual({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: true,
    });
    expect(store.csrfToken).toBe("csrf_1");
    expect(sessionStorage.getItem("dopbase.csrf")).toBe("csrf_1");
    expect(store.isAuthenticated).toBe(true);
  });

  it("logout clears session and CSRF even when the API fails", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    vi.mocked(authApi.logout).mockRejectedValueOnce(
      new ApiError(0, { NETWORK_ERROR: "down" }),
    );
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    await expect(store.logout()).rejects.toBeInstanceOf(ApiError);
    expect(store.session).toBeNull();
    expect(store.csrfToken).toBeNull();
    expect(sessionStorage.getItem("dopbase.csrf")).toBeNull();
  });

  it("bootstrap stores the session and flips state to ready", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      csrfToken: "csrf_1",
    });
    const store = useAuthStore();
    await store.bootstrap({
      setupToken: "dbs_setup",
      email: "a@b.c",
      password: "longenoughpw1",
    });
    expect(store.session?.email).toBe("a@b.c");
    expect(store.bootstrapState).toBe("ready");
    expect(store.csrfToken).toBe("csrf_1");
  });

  it("loadBootstrapStatus normalizes the public state", async () => {
    vi.mocked(bootstrapApi.fetchBootstrapStatus).mockResolvedValueOnce({
      state: "setupRequired",
    });
    const store = useAuthStore();
    await expect(store.loadBootstrapStatus()).resolves.toBe("setupRequired");
    vi.mocked(bootstrapApi.fetchBootstrapStatus).mockResolvedValueOnce({
      state: "ready",
    });
    await expect(store.loadBootstrapStatus()).resolves.toBe("ready");
  });

  it("loadBootstrapStatus survives a network failure", async () => {
    vi.mocked(bootstrapApi.fetchBootstrapStatus).mockRejectedValueOnce(
      new ApiError(0, { NETWORK_ERROR: "down" }),
    );
    const store = useAuthStore();
    await expect(store.loadBootstrapStatus()).resolves.toBe("unknown");
  });

  it("fetchSession maps the server response", async () => {
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: false,
    });
    const store = useAuthStore();
    await store.fetchSession();
    expect(store.session).toEqual({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: false,
    });
  });

  it("reauthenticate marks the session recent", async () => {
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: false,
    });
    const store = useAuthStore();
    await store.fetchSession();
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    await store.reauthenticate("pw");
    expect(store.session?.recentAuthentication).toBe(true);
  });

  it("changePassword clears session and CSRF", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      token: null,
      csrfToken: "csrf_1",
    });
    vi.mocked(authApi.changePassword).mockResolvedValueOnce();
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    await store.changePassword("pw", "newlongpassword");
    expect(store.session).toBeNull();
    expect(store.csrfToken).toBeNull();
    expect(store.isAuthenticated).toBe(false);
  });
});
