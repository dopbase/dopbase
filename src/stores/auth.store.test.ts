import { beforeEach, describe, expect, it, vi } from "vitest";
import { useQuery, useQueryCache } from "@pinia/colada";
import { useAuthStore } from "./auth.store";
import * as authApi from "~/services/auth.api";
import * as bootstrapApi from "~/services/bootstrap.api";
import { ApiError, apiRequest } from "~/services/http.client";
import { keys } from "~/queries/keys";
import { browserSession, browserSessionInfo } from "~/tests/browser-session";
import { mountController } from "~/tests/mount-controller";
import { settle } from "~/tests/people";

vi.mock("~/services/auth.api");
vi.mock("~/services/bootstrap.api");

beforeEach(() => {
  sessionStorage.clear();
});

describe("auth store", () => {
  it("login sends a browser session kind", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    expect(authApi.login).toHaveBeenCalledWith({
      email: "a@b.c",
      password: "pw",
      sessionKind: "browser",
    });
  });

  it("login stores the session and persists the CSRF token", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    expect(store.session).toEqual({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: true,
      role: "admin",
      lastLoginAt: null,
    });
    expect(store.csrfToken).toBe("csrf_1");
    expect(sessionStorage.getItem("dopbase.csrf")).toBe("csrf_1");
    expect(store.isAuthenticated).toBe(true);
  });

  it("completeSso stores the exchanged session and returns the redirect", async () => {
    vi.mocked(authApi.exchangeSso).mockResolvedValueOnce({
      session: browserSession(),
      redirectTo: "/projects/p/api",
    });
    const store = useAuthStore();
    expect(await store.completeSso("ssoh_code")).toBe("/projects/p/api");
    expect(authApi.exchangeSso).toHaveBeenCalledWith("ssoh_code");
    expect(store.isAuthenticated).toBe(true);
    expect(store.session?.recentAuthentication).toBe(true);
    expect(store.csrfToken).toBe(browserSession().csrfToken);
  });

  it("logout clears session and CSRF even when the API fails", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
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

  it("expires the local session when the server rejects its CSRF token", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            success: false,
            error: {
              AUTHORIZATION_DENIED: "A valid CSRF token is required.",
            },
          }),
          { status: 403, headers: { "Content-Type": "application/json" } },
        ),
      ),
    );

    await expect(
      apiRequest("/api/v1/projects", { method: "POST" }),
    ).rejects.toBeInstanceOf(ApiError);
    expect(store.session).toBeNull();
    expect(store.csrfToken).toBeNull();
    expect(store.sessionExpired).toBe(true);
  });

  it("bootstrap stores the session and flips state to ready", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      csrfToken: "csrf_1",
    });
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      ...browserSessionInfo(),
      role: "root",
      roleName: "Root",
      hasPassword: true,
    });
    const store = useAuthStore();
    await store.bootstrap({
      setupToken: "dbs_setup",
      email: "a@b.c",
      password: "longenoughpw1",
    });
    expect(store.session?.email).toBe("a@b.c");
    expect(store.session?.roleName).toBe("Root");
    expect(store.session?.hasPassword).toBe(true);
    expect(store.bootstrapState).toBe("ready");
    expect(store.csrfToken).toBe("csrf_1");
  });

  it("bootstrap keeps root signed in when the session fetch fails", async () => {
    vi.mocked(bootstrapApi.bootstrapAdmin).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      csrfToken: "csrf_1",
    });
    vi.mocked(authApi.fetchSession).mockRejectedValueOnce(
      new ApiError(0, { NETWORK_ERROR: "down" }),
    );
    const store = useAuthStore();
    await store.bootstrap({
      setupToken: "dbs_setup",
      email: "a@b.c",
      password: "longenoughpw1",
    });
    expect(store.session?.role).toBe("root");
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
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce(browserSessionInfo());
    const store = useAuthStore();
    await store.fetchSession();
    expect(store.session).toEqual({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: false,
      role: "admin",
      lastLoginAt: null,
    });
  });

  it("fetchSession gives a tab opened later its own CSRF token", async () => {
    const store = useAuthStore();
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce(browserSessionInfo());
    await store.fetchSession();
    expect(store.csrfToken).toBeNull();

    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      ...browserSessionInfo(),
      csrfToken: "csrf_tab",
    });
    await store.fetchSession();
    expect(store.csrfToken).toBe("csrf_tab");
  });

  it("reauthenticate marks the session recent", async () => {
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce(browserSessionInfo());
    const store = useAuthStore();
    await store.fetchSession();
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    await store.reauthenticate("pw");
    expect(store.session?.recentAuthentication).toBe(true);
  });

  it("changePassword clears session and CSRF", async () => {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    vi.mocked(authApi.changePassword).mockResolvedValueOnce();
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    await store.changePassword("pw", "newlongpassword");
    expect(store.session).toBeNull();
    expect(store.csrfToken).toBeNull();
    expect(store.isAuthenticated).toBe(false);
  });
});

describe("auth store features", () => {
  it("lets root do everything until the session says what is off", () => {
    const store = useAuthStore();
    store.session = {
      adminId: "usr_root",
      email: "root@example.com",
      sessionKind: "browser",
      recentAuthentication: true,
      role: "root",
    };
    expect(store.can("sso.manage")).toBe(true);
    expect(store.hasFeature("sso")).toBe(true);

    store.session = {
      ...store.session,
      capabilities: ["users.manage", "users.view"],
      features: ["rbac"],
    };
    expect(store.can("sso.manage")).toBe(false);
    expect(store.can("users.manage")).toBe(true);
    expect(store.hasFeature("sso")).toBe(false);
    expect(store.hasFeature("rbac")).toBe(true);
  });

  it("fetchSession keeps the features the server has on", async () => {
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      ...browserSessionInfo(),
      features: ["sso"],
    });
    const store = useAuthStore();
    await store.fetchSession();
    expect(store.session?.features).toEqual(["sso"]);
    expect(store.hasFeature("rbac.roles")).toBe(false);
  });
});

describe("auth store and the query cache", () => {
  function errorResponse(status: number, error: Record<string, string>) {
    return new Response(JSON.stringify({ success: false, error }), {
      status,
      headers: { "Content-Type": "application/json" },
    });
  }

  async function signedIn() {
    vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
    const store = useAuthStore();
    await store.login("a@b.c", "pw");
    return store;
  }

  /** Puts reads no screen is showing into the cache. */
  function seed(): void {
    const cache = useQueryCache();
    cache.setQueryData(keys.projects, [{ id: "prj_1" }]);
    cache.setQueryData(keys.users.list, [{ id: "usr_1" }]);
  }

  const cached = () =>
    useQueryCache()
      .getEntries()
      .map((entry) => entry.key);

  it.each([
    [
      "logout",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(authApi.logout).mockResolvedValueOnce();
        await store.logout();
      },
    ],
    [
      "a logout the server rejects",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(authApi.logout).mockRejectedValueOnce(
          new ApiError(0, { NETWORK_ERROR: "down" }),
        );
        await store.logout().catch(() => undefined);
      },
    ],
    [
      "an expired session",
      async () => {
        vi.stubGlobal(
          "fetch",
          vi
            .fn()
            .mockResolvedValueOnce(
              errorResponse(401, { UNAUTHORIZED: "Sign in again." }),
            ),
        );
        await apiRequest("/api/v1/projects").catch(() => undefined);
      },
    ],
    [
      "a rejected CSRF token",
      async () => {
        vi.stubGlobal(
          "fetch",
          vi.fn().mockResolvedValueOnce(
            errorResponse(403, {
              AUTHORIZATION_DENIED: "A valid CSRF token is required.",
            }),
          ),
        );
        await apiRequest("/api/v1/projects", { method: "POST" }).catch(
          () => undefined,
        );
      },
    ],
    [
      "a password change",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(authApi.changePassword).mockResolvedValueOnce();
        await store.changePassword("pw", "newlongpassword");
      },
    ],
    [
      "clearing the local session",
      async (store: ReturnType<typeof useAuthStore>) => {
        store.clearLocalSession();
      },
    ],
  ])("empties the cache after %s", async (_, end) => {
    const store = await signedIn();
    seed();
    expect(cached()).toHaveLength(2);
    await end(store);
    expect(store.session).toBeNull();
    expect(cached()).toEqual([]);
    vi.unstubAllGlobals();
  });

  it.each([
    [
      "signing in",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(authApi.login).mockResolvedValueOnce(browserSession());
        await store.login("a@b.c", "pw");
      },
    ],
    [
      "single sign-on",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(authApi.exchangeSso).mockResolvedValueOnce({
          session: browserSession(),
          redirectTo: null,
        });
        await store.completeSso("ssoh_code");
      },
    ],
    [
      "first-run setup",
      async (store: ReturnType<typeof useAuthStore>) => {
        vi.mocked(bootstrapApi.bootstrapAdmin).mockResolvedValueOnce({
          adminId: "usr_1",
          email: "a@b.c",
          csrfToken: "csrf_1",
        });
        await store.bootstrap({
          setupToken: "dbs_setup",
          email: "a@b.c",
          password: "longenoughpw1",
        });
      },
    ],
  ])("starts %s from an empty cache", async (_, start) => {
    const store = useAuthStore();
    seed();
    await start(store);
    expect(store.isAuthenticated).toBe(true);
    expect(cached()).toEqual([]);
  });

  it("keeps the cache when the session check returns the same account", async () => {
    const store = await signedIn();
    seed();
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce(browserSessionInfo());
    await store.fetchSession();
    expect(cached()).toHaveLength(2);
  });

  it("starts over when the session check returns another account", async () => {
    const store = await signedIn();
    const query = vi
      .fn()
      .mockResolvedValueOnce(["first account's projects"])
      .mockResolvedValueOnce(["second account's projects"]);
    const { controller } = mountController(() =>
      useQuery({ key: keys.projects, query }),
    );
    await vi.waitFor(() =>
      expect(controller.data.value).toEqual(["first account's projects"]),
    );
    useQueryCache().setQueryData(keys.users.list, [{ id: "usr_1" }]);

    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      ...browserSessionInfo(),
      adminId: "usr_2",
      email: "b@b.c",
    });
    await store.fetchSession();

    expect(store.session?.adminId).toBe("usr_2");
    expect(useQueryCache().getQueryData(keys.users.list)).toBeUndefined();
    await vi.waitFor(() =>
      expect(controller.data.value).toEqual(["second account's projects"]),
    );
  });

  it("empties a read a mounted screen shows, and drops its late response", async () => {
    const store = await signedIn();
    let answerLate!: (value: string[]) => void;
    const query = vi
      .fn()
      .mockResolvedValueOnce(["root's projects"])
      .mockImplementationOnce(
        () => new Promise<string[]>((resolve) => (answerLate = resolve)),
      )
      .mockResolvedValueOnce(["next account's projects"]);
    const { controller } = mountController(() =>
      useQuery({ key: keys.projects, query }),
    );
    await vi.waitFor(() =>
      expect(controller.data.value).toEqual(["root's projects"]),
    );
    void controller.refetch();

    vi.mocked(authApi.logout).mockResolvedValueOnce();
    await store.logout();
    expect(controller.data.value).toBeUndefined();
    expect(controller.status.value).toBe("pending");

    answerLate(["root's projects, late"]);
    await settle();
    expect(controller.data.value).toBeUndefined();

    // The screen still owns a live entry: the next refresh reaches it.
    await useQueryCache().invalidateQueries({ key: keys.projects });
    expect(controller.data.value).toEqual(["next account's projects"]);
  });

  it("keeps a pre-session read the login screen still shows", async () => {
    const store = await signedIn();
    const { controller } = mountController(() =>
      useQuery({
        key: keys.public.authMethods,
        query: async () => ({ password: true }),
      }),
    );
    await vi.waitFor(() =>
      expect(controller.data.value).toEqual({ password: true }),
    );
    vi.mocked(authApi.logout).mockResolvedValueOnce();
    await store.logout();
    expect(controller.data.value).toEqual({ password: true });
  });
});
